//! Runs provider checks on a background thread. Knows nothing about the UI:
//! callers hand in a callback and get a `RunReport` back.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::runtime::Runtime;

use super::{
    harvest::{harvest, HarvestReport},
    schedule::{self, now_secs, Check, Runs, Trigger},
    storage::MB,
    ProviderId, ProvidersConfig,
};
use crate::library::Library;

pub struct Service {
    runtime: Runtime,
    library: Arc<Mutex<Library>>,
    runs: Arc<Mutex<Runs>>,
    busy: Arc<AtomicBool>,
}

/// What `start` did.
#[derive(Debug)]
pub enum Start {
    /// These providers are being checked now.
    Started(Vec<ProviderId>),
    /// Another check is still running.
    Busy,
    /// Nothing may run right now; these are the ones that have to wait.
    Nothing { waiting: Vec<(ProviderId, Duration)> },
}

pub struct RunReport {
    pub providers: Vec<ProviderId>,
    pub limit_bytes: u64,
    pub outcome: Result<HarvestReport, String>,
}

/// Clears the busy flag when dropped, even if a task panics.
struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Service {
    /// Loads the library and the run log from disk and starts the runtime.
    pub fn new(cfg: &ProvidersConfig) -> Result<Self> {
        let mut library = Library::load();
        if !cfg.download_dir.is_empty() {
            let dir = PathBuf::from(&cfg.download_dir);
            if library.bind_dir(&dir) {
                eprintln!("Providers: the download folder changed, starting a fresh library for it");
            }
            let blocked = library.prune(&dir);
            if blocked > 0 {
                eprintln!("Providers: {blocked} downloaded wallpaper(s) were deleted by you and won't come back");
            }
            if let Err(e) = library.save() {
                eprintln!("Providers: could not save the library: {e}");
            }
        }
        Self::with_parts(library, Runs::load())
    }

    fn with_parts(library: Library, runs: Runs) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .context("could not start the background runtime")?;
        Ok(Self {
            runtime,
            library: Arc::new(Mutex::new(library)),
            runs: Arc::new(Mutex::new(runs)),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }

    /// One line per enabled provider: its schedule and when it is next due.
    pub fn status_lines(&self, cfg: &ProvidersConfig) -> Vec<String> {
        let runs = self.runs.lock().unwrap();
        let now = now_secs();
        cfg.enabled()
            .into_iter()
            .map(|id| {
                let next = match runs.check(cfg, id, Trigger::Scheduled, now) {
                    Check::Run => "due now".to_owned(),
                    Check::Wait(d) => format!("next check in {}", fmt_duration(d)),
                    Check::Off => "schedule is off".to_owned(),
                };
                format!(
                    "{}: every {} (minimum {}), {next}",
                    id.label(),
                    fmt_duration(schedule::interval(cfg, id)),
                    fmt_duration(schedule::floor(cfg, id)),
                )
            })
            .collect()
    }

    /// Check whichever providers are allowed to run now, in the background.
    /// `on_done` is called from a background thread when the run finishes.
    pub fn start(
        &self,
        cfg: &ProvidersConfig,
        trigger: Trigger,
        on_done: impl FnOnce(RunReport) + Send + 'static,
    ) -> Start {
        if self.busy.swap(true, Ordering::SeqCst) {
            return Start::Busy;
        }
        let guard = BusyGuard(self.busy.clone()); // cleared on every path below

        let now = now_secs();
        let mut due = Vec::new();
        let mut waiting = Vec::new();
        {
            let runs = self.runs.lock().unwrap();
            for id in ProviderId::ALL {
                match runs.check(cfg, id, trigger, now) {
                    Check::Run => due.push(id),
                    Check::Wait(d) => waiting.push((id, d)),
                    Check::Off => {}
                }
            }
        }
        if due.is_empty() {
            return Start::Nothing { waiting };
        }

        let started = due.clone();
        let (cfg, library, runs) = (cfg.clone(), self.library.clone(), self.runs.clone());
        self.runtime.spawn(async move {
            let _guard = guard;
            let outcome = harvest(&cfg, library.clone(), &due).await.map_err(|e| format!("{e:#}"));

            let now = now_secs();
            {
                let mut runs = runs.lock().unwrap();
                match &outcome {
                    Ok(report) => runs.record_all(&report.results, now),
                    Err(e) => due.iter().for_each(|&id| runs.record(id, now, Some(e.clone()))),
                }
                if let Err(e) = runs.save() {
                    eprintln!("Providers: could not save the run log: {e}");
                }
            }
            if let Err(e) = library.lock().unwrap().save() {
                eprintln!("Providers: could not save the library: {e}");
            }

            on_done(RunReport {
                providers: due,
                limit_bytes: cfg.max_folder_size_mb * MB,
                outcome,
            });
        });
        Start::Started(started)
    }
}

impl RunReport {
    pub fn lines(&self) -> Vec<String> {
        let report = match &self.outcome {
            Ok(report) => report,
            Err(e) => return vec![format!("the run failed: {e}")],
        };

        let mut lines: Vec<String> = report
            .results
            .iter()
            .map(|(id, result)| match result {
                Ok(Some(e)) => format!(
                    "{}: downloaded {} ({}×{})",
                    id.label(),
                    e.file,
                    e.wallpaper.width.unwrap_or(0),
                    e.wallpaper.height.unwrap_or(0)
                ),
                Ok(None) => format!("{}: nothing new", id.label()),
                Err(err) => format!("{}: failed: {err:#}", id.label()),
            })
            .collect();

        let c = &report.cleanup;
        if !c.deleted.is_empty() {
            lines.push(format!(
                "Storage: deleted {} old file(s), freed {}; downloads now use {} of {}",
                c.deleted.len(),
                fmt_size(c.freed_bytes),
                fmt_size(c.total_bytes),
                fmt_size(self.limit_bytes),
            ));
        }
        if c.over_limit {
            lines.push(format!(
                "Storage: still over the limit ({} of {}); favorites and new downloads are kept",
                fmt_size(c.total_bytes),
                fmt_size(self.limit_bytes),
            ));
        }
        lines
    }
}

pub fn fmt_size(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / MB as f64)
}

/// "45s", "10m", "3h 12m", "1d 2h".
pub fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let (days, hours, mins) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    match (days, hours, mins) {
        (0, 0, 0) => format!("{secs}s"),
        (0, 0, m) => format!("{m}m"),
        (0, h, 0) => format!("{h}h"),
        (0, h, m) => format!("{h}h {m}m"),
        (d, 0, _) => format!("{d}d"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        library::Entry,
        providers::{storage::Cleanup, Wallpaper},
    };

    fn service() -> Service {
        Service::with_parts(Library::default(), Runs::default()).unwrap()
    }

    #[test]
    fn durations_read_naturally() {
        let secs = Duration::from_secs;
        assert_eq!(fmt_duration(secs(45)), "45s");
        assert_eq!(fmt_duration(secs(600)), "10m");
        assert_eq!(fmt_duration(secs(6 * 3600)), "6h");
        assert_eq!(fmt_duration(secs(3 * 3600 + 12 * 60 + 40)), "3h 12m");
        assert_eq!(fmt_duration(secs(26 * 3600)), "1d 2h");
        assert_eq!(fmt_duration(secs(2 * 86_400)), "2d");
    }

    #[test]
    fn nothing_runs_when_everything_is_disabled() {
        let mut cfg = ProvidersConfig::default();
        cfg.settings.values_mut().for_each(|s| s.enabled = false);
        let service = service();
        match service.start(&cfg, Trigger::Manual, |_| {}) {
            Start::Nothing { waiting } => assert!(waiting.is_empty()),
            other => panic!("unexpected: {other:?}"),
        }
        assert!(!service.busy.load(Ordering::SeqCst), "busy flag must be released");
    }

    #[test]
    fn a_recent_check_makes_a_manual_run_wait() {
        let cfg = ProvidersConfig::default(); // Wallhaven enabled
        let service = service();
        service.runs.lock().unwrap().record(ProviderId::Wallhaven, now_secs(), None);
        match service.start(&cfg, Trigger::Manual, |_| {}) {
            Start::Nothing { waiting } => {
                assert_eq!(waiting.len(), 1);
                assert_eq!(waiting[0].0, ProviderId::Wallhaven);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn only_one_run_at_a_time() {
        let service = service();
        service.busy.store(true, Ordering::SeqCst);
        assert!(matches!(
            service.start(&ProvidersConfig::default(), Trigger::Scheduled, |_| {}),
            Start::Busy
        ));
    }

    #[test]
    fn status_lines_describe_the_schedule() {
        let service = service();
        let lines = service.status_lines(&ProvidersConfig::default());
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("Wallhaven: every 6h (minimum 10m)"), "{}", lines[0]);
        assert!(lines[0].ends_with("due now"));
    }

    #[test]
    fn report_lines() {
        let mut w = Wallpaper::new(ProviderId::Wallhaven, "abc", "https://example.invalid/x.jpg");
        w.width = Some(2560);
        w.height = Some(1440);
        let entry = Entry {
            wallpaper: w,
            file: "wallhaven_abc.jpg".into(),
            added: 0,
            favorite: false,
            blocked: false,
        };
        let report = RunReport {
            providers: vec![ProviderId::Wallhaven, ProviderId::Bing],
            limit_bytes: 256 * MB,
            outcome: Ok(HarvestReport {
                results: vec![
                    (ProviderId::Wallhaven, Ok(Some(entry))),
                    (ProviderId::Bing, Err(anyhow::anyhow!("offline"))),
                ],
                cleanup: Cleanup {
                    deleted: vec!["old.jpg".into()],
                    freed_bytes: 3 * MB,
                    total_bytes: 250 * MB,
                    over_limit: false,
                },
            }),
        };
        let lines = report.lines();
        assert_eq!(lines[0], "Wallhaven: downloaded wallhaven_abc.jpg (2560×1440)");
        assert_eq!(lines[1], "Bing Photo of the Day: failed: offline");
        assert_eq!(lines[2], "Storage: deleted 1 old file(s), freed 3.0 MB; downloads now use 250.0 MB of 256.0 MB");
    }
}
