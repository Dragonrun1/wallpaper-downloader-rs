use super::{build_client, download_and_save};
use crate::prefs::Prefs;
use anyhow::Result;

const BING_URL: &str = "https://www.bing.com/HPImageArchive.aspx";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;
    let mkt = &prefs.providers.bing.mkt;

    let idx = current_day_offset();
    let url = format!("{BING_URL}?format=js&idx={idx}&n=1&mkt={mkt}");

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let image = resp["images"]
        .as_array()
        .and_then(|a| a.get(0))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Bing: no image in response"))?;

    let full_url = format!(
        "https://www.bing.com{}",
        image["fullUrl"].as_str().unwrap_or("")
    );
    let title = image["title"].as_str().unwrap_or("bing").replace(' ', "_");

    let filename = format!("bing_{title}_{idx}.jpg");
    let saved = download_and_save(&client, prefs, &full_url, &filename).await?;

    if !saved {
        return harvest_yesterday(&client, prefs).await;
    }

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

    let full_url = format!(
        "https://www.bing.com{}",
        image["fullUrl"].as_str().unwrap_or("")
    );
    let title = image["title"].as_str().unwrap_or("bing").replace(' ', "_");

    let filename = format!("bing_{title}_{idx}.jpg");
    let saved = download_and_save(&client, prefs, &full_url, &filename).await?;

    if !saved {
        anyhow::bail!("Bing: all recent wallpapers already downloaded");
    }

    Ok(())
}

fn current_day_offset() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    ((now.as_secs() / 3600) % 7) as u32
}
