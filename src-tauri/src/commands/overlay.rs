use tauri::{AppHandle, Emitter, LogicalSize, Manager};

use crate::error::{AppError, Result};
use crate::events::EVT_HISTORY_CLEARED;

fn overlay_window(app: &AppHandle) -> Result<tauri::WebviewWindow> {
    app.get_webview_window("overlay")
        .ok_or_else(|| AppError::Internal("overlay window not found".into()))
}

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) -> Result<bool> {
    let w = overlay_window(&app)?;
    if w.is_visible().unwrap_or(false) {
        w.hide().map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(false)
    } else {
        // Deliberately no focus: the overlay must not steal focus from Meet.
        w.show().map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(true)
    }
}

#[tauri::command]
pub fn set_overlay_mode(app: AppHandle, mode: String) -> Result<()> {
    let w = overlay_window(&app)?;
    let size = match mode.as_str() {
        "interview" => LogicalSize::new(720.0, 190.0),
        _ => LogicalSize::new(440.0, 520.0),
    };
    w.set_size(size).map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn set_overlay_click_through(app: AppHandle, enabled: bool) -> Result<()> {
    overlay_window(&app)?
        .set_ignore_cursor_events(enabled)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn clear_history(app: AppHandle) {
    // Both windows listen for this and clear their session stores.
    let _ = app.emit(EVT_HISTORY_CLEARED, serde_json::json!({}));
}
