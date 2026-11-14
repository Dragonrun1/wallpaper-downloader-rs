//! When each provider may be checked. Pure rules plus a small persisted log,
//! so limits hold across restarts and none of this needs a UI or a network.

use std::{
    collections::BTreeMap,
    fs, io,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::{harvest::HarvestResult, ProviderId, ProvidersConfig};

/// Failed checks wait longer each time, up to this.
const MAX_BACKOFF: Duration = Duration::from_secs(6 * 3600);

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
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

// ─── Settings ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unit {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interval {
    pub every: u32,
    pub unit: Unit,
}

impl Unit {
    /// Matches the order of the unit drop-down in the UI.
    pub fn index(self) -> i32 {
        match self {
            Self::Minutes => 0,
            Self::Hours => 1,
            Self::Days => 2,
        }
    }

    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Self::Minutes,
            2 => Self::Days,
            _ => Self::Hours,
        }
    }
}

impl Interval {
    pub fn duration(self) -> Duration {
        let n = u64::from(self.every.max(1));
        Duration::from_secs(match self.unit {
            Unit::Minutes => n * 60,
            Unit::Hours => n * 3600,
            Unit::Days => n * 86_400,
        })
    }
}

/// The schedule shared by every provider. A provider can override the
/// interval in its own settings, but never go below its own minimum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleConfig {
    pub enabled: bool,
    pub interval: Interval,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self { enabled: true, interval: Interval { every: 6, unit: Unit::Hours } }
    }
}

/// The shortest gap allowed between two checks of a provider. It protects
/// the service and can't be configured away.
pub fn floor(cfg: &ProvidersConfig, id: ProviderId) -> Duration {
    id.source().min_interval(&cfg.settings_for(id))
}

/// The gap the user asked for (the provider's own setting, else the shared
/// one), raised to the provider's minimum if it's shorter.
pub fn interval(cfg: &ProvidersConfig, id: ProviderId) -> Duration {
    cfg.settings_for(id)
        .interval
        .unwrap_or(cfg.schedule.interval)
        .duration()
        .max(floor(cfg, id))
}

fn backoff(failures: u32, floor: Duration) -> Duration {
    if failures == 0 {
        return Duration::ZERO;
    }
    floor.saturating_mul(1 << failures.min(10)).min(MAX_BACKOFF)
}

// ─── What has happened so far ───────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderRun {
    /// Unix seconds of the last check, successful or not.
    pub last_attempt: u64,
    pub last_success: u64,
    /// Consecutive failures; reset by a success.
    pub failures: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// The timer fired: obey the interval and any backoff.
    Scheduled,
    /// The user pressed "Download now": only the provider's minimum applies.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    Run,
    /// Not yet; this long to go.
    Wait(Duration),
    /// Disabled, or the schedule is off.
    Off,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Runs {
    providers: BTreeMap<String, ProviderRun>,
}

impl Runs {
    fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "wallpaper-downloader-rs").map(|d| d.data_dir().join("runs.toml"))
    }

    /// Never fails: a missing or unreadable file means nothing has run yet.
    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, toml::to_string_pretty(self).map_err(io::Error::other)?)?;
        fs::rename(&tmp, &path)
    }

    pub fn get(&self, id: ProviderId) -> Option<&ProviderRun> {
        self.providers.get(id.key())
    }

    /// May this provider be checked right now?
    pub fn check(&self, cfg: &ProvidersConfig, id: ProviderId, trigger: Trigger, now: u64) -> Check {
        if !cfg.settings_for(id).enabled || (trigger == Trigger::Scheduled && !cfg.schedule.enabled) {
            return Check::Off;
        }
        let Some(run) = self.get(id).filter(|r| r.last_attempt > 0) else {
            return Check::Run; // never checked
        };

        let floor = floor(cfg, id);
        let gap = match trigger {
            Trigger::Manual => floor,
            Trigger::Scheduled => interval(cfg, id).max(backoff(run.failures, floor)),
        };
        // A clock that moved backwards must not freeze the provider for ages
        let ready_at = run.last_attempt.min(now) + gap.as_secs();
        if now >= ready_at {
            Check::Run
        } else {
            Check::Wait(Duration::from_secs(ready_at - now))
        }
    }

    /// A one-line status for the UI, e.g. "Next check in 3h 12m · last check 2h ago".
    pub fn describe(&self, cfg: &ProvidersConfig, id: ProviderId, now: u64) -> String {
        if !cfg.settings_for(id).enabled {
            return "Disabled".to_owned();
        }
        let next = match self.check(cfg, id, Trigger::Scheduled, now) {
            Check::Run => "Due now".to_owned(),
            Check::Wait(d) => format!("Next check in {}", fmt_duration(d)),
            Check::Off => "Automatic checks are off".to_owned(),
        };
        let last = match self.get(id) {
            Some(run) if run.last_error.is_some() => {
                let error = run.last_error.as_deref().unwrap_or_default();
                let short: String = error.chars().take(140).collect();
                let dots = if error.chars().count() > 140 { "…" } else { "" };
                format!(" · last check failed: {short}{dots}")
            }
            Some(run) if run.last_success > 0 => format!(
                " · last check {} ago",
                fmt_duration(Duration::from_secs(now.saturating_sub(run.last_success)))
            ),
            _ => String::new(),
        };
        format!("{next}{last}")
    }

    /// The providers a scheduled tick should check.
    pub fn due(&self, cfg: &ProvidersConfig, now: u64) -> Vec<ProviderId> {
        ProviderId::ALL
            .into_iter()
            .filter(|&id| self.check(cfg, id, Trigger::Scheduled, now) == Check::Run)
            .collect()
    }

    /// Note that a provider was checked. `error` is `None` on success
    /// (including "nothing new").
    pub fn record(&mut self, id: ProviderId, now: u64, error: Option<String>) {
        let run = self.providers.entry(id.key().to_owned()).or_default();
        run.last_attempt = now;
        match error {
            None => {
                run.last_success = now;
                run.failures = 0;
                run.last_error = None;
            }
            Some(e) => {
                run.failures = run.failures.saturating_add(1);
                run.last_error = Some(e);
            }
        }
    }

    pub fn record_all(&mut self, results: &[HarvestResult], now: u64) {
        for (id, result) in results {
            self.record(*id, now, result.as_ref().err().map(|e| format!("{e:#}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ProviderSettings;

    const HOUR: u64 = 3600;

    fn cfg() -> ProvidersConfig {
        let mut cfg = ProvidersConfig::default();
        cfg.schedule = ScheduleConfig { enabled: true, interval: Interval { every: 1, unit: Unit::Hours } };
        cfg.settings.insert("bing".into(), ProviderSettings { enabled: true, ..Default::default() });
        cfg
    }

    fn runs_at(id: ProviderId, last_attempt: u64, failures: u32) -> Runs {
        let mut runs = Runs::default();
        runs.providers.insert(
            id.key().to_owned(),
            ProviderRun { last_attempt, failures, ..Default::default() },
        );
        runs
    }

    #[test]
    fn never_checked_is_due() {
        let cfg = cfg();
        assert_eq!(Runs::default().check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 1000), Check::Run);
        assert_eq!(Runs::default().due(&cfg, 1000), vec![ProviderId::Wallhaven, ProviderId::Bing]);
    }

    #[test]
    fn waits_for_the_interval() {
        let cfg = cfg();
        let runs = runs_at(ProviderId::Wallhaven, 10_000, 0);
        assert_eq!(
            runs.check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 10_000 + 600),
            Check::Wait(Duration::from_secs(HOUR - 600))
        );
        assert_eq!(runs.check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 10_000 + HOUR), Check::Run);
    }

    #[test]
    fn a_short_interval_is_raised_to_the_providers_minimum() {
        let mut cfg = cfg();
        cfg.settings.get_mut("bing").unwrap().interval = Some(Interval { every: 1, unit: Unit::Minutes });
        assert_eq!(interval(&cfg, ProviderId::Bing), floor(&cfg, ProviderId::Bing));
        assert!(floor(&cfg, ProviderId::Bing) >= Duration::from_secs(HOUR));

        let runs = runs_at(ProviderId::Bing, 10_000, 0);
        assert!(matches!(
            runs.check(&cfg, ProviderId::Bing, Trigger::Scheduled, 10_000 + 600),
            Check::Wait(_)
        ));
    }

    #[test]
    fn a_provider_can_use_its_own_interval() {
        let mut cfg = cfg();
        cfg.settings.get_mut("bing").unwrap().interval = Some(Interval { every: 1, unit: Unit::Days });
        assert_eq!(interval(&cfg, ProviderId::Bing), Duration::from_secs(24 * HOUR));
        assert_eq!(interval(&cfg, ProviderId::Wallhaven), Duration::from_secs(HOUR)); // shared one
    }

    #[test]
    fn manual_only_obeys_the_minimum() {
        let cfg = cfg();
        let floor_secs = floor(&cfg, ProviderId::Wallhaven).as_secs();
        let runs = runs_at(ProviderId::Wallhaven, 10_000, 0);
        // 10 minutes later: too early for the schedule, fine for a manual run
        assert!(matches!(runs.check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 10_600), Check::Wait(_)));
        assert_eq!(runs.check(&cfg, ProviderId::Wallhaven, Trigger::Manual, 10_000 + floor_secs), Check::Run);
        assert!(matches!(runs.check(&cfg, ProviderId::Wallhaven, Trigger::Manual, 10_001), Check::Wait(_)));
    }

    #[test]
    fn off_when_disabled() {
        let mut cfg = cfg();
        cfg.schedule.enabled = false;
        assert_eq!(Runs::default().check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 1), Check::Off);
        assert_eq!(Runs::default().check(&cfg, ProviderId::Wallhaven, Trigger::Manual, 1), Check::Run);
        cfg.settings.get_mut("wallhaven").unwrap().enabled = false;
        assert_eq!(Runs::default().check(&cfg, ProviderId::Wallhaven, Trigger::Manual, 1), Check::Off);
    }

    #[test]
    fn failures_back_off_and_a_success_resets() {
        let mut cfg = cfg();
        cfg.schedule.interval = Interval { every: 1, unit: Unit::Minutes }; // raised to the minimum
        let floor = floor(&cfg, ProviderId::Wallhaven);
        assert_eq!(backoff(0, floor), Duration::ZERO);
        assert_eq!(backoff(1, floor), floor * 2);
        assert_eq!(backoff(3, floor), floor * 8);
        assert_eq!(backoff(40, floor), MAX_BACKOFF); // capped, no overflow

        let mut runs = Runs::default();
        runs.record(ProviderId::Wallhaven, 10_000, Some("boom".into()));
        assert_eq!(runs.get(ProviderId::Wallhaven).unwrap().failures, 1);
        assert!(matches!(
            runs.check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 10_000 + floor.as_secs()),
            Check::Wait(_)
        ));
        runs.record(ProviderId::Wallhaven, 20_000, None);
        let run = runs.get(ProviderId::Wallhaven).unwrap();
        assert_eq!((run.failures, run.last_success, run.last_error.clone()), (0, 20_000, None));
    }

    #[test]
    fn a_clock_that_went_backwards_doesnt_freeze_a_provider() {
        let cfg = cfg();
        let runs = runs_at(ProviderId::Wallhaven, 1_000_000, 0);
        assert!(matches!(
            runs.check(&cfg, ProviderId::Wallhaven, Trigger::Scheduled, 500),
            Check::Wait(d) if d <= interval(&cfg, ProviderId::Wallhaven)
        ));
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
    fn status_text() {
        let mut cfg = cfg();
        let mut runs = Runs::default();
        assert_eq!(runs.describe(&cfg, ProviderId::Wallhaven, 1000), "Due now");

        runs.record(ProviderId::Wallhaven, 10_000, None);
        assert_eq!(
            runs.describe(&cfg, ProviderId::Wallhaven, 10_000 + 600),
            "Next check in 50m · last check 10m ago"
        );

        runs.record(ProviderId::Wallhaven, 20_000, Some("offline".into()));
        assert!(runs.describe(&cfg, ProviderId::Wallhaven, 20_100).ends_with("last check failed: offline"));

        cfg.schedule.enabled = false;
        assert!(runs.describe(&cfg, ProviderId::Wallhaven, 20_100).starts_with("Automatic checks are off"));
        cfg.settings.get_mut("wallhaven").unwrap().enabled = false;
        assert_eq!(runs.describe(&cfg, ProviderId::Wallhaven, 20_100), "Disabled");
    }

    #[test]
    fn unit_indices_match_the_ui_order() {
        for unit in [Unit::Minutes, Unit::Hours, Unit::Days] {
            assert_eq!(Unit::from_index(unit.index()), unit);
        }
        assert_eq!(Unit::from_index(99), Unit::Hours);
    }

    #[test]
    fn runs_round_trip_through_toml() {
        let mut runs = Runs::default();
        runs.record(ProviderId::Bing, 123, Some("oops".into()));
        let back: Runs = toml::from_str(&toml::to_string_pretty(&runs).unwrap()).unwrap();
        assert_eq!(back.get(ProviderId::Bing).unwrap().last_error.as_deref(), Some("oops"));
    }
}
