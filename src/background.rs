use std::{rc::Rc, time::Duration};

use slint::{Timer, TimerMode};

use crate::{
    providers::{
        schedule::Trigger,
        service::{fmt_duration, Service, Start},
    },
    saver::Saver,
    AppTray,
};

/// Keep this alive for as long as the app runs; it owns the schedule timer.
pub struct Background {
    _timer: Timer,
}

/// Starts the provider service: checks for due providers right away and then
/// once a minute, and adds a "download now" action to the tray menu.
pub fn setup(tray: &AppTray, saver: &Rc<Saver>) -> Option<Background> {
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

    tray.on_download_now({
        let (service, saver) = (service.clone(), saver.clone());
        move || run(&service, &saver, Trigger::Manual)
    });

    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(60), {
        let (service, saver) = (service.clone(), saver.clone());
        move || run(&service, &saver, Trigger::Scheduled)
    });
    run(&service, saver, Trigger::Scheduled);

    Some(Background { _timer: timer })
}

fn run(service: &Service, saver: &Saver, trigger: Trigger) {
    let manual = trigger == Trigger::Manual;
    let started = service.start(&saver.providers(), trigger, |report| {
        for line in report.lines() {
            eprintln!("Providers: {line}");
        }
    });

    match started {
        Start::Started(ids) => {
            let names: Vec<_> = ids.iter().map(|id| id.label()).collect();
            eprintln!("Providers: checking {}…", names.join(", "));
        }
        // Scheduled ticks are silent unless they do something
        Start::Busy if manual => eprintln!("Providers: a check is already running"),
        Start::Nothing { waiting } if manual => {
            if waiting.is_empty() {
                eprintln!("Providers: nothing to check (no provider is enabled)");
            }
            for (id, wait) in waiting {
                eprintln!(
                    "Providers: {}: checked recently, next check allowed in {}",
                    id.label(),
                    fmt_duration(wait)
                );
            }
        }
        Start::Busy | Start::Nothing { .. } => {}
    }
}
