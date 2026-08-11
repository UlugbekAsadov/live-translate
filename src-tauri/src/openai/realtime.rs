//! OpenAI Realtime transcription session over WebSocket.
//!
//! One task per audio source. The local VAD gate upstream decides *what*
//! audio is sent (silence never leaves the machine); the server's turn
//! detection segments the gated stream and streams transcription deltas
//! while the speaker is still talking.
//!
//! On disconnect: exponential backoff with jitter; up to ~20 s of segment
//! audio is buffered and replayed after the session is re-established.
//! Sessions are proactively recycled after 25 minutes at a quiet moment.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::audio::vad::SegmentEvent;
use crate::events::{self, PipelineState};
use crate::openai::types::{append_payload, commit_payload, session_update_payload, FinalTranscript};
use crate::state::{Direction, Source};

// GA endpoint: no `?intent=` query and no `OpenAI-Beta` header — sending the
// beta markers now fails with `beta_api_shape_disabled`. The session becomes a
// transcription session via `session.update {type: "transcription"}`.
const REALTIME_URL: &str = "wss://api.openai.com/v1/realtime";
/// ~20 s of 24 kHz PCM16 as base64 (24k * 2 bytes * 20 * 4/3).
const REPLAY_BUDGET_BYTES: usize = 1_280_000;
const SESSION_RECYCLE: Duration = Duration::from_secs(25 * 60);
const SILENCE_TAIL_MS: usize = 500;

pub struct RealtimeParams {
    pub source: Source,
    pub api_key: String,
    pub model: String,
    pub use_server_vad: bool,
    pub direction_rx: watch::Receiver<Direction>,
    pub seg_rx: mpsc::Receiver<SegmentEvent>,
    pub final_tx: mpsc::Sender<FinalTranscript>,
    pub cancel: CancellationToken,
}

struct Backoff {
    attempt: u32,
}

impl Backoff {
    fn new() -> Self {
        Self { attempt: 0 }
    }
    fn reset(&mut self) {
        self.attempt = 0;
    }
    fn next_delay(&mut self) -> Duration {
        let base = 0.5_f64 * 2.0_f64.powi(self.attempt.min(6) as i32); // 0.5s → 32s pre-cap
        self.attempt = self.attempt.saturating_add(1);
        let capped = base.min(30.0);
        // Cheap deterministic jitter (±20%) without a rand dependency.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let jitter = 0.8 + (nanos % 400) as f64 / 1000.0;
        Duration::from_secs_f64(capped * jitter)
    }
}

fn segment_id(source: Source, item_id: &str) -> String {
    format!("{source}-{item_id}")
}

fn silence_tail_b64() -> String {
    let samples = vec![0u8; 24_000 * 2 * SILENCE_TAIL_MS / 1000];
    base64::engine::general_purpose::STANDARD.encode(samples)
}

enum SessionExit {
    /// Reconnect (network drop, server error, recycle).
    Reconnect,
    /// Stop for good (cancelled, fatal auth error, upstream closed).
    Fatal,
}

pub async fn run(app: AppHandle, mut p: RealtimeParams) {
    let mut backoff = Backoff::new();
    // Base64 audio waiting to be (re)sent after a reconnect.
    let mut replay: VecDeque<String> = VecDeque::new();
    let mut replay_bytes: usize = 0;

    loop {
        if p.cancel.is_cancelled() {
            return;
        }

        let request = match REALTIME_URL.into_client_request() {
            Ok(mut r) => {
                let auth = format!("Bearer {}", p.api_key);
                match auth.parse() {
                    Ok(v) => {
                        r.headers_mut().insert("Authorization", v);
                        r
                    }
                    Err(_) => {
                        events::emit_app_error(&app, "invalid_key", "malformed API key", Some(p.source), false);
                        return;
                    }
                }
            }
            Err(e) => {
                events::emit_app_error(&app, "internal", &e.to_string(), Some(p.source), false);
                return;
            }
        };

        match tokio_tungstenite::connect_async(request).await {
            Ok((ws, _resp)) => {
                backoff.reset();
                tracing::info!(source = %p.source, "realtime session connected");
                match run_session(&app, &mut p, ws, &mut replay, &mut replay_bytes).await {
                    SessionExit::Fatal => return,
                    SessionExit::Reconnect => {}
                }
            }
            Err(e) => {
                let msg = e.to_string();
                // 401/403 during the handshake means a bad key — do not retry.
                if msg.contains("401") || msg.contains("403") {
                    events::emit_app_error(
                        &app,
                        "invalid_key",
                        "OpenAI rejected the API key (401/403)",
                        Some(p.source),
                        false,
                    );
                    events::emit_status(&app, p.source, PipelineState::Error, None);
                    return;
                }
                tracing::warn!(source = %p.source, error = %msg, "realtime connect failed");
            }
        }

        if p.cancel.is_cancelled() {
            return;
        }
        events::emit_status(&app, p.source, PipelineState::Reconnecting, None);
        let delay = backoff.next_delay();

        // While waiting to reconnect, keep draining the segment channel into
        // the bounded replay buffer so the pipeline never backs up.
        let deadline = tokio::time::Instant::now() + delay;
        loop {
            tokio::select! {
                _ = p.cancel.cancelled() => return,
                _ = tokio::time::sleep_until(deadline) => break,
                seg = p.seg_rx.recv() => match seg {
                    None => return,
                    Some(SegmentEvent::Audio { pcm, .. }) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(pcm_bytes(&pcm));
                        push_replay(&mut replay, &mut replay_bytes, b64);
                    }
                    Some(_) => {}
                },
            }
        }
    }
}

fn pcm_bytes(pcm: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pcm.len() * 2);
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn push_replay(replay: &mut VecDeque<String>, bytes: &mut usize, b64: String) {
    *bytes += b64.len();
    replay.push_back(b64);
    while *bytes > REPLAY_BUDGET_BYTES {
        if let Some(front) = replay.pop_front() {
            *bytes -= front.len();
        } else {
            break;
        }
    }
}

async fn run_session(
    app: &AppHandle,
    p: &mut RealtimeParams,
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    replay: &mut VecDeque<String>,
    replay_bytes: &mut usize,
) -> SessionExit {
    let (mut tx, mut rx) = ws.split();

    let direction = *p.direction_rx.borrow();
    let update = session_update_payload(&p.model, direction, p.use_server_vad);
    if tx.send(Message::text(update.to_string())).await.is_err() {
        return SessionExit::Reconnect;
    }

    // Replay audio buffered while we were disconnected.
    while let Some(b64) = replay.pop_front() {
        *replay_bytes = replay_bytes.saturating_sub(b64.len());
        if tx
            .send(Message::text(append_payload(&b64).to_string()))
            .await
            .is_err()
        {
            return SessionExit::Reconnect;
        }
    }

    events::emit_status(app, p.source, PipelineState::Listening, None);

    // Cumulative partial text per server item id.
    let mut partials: HashMap<String, String> = HashMap::new();
    let mut in_speech = false;
    let mut appended_ms: u64 = 0;
    let recycle_at = tokio::time::Instant::now() + SESSION_RECYCLE;
    let mut ping = tokio::time::interval(Duration::from_secs(15));
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = p.cancel.cancelled() => {
                let _ = tx.send(Message::Close(None)).await;
                return SessionExit::Fatal;
            }

            _ = tokio::time::sleep_until(recycle_at), if !in_speech => {
                tracing::info!(source = %p.source, "proactive session recycle");
                let _ = tx.send(Message::Close(None)).await;
                return SessionExit::Reconnect;
            }

            _ = ping.tick() => {
                if tx.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return SessionExit::Reconnect;
                }
            }

            changed = p.direction_rx.changed() => {
                if changed.is_ok() {
                    let d = *p.direction_rx.borrow_and_update();
                    let update = session_update_payload(&p.model, d, p.use_server_vad);
                    if tx.send(Message::text(update.to_string())).await.is_err() {
                        return SessionExit::Reconnect;
                    }
                }
            }

            seg = p.seg_rx.recv() => match seg {
                None => {
                    let _ = tx.send(Message::Close(None)).await;
                    return SessionExit::Fatal;
                }
                Some(SegmentEvent::Start { .. }) => {
                    in_speech = true;
                    events::emit_status(app, p.source, PipelineState::Speech, None);
                }
                Some(SegmentEvent::Audio { pcm, .. }) => {
                    appended_ms += (pcm.len() as u64) * 1000 / 24_000;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(pcm_bytes(&pcm));
                    if tx.send(Message::text(append_payload(&b64).to_string())).await.is_err() {
                        push_replay(replay, replay_bytes, b64);
                        return SessionExit::Reconnect;
                    }
                }
                Some(SegmentEvent::End { .. }) => {
                    in_speech = false;
                    tracing::debug!(source = %p.source, appended_ms, "segment ended (audio sent so far)");
                    let msg = if p.use_server_vad {
                        // Push enough trailing silence for server turn
                        // detection to finalize; nothing else is sent
                        // between segments, so silence stays cheap.
                        append_payload(&silence_tail_b64()).to_string()
                    } else {
                        commit_payload().to_string()
                    };
                    if tx.send(Message::text(msg)).await.is_err() {
                        return SessionExit::Reconnect;
                    }
                    events::emit_status(app, p.source, PipelineState::Listening, None);
                }
            },

            msg = rx.next() => match msg {
                None => return SessionExit::Reconnect,
                Some(Err(e)) => {
                    tracing::warn!(source = %p.source, error = %e, "realtime read error");
                    return SessionExit::Reconnect;
                }
                Some(Ok(Message::Text(text))) => {
                    match handle_server_event(app, p, &text, &mut partials).await {
                        ServerEventOutcome::Continue => {}
                        ServerEventOutcome::FatalAuth => {
                            events::emit_status(app, p.source, PipelineState::Error, None);
                            return SessionExit::Fatal;
                        }
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    let _ = tx.send(Message::Pong(data)).await;
                }
                Some(Ok(Message::Close(_))) => return SessionExit::Reconnect,
                Some(Ok(_)) => {}
            },
        }
    }
}

enum ServerEventOutcome {
    Continue,
    FatalAuth,
}

async fn handle_server_event(
    app: &AppHandle,
    p: &RealtimeParams,
    text: &str,
    partials: &mut HashMap<String, String>,
) -> ServerEventOutcome {
    let v: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return ServerEventOutcome::Continue,
    };
    let event_type = v["type"].as_str().unwrap_or_default();

    match event_type {
        "conversation.item.input_audio_transcription.delta" => {
            let item_id = v["item_id"].as_str().unwrap_or_default();
            let delta = v["delta"].as_str().unwrap_or_default();
            if item_id.is_empty() || delta.is_empty() {
                return ServerEventOutcome::Continue;
            }
            let entry = partials.entry(item_id.to_string()).or_default();
            entry.push_str(delta);
            events::emit_transcript_partial(app, p.source, &segment_id(p.source, item_id), entry);
        }
        "conversation.item.input_audio_transcription.completed" => {
            let item_id = v["item_id"].as_str().unwrap_or_default();
            let transcript = v["transcript"].as_str().unwrap_or_default().trim();
            partials.remove(item_id);
            if transcript.is_empty() {
                return ServerEventOutcome::Continue;
            }
            let sid = segment_id(p.source, item_id);
            events::emit_transcript_final(app, p.source, &sid, transcript);
            let _ = p
                .final_tx
                .send(FinalTranscript {
                    segment_id: sid,
                    text: transcript.to_string(),
                    ts: events::now_ms(),
                })
                .await;
        }
        "error" => {
            let code = v["error"]["code"].as_str().unwrap_or_default();
            let message = v["error"]["message"].as_str().unwrap_or("realtime API error");
            tracing::warn!(source = %p.source, code, message, "realtime server error");
            if code.contains("invalid_api_key") || code.contains("auth") {
                events::emit_app_error(app, "invalid_key", message, Some(p.source), false);
                return ServerEventOutcome::FatalAuth;
            }
            // Non-fatal server errors (e.g. commit on empty buffer) are logged only.
        }
        _ => {
            tracing::trace!(source = %p.source, event_type, "unhandled realtime event");
        }
    }
    ServerEventOutcome::Continue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps_with_jitter_bounds() {
        let mut b = Backoff::new();
        let mut prev = Duration::ZERO;
        for i in 0..10 {
            let d = b.next_delay();
            // ±20% jitter around 0.5 * 2^min(i,6), capped at 30 s
            let base = (0.5 * 2.0_f64.powi(i.min(6))).min(30.0);
            assert!(d.as_secs_f64() >= base * 0.8 - 1e-9, "attempt {i}: {d:?}");
            assert!(d.as_secs_f64() <= base * 1.2 + 1e-9, "attempt {i}: {d:?}");
            if i > 0 && i < 6 {
                assert!(d.as_secs_f64() > prev.as_secs_f64() * 0.9);
            }
            prev = d;
        }
        b.reset();
        assert!(b.next_delay().as_secs_f64() <= 0.6 + 1e-9);
    }

    #[test]
    fn replay_buffer_trims_oldest_beyond_budget() {
        let mut replay = VecDeque::new();
        let mut bytes = 0usize;
        // 20 chunks of 100 kB = 2 MB, budget is ~1.28 MB
        for i in 0..20 {
            let mut chunk = "0".repeat(100_000 - 2);
            chunk.push_str(&format!("{i:0>2}"));
            push_replay(&mut replay, &mut bytes, chunk);
        }
        assert!(bytes <= REPLAY_BUDGET_BYTES);
        assert!(replay.len() < 20);
        // Oldest entries were dropped, newest kept.
        assert!(replay.back().unwrap().ends_with("19"));
    }

    #[test]
    fn pcm_bytes_is_little_endian() {
        assert_eq!(pcm_bytes(&[1, -2]), vec![1, 0, 0xFE, 0xFF]);
    }

    #[test]
    fn silence_tail_is_expected_length() {
        let b64 = silence_tail_b64();
        let decoded = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        assert_eq!(decoded.len(), 24_000 * 2 * SILENCE_TAIL_MS / 1000);
        assert!(decoded.iter().all(|&b| b == 0));
    }
}
