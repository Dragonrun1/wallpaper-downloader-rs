use anyhow::Result;
use crate::prefs::Prefs;
use super::{build_client, download_and_save};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;

    let url = format!(
        "https://api.deviantart.com/v1/oauth2/browse/deviations?q={}&mature_content=off&has_nsfw_marker=false&width={}&height={}&order=popular&offset=0&limit=1",
        urlencoding::encode(&prefs.providers.wallhaven.keywords),
        prefs.resolution.width,
        prefs.resolution.height
    );

    let resp: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "WallpaperDownloader/4.5.0")
        .send()
        .await?
        .json()
        .await?;

    let result = resp["result"].as_array()
        .ok_or_else(|| anyhow::anyhow!("DeviantArt: no result array"))?;

    if result.is_empty() {
        anyhow::bail!("DeviantArt: no wallpapers found");
    }

    let item = &result[0];
    let download_url = item["src"].as_str()
        .ok_or_else(|| anyhow::anyhow!("DeviantArt: missing download URL"))?;

    let filename = format!("deviantart_{}.jpg", timestamp());
    let saved = download_and_save(&client, prefs, download_url, &filename).await?;

    if !saved {
        anyhow::bail!("DeviantArt: wallpaper already exists");
    }

    Ok(())
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
