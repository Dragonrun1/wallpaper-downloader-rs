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

        if prefs.application_changer > 0 {
            if let Err(e) = change_wallpaper(&app_handle).await {
                tracing::error!("Changer error: {}", e);
            }
        }

        let delay = timer_to_duration(prefs.application_changer, &prefs.application_changer_units);
        tokio::time::sleep(delay).await;
    }
}

async fn change_wallpaper(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let prefs = app.state::<Prefs>();

    // Get next wallpaper from changer folders
    let folders: Vec<&str> = prefs.application_changer_folder.split(';').filter(|s| !s.is_empty()).collect();
    if folders.is_empty() {
        return Ok(());
    }

    // Pick a random file from the folders
    let next = crate::wallpaper::pick_random_wallpaper(&folders)?;
    crate::wallpaper::set_wallpaper(&next)?;

    app.emit("wallpaper-changed", next.to_string_lossy().to_string()).ok();
    Ok(())
}

fn timer_to_duration(value: u32, units: &str) -> Duration {
    let secs = match units {
        "s" => value as u64,
        "m" => value as u64 * 60,
        "h" => value as u64 * 3600,
        _ => value as u64 * 60,
    };
    Duration::from_secs(secs)
}
