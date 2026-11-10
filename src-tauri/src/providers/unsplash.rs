use super::{build_client, download_and_save};
use crate::prefs::Prefs;
use anyhow::Result;

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let api_key = std::env::var("UNSPLASH_ACCESS_KEY")
        .unwrap_or_else(|_| prefs.providers.unsplash.api_key.clone());

    if api_key.is_empty() {
        anyhow::bail!("Unsplash: no API key configured");
    }

    let client = build_client()?;
    let url = format!(
        "https://api.unsplash.com/photos/random?query={}&orientation=landscape&client_id={}",
        urlencoding::encode(&prefs.providers.wallhaven.keywords),
        api_key
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let download_url = resp["urls"]["full"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Unsplash: missing download URL"))?;

    let filename = format!("unsplash_{}.jpg", timestamp());
    let saved = download_and_save(&client, prefs, download_url, &filename).await?;

    if !saved {
        anyhow::bail!("Unsplash: wallpaper already exists");
    }

    Ok(())
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
