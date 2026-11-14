//! Finding and downloading wallpapers. No UI types in here, so it can run on
//! a background thread and be tested on its own.

pub mod bing;
pub mod harvest;
pub mod storage;
pub mod wallhaven;

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::default_wallpaper_dir;

// ─── Identity ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    Wallhaven,
    Bing,
}

impl ProviderId {
    pub const ALL: [ProviderId; 2] = [Self::Wallhaven, Self::Bing];

    /// Stable key used in file names, the library index and the config file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Wallhaven => "wallhaven",
            Self::Bing => "bing",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Wallhaven => "Wallhaven",
            Self::Bing => "Bing Photo of the Day",
        }
    }

    pub fn source(self) -> Box<dyn Source> {
        match self {
            Self::Wallhaven => Box::new(wallhaven::Wallhaven),
            Self::Bing => Box::new(bing::Bing),
        }
    }
}

// ─── Shared vocabulary ──────────────────────────────────────────────────────

/// One sort order for every provider. A provider maps it to its own
/// parameters, or ignores it if `Capabilities::sorts` doesn't list it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sort {
    Relevance,
    Newest,
    Top,
    Views,
    Favorites,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUse {
    None,
    Optional,
    Required,
}

/// Which settings a provider actually uses. The Providers tab reads this to
/// decide which controls to show.
#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    pub sorts: &'static [Sort],
    pub keywords: bool,
    pub api_key: KeyUse,
    pub market: bool,
    /// Has sketchy / NSFW content levels (`allow_sketchy`, `allow_nsfw`).
    pub purity: bool,
}

/// Everything we know about one image. Optional wherever a source can't say.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallpaper {
    pub provider: ProviderId,
    /// The provider's own ID: gives stable file names and reliable de-duplication.
    pub id: String,
    pub image_url: String,
    pub page_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rating: Option<f32>,
    pub favorites: Option<u32>,
}

impl Wallpaper {
    pub fn new(provider: ProviderId, id: impl Into<String>, image_url: impl Into<String>) -> Self {
        Self {
            provider,
            id: id.into(),
            image_url: image_url.into(),
            page_url: None,
            title: None,
            description: None,
            author: None,
            tags: Vec::new(),
            width: None,
            height: None,
            rating: None,
            favorites: None,
        }
    }
}

// ─── Settings ───────────────────────────────────────────────────────────────

/// One struct for every provider; fields a provider doesn't use stay `None`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderSettings {
    pub enabled: bool,
    pub sort: Option<Sort>,
    pub api_key: Option<String>,
    /// Bing: locale like "en-US". Empty means the system locale.
    pub market: Option<String>,
    /// Wallhaven: include "sketchy" images. Off unless asked for.
    pub allow_sketchy: bool,
    /// Wallhaven: include NSFW images. Only honoured with an API key.
    pub allow_nsfw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProvidersConfig {
    pub download_dir: String,
    /// The app's own downloads may use at most this many MB; the oldest
    /// non-favorites are deleted past that. 0 means unlimited.
    pub max_folder_size_mb: u64,
    /// Shared by every provider that supports searching. Separate several
    /// with `;` and one is picked at random for each run.
    pub keywords: String,
    /// Images with a tag containing any of these (case-insensitive) are skipped.
    pub forbidden_tags: Vec<String>,
    /// Keyed by `ProviderId::key()`.
    pub settings: BTreeMap<String, ProviderSettings>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        let mut settings = BTreeMap::new();
        settings.insert(
            ProviderId::Wallhaven.key().to_owned(),
            ProviderSettings { enabled: true, ..Default::default() },
        );
        settings.insert(
            ProviderId::Bing.key().to_owned(),
            ProviderSettings::default(), // Bing: market empty = system locale
        );
        Self {
            download_dir: default_wallpaper_dir()
                .and_then(|p| p.to_str().map(str::to_owned))
                .unwrap_or_default(),
            max_folder_size_mb: 256,
            keywords: String::new(),
            forbidden_tags: Vec::new(),
            settings,
        }
    }
}

impl ProvidersConfig {
    pub fn settings_for(&self, id: ProviderId) -> ProviderSettings {
        self.settings.get(id.key()).cloned().unwrap_or_default()
    }

    pub fn enabled(&self) -> Vec<ProviderId> {
        ProviderId::ALL
            .into_iter()
            .filter(|&id| self.settings_for(id).enabled)
            .collect()
    }
}

// ─── The provider interface ─────────────────────────────────────────────────

/// What a provider needs to do its job.
pub struct Ctx {
    pub client: reqwest::Client,
    pub keywords: String,
    /// Prefer images at least this big (the largest connected screen).
    pub min_resolution: Option<(u32, u32)>,
    pub settings: ProviderSettings,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> Capabilities;

    /// Find candidate images. They may be incomplete (e.g. no tags yet).
    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>>;

    /// Fill in anything expensive to fetch, just before download. Called only
    /// for candidates that aren't already in the library.
    async fn details(&self, _ctx: &Ctx, wallpaper: Wallpaper) -> Result<Wallpaper> {
        Ok(wallpaper)
    }
}
