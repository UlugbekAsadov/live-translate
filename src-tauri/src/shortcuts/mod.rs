//! Global keyboard shortcuts. Each registered shortcut simply emits a
//! `shortcut:action` event; the main window's frontend performs the action
//! (it owns settings and knows which sources are enabled).

use std::collections::HashMap;

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::events::EVT_SHORTCUT_ACTION;

const KNOWN_ACTIONS: [&str; 5] = [
    "toggle_overlay",
    "start_stop",
    "swap_direction",
    "pause_resume",
    "clear_history",
];

/// (Re-)register all shortcuts. Returns per-action success so the Settings UI
/// can flag combinations another application already owns.
#[tauri::command]
pub fn apply_shortcuts(
    app: AppHandle,
    shortcuts: HashMap<String, String>,
) -> HashMap<String, bool> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let mut results = HashMap::new();
    for (action, accel) in shortcuts {
        if !KNOWN_ACTIONS.contains(&action.as_str()) {
            results.insert(action, false);
            continue;
        }
        let parsed: Result<Shortcut, _> = accel.parse();
        let ok = match parsed {
            Ok(shortcut) => {
                let action_name = action.clone();
                gs.on_shortcut(shortcut, move |app, _sc, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = app.emit(
                            EVT_SHORTCUT_ACTION,
                            serde_json::json!({ "action": action_name }),
                        );
                    }
                })
                .is_ok()
            }
            Err(_) => false,
        };
        if !ok {
            tracing::warn!(action = %action, accel = %accel, "failed to register shortcut");
        }
        results.insert(action, ok);
    }
    results
}
