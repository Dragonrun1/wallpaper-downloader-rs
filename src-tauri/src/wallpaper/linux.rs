use super::{DesktopEnvironment, run_cmd};
use anyhow::Result;
use std::path::Path;

pub fn is_app_installed(app: &str) -> bool {
    std::process::Command::new("which")
        .arg(app)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn set(path: &Path, de: &DesktopEnvironment) -> Result<()> {
    match de {
        DesktopEnvironment::Gnome
        | DesktopEnvironment::Unity
        | DesktopEnvironment::Pantheon
        | DesktopEnvironment::Budgie => gnome(path),
        DesktopEnvironment::MATE => mate(path),
        DesktopEnvironment::KDE => kde(path),
        DesktopEnvironment::XFCE => xfce(path),
        DesktopEnvironment::Cinnamon => cinnamon(path),
        DesktopEnvironment::Deepin => deepin(path),
        DesktopEnvironment::LXQt => lxqt(path),
        DesktopEnvironment::Cosmic => cosmic(path),
        DesktopEnvironment::Hyprland => hyprland(path),
        DesktopEnvironment::Sway
        | DesktopEnvironment::Niri
        | DesktopEnvironment::River
        | DesktopEnvironment::I3 => wlroots(path),
        DesktopEnvironment::Openbox => feh(path),
        DesktopEnvironment::Unknown => {
            if is_app_installed("swww") {
                swww(path)
            } else if is_app_installed("swaybg") {
                swaybg(path)
            } else if is_app_installed("feh") {
                feh(path)
            } else if is_app_installed("xdg-background") {
                run_cmd("xdg-background", &[&path.to_string_lossy()])
            } else {
                anyhow::bail!("No wallpaper setter available")
            }
        }
    }
}

fn cinnamon(path: &Path) -> Result<()> {
    let uri = format!("file://{}", path.display());
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.cinnamon.desktop.background",
            "picture-uri",
            &uri,
        ],
    )
}

fn cosmic(path: &Path) -> Result<()> {
    if is_app_installed("cosmic-bg") {
        run_cmd("cosmic-bg", &[&path.to_string_lossy()])
    } else {
        let uri = format!("file://{}", path.display());
        run_cmd(
            "gsettings",
            &["set", "org.gnome.desktop.background", "picture-uri", &uri],
        )
    }
}

fn deepin(path: &Path) -> Result<()> {
    run_cmd(
        "gsettings",
        &[
            "set",
            "com.deepin.wrap.gnome.desktop.background",
            "picture-uri",
            &path.to_string_lossy(),
        ],
    )
}

fn feh(path: &Path) -> Result<()> {
    run_cmd("feh", &["--bg-fill", &path.to_string_lossy()])
}

fn get_xrandr_monitors() -> Option<Vec<String>> {
    let output = std::process::Command::new("xrandr").output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(
        stdout
            .lines()
            .filter_map(|line| {
                if line.contains(" connected") && !line.contains("disconnected") {
                    line.split_whitespace().next().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn gnome(path: &Path) -> Result<()> {
    let uri = format!("file://{}", path.display());
    run_cmd(
        "gsettings",
        &["set", "org.gnome.desktop.background", "picture-uri", &uri],
    )?;
    let _ = run_cmd(
        "gsettings",
        &[
            "set",
            "org.gnome.desktop.background",
            "picture-uri-dark",
            &uri,
        ],
    );
    Ok(())
}

fn kde(path: &Path) -> Result<()> {
    let uri = format!("file://{}", path.display());

    let output = std::process::Command::new("qdbus")
        .args(["org.kde.KWin", "/KWin", "org.kde.KWin.setWallpaper", &uri])
        .output()
        .map_err(|e| anyhow::anyhow!("qdbus failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for "widgets locked" D-Bus error
    if stdout.contains("org.freedesktop.DBus.Error.Failed")
        || stderr.contains("org.freedesktop.DBus.Error.Failed")
    {
        anyhow::bail!("KDE: widgets are locked, cannot change wallpaper");
    }

    if !output.status.success() {
        // Fall through to kwriteconfig fallback
        return kde_fallback(path);
    }

    Ok(())
}

fn kde_fallback(path: &Path) -> Result<()> {
    let config = dirs::home_dir()
        .map(|h| h.join(".config/plasmarc"))
        .unwrap_or_default();

    if config.exists() {
        let content = std::fs::read_to_string(&config)?;
        let new_content = content
            .replace("WallpaperPath=", &format!("WallpaperPath={}", path.display()));
        std::fs::write(&config, new_content)?;
        let _ = std::process::Command::new("qdbus")
            .args(["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.reload"])
            .status();
    } else {
        run_cmd("kwriteconfig5", &[
            "--file", "plasmarc",
            "--group", "Wallpapers",
            "--key", "WallpaperPath",
            &path.to_string_lossy(),
        ])?;
        let _ = std::process::Command::new("qdbus")
            .args(["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.reload"])
            .status();
    }

    Ok(())
}

fn hyprland(path: &Path) -> Result<()> {
    if is_app_installed("hyprctl") {
        let arg = format!(",{}", path.display());
        run_cmd("hyprctl", &["hyprpaper", "wallpaper", &arg])
    } else if is_app_installed("swww") {
        swww(path)
    } else {
        anyhow::bail!("Hyprland: neither hyprctl nor swww available")
    }
}

fn lxqt(path: &Path) -> Result<()> {
    run_cmd("pcmanfm", &["--set-wallpaper", &path.to_string_lossy()])
}

fn mate(path: &Path) -> Result<()> {
    run_cmd(
        "gsettings",
        &[
            "set",
            "org.mate.background",
            "picture-filename",
            &path.to_string_lossy(),
        ],
    )
}

fn swaybg(path: &Path) -> Result<()> {
    run_cmd("swaybg", &["-m", "fill", "-i", &path.to_string_lossy()])
}

fn swww(path: &Path) -> Result<()> {
    run_cmd("swww", &["img", &path.to_string_lossy()])
}

fn wlroots(path: &Path) -> Result<()> {
    if is_app_installed("swww") {
        swww(path)
    } else if is_app_installed("swaybg") {
        swaybg(path)
    } else if is_app_installed("wpaperctl") {
        run_cmd("wpaperctl", &["set", &path.to_string_lossy()])
    } else if is_app_installed("feh") {
        feh(path)
    } else {
        anyhow::bail!("No wallpaper tool found (tried: swww, swaybg, wpaperctl, feh)")
    }
}

fn xfce(path: &Path) -> Result<()> {
    let monitors = get_xrandr_monitors().unwrap_or_default();

    for monitor in &monitors {
        let property = format!("/backdrop/screen0/monitor{}/workspace0/last-image", monitor);
        let _ = run_cmd(
            "xfconf-query",
            &[
                "--channel",
                "xfce4-desktop",
                "--property",
                &property,
                "--set",
                &path.to_string_lossy(),
            ],
        );
    }

    if monitors.is_empty() {
        run_cmd(
            "xfconf-query",
            &[
                "--channel",
                "xfce4-desktop",
                "--property",
                "/backdrop/screen0/monitor0/workspace0/last-image",
                "--set",
                &path.to_string_lossy(),
            ],
        )?;
    }

    Ok(())
}
