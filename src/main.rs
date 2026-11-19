use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

mod background;
mod changer;
mod config;
mod gallery;
mod library;
mod lifecycle;
mod providers;
mod providers_tab;
mod saver;
mod screens;
mod state;
mod thumbs;
mod tray;
mod wallpaper;
mod wallpapers_tab;

use crate::library::Library;
use config::Config;
use saver::Saver;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let tray = AppTray::new()?;

    let cfg = Config::load();
    let changer_cfg = cfg.changer.clone();
    let providers_cfg = cfg.providers.clone();
    let saver = Saver::new(&window, cfg);

    tray::setup(&tray, &window);
    let screens = screens::Screens::new(&window);
    let engine = changer::setup(&window, &tray, &saver, &screens, &changer_cfg);
    providers_tab::setup(&window, &saver, &providers_cfg);
    let background = background::setup(&window, &tray, &saver);
    let library = background
        .as_ref()
        .map_or_else(|| Arc::new(Mutex::new(Library::load())), |b| b.library.clone());
    wallpapers_tab::setup(&window, &saver, library, engine, screens.clone());
    let _minimize_watcher = lifecycle::install(&window, &saver);

    window.show()?;
    tray.show()?;
    let result = slint::run_event_loop();
    saver.flush(); // Covers quitting from the tray menu
    result
}
