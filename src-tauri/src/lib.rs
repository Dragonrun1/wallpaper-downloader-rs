mod changer;
mod commands;
mod harvester;
mod notify;
mod prefs;
mod providers;
mod tray;
mod wallpaper;

use crate::harvester::AppState;
use std::sync::{Arc, RwLock};
use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard::init())
        .invoke_handler(tauri::generate_handler![
            // Prefs
            commands::get_prefs,
            commands::save_prefs,
            // Harvester
            commands::pause_harvesting,
            commands::resume_harvesting,
            // Wallpaper
            commands::set_wallpaper,
            commands::can_change_wallpaper,
            commands::set_random_wallpaper,
            commands::set_next_wallpaper,
            // Disk
            commands::get_disk_usage,
            // Wallpapers (list)
            commands::get_recent_wallpapers,
            commands::get_all_wallpapers,
            commands::get_favorite_wallpapers,
            commands::get_no_favorite_wallpapers,
            commands::count_wallpapers,
            commands::count_all_wallpapers,
            commands::get_wallpaper_info,
            // Wallpaper actions
            commands::delete_wallpapers,
            commands::set_favorite,
            commands::move_favorite_wallpapers,
            // Folder operations
            commands::pick_folder,
            commands::change_downloads_dir,
            commands::add_changer_folder,
            commands::remove_changer_folder,
            commands::change_move_dir,
            // System
            commands::set_autostart,
            commands::open_downloads_dir,
            commands::copy_to_clipboard,
            commands::reset_settings,
            // Windows
            commands::open_manager,
            commands::open_choose,
            commands::open_preview,
        ])
        .setup(|app| {
            // Load preferences
            let prefs = prefs::Prefs::load()
                .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!(e)))?;
            app.manage(prefs);

            // Shared state for pause/resume
            app.manage(AppState {
                downloading: Arc::new(RwLock::new(true)),
            });

            // Set up system tray
            tray::setup_tray(app)?;

            // Spawn background tasks
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn({
                let handle = app_handle.clone();
                async move { harvester::run_harvest_loop(handle).await; }
            });

            let app_handle2 = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                changer::run_changer_loop(app_handle2).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
