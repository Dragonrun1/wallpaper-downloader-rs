pub mod wallhaven;
pub mod unsplash;
pub mod bing;
pub mod deviantart;
pub mod dual_monitor;
pub mod social_wallpapering;
pub mod wallpaper_fusion;

use crate::prefs::Prefs;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Provider {
    Bing,
    DeviantArt,
    DualMonitor,
    SocialWallpapering,
    Unsplash,
    Wallhaven,
    WallpaperFusion,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Provider {
    pub async fn harvest(&self, prefs: &Prefs) -> anyhow::Result<()> {
        match self {
            Self::Bing        => bing::harvest(prefs).await,
            Self::DeviantArt  => deviantart::harvest(prefs).await,
            Self::DualMonitor => dual_monitor::harvest(prefs).await,
            Self::SocialWallpapering => social_wallpapering::harvest(prefs).await,
            Self::Unsplash    => unsplash::harvest(prefs).await,
            Self::Wallhaven   => wallhaven::harvest(prefs).await,
            Self::WallpaperFusion => wallpaper_fusion::harvest(prefs).await,
        }
    }

    /// Returns all enabled providers based on prefs
    pub fn enabled(prefs: &Prefs) -> Vec<Self> {
        let mut v = Vec::new();
        if prefs.providers.bing.enabled { v.push(Self::Bing); }
        if prefs.providers.deviantart.enabled { v.push(Self::DeviantArt); }
        if prefs.providers.dual_monitor.enabled { v.push(Self::DualMonitor); }
        if prefs.providers.social_wallpapering.enabled {v.push(Self::SocialWallpapering); }
        if prefs.providers.unsplash.enabled { v.push(Self::Unsplash); }
        if prefs.providers.wallhaven.enabled { v.push(Self::Wallhaven); }
        if prefs.providers.wallpaper_fusion.enabled {v.push(Self::WallpaperFusion); }
        v
    }
}
