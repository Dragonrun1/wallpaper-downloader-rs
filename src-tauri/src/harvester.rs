use crate::{Arc, RwLock};
use crate::prefs::Prefs;
use crate::providers::Provider;
use anyhow::Result;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct AppState {
    pub downloading: Arc<RwLock<bool>>,
}

pub async fn run_harvest_loop(app_handle: AppHandle) {
    loop {
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

        let delay = if should_download {
            let prefs = app_handle.state::<Prefs>();
            Duration::from_secs(
                prefs
                    .harvester
                    .interval_units
                    .to_seconds(prefs.harvester.interval),
            )
        } else {
            Duration::from_secs(1)
        };
        tokio::time::sleep(delay).await;
    }
}

async fn harvest_once(app: &AppHandle, prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    let enabled = Provider::enabled(prefs);

    if enabled.is_empty() {
        tracing::debug!("No providers enabled, skipping harvest");
        return Ok(());
    }

    let mut handles = Vec::new();
    for provider in &enabled {
        let prefs = prefs.clone();
        let provider = *provider; // Copy
        handles.push(tokio::spawn(async move {
            let result = provider.harvest(&prefs).await;
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
                failures.push(name.to_string());
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
