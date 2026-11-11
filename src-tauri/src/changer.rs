use crate::prefs::{ChangerMode, Prefs};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use walkdir::WalkDir;

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

        let delay = Duration::from_secs(
            prefs
                .changer
                .interval_units
                .to_seconds(prefs.changer.interval),
        );
        tokio::time::sleep(delay).await;
    }
}

async fn change_wallpaper(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let prefs = app.state::<Prefs>().clone();

    if prefs.changer.folders.is_empty() {
        return Ok(());
    }

    let next = match prefs.changer.mode {
        ChangerMode::Random => pick_random(&prefs.changer.folders)?,
        ChangerMode::Sequential => {
            pick_sequential(&prefs.changer.folders, &prefs.changer.last_wallpaper, false)?
        }
        ChangerMode::Reverse => {
            pick_sequential(&prefs.changer.folders, &prefs.changer.last_wallpaper, true)?
        }
    };

    crate::wallpaper::set_wallpaper(&next)?;

    // Persist resume state (only meaningful for sequential modes)
    if !matches!(prefs.changer.mode, ChangerMode::Random) {
        let prefs = app.state::<Prefs>();
        // Note: in production you'd use a Mutex/Arc<RwLock> around Prefs
        // For the skeleton, we just log — the actual persistence happens
        // via a command when the user saves prefs
        tracing::debug!("Changer: set {}", next.display());
    }

    app.emit("wallpaper-changed", next.to_string_lossy().to_string())
        .ok();
    Ok(())
}

// ─── Random Mode ────────────────────────────────────────────────────────────

pub fn pick_random(folders: &[PathBuf]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let candidates = collect_images(folders)?;
    if candidates.is_empty() {
        return Err("No wallpapers found".into());
    }
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as usize)
        % candidates.len();
    Ok(candidates[idx].clone())
}

// ─── Sequential / Reverse Mode ──────────────────────────────────────────────

/// Picks the next wallpaper in mtime order, resuming from `last_wallpaper` if provided.
/// When the queue is exhausted, resets and starts from the beginning.
pub fn pick_sequential(
    folders: &[PathBuf],
    last_wallpaper: &Option<PathBuf>,
    reverse: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut entries = collect_sorted(folders, reverse)?;

    if entries.is_empty() {
        return Err("No wallpapers found".into());
    }

    // Resume from after last_wallpaper if provided
    if let Some(last) = last_wallpaper {
        if let Some(pos) = entries.iter().position(|(p, _)| p == last) {
            entries.drain(..=pos);
        }
    }

    // Queue exhausted — start over from the beginning
    if entries.is_empty() {
        entries = collect_sorted(folders, reverse)?;
    }

    Ok(entries.remove(0).0)
}

fn collect_sorted(folders: &[PathBuf], reverse: bool) -> Result<Vec<(PathBuf, u64)>, Box<dyn std::error::Error>> {
    let mut entries: Vec<(PathBuf, u64)> = Vec::new();

    for folder in folders {
        for entry in WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path().to_path_buf();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "bmp") {
                continue;
            }
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push((path, mtime));
        }
    }

    if reverse {
        entries.sort_by(|a, b| b.1.cmp(&a.1));
    } else {
        entries.sort_by(|a, b| a.1.cmp(&b.1));
    }

    Ok(entries)
}

// ─── Shared ─────────────────────────────────────────────────────────────────

fn collect_images(folders: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    for folder in folders {
        for entry in WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "bmp") {
                candidates.push(entry.path().to_path_buf());
            }
        }
    }
    Ok(candidates)
}
