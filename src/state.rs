use std::{collections::BTreeMap, fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// What the app last applied. Runtime state, not user settings.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Last image used, so sequential order continues across restarts.
    pub last: Option<String>,
    /// Screen name -> wallpaper path.
    pub screens: BTreeMap<String, String>,
}

impl State {
    fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "wallpaper-downloader-rs")
            .map(|d| d.data_dir().join("state.toml"))
    }

    /// Never fails: missing or unreadable state just means a fresh start.
    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                eprintln!("Ignoring invalid state file {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &path)
    }
}
