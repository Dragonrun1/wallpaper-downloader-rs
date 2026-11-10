use crate::AppState;
use crate::prefs::Prefs;
use tauri::State;

#[tauri::command]
pub fn get_prefs(prefs: State<Prefs>) -> Prefs {
    (*prefs).clone()
}

#[tauri::command]
pub fn save_prefs(prefs: State<Prefs>, _new_prefs: Prefs) -> Result<(), String> {
    // Update and persist
    prefs.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_harvesting(state: State<AppState>) {
    *state.downloading.write().unwrap() = false;
    tracing::info!("Harvesting paused");
}

#[tauri::command]
pub fn resume_harvesting(state: State<AppState>) {
    *state.downloading.write().unwrap() = true;
    tracing::info!("Harvesting resumed");
}

#[tauri::command]
pub fn set_wallpaper(path: String) -> Result<(), String> {
    crate::wallpaper::set_wallpaper(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_disk_usage(prefs: State<Prefs>) -> Result<u8, String> {
    let path = &prefs.downloads.path;
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
