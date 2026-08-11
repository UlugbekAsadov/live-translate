//! Per-source pipeline task graph:
//!
//! capture thread → bounded channel → this task (downmix, resample, VAD gate)
//!   → realtime STT task → translate task
//!
//! All child tasks share one `CancellationToken`; `stop_pipeline` cancels it
//! and everything unwinds. If the capture device dies mid-stream, capture is
//! respawned up to 3 times (falling back to the default device on the last
//! attempt) before giving up with a `device_lost` error.

use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::audio::capture::{self, AudioMeta, CaptureHandle};
use crate::audio::resample::{downmix, rms, to_i16, LinearResampler};
use crate::audio::vad::{EnergyVad, SegmentEvent, SileroVad, VadGate, VAD_SAMPLE_RATE};
use crate::events::{self, PipelineState};
use crate::openai::realtime::{self, RealtimeParams};
use crate::openai::translate::{self, TranslateParams};
use crate::openai::types::FinalTranscript;
use crate::state::{AppState, LangPair, Source, TranslationStyle};

const OPENAI_SAMPLE_RATE: u32 = 24_000;
const CAPTURE_RETRIES: u32 = 3;
const LEVEL_INTERVAL: Duration = Duration::from_millis(100);

pub struct PipelineConfig {
    pub source: Source,
    pub device_id: Option<String>,
    pub api_key: String,
    pub stt_model: String,
    pub translation_model: String,
    pub use_server_vad: bool,
    pub style: TranslationStyle,
}

struct ActiveCapture {
    handle: CaptureHandle,
    audio_rx: mpsc::Receiver<Vec<f32>>,
    err_rx: mpsc::Receiver<String>,
    meta: AudioMeta,
}

fn start_capture(source: Source, device_id: Option<String>) -> Result<ActiveCapture, String> {
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>(64);
    let (err_tx, err_rx) = mpsc::channel::<String>(8);
    let (meta_tx, meta_rx) = std_mpsc::channel();
    let handle = capture::spawn_capture(source, device_id, audio_tx, meta_tx, err_tx);
    match meta_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(meta)) => Ok(ActiveCapture {
            handle,
            audio_rx,
            err_rx,
            meta,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("timed out waiting for the audio device".into()),
    }
}

fn make_gate(source: Source) -> VadGate {
    match SileroVad::new() {
        Ok(engine) => VadGate::new(Box::new(engine), source),
        Err(e) => {
            tracing::warn!(error = %e, "Silero VAD unavailable, falling back to energy gate");
            VadGate::new(Box::new(EnergyVad { threshold: 0.015 }), source)
        }
    }
}

pub async fn run(
    app: AppHandle,
    cfg: PipelineConfig,
    cancel: CancellationToken,
    lang_rx: watch::Receiver<LangPair>,
    paused_rx: watch::Receiver<bool>,
) {
    let source = cfg.source;
    events::emit_status(&app, source, PipelineState::Starting, None);

    // spawn_blocking: start_capture blocks up to 5 s waiting for the device.
    let device_id = cfg.device_id.clone();
    let capture_result =
        tokio::task::spawn_blocking(move || start_capture(source, device_id)).await;
    let mut active = match capture_result {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            events::emit_app_error(&app, "no_device", &e, Some(source), false);
            events::emit_status(&app, source, PipelineState::Error, Some(e));
            cleanup(&app, source).await;
            return;
        }
        Err(e) => {
            events::emit_status(&app, source, PipelineState::Error, Some(e.to_string()));
            cleanup(&app, source).await;
            return;
        }
    };

    // Child tasks.
    let (seg_tx, seg_rx) = mpsc::channel::<SegmentEvent>(256);
    let (final_tx, final_rx) = mpsc::channel::<FinalTranscript>(64);

    let ws_task = tauri::async_runtime::spawn(realtime::run(
        app.clone(),
        RealtimeParams {
            source,
            api_key: cfg.api_key.clone(),
            model: cfg.stt_model.clone(),
            use_server_vad: cfg.use_server_vad,
            seg_rx,
            final_tx,
            cancel: cancel.clone(),
        },
    ));

    let http = app.state::<AppState>().http.clone();
    let translate_task = tauri::async_runtime::spawn(translate::run(
        app.clone(),
        http,
        TranslateParams {
            source,
            api_key: cfg.api_key,
            model: cfg.translation_model,
            style: cfg.style,
            lang_rx,
            final_rx,
            cancel: cancel.clone(),
        },
    ));

    // Processing state, rebuilt if the capture device changes.
    let mut r16 = LinearResampler::new(active.meta.sample_rate, VAD_SAMPLE_RATE);
    let mut r24 = LinearResampler::new(active.meta.sample_rate, OPENAI_SAMPLE_RATE);
    let mut gate = make_gate(source);
    let mut last_level = Instant::now() - LEVEL_INTERVAL;
    let mut capture_retries = 0u32;

    'main: loop {
        tokio::select! {
            _ = cancel.cancelled() => break 'main,

            err = active.err_rx.recv() => {
                let msg = err.unwrap_or_else(|| "audio stream closed".into());
                tracing::warn!(%source, error = %msg, "capture error");
                // Tear down and retry, falling back to the default device last.
                let _ = active.handle.shutdown_tx.send(());
                capture_retries += 1;
                if capture_retries > CAPTURE_RETRIES {
                    events::emit_app_error(&app, "device_lost", &msg, Some(source), false);
                    events::emit_status(&app, source, PipelineState::Error, Some(msg));
                    break 'main;
                }
                events::emit_status(&app, source, PipelineState::Reconnecting,
                    Some("audio device lost, retrying".into()));
                tokio::time::sleep(Duration::from_secs(2)).await;
                let retry_device = if capture_retries == CAPTURE_RETRIES {
                    None // last attempt: whatever the current default is
                } else {
                    cfg.device_id.clone()
                };
                let respawn = tokio::task::spawn_blocking(move || {
                    start_capture(source, retry_device)
                }).await;
                match respawn {
                    Ok(Ok(a)) => {
                        r16 = LinearResampler::new(a.meta.sample_rate, VAD_SAMPLE_RATE);
                        r24 = LinearResampler::new(a.meta.sample_rate, OPENAI_SAMPLE_RATE);
                        active = a;
                        events::emit_status(&app, source, PipelineState::Listening, None);
                    }
                    _ => continue 'main, // err_rx now closed; next recv triggers another retry
                }
            }

            chunk = active.audio_rx.recv() => {
                let Some(chunk) = chunk else { continue 'main };
                let mono = downmix(&chunk, active.meta.channels);

                if last_level.elapsed() >= LEVEL_INTERVAL {
                    last_level = Instant::now();
                    events::emit_audio_level(&app, source, rms(&mono));
                }

                if *paused_rx.borrow() {
                    continue 'main;
                }

                let s16 = r16.process(&mono);
                let s24 = to_i16(&r24.process(&mono));
                for event in gate.push(&s16, &s24) {
                    if seg_tx.send(event).await.is_err() {
                        break 'main; // STT task is gone
                    }
                }
            }
        }
    }

    // Shutdown: stop capture thread, cancel children, wait briefly.
    let dropped = active
        .handle
        .dropped_chunks
        .load(std::sync::atomic::Ordering::Relaxed);
    if dropped > 0 {
        tracing::warn!(%source, dropped, "audio chunks dropped due to backpressure");
    }
    let _ = active.handle.shutdown_tx.send(());
    drop(seg_tx);
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        let _ = ws_task.await;
        let _ = translate_task.await;
    })
    .await;

    cleanup(&app, source).await;
    events::emit_status(&app, source, PipelineState::Idle, None);
    tracing::info!(%source, "pipeline stopped");
}

/// Remove this pipeline's handle from shared state so it can be restarted.
async fn cleanup(app: &AppHandle, source: Source) {
    let state = app.state::<AppState>();
    state.pipelines.lock().await.remove(&source);
}
