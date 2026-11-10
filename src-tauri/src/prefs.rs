use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Prefs {
    pub providers: Providers,
    pub resolution: Resolution,
    pub harvester: Harvester,
    pub changer: Changer,
    pub downloads: Downloads,
    pub ui: Ui,
}

// ─── Providers ───────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Providers {
    pub wallhaven: Wallhaven,
    pub unsplash: Unsplash,
    pub bing: Bing,
    pub deviantart: DeviantArt,
    pub social_wallpapering: SocialWallpapering,
    pub wallpaper_fusion: WallpaperFusion,
    pub dual_monitor: DualMonitor,
    pub download_policy: DownloadPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WallpaperFusion {
    pub enabled: bool,
    pub sort: WfSort,
    pub monitors: u8,       // 0=all, 1, 2, 3, 4
    pub min_rating: u8,     // 0=any, 1-5 stars (maps to RateAll)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WfSort {
    Relevance,   // 0
    Popular,     // 1
    Newest,      // 4
}

impl WfSort {
    pub fn as_param(&self) -> &str {
        match self {
            Self::Relevance => "0",
            Self::Popular   => "1",
            Self::Newest    => "4",
        }
    }
}

impl Default for WallpaperFusion {
    fn default() -> Self {
        Self { enabled: false, sort: WfSort::Relevance, monitors: 0, min_rating: 0 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bing {
    pub enabled: bool,
    pub mkt: String,
}

impl Default for Bing {
    fn default() -> Self {
        Self { enabled: false, mkt: "en-US".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Wallhaven {
    pub enabled: bool,
    pub search_type: SearchType,
    pub keywords: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Unsplash {
    pub enabled: bool,
    pub search_type: SearchType,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviantArt {
    pub enabled: bool,
    pub search_type: DeviantArtSearchType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SocialWallpapering {
    pub enabled: bool,
    pub ignore_keywords: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DualMonitor {
    pub enabled: bool,
    pub search_type: SearchType,
}

// ─── Enums (replace magic numbers) ──────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    Relevant,
    Top,
    Random,
    DateAdded,
    Views,
    Favorites,
}

impl Default for SearchType {
    fn default() -> Self { Self::Relevant }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeviantArtSearchType {
    Relevance,
    Newest,
    Oldest,
    Random,
    ViewCount,
    FaveCount,
    // ... add the rest from your Java combo box
}

impl Default for DeviantArtSearchType {
    fn default() -> Self { Self::Relevance }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadPolicy {
    /// Download only if current wallpaper matches resolution
    MatchResolution,
    /// Always download
    Always,
    /// Skip if file already exists
    SkipExisting,
}

impl Default for DownloadPolicy {
    fn default() -> Self { Self::MatchResolution }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangerMode {
    Random,
    Sequential,
    Reverse,
}

impl Default for ChangerMode {
    fn default() -> Self { Self::Random }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
}

impl Default for TimeUnit {
    fn default() -> Self { Self::Minutes }
}

impl TimeUnit {
    pub fn to_seconds(&self, value: u32) -> u64 {
        match self {
            TimeUnit::Seconds => value as u64,
            TimeUnit::Minutes => value as u64 * 60,
            TimeUnit::Hours   => value as u64 * 3600,
        }
    }
}

// ─── Resolution ─────────────────────────────────────────────

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
        Self { width: 1920, height: 1080 }
    }
}

// ─── Harvester (download loop) ──────────────────────────────

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

// ─── Changer (wallpaper rotation) ───────────────────────────

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

// ─── Downloads ──────────────────────────────────────────────

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

// ─── UI ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ui {
    pub i18n: u8,
    pub notifications: NotificationLevel,
    pub start_minimized: bool,
    pub time_to_minimize: u32,
    pub system_tray_icon: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    None,
    Minimal,
    Full,
}

impl Default for NotificationLevel {
    fn default() -> Self { Self::Minimal }
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

// ─── Prefs Default ──────────────────────────────────────────

impl Default for Prefs {
    fn default() -> Self {
        Self {
            providers: Providers {
                wallhaven: Wallhaven { enabled: true, search_type: SearchType::default(), keywords: String::new() },
                unsplash: Unsplash { enabled: false, search_type: SearchType::default(), api_key: String::new() },
                bing: Bing::default(),
                deviantart: DeviantArt { enabled: false, search_type: DeviantArtSearchType::default() },
                social_wallpapering: SocialWallpapering { enabled: false, ignore_keywords: false },
                wallpaper_fusion: WallpaperFusion::default(),
                dual_monitor: DualMonitor { enabled: false, search_type: SearchType::default() },
                download_policy: DownloadPolicy::default(),
            },
            resolution: Resolution::default(),
            harvester: Harvester::default(),
            changer: Changer::default(),
            downloads: Downloads::default(),
            ui: Ui::default(),
        }
    }
}

fn default_downloads_path() -> String {
    dirs::home_dir()
        .map(|h| h.join("Pictures").join("Wallpapers").to_string_lossy().to_string())
        .unwrap_or_else(|| "wallpapers".into())
}

impl Prefs {
    pub fn config_path() -> PathBuf {
        let base = if cfg!(target_os = "macos") {
            dirs::home_dir().unwrap().join("Library/Application Support/wallpaper-downloader")
        } else {
            dirs::home_dir().unwrap().join(".config/wallpaper-downloader")
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
