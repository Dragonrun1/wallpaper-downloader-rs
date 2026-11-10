use serde::{Deserialize, Serialize};

// ─── Search Types ───────────────────────────────────────────────────────────

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
    fn default() -> Self {
        Self::Relevant
    }
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
}

impl Default for DeviantArtSearchType {
    fn default() -> Self {
        Self::Relevance
    }
}

// ─── Download Policy ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadPolicy {
    MatchResolution,
    Always,
    SkipExisting,
}

impl Default for DownloadPolicy {
    fn default() -> Self {
        Self::MatchResolution
    }
}

// ─── Changer Mode ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangerMode {
    Random,
    Sequential,
    Reverse,
}

impl Default for ChangerMode {
    fn default() -> Self {
        Self::Random
    }
}

// ─── Time Units ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
}

impl Default for TimeUnit {
    fn default() -> Self {
        Self::Minutes
    }
}

impl TimeUnit {
    pub fn to_seconds(&self, value: u32) -> u64 {
        match self {
            TimeUnit::Seconds => value as u64,
            TimeUnit::Minutes => value as u64 * 60,
            TimeUnit::Hours => value as u64 * 3600,
        }
    }
}

// ─── Notification Level ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    None,
    Minimal,
    Full,
}

impl Default for NotificationLevel {
    fn default() -> Self {
        Self::Minimal
    }
}
