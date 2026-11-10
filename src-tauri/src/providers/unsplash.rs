use crate::prefs::Prefs;
use anyhow::Result;
use std::fs::{create_dir_all, write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let api_key = std::env::var("UNSPLASH_ACCESS_KEY")
        .unwrap_or_else(|_| prefs.providers.unsplash.api_key.clone());

    if api_key.is_empty() {
        anyhow::bail!("Unsplash: no API key configured");
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.unsplash.com/photos/random?query={}&orientation=landscape&client_id={}",
        urlencoding::encode(&prefs.providers.wallhaven.keywords),
        api_key
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let download_url = resp["urls"]["full"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Unsplash: missing download URL"))?;

    let bytes = client.get(download_url).send().await?.bytes().await?;

    create_dir_all(&prefs.downloads.path)?;
    let filename = format!(
        "unsplash_{}.jpg",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    write(prefs.downloads.path.join(&filename), &bytes)?;

    Ok(())
}
