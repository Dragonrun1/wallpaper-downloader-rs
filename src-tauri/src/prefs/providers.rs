use super::enums::{DeviantArtSearchType, DownloadPolicy, SearchType};
use serde::{Deserialize, Serialize};

// ─── Providers (top-level container) ────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Providers {
    pub bing: Bing,
    pub deviantart: DeviantArt,
    pub download_policy: DownloadPolicy,
    pub dual_monitor: DualMonitor,
    pub forbidden_keywords: String,
    pub social_wallpapering: SocialWallpapering,
    pub unsplash: Unsplash,
    pub wallhaven: Wallhaven,
    pub wallpaper_fusion: WallpaperFusion,
}

impl Default for Providers {
    fn default() -> Self {
        Self {
            bing: Bing::default(),
            deviantart: DeviantArt::default(),
            download_policy: DownloadPolicy::default(),
            dual_monitor: DualMonitor::default(),
            forbidden_keywords: String::new(),
            social_wallpapering: SocialWallpapering::default(),
            unsplash: Unsplash::default(),
            wallhaven: Wallhaven::default(),
            wallpaper_fusion: WallpaperFusion::default(),
        }
    }
}

// ─── Individual Providers ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Wallhaven {
    pub enabled: bool,
    pub search_type: SearchType,
    pub keywords: String,
}

impl Default for Wallhaven {
    fn default() -> Self {
        Self {
            enabled: true,
            search_type: SearchType::default(),
            keywords: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Unsplash {
    pub enabled: bool,
    pub search_type: SearchType,
    pub api_key: String,
}

impl Default for Unsplash {
    fn default() -> Self {
        Self {
            enabled: false,
            search_type: SearchType::default(),
            api_key: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bing {
    pub enabled: bool,
    pub mkt: String,
}

impl Default for Bing {
    fn default() -> Self {
        Self {
            enabled: false,
            mkt: "en-US".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviantArt {
    pub enabled: bool,
    pub search_type: DeviantArtSearchType,
}

impl Default for DeviantArt {
    fn default() -> Self {
        Self {
            enabled: false,
            search_type: DeviantArtSearchType::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SocialWallpapering {
    pub enabled: bool,
    pub ignore_keywords: bool,
}

impl Default for SocialWallpapering {
    fn default() -> Self {
        Self {
            enabled: false,
            ignore_keywords: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DualMonitor {
    pub enabled: bool,
    pub search_type: SearchType,
}

impl Default for DualMonitor {
    fn default() -> Self {
        Self {
            enabled: false,
            search_type: SearchType::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WallpaperFusion {
    pub enabled: bool,
    pub sort: WfSort,
    pub monitors: u8,
    pub min_rating: u8,
}

impl Default for WallpaperFusion {
    fn default() -> Self {
        Self {
            enabled: false,
            sort: WfSort::Relevance,
            monitors: 0,
            min_rating: 0,
        }
    }
}

// ─── WallpaperFusion-specific enum ──────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WfSort {
    Relevance,
    Popular,
    Newest,
}

impl WfSort {
    pub fn as_param(&self) -> &str {
        match self {
            Self::Relevance => "0",
            Self::Popular => "1",
            Self::Newest => "4",
        }
    }
}
