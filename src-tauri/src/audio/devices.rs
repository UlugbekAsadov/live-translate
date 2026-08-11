use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::{AppError, Result};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceList {
    /// Microphones and other capture devices.
    pub inputs: Vec<Device>,
    /// Playback devices — loopback candidates for system-audio capture.
    pub outputs: Vec<Device>,
}

/// Stable device ID string (cpal 0.18 `DeviceId`), falling back to the
/// display name if the ID is unavailable.
pub fn device_id_string(d: &cpal::Device) -> String {
    d.id()
        .map(|id| id.to_string())
        .unwrap_or_else(|_| d.to_string())
}

fn describe(d: &cpal::Device, default_id: &Option<String>) -> Device {
    let id = device_id_string(d);
    Device {
        is_default: default_id.as_deref() == Some(id.as_str()),
        name: d.to_string(),
        id,
    }
}

pub fn list_devices() -> Result<DeviceList> {
    let host = cpal::default_host();

    let default_in = host.default_input_device().map(|d| device_id_string(&d));
    let default_out = host.default_output_device().map(|d| device_id_string(&d));

    let inputs = host
        .input_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?
        .map(|d| describe(&d, &default_in))
        .collect();

    let outputs = host
        .output_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?
        .map(|d| describe(&d, &default_out))
        .collect();

    Ok(DeviceList { inputs, outputs })
}
