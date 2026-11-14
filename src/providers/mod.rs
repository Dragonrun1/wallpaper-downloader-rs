//! Finding and downloading wallpapers. No UI types in here, so it can run on
//! a background thread and be tested on its own.

pub mod bing;
pub mod harvest;
pub mod storage;
pub mod wallhaven;

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use display_info::DisplayInfo;
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

// ─── Resolution ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionPolicy {
    /// Don't care about size.
    Any,
    /// Big enough for at least one of the target resolutions.
    AtLeast,
    /// Exactly one of the target resolutions.
    Exact,
}

/// The sizes of the connected screens (one entry per distinct size).
/// Falls back to 1920×1080 if they can't be read.
pub fn detect_screens() -> Vec<Resolution> {
    let mut found: Vec<Resolution> = Vec::new();
    for d in DisplayInfo::all().unwrap_or_default() {
        let r = Resolution::new(d.width, d.height);
        if !found.contains(&r) {
            found.push(r);
        }
    }
    if found.is_empty() {
        found.push(Resolution::new(1920, 1080));
    }
    found
}

/// Which image sizes we want. The targets start out as the connected screens
/// and can be edited freely afterwards.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResolutionFilter {
    pub policy: ResolutionPolicy,
    pub targets: Vec<Resolution>,
}

impl Default for ResolutionFilter {
    fn default() -> Self {
        Self { policy: ResolutionPolicy::AtLeast, targets: detect_screens() }
    }
}

impl ResolutionFilter {
    /// Does an image of this size satisfy the filter?
    pub fn accepts(&self, width: u32, height: u32) -> bool {
        if self.policy == ResolutionPolicy::Any || self.targets.is_empty() {
            return true;
        }
        self.targets.iter().any(|t| match self.policy {
            ResolutionPolicy::Exact => width == t.width && height == t.height,
            _ => width >= t.width && height >= t.height,
        })
    }

    /// The loosest lower bound covering every target (smallest width and
    /// smallest height). It's what we ask a provider's search for; `accepts`
    /// then does the strict check.
    pub fn minimum(&self) -> Option<Resolution> {
        let width = self.targets.iter().map(|t| t.width).min()?;
        let height = self.targets.iter().map(|t| t.height).min()?;
        Some(Resolution::new(width, height))
    }

    /// The target with the most pixels.
    pub fn largest(&self) -> Option<Resolution> {
        self.targets.iter().copied().max_by_key(|t| u64::from(t.width) * u64::from(t.height))
    }
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
    pub resolution: ResolutionFilter,
    /// Shared by every provider that supports searching. One is picked at
    /// random for each run; empty means no keyword.
    pub keywords: Vec<String>,
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
            resolution: ResolutionFilter::default(),
            keywords: Vec::new(),
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
    /// The keyword chosen for this run (empty = none).
    pub keywords: String,
    pub resolution: ResolutionFilter,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(policy: ResolutionPolicy, targets: &[(u32, u32)]) -> ResolutionFilter {
        ResolutionFilter {
            policy,
            targets: targets.iter().map(|&(w, h)| Resolution::new(w, h)).collect(),
        }
    }

    #[test]
    fn at_least_accepts_anything_big_enough_for_one_target() {
        let f = filter(ResolutionPolicy::AtLeast, &[(1920, 1080), (1080, 1920)]);
        assert!(f.accepts(2560, 1440)); // fits the landscape screen
        assert!(f.accepts(1080, 1920)); // fits the portrait screen
        assert!(!f.accepts(1280, 720));
        assert!(!f.accepts(1000, 3000)); // too narrow for either
    }

    #[test]
    fn exact_needs_a_matching_size() {
        let f = filter(ResolutionPolicy::Exact, &[(1920, 1080)]);
        assert!(f.accepts(1920, 1080));
        assert!(!f.accepts(2560, 1440));
    }

    #[test]
    fn any_and_empty_targets_accept_everything() {
        assert!(filter(ResolutionPolicy::Any, &[(1920, 1080)]).accepts(10, 10));
        assert!(filter(ResolutionPolicy::Exact, &[]).accepts(10, 10));
    }

    #[test]
    fn minimum_and_largest() {
        let f = filter(ResolutionPolicy::AtLeast, &[(3840, 2160), (1920, 1200), (1080, 1920)]);
        assert_eq!(f.minimum(), Some(Resolution::new(1080, 1200)));
        assert_eq!(f.largest(), Some(Resolution::new(3840, 2160)));
        assert_eq!(filter(ResolutionPolicy::AtLeast, &[]).minimum(), None);
    }

    #[test]
    fn config_round_trips_through_toml() {
        let mut cfg = ProvidersConfig::default();
        cfg.keywords = vec!["mountains".into(), "ocean".into()];
        cfg.forbidden_tags = vec!["people".into()];
        cfg.resolution = filter(ResolutionPolicy::Exact, &[(1920, 1080), (2560, 1440)]);
        cfg.settings.get_mut("wallhaven").unwrap().sort = Some(Sort::Newest);

        let text = toml::to_string_pretty(&cfg).unwrap();
        println!("{text}");
        let back: ProvidersConfig = toml::from_str(&text).unwrap();
        assert_eq!(back.keywords, cfg.keywords);
        assert_eq!(back.resolution.targets, cfg.resolution.targets);
        assert_eq!(back.resolution.policy, ResolutionPolicy::Exact);
        assert_eq!(back.settings_for(ProviderId::Wallhaven).sort, Some(Sort::Newest));
        assert_eq!(back.enabled(), vec![ProviderId::Wallhaven]);
    }

    #[test]
    fn detected_screens_are_never_empty() {
        assert!(!detect_screens().is_empty());
    }
}
