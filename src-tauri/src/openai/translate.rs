//! Streaming translation of finalized transcript segments via the OpenAI
//! Chat Completions API (model configurable, `gpt-4o-mini` by default).
//!
//! Queue behavior:
//! - one request in flight at a time per source;
//! - finals that pile up while a request is running are coalesced into one
//!   request when they arrived close together (rapid-fire short sentences);
//! - consecutive duplicate texts are dropped;
//! - 429/5xx are retried with backoff, then the item is dropped so the queue
//!   never stalls.

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::json;
use tauri::AppHandle;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::events;
use crate::openai::types::FinalTranscript;
use crate::state::{LangPair, Source, TranslationStyle};

const CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const COALESCE_WINDOW_MS: u64 = 2_000;
const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct TranslateParams {
    pub source: Source,
    pub api_key: String,
    pub model: String,
    pub style: TranslationStyle,
    pub lang_rx: watch::Receiver<LangPair>,
    pub final_rx: mpsc::Receiver<FinalTranscript>,
    pub cancel: CancellationToken,
}

/// English name for an ISO 639-1 code from the frontend language dropdowns.
/// Unknown codes pass through verbatim so new languages work without a
/// backend change.
pub fn lang_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "uz" => "Uzbek (Latin script)",
        "ru" => "Russian",
        "tr" => "Turkish",
        "kk" => "Kazakh",
        "ky" => "Kyrgyz",
        "tg" => "Tajik",
        "az" => "Azerbaijani",
        "ar" => "Arabic",
        "fa" => "Persian",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "hi" => "Hindi",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "uk" => "Ukrainian",
        "id" => "Indonesian",
        "vi" => "Vietnamese",
        other => other,
    }
}

pub fn build_system_prompt(pair: &LangPair, style: TranslationStyle) -> String {
    let target = lang_name(&pair.target);
    let direction_line = if pair.source == "auto" {
        format!(
            "Detect the language of the user's text and translate it into {target}. \
             If the text is already in {target}, return it unchanged."
        )
    } else {
        format!(
            "Translate the user's text from {} into {target}.",
            lang_name(&pair.source)
        )
    };

    let style_line = match style {
        TranslationStyle::Natural => {
            "Translate meaning naturally and idiomatically — never word-by-word. Prefer how a \
             native speaker would actually phrase it in a live meeting."
        }
        TranslationStyle::Literal => {
            "Stay close to the original wording while keeping the output grammatical."
        }
    };

    format!(
        "You are a professional simultaneous interpreter for live technical meetings and \
         interviews.\n\n{direction_line}\n\nRules:\n\
         - {style_line}\n\
         - Keep ALL technical terms, product names, and programming vocabulary in their \
         original (usually English) form: React, TypeScript, Next.js, API, backend, frontend, \
         deployment, Docker, Kubernetes, PostgreSQL, commit, sprint, endpoint, merge request, \
         and similar.\n\
         - Keep numbers, dates, URLs, emails, and code identifiers exactly as they are.\n\
         - Preserve the speaker's tone: a question stays a question.\n\
         - The text is live speech recognition output: it may be a fragment and may contain \
         fillers or recognition errors. Translate the most plausible intended meaning and drop \
         meaningless fillers.\n\
         - Output ONLY the translation — no commentary, no notes, no quotation marks."
    )
}

/// Merge finals that arrived within the coalescing window into one text.
/// Returns the merged transcript (attributed to the newest segment id).
pub fn coalesce(mut items: Vec<FinalTranscript>) -> FinalTranscript {
    let mut base = items.remove(0);
    for item in items {
        if item.ts.saturating_sub(base.ts) <= COALESCE_WINDOW_MS {
            base.text.push(' ');
            base.text.push_str(&item.text);
            base.segment_id = item.segment_id;
            base.ts = item.ts;
        } else {
            // Too far apart to merge — still translated together rather than
            // dropped, joined as separate sentences.
            base.text.push_str("\n");
            base.text.push_str(&item.text);
            base.segment_id = item.segment_id;
            base.ts = item.ts;
        }
    }
    base
}

/// Incremental SSE parser: feed raw bytes, get complete `data:` payloads.
pub fn parse_sse(buffer: &mut String, chunk: &str) -> Vec<String> {
    buffer.push_str(chunk);
    let mut out = Vec::new();
    while let Some(pos) = buffer.find('\n') {
        let line: String = buffer.drain(..=pos).collect();
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() {
                out.push(data.to_string());
            }
        }
    }
    out
}

pub async fn run(app: AppHandle, http: reqwest::Client, mut p: TranslateParams) {
    let mut last_translated = String::new();

    loop {
        let first = tokio::select! {
            _ = p.cancel.cancelled() => return,
            item = p.final_rx.recv() => match item {
                None => return,
                Some(i) => i,
            },
        };

        // Coalesce anything else already queued.
        let mut batch = vec![first];
        while let Ok(next) = p.final_rx.try_recv() {
            batch.push(next);
        }
        let item = coalesce(batch);

        let trimmed = item.text.trim().to_string();
        if trimmed.is_empty() || trimmed == last_translated {
            continue;
        }

        let pair = p.lang_rx.borrow().clone();
        match translate_one(&app, &http, &mut p, &item, &pair).await {
            Ok(()) => last_translated = trimmed,
            Err(TranslateError::Cancelled) => return,
            Err(TranslateError::Failed(msg)) => {
                tracing::warn!(source = %p.source, error = %msg, "translation dropped");
            }
        }
    }
}

enum TranslateError {
    Cancelled,
    Failed(String),
}

async fn translate_one(
    app: &AppHandle,
    http: &reqwest::Client,
    p: &mut TranslateParams,
    item: &FinalTranscript,
    pair: &LangPair,
) -> Result<(), TranslateError> {
    let body = json!({
        "model": p.model,
        "stream": true,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": build_system_prompt(pair, p.style) },
            { "role": "user", "content": item.text }
        ]
    });

    let mut attempt = 0u32;
    loop {
        if p.cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let response = http
            .post(CHAT_URL)
            .bearer_auth(&p.api_key)
            .json(&body)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                return stream_response(app, p, item, pair, resp).await;
            }
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    events::emit_app_error(
                        app,
                        "invalid_key",
                        "OpenAI rejected the API key during translation",
                        Some(p.source),
                        false,
                    );
                    return Err(TranslateError::Failed("invalid key".into()));
                }
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                attempt += 1;
                if attempt > MAX_RETRIES {
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        events::emit_app_error(
                            app,
                            "rate_limit",
                            "translation delayed by OpenAI rate limits; a segment was skipped",
                            Some(p.source),
                            true,
                        );
                    }
                    return Err(TranslateError::Failed(format!("HTTP {status}")));
                }
                let delay = retry_after
                    .map(Duration::from_secs)
                    .unwrap_or_else(|| Duration::from_secs(1 << (attempt - 1)));
                tokio::select! {
                    _ = p.cancel.cancelled() => return Err(TranslateError::Cancelled),
                    _ = tokio::time::sleep(delay) => {}
                }
            }
            Err(e) => {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    events::emit_app_error(
                        app,
                        "network",
                        &format!("translation request failed: {e}"),
                        Some(p.source),
                        true,
                    );
                    return Err(TranslateError::Failed(e.to_string()));
                }
                tokio::select! {
                    _ = p.cancel.cancelled() => return Err(TranslateError::Cancelled),
                    _ = tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))) => {}
                }
            }
        }
    }
}

async fn stream_response(
    app: &AppHandle,
    p: &TranslateParams,
    item: &FinalTranscript,
    pair: &LangPair,
    resp: reqwest::Response,
) -> Result<(), TranslateError> {
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();

    loop {
        let chunk = tokio::select! {
            _ = p.cancel.cancelled() => return Err(TranslateError::Cancelled),
            c = stream.next() => c,
        };
        let chunk = match chunk {
            None => break,
            Some(Ok(c)) => c,
            Some(Err(e)) => return Err(TranslateError::Failed(e.to_string())),
        };

        for data in parse_sse(&mut buffer, &String::from_utf8_lossy(&chunk)) {
            if data == "[DONE]" {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    if !delta.is_empty() {
                        full_text.push_str(delta);
                        events::emit_translation_delta(app, p.source, &item.segment_id, delta);
                    }
                }
            }
        }
    }

    let final_text = full_text.trim();
    if !final_text.is_empty() {
        events::emit_translation_final(app, p.source, &item.segment_id, final_text, &pair.target);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(source: &str, target: &str) -> LangPair {
        LangPair {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn prompt_mentions_direction_and_terms() {
        let en_uz = build_system_prompt(&pair("en", "uz"), TranslationStyle::Natural);
        assert!(en_uz.contains("from English into Uzbek (Latin script)"));
        assert!(en_uz.contains("React"));
        assert!(en_uz.contains("ONLY the translation"));

        let auto = build_system_prompt(&pair("auto", "uz"), TranslationStyle::Natural);
        assert!(auto.contains("Detect the language"));
        assert!(auto.contains("return it unchanged"));

        let literal = build_system_prompt(&pair("uz", "en"), TranslationStyle::Literal);
        assert!(literal.contains("from Uzbek (Latin script) into English"));
        assert!(literal.contains("close to the original wording"));

        // Any language pair from the dropdowns works, including new ones.
        let ru_tr = build_system_prompt(&pair("ru", "tr"), TranslationStyle::Natural);
        assert!(ru_tr.contains("from Russian into Turkish"));

        // Unknown codes pass through so the list can grow frontend-only.
        let custom = build_system_prompt(&pair("auto", "sw"), TranslationStyle::Natural);
        assert!(custom.contains("into sw"));
    }

    fn ft(id: &str, text: &str, ts: u64) -> FinalTranscript {
        FinalTranscript {
            segment_id: id.into(),
            text: text.into(),
            ts,
        }
    }

    #[test]
    fn coalesce_merges_close_finals_with_space() {
        let merged = coalesce(vec![ft("a", "Hello", 1000), ft("b", "there", 1500)]);
        assert_eq!(merged.text, "Hello there");
        assert_eq!(merged.segment_id, "b");
    }

    #[test]
    fn coalesce_separates_distant_finals_with_newline() {
        let merged = coalesce(vec![ft("a", "One", 1000), ft("b", "Two", 9000)]);
        assert_eq!(merged.text, "One\nTwo");
    }

    #[test]
    fn coalesce_single_item_is_identity() {
        let merged = coalesce(vec![ft("a", "Hi", 5)]);
        assert_eq!(merged.text, "Hi");
        assert_eq!(merged.segment_id, "a");
    }

    #[test]
    fn sse_parser_handles_lines_split_across_chunks() {
        let mut buf = String::new();
        let first = parse_sse(&mut buf, "data: {\"a\":");
        assert!(first.is_empty()); // incomplete line stays buffered

        let second = parse_sse(&mut buf, "1}\n\ndata: [DONE]\n");
        assert_eq!(second, vec!["{\"a\":1}".to_string(), "[DONE]".to_string()]);
    }

    #[test]
    fn sse_parser_ignores_comments_and_blank_lines() {
        let mut buf = String::new();
        let out = parse_sse(&mut buf, ": keep-alive\n\ndata: x\n");
        assert_eq!(out, vec!["x".to_string()]);
    }
}
