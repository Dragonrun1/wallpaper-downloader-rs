use crate::prefs::Prefs;
use anyhow::Result;
use std::fs::{create_dir_all, write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = reqwest::Client::new();

    let url = format!(
        "https://wallhaven.cc/api/v1/search?q={}&atleast={}x{}&sorting=random&show=0",
        urlencoding::encode(&prefs.providers.wallhaven.keywords),
        prefs.resolution.width,
        prefs.resolution.height
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    if resp["data"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        anyhow::bail!("Wallhaven: no wallpapers found");
    }

    let wallpaper = &resp["data"][0];
    let download_url = wallpaper["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Wallhaven: missing download URL"))?;

    let bytes = client.get(download_url).send().await?.bytes().await?;

    create_dir_all(&prefs.downloads.path)?;
    let filename = format!(
        "wallhaven_{}.jpg",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    write(prefs.downloads.path.join(&filename), &bytes)?;

    Ok(())
}
