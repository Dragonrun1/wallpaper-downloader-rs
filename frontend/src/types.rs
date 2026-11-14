use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Prefs {
    pub providers: Providers,
    pub resolution: Resolution,
    pub harvester: Harvester,
    pub changer: Changer,
    pub downloads: Downloads,
    pub ui: Ui,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Providers {
    pub wallhaven: Wallhaven,
    pub unsplash: Unsplash,
    pub bing: Bing,
    pub deviantart: DeviantArt,
    pub social_wallpapering: SocialWallpapering,
    pub wallpaper_fusion: WallpaperFusion,
    pub dual_monitor: DualMonitor,
    pub download_policy: u8,
    pub forbidden_keywords: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Wallhaven {
    pub enabled: bool,
    pub search_type: String,
    pub keywords: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Unsplash {
    pub enabled: bool,
    pub search_type: String,
    pub api_key: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bing {
    pub enabled: bool,
    pub mkt: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeviantArt {
    pub enabled: bool,
    pub search_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SocialWallpapering {
    pub enabled: bool,
    pub ignore_keywords: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WallpaperFusion {
    pub enabled: bool,
    pub sort: String,
    pub monitors: u8,
    pub min_rating: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DualMonitor {
    pub enabled: bool,
    pub search_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Harvester {
    pub interval: u32,
    pub interval_units: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Changer {
    pub interval: u32,
    pub interval_units: String,
    pub mode: String,
    pub folders: Vec<String>,
    pub multi_monitor: bool,
    pub last_wallpaper: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Downloads {
    pub path: String,
    pub max_size_gb: u32,
    pub move_favorite: bool,
    pub move_favorite_folder: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ui {
    pub i18n: u8,
    pub notifications: String,
    pub start_minimized: bool,
    pub time_to_minimize: u32,
    pub system_tray_icon: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WallpaperInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub provider: String,
    pub author: String,
    pub author_url: String,
    pub provider_url: String,
}
