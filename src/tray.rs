use slint::{ComponentHandle, Weak};

use crate::{AppTray, MainWindow};

pub fn setup(tray: &AppTray, window: &MainWindow) {
    tray.set_icon_size(if cfg!(target_os = "windows") { 32 } else { 64 });

    let weak = window.as_weak();
    tray.on_tray_clicked({
        let weak = weak.clone();
        move || show_window(&weak, -1)
    });
    tray.on_open_tab(move |tab| show_window(&weak, tab));
    tray.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
}

/// Show the window, optionally switching tab (-1 keeps the current tab).
fn show_window(weak: &Weak<MainWindow>, tab: i32) {
    if let Some(w) = weak.upgrade() {
        if tab >= 0 {
            w.set_current_tab(tab);
        }
        let _ = w.show();
    }
}
