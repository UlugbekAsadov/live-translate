//! Rust → frontend event contract. Mirrored by `src/types/ipc.ts`.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::Source;

pub const EVT_TRANSCRIPT_PARTIAL: &str = "transcript:partial";
pub const EVT_TRANSCRIPT_FINAL: &str = "transcript:final";
pub const EVT_TRANSLATION_DELTA: &str = "translation:delta";
pub const EVT_TRANSLATION_FINAL: &str = "translation:final";
pub const EVT_PIPELINE_STATUS: &str = "pipeline:status";
pub const EVT_AUDIO_LEVEL: &str = "audio:level";
pub const EVT_APP_ERROR: &str = "app:error";
pub const EVT_HISTORY_CLEARED: &str = "history:cleared";
pub const EVT_SHORTCUT_ACTION: &str = "shortcut:action";

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPayload {
    pub source: Source,
    pub segment_id: String,
    pub text: String,
    pub ts: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationDeltaPayload {
    pub source: Source,
    pub segment_id: String,
    pub delta: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFinalPayload {
    pub source: Source,
    pub segment_id: String,
    pub text: String,
    pub target_lang: String,
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    Idle,
    Starting,
    Listening,
    Speech,
    Paused,
    Reconnecting,
    Error,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStatusPayload {
    pub source: Source,
    pub state: PipelineState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelPayload {
    pub source: Source,
    pub rms: f32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub recoverable: bool,
}

pub fn emit_transcript_partial(app: &AppHandle, source: Source, segment_id: &str, text: &str) {
    let _ = app.emit(
        EVT_TRANSCRIPT_PARTIAL,
        TranscriptPayload {
            source,
            segment_id: segment_id.to_string(),
            text: text.to_string(),
            ts: now_ms(),
        },
    );
}

pub fn emit_transcript_final(app: &AppHandle, source: Source, segment_id: &str, text: &str) {
    let _ = app.emit(
        EVT_TRANSCRIPT_FINAL,
        TranscriptPayload {
            source,
            segment_id: segment_id.to_string(),
            text: text.to_string(),
            ts: now_ms(),
        },
    );
}

pub fn emit_translation_delta(app: &AppHandle, source: Source, segment_id: &str, delta: &str) {
    let _ = app.emit(
        EVT_TRANSLATION_DELTA,
        TranslationDeltaPayload {
            source,
            segment_id: segment_id.to_string(),
            delta: delta.to_string(),
        },
    );
}

pub fn emit_translation_final(
    app: &AppHandle,
    source: Source,
    segment_id: &str,
    text: &str,
    target_lang: &str,
) {
    let _ = app.emit(
        EVT_TRANSLATION_FINAL,
        TranslationFinalPayload {
            source,
            segment_id: segment_id.to_string(),
            text: text.to_string(),
            target_lang: target_lang.to_string(),
        },
    );
}

pub fn emit_status(app: &AppHandle, source: Source, state: PipelineState, detail: Option<String>) {
    let _ = app.emit(
        EVT_PIPELINE_STATUS,
        PipelineStatusPayload {
            source,
            state,
            detail,
        },
    );
}

pub fn emit_audio_level(app: &AppHandle, source: Source, rms: f32) {
    let _ = app.emit(EVT_AUDIO_LEVEL, AudioLevelPayload { source, rms });
}

pub fn emit_app_error(
    app: &AppHandle,
    code: &str,
    message: &str,
    source: Option<Source>,
    recoverable: bool,
) {
    let _ = app.emit(
        EVT_APP_ERROR,
        AppErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            source,
            recoverable,
        },
    );
}
