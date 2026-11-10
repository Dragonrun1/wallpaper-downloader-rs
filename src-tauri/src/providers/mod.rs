pub mod bing;
pub mod deviantart;
pub mod dual_monitor;
pub mod social_wallpapering;
pub mod unsplash;
pub mod wallhaven;
pub mod wallpaper_fusion;

use crate::prefs::Prefs;
use anyhow::Result;
use scraper::Html;

// ─── Provider Enum ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Provider {
    Wallhaven,
    Unsplash,
    Bing,
    DeviantArt,
    DualMonitor,
    SocialWallpapering,
    WallpaperFusion,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Provider {
    pub async fn harvest(&self, prefs: &Prefs) -> Result<()> {
        match self {
            Self::Wallhaven => wallhaven::harvest(prefs).await,
            Self::Unsplash => unsplash::harvest(prefs).await,
            Self::Bing => bing::harvest(prefs).await,
            Self::DeviantArt => deviantart::harvest(prefs).await,
            Self::DualMonitor => dual_monitor::harvest(prefs).await,
            Self::SocialWallpapering => social_wallpapering::harvest(prefs).await,
            Self::WallpaperFusion => wallpaper_fusion::harvest(prefs).await,
        }
    }

    pub fn enabled(prefs: &Prefs) -> Vec<Self> {
        let mut v = Vec::new();
        if prefs.providers.wallhaven.enabled {
            v.push(Self::Wallhaven);
        }
        if prefs.providers.unsplash.enabled {
            v.push(Self::Unsplash);
        }
        if prefs.providers.bing.enabled {
            v.push(Self::Bing);
        }
        if prefs.providers.deviantart.enabled {
            v.push(Self::DeviantArt);
        }
        if prefs.providers.dual_monitor.enabled {
            v.push(Self::DualMonitor);
        }
        if prefs.providers.social_wallpapering.enabled {
            v.push(Self::SocialWallpapering);
        }
        if prefs.providers.wallpaper_fusion.enabled {
            v.push(Self::WallpaperFusion);
        }
        v
    }

    pub fn filename_prefix(&self) -> &'static str {
        match self {
            Self::Wallhaven => "wallhaven_",
            Self::Unsplash => "unsplash_",
            Self::Bing => "bing_",
            Self::DeviantArt => "deviantart_",
            Self::DualMonitor => "wd_dm_",
            Self::SocialWallpapering => "wd_sw_",
            Self::WallpaperFusion => "wd_wf-",
        }
    }
}

// ─── Shared Helpers ─────────────────────────────────────────────────────────

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

pub async fn download_and_save(
    client: &reqwest::Client,
    prefs: &Prefs,
    url: &str,
    filename: &str,
) -> Result<bool> {
    let filepath = prefs.downloads.path.join(filename);
    if filepath.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(&prefs.downloads.path)?;
    let bytes = client.get(url).send().await?.bytes().await?;
    std::fs::write(&filepath, &bytes)?;

    tracing::info!("Saved: {}", filepath.display());
    Ok(true)
}

pub fn has_forbidden_keyword(doc: &Html, selector: &str, prefs: &Prefs) -> bool {
    if prefs.harvester.avoid_keywords.is_empty() {
        return false;
    }

    let forbidden: Vec<String> = prefs
        .harvester
        .avoid_keywords
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect();

    let tag_sel = scraper::Selector::parse(selector).unwrap();
    doc.select(&tag_sel).any(|tag| {
        let text: String = tag.text().collect::<String>().to_lowercase();
        forbidden.iter().any(|f| text.contains(f.as_str()))
    })
}
