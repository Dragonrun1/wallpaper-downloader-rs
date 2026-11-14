use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use display_info::DisplayInfo;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

use crate::{wallpaper::Outcome, MainWindow, ScreenInfo};

/// Owns the screen list shown in the diagram plus the wallpaper names shown on it.
pub struct Screens {
    window: Weak<MainWindow>,
    displays: RefCell<Vec<DisplayInfo>>,
    wallpapers: RefCell<HashMap<String, String>>,
    shared: RefCell<Option<String>>,
}

impl Screens {
    pub fn new(window: &MainWindow) -> Rc<Self> {
        let this = Rc::new(Self {
            window: window.as_weak(),
            displays: RefCell::default(),
            wallpapers: RefCell::default(),
            shared: RefCell::default(),
        });
        this.refresh();

        let handler = this.clone();
        window.on_changer_refresh_screens(move || handler.refresh());
        this
    }

    /// Re-read the connected screens.
    pub fn refresh(&self) {
        *self.displays.borrow_mut() = DisplayInfo::all().unwrap_or_else(|e| {
            eprintln!("Could not read display info: {e}");
            Vec::new()
        });
        self.render();
    }

    /// Show the result of a wallpaper change on the diagram.
    pub fn set_outcome(&self, outcome: &Outcome) {
        *self.wallpapers.borrow_mut() = outcome.by_screen.clone();
        *self.shared.borrow_mut() = outcome.shared.clone();
        self.render();
    }

    fn render(&self) {
        let Some(w) = self.window.upgrade() else { return };
        let displays = self.displays.borrow();
        let wallpapers = self.wallpapers.borrow();
        let shared = self.shared.borrow();

        // Shift so the top-left of the combined layout is (0, 0)
        let min_x = displays.iter().map(|d| d.x).min().unwrap_or(0);
        let min_y = displays.iter().map(|d| d.y).min().unwrap_or(0);
        let max_x = displays.iter().map(|d| d.x + d.width as i32).max().unwrap_or(1);
        let max_y = displays.iter().map(|d| d.y + d.height as i32).max().unwrap_or(1);

        let screens: Vec<ScreenInfo> = displays
            .iter()
            .map(|d| {
                let path = wallpapers.get(&d.name).or(shared.as_ref());
                ScreenInfo {
                    name: SharedString::from(d.name.as_str()),
                    x: d.x - min_x,
                    y: d.y - min_y,
                    width: d.width as i32,
                    height: d.height as i32,
                    primary: d.is_primary,
                    wallpaper: path.map(|p| file_name(p)).unwrap_or_default().into(),
                }
            })
            .collect();

        w.set_changer_screens(ModelRc::new(VecModel::from(screens)));
        w.set_changer_screens_width((max_x - min_x).max(1));
        w.set_changer_screens_height((max_y - min_y).max(1));
    }
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
