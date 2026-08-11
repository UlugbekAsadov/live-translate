use crate::audio::devices::{list_devices, DeviceList};
use crate::error::Result;

#[tauri::command]
pub async fn list_audio_devices() -> Result<DeviceList> {
    // Device enumeration can briefly block; run it off the main thread.
    tokio::task::spawn_blocking(list_devices)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}
