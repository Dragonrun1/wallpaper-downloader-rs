use tauri::AppHandle;

pub fn notify(app: &AppHandle, message: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("WallpaperDownloader")
        .body(message)
        .show();
}
