use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use crate::prefs::Prefs;

const BING_URL: &str = "https://www.bing.com/HPImageArchive.aspx";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    let idx = current_day_offset();
    let mkt = &prefs.providers.bing.mkt;

    let url = format!("{BING_URL}?format=js&idx={idx}&n=1&mkt={mkt}");

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let image = resp["images"]
        .as_array()
        .and_then(|a| a.get(0))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Bing: no image in response"))?;

    let full_url = format!("https://www.bing.com{}", image["fullUrl"].as_str().unwrap_or(""));
    let title = image["title"].as_str().unwrap_or("bing").replace(' ', "_");

    let bytes = client.get(&full_url).send().await?.bytes().await?;

    std::fs::create_dir_all(&prefs.downloads.path)?;
    let filename = format!("bing_{title}_{idx}.jpg");
    let filepath = prefs.downloads.path.join(&filename);

    if filepath.exists() {
        return harvest_yesterday(&client, prefs).await;
    }

    std::fs::write(&filepath, &bytes)?;
    tracing::info!("Bing: saved {}", filepath.display());
    Ok(())
}

async fn harvest_yesterday(client: &reqwest::Client, prefs: &Prefs) -> Result<()> {
    let idx = current_day_offset().wrapping_sub(1);
    let mkt = &prefs.providers.bing.mkt;

    let url = format!("{BING_URL}?format=js&idx={idx}&n=1&mkt={mkt}");

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let image = resp["images"]
        .as_array()
        .and_then(|a| a.get(0))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Bing: no image in response"))?;

    let full_url = format!("https://www.bing.com{}", image["fullUrl"].as_str().unwrap_or(""));
    let title = image["title"].as_str().unwrap_or("bing").replace(' ', "_");

    let bytes = client.get(&full_url).send().await?.bytes().await?;

    let filename = format!("bing_{title}_{idx}.jpg");
    let filepath = prefs.downloads.path.join(&filename);

    if filepath.exists() {
        anyhow::bail!("Bing: all recent wallpapers already downloaded");
    }

    std::fs::write(&filepath, &bytes)?;
    tracing::info!("Bing: saved {}", filepath.display());
    Ok(())
}

fn current_day_offset() -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    ((now.as_secs() / 3600) % 7) as u32
}
