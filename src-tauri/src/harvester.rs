use crate::AppState;
use crate::prefs::Prefs;
use anyhow::Result;
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
            Duration::from_secs(prefs.harvester.interval_units.to_seconds(prefs.harvester.interval))
        } else {
            Duration::from_secs(1)
        };
        tokio::time::sleep(delay).await;
    }
}

async fn harvest_once(app: &AppHandle, prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    let enabled: Vec<&str> = [
        (prefs.providers.wallhaven.enabled, "wallhaven"),
        (prefs.providers.unsplash.enabled, "unsplash"),
        (prefs.providers.bing, "bing"),
        (prefs.providers.deviantart.enabled, "deviantart"),
    ].into_iter()
        .filter_map(|(on, name)| if on { Some(name) } else { None })
        .collect();

    if enabled.is_empty() {
        tracing::debug!("No providers enabled, skipping harvest");
        return Ok(());
    }

    let mut handles = Vec::new();
    for provider in &enabled {
        let prefs = prefs.clone();
        let provider = provider.to_string();
        handles.push(tokio::spawn(async move {
            let result: anyhow::Result<()> = match provider.as_str() {
                "wallhaven"  => crate::providers::wallhaven::harvest(&prefs).await,
                "unsplash"   => crate::providers::unsplash::harvest(&prefs).await,
                "bing"       => crate::providers::bing::harvest(&prefs).await,
                "deviantart" => crate::providers::deviantart::harvest(&prefs).await,
                _            => Ok(()),
            };
            (provider, result)
        }));
    }

    let mut success = 0;
    let mut failures = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((name, Ok(()))) => {
                tracing::info!("Success: {}", name);
                success += 1;
            }
            Ok((name, Err(e))) => {
                tracing::warn!("Failed: {} — {}", name, e);
                failures.push(name);
            }
            Err(e) => {
                tracing::error!("Task panicked: {}", e);
            }
        }
    }

    if success > 0 {
        app.emit("wallpaper-downloaded", success).ok();
        Ok(())
    } else {
        Err(format!("All providers failed: {:?}", failures).into())
    }
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
