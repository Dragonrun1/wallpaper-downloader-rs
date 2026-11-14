use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, Timer, TimerMode, Weak};

use crate::{
    providers::{
        schedule::{fmt_duration, Trigger},
        service::{Service, Start},
    },
    saver::Saver,
    AppTray, MainWindow, ProvidersState,
};
use crate::providers_tab::refresh_status;

/// Keep this alive for as long as the app runs; it owns the schedule timer.
pub struct Background {
    _timer: Timer,
}

/// Starts the provider service: checks whatever is due right away and then
/// once a minute, wires the "Download now" actions (tray and tab), and keeps
/// the tab's status lines fresh.
pub fn setup(window: &MainWindow, tray: &AppTray, saver: &Rc<Saver>) -> Option<Background> {
    let cfg = saver.providers();
    let service = match Service::new(&cfg) {
        Ok(service) => Rc::new(service),
        Err(e) => {
            eprintln!("Providers: could not start the background service: {e:#}");
            return None;
        }
    };

    if cfg.enabled().is_empty() {
        eprintln!("Providers: none enabled");
    }
    for line in service.status_lines(&cfg) {
        eprintln!("Providers: {line}");
    }

    let weak = window.as_weak();
    let manual: Rc<dyn Fn()> = {
        let (weak, service, saver) = (weak.clone(), service.clone(), saver.clone());
        Rc::new(move || run(&weak, &service, &saver, Trigger::Manual))
    };
    tray.on_download_now({
        let manual = manual.clone();
        move || manual()
    });
    window.global::<ProvidersState>().on_download_now(move || manual());

    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(60), {
        let (weak, service, saver) = (weak.clone(), service.clone(), saver.clone());
        move || {
            run(&weak, &service, &saver, Trigger::Scheduled);
            refresh(&weak, &service, &saver);
        }
    });
    run(&weak, &service, saver, Trigger::Scheduled);
    refresh(&weak, &service, saver);

    Some(Background { _timer: timer })
}

fn refresh(weak: &Weak<MainWindow>, service: &Service, saver: &Saver) {
    if let Some(window) = weak.upgrade() {
        refresh_status(&window, &service.runs().lock().unwrap(), &saver.providers());
    }
}

fn run(weak: &Weak<MainWindow>, service: &Service, saver: &Saver, trigger: Trigger) {
    let manual = trigger == Trigger::Manual;
    let cfg = saver.providers();

    // What the background thread needs to report back to the UI
    let (done_weak, done_cfg, runs) = (weak.clone(), cfg.clone(), service.runs());
    let started = service.start(&cfg, trigger, move |report| {
        let lines = report.lines();
        for line in &lines {
            eprintln!("Providers: {line}");
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = done_weak.upgrade() {
                let state = window.global::<ProvidersState>();
                state.set_busy(false);
                state.set_run_status(lines.join("\n").into());
                refresh_status(&window, &runs.lock().unwrap(), &done_cfg);
            }
        });
    });

    match started {
        Start::Started(ids) => {
            let names: Vec<_> = ids.iter().map(|id| id.label()).collect();
            let text = format!("Checking {}…", names.join(", "));
            eprintln!("Providers: {text}");
            if let Some(window) = weak.upgrade() {
                let state = window.global::<ProvidersState>();
                state.set_busy(true);
                state.set_run_status(text.into());
            }
        }
        // Scheduled ticks stay quiet unless they do something
        Start::Busy if manual => eprintln!("Providers: a check is already running"),
        Start::Nothing { waiting } if manual => {
            let mut lines = Vec::new();
            if waiting.is_empty() {
                lines.push("Nothing to check: no provider is enabled.".to_owned());
            }
            for (id, wait) in waiting {
                lines.push(format!(
                    "{}: checked recently, next check allowed in {}",
                    id.label(),
                    fmt_duration(wait)
                ));
            }
            for line in &lines {
                eprintln!("Providers: {line}");
            }
            if let Some(window) = weak.upgrade() {
                window.global::<ProvidersState>().set_run_status(lines.join("\n").into());
            }
        }
        Start::Busy | Start::Nothing { .. } => {}
    }
}
