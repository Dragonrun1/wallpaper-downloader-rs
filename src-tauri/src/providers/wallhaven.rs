use crate::prefs::Prefs;

pub async fn harvest(prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Parse resolution
    let (width, height) = prefs.wallpaper_resolution.split_once('x')
        .map(|(w, h)| (w.parse::<u32>().unwrap_or(1920), h.parse::<u32>().unwrap_or(1080)))
        .unwrap_or((1920, 1080));

    let url = format!(
        "https://wallhaven.cc/api/v1/search?q={}&atleast={}x{}&sorting=random&show=0",
        urlencoding::encode(&prefs.wallhaven_keywords),
        width, height
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    if resp["data"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
        return Err("No wallpapers found".into());
    }

    let wallpaper = &resp["data"][0];
    let download_url = wallpaper["path"].as_str().unwrap_or("");
    let _thumb_url = wallpaper["thumbs"]["large"].as_str().unwrap_or("");

    // Download full image
    let bytes = client.get(download_url).send().await?.bytes().await?;

    // Save to downloads folder
    let dir = std::path::Path::new(&prefs.downloads_path);
    std::fs::create_dir_all(dir)?;
    let filename = format!("wallhaven_{}.jpg", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let filepath = dir.join(&filename);
    std::fs::write(&filepath, &bytes)?;

    tracing::info!("Downloaded: {}", filepath.display());
    Ok(())
}

