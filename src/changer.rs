use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel, Weak};

use crate::{
    config::{ChangeOrder, ChangerConfig, IntervalUnit, MonitorMode},
    saver::Saver,
    screens::Screens,
    wallpaper::Engine,
    AppTray, MainWindow,
};

/// Wire up everything for the Changer tab.
pub fn setup(
    window: &MainWindow,
    tray: &AppTray,
    saver: &Rc<Saver>,
    screens: &Rc<Screens>,
    initial: &ChangerConfig,
) {
    let engine = Rc::new(Engine::new());
    window.set_changer_per_screen_supported(engine.per_screen_supported());

    let dirs = apply_config(window, initial);
    wire_directories(window, &dirs, saver);
    wire_schedule(window, saver, &engine, screens);
    wire_change_now(window, tray, &engine, screens);
}

/// UI state -> config struct.
pub fn read_config(w: &MainWindow) -> ChangerConfig {
    ChangerConfig {
        enabled: w.get_changer_enabled(),
        interval: w.get_changer_interval().max(1) as u32,
        interval_unit: IntervalUnit::from_index(w.get_changer_interval_unit()),
        order: ChangeOrder::from_index(w.get_changer_order()),
        directories: w.get_changer_directories().iter().map(|d| d.to_string()).collect(),
        monitor_mode: MonitorMode::from_index(w.get_changer_monitor_mode()),
    }
}

/// Config struct -> UI state. Returns the directory model so Rust can edit it.
fn apply_config(w: &MainWindow, c: &ChangerConfig) -> Rc<VecModel<SharedString>> {
    w.set_changer_enabled(c.enabled);
    w.set_changer_interval(c.interval.clamp(1, 9999) as i32);
    w.set_changer_interval_unit(c.interval_unit.index());
    w.set_changer_order(c.order.index());
    w.set_changer_monitor_mode(c.monitor_mode.index());

    let dirs = Rc::new(VecModel::from(
        c.directories.iter().map(|d| SharedString::from(d.as_str())).collect::<Vec<_>>(),
    ));
    w.set_changer_directories(ModelRc::from(dirs.clone()));
    dirs
}

fn pick_folder() -> Option<SharedString> {
    let path = rfd::FileDialog::new().pick_folder()?;
    Some(path.to_string_lossy().as_ref().into())
}

fn wire_directories(w: &MainWindow, dirs: &Rc<VecModel<SharedString>>, saver: &Rc<Saver>) {
    w.on_changer_add_directory({
        let (dirs, saver) = (dirs.clone(), saver.clone());
        move || {
            if let Some(s) = pick_folder() {
                if !dirs.iter().any(|d| d == s) {
                    dirs.push(s);
                    saver.mark_dirty();
                }
            }
        }
    });
    w.on_changer_browse_directory({
        let (dirs, saver) = (dirs.clone(), saver.clone());
        move |i| {
            if let Some(s) = pick_folder() {
                dirs.set_row_data(i as usize, s);
                saver.mark_dirty();
            }
        }
    });
    w.on_changer_remove_directory({
        let (dirs, saver) = (dirs.clone(), saver.clone());
        move |i| {
            dirs.remove(i as usize);
            saver.mark_dirty();
        }
    });
}

fn wire_schedule(
    window: &MainWindow,
    saver: &Rc<Saver>,
    engine: &Rc<Engine>,
    screens: &Rc<Screens>,
) {
    let timer = Timer::default();
    reschedule(window, &timer, engine, screens); // apply the loaded settings

    let weak = window.as_weak();
    let (saver, engine, screens) = (saver.clone(), engine.clone(), screens.clone());
    window.on_changer_settings_changed(move || {
        if let Some(w) = weak.upgrade() {
            reschedule(&w, &timer, &engine, &screens); // takes effect immediately
        }
        saver.mark_dirty();
    });
}

fn reschedule(window: &MainWindow, timer: &Timer, engine: &Rc<Engine>, screens: &Rc<Screens>) {
    timer.stop();
    let cfg = read_config(window);
    if !cfg.enabled {
        return;
    }
    let weak = window.as_weak();
    let (engine, screens) = (engine.clone(), screens.clone());
    timer.start(TimerMode::Repeated, cfg.interval_duration(), move || {
        if let Some(w) = weak.upgrade() {
            change_wallpaper(&w, &engine, &screens);
        }
    });
}

fn wire_change_now(
    window: &MainWindow,
    tray: &AppTray,
    engine: &Rc<Engine>,
    screens: &Rc<Screens>,
) {
    window.on_changer_change_now(change_now_handler(window.as_weak(), engine.clone(), screens.clone()));
    tray.on_change_now(change_now_handler(window.as_weak(), engine.clone(), screens.clone()));
}

fn change_now_handler(
    weak: Weak<MainWindow>,
    engine: Rc<Engine>,
    screens: Rc<Screens>,
) -> impl Fn() + 'static {
    move || {
        if let Some(w) = weak.upgrade() {
            change_wallpaper(&w, &engine, &screens);
        }
    }
}

fn change_wallpaper(w: &MainWindow, engine: &Engine, screens: &Screens) {
    match engine.change(&read_config(w)) {
        Ok(outcome) => {
            w.set_changer_status("".into());
            screens.set_outcome(&outcome);
        }
        Err(e) => w.set_changer_status(format!("Error: {e}").into()),
    }
}
