use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use crate::prefs::Prefs;
use crate::harvester::AppState;
use walkdir::WalkDir;

// ─── Prefs ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_prefs(prefs: State<Mutex<Prefs>>) -> Prefs {
    prefs.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_prefs(prefs: State<Mutex<Prefs>>, new_prefs: Prefs) -> Result<(), String> {
    new_prefs.save().map_err(|e| e.to_string())?;
    *prefs.lock().unwrap() = new_prefs;
    Ok(())
}

#[tauri::command]
pub fn reset_settings(prefs: State<Mutex<Prefs>>) -> Result<(), String> {
    let default = Prefs::default();
    default.save().map_err(|e| e.to_string())?;
    *prefs.lock().unwrap() = default;
    Ok(())
}

// ─── Harvester ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn pause_harvesting(state: State<AppState>, app: AppHandle) {
    *state.downloading.write().unwrap() = false;
    crate::notify::notify(&app, "Harvesting paused");
}

#[tauri::command]
pub fn resume_harvesting(state: State<AppState>, app: AppHandle) {
    *state.downloading.write().unwrap() = true;
    crate::notify::notify(&app, "Harvesting resumed");
}

// ─── Changer ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn add_changer_folder(prefs: State<Mutex<Prefs>>, new_path: String) -> Result<(), String> {
    let path = PathBuf::from(&new_path);
    if !path.exists() {
        return Err(format!("Directory doesn't exist: {}", new_path));
    }
    let mut prefs = prefs.lock().unwrap();
    if !prefs.changer.folders.contains(&path) {
        prefs.changer.folders.push(path);
    }
    Ok(())
}

#[tauri::command]
pub fn remove_changer_folder(prefs: State<Mutex<Prefs>>, path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    prefs.lock().unwrap().changer.folders.retain(|p| p != &target);
    Ok(())
}

#[tauri::command]
pub fn set_random_wallpaper(prefs: State<Mutex<Prefs>>) -> Result<(), String> {
    let folders: Vec<PathBuf> = prefs.lock().unwrap().changer.folders.clone();
    if folders.is_empty() {
        return Err("No changer folders configured".into());
    }
    let paths: Vec<&std::path::Path> = folders.iter().map(|p| p.as_path()).collect();
    let next = crate::wallpaper::pick_random_wallpaper(&paths).map_err(|e| e.to_string())?;
    crate::wallpaper::set_wallpaper(&next).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_next_wallpaper(prefs: State<Mutex<Prefs>>) -> Result<(), String> {
    let prefs = prefs.lock().unwrap();
    let folders = prefs.changer.folders.clone();
    let last = prefs.changer.last_wallpaper.clone();
    let reverse = matches!(prefs.changer.mode, crate::prefs::ChangerMode::Reverse);
    drop(prefs);

    if folders.is_empty() {
        return Err("No changer folders configured".into());
    }
    let next = crate::changer::pick_sequential(&folders, &last, reverse)
        .map_err(|e| e.to_string())?;
    crate::wallpaper::set_wallpaper(&next).map_err(|e| e.to_string())
}

// ─── Wallpaper ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_wallpaper(path: String) -> Result<(), String> {
    crate::wallpaper::set_wallpaper(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn can_change_wallpaper() -> bool {
    crate::wallpaper::DesktopEnvironment::detect().is_changeable()
}

#[tauri::command]
pub fn list_images(dir: String, offset: usize, limit: usize) -> Result<Vec<String>, String> {
    let mut paths: Vec<String> = Vec::new();
    for entry in WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "bmp") {
            paths.push(entry.path().to_string_lossy().to_string());
        }
    }
    paths.sort();
    Ok(paths.into_iter().skip(offset).take(limit).collect())
}

// ─── Disk / Downloads ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_disk_usage(prefs: State<Mutex<Prefs>>) -> Result<u8, String> {
    let path = prefs.lock().unwrap().downloads.path.clone();
    if !path.exists() {
        return Ok(0);
    }

    let disks = sysinfo::Disks::new_with_refreshed_list()
        .map_err(|e| format!("Failed to get disk info: {}", e))?;

    for disk in &disks {
        if path.starts_with(disk.mount_point()) {
            let total = disk.total_space();
            let free = disk.available_space();
            if total == 0 {
                return Ok(0);
            }
            return Ok(((total - free) * 100 / total) as u8);
        }
    }

    Ok(0)
}

#[tauri::command]
pub fn change_downloads_dir(prefs: State<Mutex<Prefs>>, new_path: String, delete_old: bool) -> Result<(), String> {
    let new_path_buf = PathBuf::from(&new_path);
    if !new_path_buf.exists() {
        return Err(format!("Directory doesn't exist: {}", new_path));
    }

    let mut prefs = prefs.lock().unwrap();
    let old_path = prefs.downloads.path.clone();
    if old_path != new_path_buf {
        for entry in std::fs::read_dir(&old_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let dest = new_path_buf.join(entry.file_name());
            std::fs::rename(entry.path(), &dest).map_err(|e| e.to_string())?;
        }
        if delete_old {
            let _ = std::fs::remove_dir(&old_path);
        }
        prefs.downloads.path = new_path_buf;
    }
    prefs.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_downloads_dir(prefs: State<Mutex<Prefs>>) -> Result<(), String> {
    let path = prefs.lock().unwrap().downloads.path.clone();
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ─── Wallpapers (list) ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_recent_wallpapers(prefs: State<Mutex<Prefs>>, count: usize) -> Result<Vec<String>, String> {
    let dir = prefs.lock().unwrap().downloads.path.clone();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut entries: Vec<(PathBuf, u64)> = Vec::new();
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("wd_") {
            continue;
        }
        let mtime = entry.metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entries.push((entry.path().to_path_buf(), mtime));
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(entries.into_iter().take(count).map(|(p, _)| p.to_string_lossy().to_string()).collect())
}

#[tauri::command]
pub fn get_all_wallpapers(prefs: State<Mutex<Prefs>>, offset: usize, limit: usize) -> Result<Vec<String>, String> {
    let dir = prefs.lock().unwrap().downloads.path.clone();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut paths: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("wd_") {
            paths.push(entry.path().to_string_lossy().to_string());
        }
    }

    paths.sort();
    Ok(paths.into_iter().skip(offset).take(limit).collect())
}

#[tauri::command]
pub fn get_favorite_wallpapers(prefs: State<Mutex<Prefs>>, offset: usize, limit: usize) -> Result<Vec<String>, String> {
    let prefs = prefs.lock().unwrap();
    filter_wallpapers(&prefs, true, offset, limit)
}

#[tauri::command]
pub fn get_no_favorite_wallpapers(prefs: State<Mutex<Prefs>>, offset: usize, limit: usize) -> Result<Vec<String>, String> {
    let prefs = prefs.lock().unwrap();
    filter_wallpapers(&prefs, false, offset, limit)
}

fn filter_wallpapers(prefs: &Prefs, favorite: bool, offset: usize, limit: usize) -> Result<Vec<String>, String> {
    let dir = &prefs.downloads.path;
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut paths: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_fav = name.starts_with("wd_fav_");
        if favorite && is_fav {
            paths.push(entry.path().to_string_lossy().to_string());
        } else if !favorite && name.starts_with("wd_") && !is_fav {
            paths.push(entry.path().to_string_lossy().to_string());
        }
    }

    paths.sort();
    Ok(paths.into_iter().skip(offset).take(limit).collect())
}

#[tauri::command]
pub fn count_wallpapers(prefs: State<Mutex<Prefs>>, favorite: bool) -> Result<usize, String> {
    let dir = prefs.lock().unwrap().downloads.path.clone();
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_fav = name.starts_with("wd_fav_");
        if favorite && is_fav {
            count += 1;
        } else if !favorite && name.starts_with("wd_") && !is_fav {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn count_all_wallpapers(prefs: State<Mutex<Prefs>>) -> Result<usize, String> {
    let dir = prefs.lock().unwrap().downloads.path.clone();
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("wd_") {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn get_wallpaper_info(path: String) -> Result<WallpaperInfo, String> {
    let img = image::ImageReader::open(&path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    let filename = std::path::Path::new(&path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let provider = filename
        .strip_prefix("wd_fav_")
        .or_else(|| filename.strip_prefix("wd_"))
        .and_then(|s| {
            if s.starts_with("wf-") {
                Some("wallpaperfusion")
            } else {
                s.split('_').next()
            }
        })
        .unwrap_or("unknown");

    let provider_url = match provider {
        "wallhaven" => "https://wallhaven.cc",
        "unsplash" => "https://unsplash.com",
        "bing" => "https://www.bing.com",
        "deviantart" => "https://www.deviantart.com",
        "dualmonitor" | "dm" => "http://www.dualmonitorbackgrounds.com",
        "sw" => "http://socialwallpapering.com",
        "wallpaperfusion" => "https://www.wallpaperfusion.com",
        _ => "",
    };

    Ok(WallpaperInfo {
        path,
        width: img.width(),
        height: img.height(),
        provider: provider.to_string(),
        author: String::new(),
        author_url: String::new(),
        provider_url: provider_url.to_string(),
    })
}

#[derive(serde::Serialize)]
pub struct WallpaperInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub provider: String,
    pub author: String,
    pub author_url: String,
    pub provider_url: String,
}

// ─── Wallpaper actions ──────────────────────────────────────────────────────

#[tauri::command]
pub fn delete_wallpapers(paths: Vec<String>) -> Result<(), String> {
    for path in &paths {
        std::fs::remove_file(path).map_err(|e| format!("Failed to delete {}: {}", path, e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_favorite(paths: Vec<String>, favorite: bool) -> Result<(), String> {
    for path in &paths {
        let p = std::path::Path::new(path);
        let filename = p.file_name().unwrap().to_string_lossy().to_string();
        let new_name = if favorite {
            filename.replace("wd_", "wd_fav_")
        } else {
            filename.replace("wd_fav_", "wd_")
        };
        let new_path = p.with_file_name(&new_name);
        std::fs::rename(p, &new_path).map_err(|e| format!("Failed to rename {}: {}", path, e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn move_favorite_wallpapers(prefs: State<Mutex<Prefs>>, app: AppHandle) -> Result<(), String> {
    let dest = prefs.lock().unwrap().downloads.move_favorite_folder.clone();
    if dest.as_os_str().is_empty() {
        return Err("No move destination configured".into());
    }

    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let prefs = prefs.lock().unwrap();
    let src = prefs.downloads.path.clone();
    drop(prefs);

    for entry in std::fs::read_dir(&src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("wd_fav_") {
            let dest_path = dest.join(&name);
            std::fs::rename(entry.path(), &dest_path)
                .map_err(|e| format!("Failed to move {}: {}", name, e))?;
        }
    }

    crate::notify::notify(&app, "Favorite wallpapers moved");
    Ok(())
}

// ─── Folders ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle.dialog().file().blocking_pick_folder()
    })
        .await
        .map(|p| p.map(|p| p.to_string()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn change_move_dir(prefs: State<Mutex<Prefs>>, new_path: String) -> Result<(), String> {
    let p = PathBuf::from(&new_path);
    if !p.exists() {
        return Err(format!("Directory doesn't exist: {}", new_path));
    }
    prefs.lock().unwrap().downloads.move_favorite_folder = p;
    Ok(())
}

// ─── System ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let dest = dirs::config_dir()
            .ok_or("No config dir")?
            .join("autostart")
            .join("wallpaper-downloader.desktop");

        if enable {
            std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            std::fs::write(&dest, format!(
                "[Desktop Entry]\nType=Application\nName=WallpaperDownloader\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                exe.display()
            )).map_err(|e| e.to_string())?;
        } else {
            let _ = std::fs::remove_file(&dest);
        }
    }

    #[cfg(not(target_os = "linux"))]
    { let _ = enable; }

    Ok(())
}

#[tauri::command]
pub fn copy_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

// ─── Secondary Windows ──────────────────────────────────────────────────────

#[tauri::command]
pub fn open_manager(app: AppHandle) -> tauri::Result<()> {
    let window = tauri::WebviewWindowBuilder::new(
        &app, "manager",
        tauri::WebviewUrl::App("index.html".into())
    ).inner_size(774.0, 621.0).center().build()?;
    window.set_focus()?;
    Ok(())
}

#[tauri::command]
pub fn open_choose(app: AppHandle) -> tauri::Result<()> {
    let window = tauri::WebviewWindowBuilder::new(
        &app, "choose",
        tauri::WebviewUrl::App("index.html".into())
    ).inner_size(774.0, 546.0).center().build()?;
    window.set_focus()?;
    Ok(())
}

#[tauri::command]
pub fn open_preview(app: AppHandle, path: String) -> tauri::Result<()> {
    let encoded = urlencoding::encode(&path);
    let window = tauri::WebviewWindowBuilder::new(
        &app, "preview",
        tauri::WebviewUrl::App(format!("index.html?path={}", encoded).into())
    ).inner_size(1024.0, 768.0).center().build()?;
    window.set_focus()?;
    Ok(())
}
