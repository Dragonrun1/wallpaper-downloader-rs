#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use std::path::Path;
use walkdir::WalkDir;
use anyhow::Result;

// ─── Desktop Environment Detection ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DesktopEnvironment {
    Gnome,
    Unity,
    MATE,
    KDE,
    XFCE,
    Cinnamon,
    Pantheon,
    Deepin,
    Budgie,
    LXQt,
    Cosmic,
    Sway,
    Hyprland,
    Niri,
    River,
    I3,
    Openbox,
    Unknown,
}

impl DesktopEnvironment {
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
            let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

            tracing::info!("XDG_CURRENT_DESKTOP={:?}, XDG_SESSION_TYPE={:?}", desktop, session);

            for part in desktop.split(';') {
                let p = part.to_lowercase();
                let p = p.split(':').last().unwrap_or(&p);

                if let Some(de) = match p {
                    "gnome" | "ubuntu" => Some(Self::Gnome),
                    "unity" => Some(Self::Unity),
                    "mate" => Some(Self::MATE),
                    "kde" | "plasma" => Some(Self::KDE),
                    "xfce" => Some(Self::XFCE),
                    "cinnamon" => Some(Self::Cinnamon),
                    "pantheon" | "elementary" => Some(Self::Pantheon),
                    "deepin" => Some(Self::Deepin),
                    "budgie" => Some(Self::Budgie),
                    "lxqt" => Some(Self::LXQt),
                    "cosmic" => Some(Self::Cosmic),
                    "hyprland" => Some(Self::Hyprland),
                    "sway" => Some(Self::Sway),
                    "niri" => Some(Self::Niri),
                    "river" => Some(Self::River),
                    "i3" | "i3wm" => Some(Self::I3),
                    "openbox" => Some(Self::Openbox),
                    _ => None,
                } {
                    return de;
                }
            }

            if linux::is_app_installed("swww") {
                return Self::Sway;
            }
            if linux::is_app_installed("hyprpaper") {
                return Self::Hyprland;
            }
            if linux::is_app_installed("feh") && session == "x11" {
                return Self::Openbox;
            }

            Self::Unknown
        }

        #[cfg(not(target_os = "linux"))]
        {
            Self::Unknown
        }
    }

    pub fn is_changeable(&self) -> bool {
        matches!(self,
            Self::Gnome | Self::Unity | Self::MATE | Self::KDE |
            Self::XFCE | Self::Cinnamon | Self::Pantheon | Self::Deepin |
            Self::Budgie | Self::LXQt | Self::Cosmic |
            Self::Sway | Self::Hyprland | Self::Niri | Self::River |
            Self::I3 | Self::Openbox
        )
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Gnome => "GNOME",
            Self::Unity => "Unity",
            Self::MATE => "MATE",
            Self::KDE => "KDE/Plasma",
            Self::XFCE => "XFCE",
            Self::Cinnamon => "Cinnamon",
            Self::Pantheon => "Pantheon",
            Self::Deepin => "Deepin",
            Self::Budgie => "Budgie",
            Self::LXQt => "LXQt",
            Self::Cosmic => "COSMIC",
            Self::Sway => "Sway",
            Self::Hyprland => "Hyprland",
            Self::Niri => "Niri",
            Self::River => "River",
            Self::I3 => "i3",
            Self::Openbox => "Openbox",
            Self::Unknown => "Unknown",
        }
    }
}

// ─── Main Entry Point ───────────────────────────────────────────────────────

pub fn set_wallpaper(path: &Path) -> Result<()> {
    if !path.exists() || path.is_dir() {
        anyhow::bail!("Wallpaper {} doesn't exist or is a directory", path.display());
    }

    tracing::info!("Setting wallpaper: {}", path.display());

    #[cfg(target_os = "macos")]
    {
        macos::set(path)?;
    }

    #[cfg(target_os = "windows")]
    {
        windows::set(path)?;
    }

    #[cfg(target_os = "linux")]
    {
        let de = DesktopEnvironment::detect();
        tracing::info!("Desktop environment: {}", de.name());
        linux::set(path, &de)?;
    }

    Ok(())
}

// ─── Wallpaper Picker ───────────────────────────────────────────────────────

pub fn pick_random_wallpaper(folders: &[&Path]) -> Result<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    for folder in folders {
        for entry in WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "bmp") {
                candidates.push(entry.path().to_path_buf());
            }
        }
    }
    if candidates.is_empty() {
        anyhow::bail!("No wallpapers found in configured folders");
    }
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as usize)
        % candidates.len();
    Ok(candidates[idx].clone())
}

// ─── Shared Utility ─────────────────────────────────────────────────────────

pub fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run {}: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            tracing::warn!("{} stderr: {}", program, stderr.trim());
        }
        return Err(anyhow::anyhow!("{} exited with {}", program, output.status));
    }

    Ok(())
}
