use crate::AppState;
use crate::prefs::Prefs;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

pub async fn run_harvest_loop(app_handle: AppHandle) {
    loop {
        // Check if downloading is enabled
        let should_download = {
            let state: State<AppState> = app_handle.state();
            *state.downloading.read().unwrap()
        };

        if should_download {
            let prefs = {
                let state: State<Prefs> = app_handle.state();
                state.clone()
            };

            if let Err(e) = harvest_once(&app_handle, &prefs).await {
                tracing::error!("Harvest error: {}", e);
            }
        }

        // Sleep for the configured interval (check pause more frequently)
        let delay = if should_download {
            let prefs = app_handle.state::<Prefs>();
            timer_to_duration(prefs.application_timer, &prefs.application_timer_units)
        } else {
            Duration::from_secs(1)
        };
        tokio::time::sleep(delay).await;
    }
}

async fn harvest_once(app: &AppHandle, prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    // Pick an enabled provider in priority order
    if prefs.provider_wallhaven {
        crate::providers::wallhaven::harvest(prefs).await?;
    } else if prefs.provider_unsplash {
        crate::providers::unsplash::harvest(prefs).await?;
    } else if prefs.provider_bing {
        crate::providers::bing::harvest(prefs).await?;
    // } else if prefs.provider_deviantart {
    //     crate::providers::deviantart::harvest(prefs).await?;
    } else {
        tracing::debug!("No providers enabled, skipping harvest");
        return Ok(());
    }

    // Notify frontend of new wallpaper
    app.emit("wallpaper-downloaded", ()).ok();
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
