use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn set_wallpaper(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        let uri = format!("file://{}", path.display());
        let gnome = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
            .status()?;
        if !gnome.success() {
            let _ = std::process::Command::new("qdbus")
                .args(["org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
                    "org.freedesktop.portal.Background", "SetForScreen",
                    "0", &uri])
                .status();
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Use SystemParametersInfo via winapi
        // For now, shell execute
        let _ = std::process::Command::new("rundll32")
            .args(["shell32.dll,Control_RunPage", "desktop"])
            .status();
        // TODO: proper SystemParametersInfoW call
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!(
                "tell application \"System Events\" to set picture of every desktop to \"{}\"", path
            )])
            .status()?;
    }

    Ok(())
}

pub fn pick_random_wallpaper(folders: &[&Path]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
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
