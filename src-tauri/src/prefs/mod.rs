pub mod enums;
pub mod providers;

use enums::{ChangerMode, DownloadPolicy, NotificationLevel, TimeUnit};
use providers::Providers;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Re-export for convenience
pub use enums::{DeviantArtSearchType, SearchType};
pub use providers::{
    Bing, DeviantArt, DualMonitor, SocialWallpapering, Unsplash, Wallhaven, WallpaperFusion, WfSort,
};

// ─── Prefs ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Prefs {
    pub providers: Providers,
    pub resolution: Resolution,
    pub harvester: Harvester,
    pub changer: Changer,
    pub downloads: Downloads,
    pub ui: Ui,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            providers: Providers::default(),
            resolution: Resolution::default(),
            harvester: Harvester::default(),
            changer: Changer::default(),
            downloads: Downloads::default(),
            ui: Ui::default(),
        }
    }
}

impl Prefs {
    pub fn config_path() -> PathBuf {
        let base = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .unwrap()
                .join("Library/Application Support/wallpaper-downloader")
        } else {
            dirs::home_dir()
                .unwrap()
                .join(".config/wallpaper-downloader")
        };
        base.join("prefs.toml")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let prefs: Prefs = toml::from_str(&content).map_err(|e| e.to_string())?;
            Ok(prefs)
        } else {
            let prefs = Prefs::default();
            prefs.save()?;
            Ok(prefs)
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ─── Resolution ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

// ─── Harvester ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Harvester {
    pub enabled: bool,
    pub interval: u32,
    pub interval_units: TimeUnit,
    pub avoid_keywords: String,
}

impl Default for Harvester {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: 5,
            interval_units: TimeUnit::Minutes,
            avoid_keywords: String::new(),
        }
    }
}

// ─── Changer ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Changer {
    pub interval: u32,
    pub interval_units: TimeUnit,
    pub mode: ChangerMode,
    pub folders: Vec<PathBuf>,
    pub multi_monitor: bool,
}

impl Default for Changer {
    fn default() -> Self {
        Self {
            interval: 5,
            interval_units: TimeUnit::Minutes,
            mode: ChangerMode::Random,
            folders: Vec::new(),
            multi_monitor: false,
        }
    }
}

// ─── Downloads ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Downloads {
    pub path: PathBuf,
    pub max_size_mb: u32,
    pub move_favorite: bool,
    pub move_favorite_folder: PathBuf,
}

impl Default for Downloads {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            path: home.join("Pictures").join("Wallpapers"),
            max_size_mb: 2048,
            move_favorite: false,
            move_favorite_folder: PathBuf::new(),
        }
    }
}

// ─── UI ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ui {
    pub i18n: u8,
    pub notifications: NotificationLevel,
    pub start_minimized: bool,
    pub time_to_minimize: u32,
    pub system_tray_icon: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            i18n: 0,
            notifications: NotificationLevel::Minimal,
            start_minimized: false,
            time_to_minimize: 3,
            system_tray_icon: true,
        }
    }
}
