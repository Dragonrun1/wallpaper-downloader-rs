use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let open_downloads =
        MenuItem::with_id(app, "open_downloads", "Open Downloads", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause Downloading", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume Downloading", true, None::<&str>)?;
    let change_wallpaper =
        MenuItem::with_id(app, "change", "Change Wallpaper", true, None::<&str>)?;
    let next_wallpaper = MenuItem::with_id(app, "next", "Next Wallpaper", true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", "Manage Wallpapers", true, None::<&str>)?;
    let choose = MenuItem::with_id(app, "choose", "Choose Wallpaper", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &open_downloads,
            &pause,
            &resume,
            &PredefinedMenuItem::separator(app)?,
            &change_wallpaper,
            &next_wallpaper,
            &PredefinedMenuItem::separator(app)?,
            &manage,
            &choose,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("WallpaperDownloader")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "open_downloads" => {
                let _ = app.emit("tray-open-downloads", ());
            }
            "pause" => {
                let _ = app.emit("tray-pause", ());
            }
            "resume" => {
                let _ = app.emit("tray-resume", ());
            }
            "change" => {
                let _ = app.emit("tray-change-wallpaper", ());
            }
            "next" => {
                let _ = app.emit("tray-next-wallpaper", ());
            }
            "manage" => {
                let _ = app.emit("tray-manage", ());
            }
            "choose" => {
                let _ = app.emit("tray-choose", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
