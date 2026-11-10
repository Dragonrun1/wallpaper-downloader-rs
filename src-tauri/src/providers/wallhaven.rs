use anyhow::Result;
use crate::prefs::Prefs;
use super::{build_client, download_and_save};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;

    let url = format!(
        "https://wallhaven.cc/api/v1/search?q={}&atleast={}x{}&sorting=random&show=0",
        urlencoding::encode(&prefs.providers.wallhaven.keywords),
        prefs.resolution.width,
        prefs.resolution.height
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    if resp["data"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
        anyhow::bail!("Wallhaven: no wallpapers found");
    }

    let wallpaper = &resp["data"][0];
    let download_url = wallpaper["path"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Wallhaven: missing download URL"))?;

    let filename = format!("wallhaven_{}.jpg", timestamp());
    let saved = download_and_save(&client, prefs, download_url, &filename).await?;

    if !saved {
        anyhow::bail!("Wallhaven: wallpaper already exists");
    }

    Ok(())
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
