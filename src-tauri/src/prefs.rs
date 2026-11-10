use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Prefs {
    // Providers
    pub provider_wallhaven: bool,
    pub wallpaper_search_type: u8,
    pub provider_deviantart: bool,
    pub wallpaper_deviantart_search_type: u8,
    pub provider_bing: bool,
    pub provider_social_wallpapering: bool,
    pub provider_social_wallpapering_ignore_keywords: bool,
    pub provider_wallpaper_fusion: bool,
    pub provider_dual_monitor: bool,
    pub provider_dual_monitor_search_type: u8,
    pub provider_unsplash: bool,
    pub provider_unsplash_search_type: u8,

    // Keywords
    pub wallhaven_keywords: String,
    pub avoid_keywords: String,

    // Resolution
    pub wallpaper_resolution: String, // e.g. "1920x1080"

    // Download policy
    pub download_policy: u8,

    // Timer
    pub application_timer: u32,
    pub application_timer_units: String, // "s", "m", "h"

    // Changer
    pub application_changer: u32,
    pub application_changer_units: String,
    pub application_changer_mode: u8,
    pub application_changer_folder: String, // semicolon-separated
    pub application_changer_multimonitor: bool,

    // Downloads
    pub downloads_path: String,
    pub application_max_download_folder_size: u32, // MB
    pub downloading_process: bool,

    // Move favorites
    pub move_favorite: bool,
    pub move_favorite_folder: String,

    // UI
    pub start_minimized: bool,
    pub time_to_minimize: u32,
    pub system_tray_icon: bool,
    pub application_notifications: u8,
    pub application_i18n: u8,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            provider_wallhaven: true,
            wallpaper_search_type: 0,
            provider_deviantart: false,
            wallpaper_deviantart_search_type: 0,
            provider_bing: false,
            provider_social_wallpapering: false,
            provider_social_wallpapering_ignore_keywords: false,
            provider_wallpaper_fusion: false,
            provider_dual_monitor: false,
            provider_dual_monitor_search_type: 0,
            provider_unsplash: false,
            provider_unsplash_search_type: 0,
            wallhaven_keywords: String::new(),
            avoid_keywords: String::new(),
            wallpaper_resolution: "1920x1080".into(),
            download_policy: 0,
            application_timer: 5,
            application_timer_units: "m".into(),
            application_changer: 5,
            application_changer_units: "m".into(),
            application_changer_mode: 0,
            application_changer_folder: String::new(),
            application_changer_multimonitor: false,
            downloads_path: default_downloads_path(),
            application_max_download_folder_size: 2048,
            downloading_process: true,
            move_favorite: false,
            move_favorite_folder: String::new(),
            start_minimized: false,
            time_to_minimize: 3,
            system_tray_icon: true,
            application_notifications: 1,
            application_i18n: 0,
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
