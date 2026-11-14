use display_info::DisplayInfo;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{MainWindow, ScreenInfo};

pub fn setup(window: &MainWindow) {
    refresh(window);

    let weak = window.as_weak();
    window.on_changer_refresh_screens(move || {
        if let Some(w) = weak.upgrade() {
            refresh(&w);
        }
    });
}

fn refresh(w: &MainWindow) {
    let displays = DisplayInfo::all().unwrap_or_else(|e| {
        eprintln!("Could not read display info: {e}");
        Vec::new()
    });

    // Shift so the top-left of the combined layout is (0, 0)
    let min_x = displays.iter().map(|d| d.x).min().unwrap_or(0);
    let min_y = displays.iter().map(|d| d.y).min().unwrap_or(0);
    let max_x = displays.iter().map(|d| d.x + d.width as i32).max().unwrap_or(1);
    let max_y = displays.iter().map(|d| d.y + d.height as i32).max().unwrap_or(1);

    let screens: Vec<ScreenInfo> = displays
        .iter()
        .map(|d| ScreenInfo {
            name: SharedString::from(d.name.as_str()),
            x: d.x - min_x,
            y: d.y - min_y,
            width: d.width as i32,
            height: d.height as i32,
            primary: d.is_primary,
        })
        .collect();

    w.set_changer_screens(ModelRc::new(VecModel::from(screens)));
    w.set_changer_screens_width((max_x - min_x).max(1));
    w.set_changer_screens_height((max_y - min_y).max(1));
}
