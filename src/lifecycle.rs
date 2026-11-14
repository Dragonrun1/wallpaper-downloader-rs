use std::{rc::Rc, time::Duration};

use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};

use crate::{saver::Saver, MainWindow};

/// Flush pending changes on window close, tab switch and minimize.
/// The returned timer polls for minimize, so keep it alive.
pub fn install(window: &MainWindow, saver: &Rc<Saver>) -> Timer {
    window.window().on_close_requested({
        let saver = saver.clone();
        move || {
            saver.flush();
            CloseRequestResponse::HideWindow
        }
    });
    window.on_tab_changed({
        let saver = saver.clone();
        move || saver.flush()
    });
    watch_minimize(window, saver)
}

fn watch_minimize(window: &MainWindow, saver: &Rc<Saver>) -> Timer {
    let timer = Timer::default();
    let weak = window.as_weak();
    let saver = saver.clone();
    let mut was_minimized = false;
    timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
        let Some(w) = weak.upgrade() else { return };
        let now = w.window().is_minimized();
        if now && !was_minimized {
            saver.flush();
        }
        was_minimized = now;
    });
    timer
}
