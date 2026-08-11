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

/// A translation language pair. `source` may be `"auto"` for auto-detection;
/// codes are ISO 639-1 strings chosen in the frontend language dropdowns.
/// The STT session ignores this (the live API rejects language hints for the
/// transcription model); it only shapes the translation prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LangPair {
    pub source: String,
    pub target: String,
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
    pub lang_tx: watch::Sender<LangPair>,
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
