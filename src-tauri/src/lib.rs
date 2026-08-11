mod audio;
mod commands;
mod error;
mod events;
mod openai;
mod security;
mod shortcuts;
mod state;

use tauri::Manager;

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        // single-instance must be registered first: a second launch focuses
        // the existing main window instead of starting another app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            init_tracing(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::keys::set_api_key,
            commands::keys::has_api_key,
            commands::keys::delete_api_key,
            commands::keys::test_api_key,
            commands::devices::list_audio_devices,
            commands::pipeline::start_pipeline,
            commands::pipeline::stop_pipeline,
            commands::pipeline::pause_pipeline,
            commands::pipeline::set_direction,
            commands::overlay::toggle_overlay,
            commands::overlay::set_overlay_mode,
            commands::overlay::set_overlay_click_through,
            commands::overlay::clear_history,
            shortcuts::apply_shortcuts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Console + rolling file logging. Never logs API keys, raw audio, or
/// meeting content above DEBUG level.
fn init_tracing(app: &tauri::AppHandle) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let env_filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,live_translate_lib=debug"))
    };

    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter()));

    match app.path().app_log_dir() {
        Ok(dir) => {
            let _ = std::fs::create_dir_all(&dir);
            let appender = tracing_appender::rolling::daily(dir, "live-translate.log");
            let _ = registry
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(appender)
                        .with_ansi(false)
                        .with_filter(env_filter()),
                )
                .try_init();
        }
        Err(_) => {
            let _ = registry.try_init();
        }
    }
}
