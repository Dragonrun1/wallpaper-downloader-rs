use std::{fs, io, path::PathBuf};
use std::time::Duration;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntervalUnit {
    Seconds,
    Minutes,
    Hours,
}

impl IntervalUnit {
    // Indices match the ComboBox order in changer.slint
    pub fn index(self) -> i32 {
        match self {
            Self::Seconds => 0,
            Self::Minutes => 1,
            Self::Hours => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Self::Seconds,
            2 => Self::Hours,
            _ => Self::Minutes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeOrder {
    Random,
    OldestToNewest,
    NewestToOldest,
}

impl ChangeOrder {
    pub fn index(self) -> i32 {
        match self {
            Self::Random => 0,
            Self::OldestToNewest => 1,
            Self::NewestToOldest => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Self::OldestToNewest,
            2 => Self::NewestToOldest,
            _ => Self::Random,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangerConfig {
    pub directories: Vec<String>,
    pub enabled: bool,
    pub interval: u32,
    pub interval_unit: IntervalUnit,
    pub monitor_mode: MonitorMode,
    pub order: ChangeOrder,
}

impl ChangerConfig {
    pub fn interval_duration(&self) -> Duration {
        let n = u64::from(self.interval.max(1));
        let secs = match self.interval_unit {
            IntervalUnit::Seconds => n,
            IntervalUnit::Minutes => n * 60,
            IntervalUnit::Hours => n * 3600,
        };
        Duration::from_secs(secs)
    }
}

impl Default for ChangerConfig {
    fn default() -> Self {
        Self {
            directories: Vec::new(),
            enabled: false,
            interval: 30,
            interval_unit: IntervalUnit::Minutes,
            monitor_mode: MonitorMode::DifferentPerMonitor,
            order: ChangeOrder::Random,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub changer: ChangerConfig,
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "wallpaper-downloader-rs")
            .map(|d| d.config_dir().join("config.toml"))
    }

    /// Never fails: a missing file gives defaults, and an unreadable or
    /// corrupt file is moved aside to `config.toml.bak` and replaced by defaults.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("Config {} is invalid ({e}); using defaults", path.display());
                    let _ = fs::rename(&path, path.with_extension("toml.bak"));
                    Self::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                eprintln!("Could not read config {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// Writes to a temp file first, then renames, so a crash mid-write
    /// can't leave a half-written configuration file.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MonitorMode {
    PrimaryOnly,
    DifferentPerMonitor,
    SameOnAll,
}

impl MonitorMode {
    // Indices match the ComboBox order in changer.slint
    pub fn index(self) -> i32 {
        match self {
            Self::PrimaryOnly => 0,
            Self::DifferentPerMonitor => 1,
            Self::SameOnAll => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Self::PrimaryOnly,
            2 => Self::SameOnAll,
            _ => Self::DifferentPerMonitor,
        }
    }
}
