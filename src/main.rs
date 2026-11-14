use slint::ComponentHandle;

mod changer;
mod config;
mod lifecycle;
mod saver;
mod screens;
mod state;
mod tray;
mod wallpaper;
mod providers;
mod library;

use config::Config;
use saver::Saver;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let tray = AppTray::new()?;

    let cfg = Config::load();
    let changer_cfg = cfg.changer.clone();
    let saver = Saver::new(&window, cfg);

    tray::setup(&tray, &window);
    let screens = screens::Screens::new(&window);
    changer::setup(&window, &tray, &saver, &screens, &changer_cfg);
    let _minimize_watcher = lifecycle::install(&window, &saver);

    window.show()?;
    tray.show()?;
    let result = slint::run_event_loop();
    saver.flush(); // covers quitting from the tray menu
    result
}
