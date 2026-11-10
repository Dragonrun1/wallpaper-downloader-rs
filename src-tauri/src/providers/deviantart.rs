use crate::prefs::Prefs;
use anyhow::Result;
use std::fs::{create_dir_all, write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = reqwest::Client::new();

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

    let result = resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("DeviantArt: no result array"))?;

    if result.is_empty() {
        anyhow::bail!("DeviantArt: no wallpapers found");
    }

    let item = &result[0];
    let download_url = item["src"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("DeviantArt: missing download URL"))?;

    let bytes = client.get(download_url).send().await?.bytes().await?;

    create_dir_all(&prefs.downloads.path)?;
    let filename = format!(
        "deviantart_{}.jpg",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    write(prefs.downloads.path.join(&filename), &bytes)?;

    Ok(())
}
