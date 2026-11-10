use std::path::Path;
use tauri::Emitter;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use crate::prefs::Prefs;

pub async fn run_changer_loop(app_handle: AppHandle) {
    loop {
        let prefs = {
            let state: State<Prefs> = app_handle.state();
            state.clone()
        };

        if prefs.changer.interval > 0 {
            if let Err(e) = change_wallpaper(&app_handle).await {
                tracing::error!("Changer error: {}", e);
            }
        }

        let unit= &prefs.changer.interval_units;
        let delay = Duration::from_secs(unit.to_seconds(prefs.changer.interval));
        tokio::time::sleep(delay).await;
    }
}

async fn change_wallpaper(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let prefs = app.state::<Prefs>();

    if prefs.changer.folders.is_empty() {
        return Ok(());
    }

    let folders: Vec<&Path> = prefs.changer.folders.iter().map(|p| p.as_path()).collect();
    let next = crate::wallpaper::pick_random_wallpaper(&folders)?;
    crate::wallpaper::set_wallpaper(&next)?;

    app.emit("wallpaper-changed", next.to_string_lossy().to_string()).ok();
    Ok(())
}
