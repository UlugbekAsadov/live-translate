//! cpal audio capture.
//!
//! - Microphone: a regular input stream on an input device.
//! - System audio: WASAPI loopback — on Windows, cpal opens an OUTPUT device
//!   with `build_input_stream`, which sets `AUDCLNT_STREAMFLAGS_LOOPBACK`.
//!
//! cpal's `Stream` is `!Send`, so each capture owns a dedicated OS thread:
//! the stream is created, played, and dropped on that thread. Only channel
//! senders cross into async land. The audio callback runs on the OS audio
//! thread and must never block: it only does a `try_send`; if the channel is
//! full the chunk is dropped and counted.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample as _, SampleFormat, SizedSample};
use tokio::sync::mpsc;

use crate::audio::devices::device_id_string;
use crate::state::Source;

#[derive(Clone, Copy, Debug)]
pub struct AudioMeta {
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct CaptureHandle {
    /// Sending (or dropping) this stops the capture thread.
    pub shutdown_tx: std_mpsc::Sender<()>,
    #[allow(dead_code)] // kept so the thread can be joined in future teardown logic
    pub join: std::thread::JoinHandle<()>,
    pub dropped_chunks: Arc<AtomicU64>,
}

fn resolve_device(
    host: &cpal::Host,
    source: Source,
    device_id: &Option<String>,
) -> Result<cpal::Device, String> {
    let (devices, default) = match source {
        Source::Mic => (
            host.input_devices().map_err(|e| e.to_string())?,
            host.default_input_device(),
        ),
        Source::System => (
            host.output_devices().map_err(|e| e.to_string())?,
            host.default_output_device(),
        ),
    };

    match device_id {
        Some(id) => {
            for d in devices {
                // Match by stable device ID, falling back to display name.
                if &device_id_string(&d) == id || &d.to_string() == id {
                    return Ok(d);
                }
            }
            Err(format!("audio device not found: {id}"))
        }
        None => default.ok_or_else(|| "no default audio device".to_string()),
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio_tx: mpsc::Sender<Vec<f32>>,
    err_tx: mpsc::Sender<String>,
    dropped: Arc<AtomicU64>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config.clone(),
            move |data: &[T], _| {
                let chunk: Vec<f32> = data.iter().map(|s| f32::from_sample(*s)).collect();
                if audio_tx.try_send(chunk).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            },
            move |e| {
                let _ = err_tx.try_send(e.to_string());
            },
            None,
        )
        .map_err(|e| e.to_string())
}

/// Spawn the capture thread. The actual device format is reported through
/// `meta_tx` once the stream is up (or an error string if it failed).
pub fn spawn_capture(
    source: Source,
    device_id: Option<String>,
    audio_tx: mpsc::Sender<Vec<f32>>,
    meta_tx: std_mpsc::Sender<Result<AudioMeta, String>>,
    err_tx: mpsc::Sender<String>,
) -> CaptureHandle {
    let (shutdown_tx, shutdown_rx) = std_mpsc::channel::<()>();
    let dropped_chunks = Arc::new(AtomicU64::new(0));
    let dropped = dropped_chunks.clone();

    let join = std::thread::Builder::new()
        .name(format!("capture-{source}"))
        .spawn(move || {
            let host = cpal::default_host();
            let device = match resolve_device(&host, source, &device_id) {
                Ok(d) => d,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            // Loopback capture uses the device's OUTPUT (mix) format.
            let supported = match source {
                Source::Mic => device.default_input_config(),
                Source::System => device.default_output_config(),
            };
            let supported = match supported {
                Ok(c) => c,
                Err(e) => {
                    let _ = meta_tx.send(Err(e.to_string()));
                    return;
                }
            };

            let sample_format = supported.sample_format();
            let config: cpal::StreamConfig = supported.into();
            let meta = AudioMeta {
                sample_rate: config.sample_rate,
                channels: config.channels,
            };

            let stream = match sample_format {
                SampleFormat::F32 => {
                    build_stream::<f32>(&device, &config, audio_tx, err_tx, dropped)
                }
                SampleFormat::I16 => {
                    build_stream::<i16>(&device, &config, audio_tx, err_tx, dropped)
                }
                SampleFormat::U16 => {
                    build_stream::<u16>(&device, &config, audio_tx, err_tx, dropped)
                }
                SampleFormat::I32 => {
                    build_stream::<i32>(&device, &config, audio_tx, err_tx, dropped)
                }
                other => Err(format!("unsupported sample format: {other:?}")),
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    let _ = meta_tx.send(Err(e));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = meta_tx.send(Err(e.to_string()));
                return;
            }
            let _ = meta_tx.send(Ok(meta));
            tracing::info!(%source, rate = meta.sample_rate, channels = meta.channels, "capture started");

            // Park until the pipeline drops `shutdown_tx` (or sends a value).
            let _ = shutdown_rx.recv();
            drop(stream);
            tracing::info!(%source, "capture stopped");
        })
        .expect("failed to spawn capture thread");

    CaptureHandle {
        shutdown_tx,
        join,
        dropped_chunks,
    }
}
