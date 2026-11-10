mod changer;
mod commands;
mod harvester;
mod prefs;
mod providers;
mod tray;
mod wallpaper;

use std::sync::{Arc, RwLock};
use tauri::Manager;

#[derive(Default)]
struct AppState {
    downloading: Arc<RwLock<bool>>,
}

pub fn run() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_prefs,
            commands::save_prefs,
            commands::pause_harvesting,
            commands::resume_harvesting,
            commands::set_wallpaper,
            commands::get_disk_usage,
        ])
        .setup(|app| {
            // Load preferences
            let prefs =
                prefs::Prefs::load().map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!(e)))?;
            app.manage(prefs);
            app.manage(AppState { downloading: Arc::new(RwLock::new(true)) });

            // Set up system tray
            tray::setup_tray(app)?;

            // Spawn background tasks
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn({
                let handle = app_handle.clone();
                async move {
                    harvester::run_harvest_loop(handle).await;
                }
            });

            tauri::async_runtime::spawn({
                let handle = app_handle.clone();
                async move {
                    changer::run_changer_loop(handle).await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
