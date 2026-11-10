use crate::prefs::Prefs;

pub async fn harvest(prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // Bing daily wallpaper (no API key needed)
    let resp: serde_json::Value = client
        .get("https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1")
        .send()
        .await?
        .json()
        .await?;

    let full_url = format!("https://www.bing.com{}", resp["images"][0]["fullUrl"].as_str().unwrap_or(""));
    let bytes = client.get(&full_url).send().await?.bytes().await?;

    let dir = std::path::Path::new(&prefs.downloads_path);
    std::fs::create_dir_all(dir)?;
    let filename = format!("bing_{}.jpg", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    std::fs::write(dir.join(&filename), &bytes)?;

    Ok(())
}
