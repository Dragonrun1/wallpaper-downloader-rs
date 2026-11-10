use std::path::Path;
use anyhow::Result;

pub fn set(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    let script = format!(
        "tell application \"System Events\" to set picture of every desktop to \"{}\"",
        path_str
    );

    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript failed: {}", stderr.trim());
    }

    // Sonoma+ cache workaround
    let _ = std::process::Command::new("killall")
        .arg("WallpaperAgent")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    Ok(())
}
