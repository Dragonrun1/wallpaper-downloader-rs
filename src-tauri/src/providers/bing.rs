use crate::prefs::Prefs;
use anyhow::Result;
use std::fs::{create_dir_all, write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get("https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1")
        .send()
        .await?
        .json()
        .await?;

    let full_url = format!(
        "https://www.bing.com{}",
        resp["images"][0]["fullUrl"].as_str().unwrap_or("")
    );
    let bytes = client.get(&full_url).send().await?.bytes().await?;

    create_dir_all(&prefs.downloads.path)?;
    let filename = format!(
        "bing_{}.jpg",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    write(prefs.downloads.path.join(&filename), &bytes)?;

    Ok(())
}
