use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    System,
    Mic,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::System => write!(f, "system"),
            Source::Mic => write!(f, "mic"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    EnUz,
    UzEn,
    AutoUz,
    AutoEn,
}

impl Direction {
    /// ISO 639-1 hints for the transcription session.
    ///
    /// `gpt-live-transcribe` rejects `uz` as a language hint (it is not in
    /// the supported-hint list), so any direction that may carry Uzbek audio
    /// sends no hint at all and lets the model detect the language itself.
    pub fn stt_languages(&self) -> Vec<&'static str> {
        match self {
            Direction::EnUz => vec!["en"],
            Direction::UzEn | Direction::AutoUz | Direction::AutoEn => vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationStyle {
    Natural,
    Literal,
}

/// Control handles for one running per-source pipeline.
pub struct PipelineHandle {
    pub cancel: CancellationToken,
    pub direction_tx: watch::Sender<Direction>,
    pub paused_tx: watch::Sender<bool>,
}

pub struct AppState {
    pub pipelines: Mutex<HashMap<Source, PipelineHandle>>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pipelines: Mutex::new(HashMap::new()),
            http: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
        }
    }
}
