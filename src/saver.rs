use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak as RcWeak},
    time::Duration,
};

use slint::{ComponentHandle, Timer, TimerMode, Weak};

use crate::{changer, config::Config, MainWindow};

const SAVE_DELAY: Duration = Duration::from_secs(10);

/// Owns the in-memory config and writes it to disk after `SAVE_DELAY`
/// of quiet, or immediately on `flush()`.
pub struct Saver {
    window: Weak<MainWindow>,
    cfg: RefCell<Config>,
    dirty: Cell<bool>,
    timer: Timer,
    this: RcWeak<Saver>,
}

impl Saver {
    pub fn new(window: &MainWindow, cfg: Config) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            window: window.as_weak(),
            cfg: RefCell::new(cfg),
            dirty: Cell::new(false),
            timer: Timer::default(),
            this: this.clone(),
        })
    }

    /// Something changed: (re)start the countdown.
    pub fn mark_dirty(&self) {
        self.dirty.set(true);
        let this = self.this.clone();
        self.timer.start(TimerMode::SingleShot, SAVE_DELAY, move || {
            if let Some(saver) = this.upgrade() {
                saver.flush();
            }
        });
    }

    /// Save now if anything is pending.
    pub fn flush(&self) {
        if !self.dirty.replace(false) {
            return;
        }
        let Some(window) = self.window.upgrade() else { return };
        let mut cfg = self.cfg.borrow_mut();
        cfg.changer = changer::read_config(&window);
        if let Err(e) = cfg.save() {
            eprintln!("Could not save config: {e}");
            self.dirty.set(true); // retry on the next flush
        }
    }
}
