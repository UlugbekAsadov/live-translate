use serde::Deserialize;
use tauri::{AppHandle, State};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::audio::pipeline::{self, PipelineConfig};
use crate::error::{AppError, Result};
use crate::events::{self, PipelineState};
use crate::security::keys;
use crate::state::{AppState, Direction, PipelineHandle, Source, TranslationStyle};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPipelineParams {
    pub source: Source,
    pub device_id: Option<String>,
    pub direction: Direction,
    pub stt_model: String,
    pub translation_model: String,
    pub use_server_vad: bool,
    pub translation_style: TranslationStyle,
}

#[tauri::command]
pub async fn start_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    params: StartPipelineParams,
) -> Result<()> {
    // Fail fast if there is no key — before any audio starts.
    let api_key = keys::get_api_key()?;

    let mut pipelines = state.pipelines.lock().await;
    if pipelines.contains_key(&params.source) {
        return Ok(()); // already running
    }

    let cancel = CancellationToken::new();
    let (direction_tx, direction_rx) = watch::channel(params.direction);
    let (paused_tx, paused_rx) = watch::channel(false);

    pipelines.insert(
        params.source,
        PipelineHandle {
            cancel: cancel.clone(),
            direction_tx,
            paused_tx,
        },
    );
    drop(pipelines);

    tauri::async_runtime::spawn(pipeline::run(
        app,
        PipelineConfig {
            source: params.source,
            device_id: params.device_id,
            api_key,
            stt_model: params.stt_model,
            translation_model: params.translation_model,
            use_server_vad: params.use_server_vad,
            style: params.translation_style,
        },
        cancel,
        direction_rx,
        paused_rx,
    ));

    Ok(())
}

#[tauri::command]
pub async fn stop_pipeline(state: State<'_, AppState>, source: Source) -> Result<()> {
    if let Some(handle) = state.pipelines.lock().await.remove(&source) {
        handle.cancel.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn pause_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    source: Source,
    paused: bool,
) -> Result<()> {
    let pipelines = state.pipelines.lock().await;
    let handle = pipelines
        .get(&source)
        .ok_or_else(|| AppError::Internal(format!("{source} pipeline is not running")))?;
    let _ = handle.paused_tx.send(paused);
    events::emit_status(
        &app,
        source,
        if paused {
            PipelineState::Paused
        } else {
            PipelineState::Listening
        },
        None,
    );
    Ok(())
}

#[tauri::command]
pub async fn set_direction(
    state: State<'_, AppState>,
    source: Source,
    direction: Direction,
) -> Result<()> {
    // Settings are owned by the frontend; this only hot-swaps a running
    // pipeline (updates the STT language hints and the translation prompt).
    if let Some(handle) = state.pipelines.lock().await.get(&source) {
        let _ = handle.direction_tx.send(direction);
    }
    Ok(())
}
