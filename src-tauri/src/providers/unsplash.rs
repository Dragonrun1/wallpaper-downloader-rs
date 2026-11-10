use crate::prefs::Prefs;

pub async fn harvest(prefs: &Prefs) -> Result<(), Box<dyn std::error::Error>> {
    // Unsplash API requires an access key
    // TODO: store API key in prefs or env
    let api_key = std::env::var("UNSPLASH_ACCESS_KEY")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.unsplash.com/photos/random?query={}&orientation=landscape&client_id={}",
        urlencoding::encode(&prefs.wallhaven_keywords), // reuse keywords or add unsplash-specific
        api_key
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let download_url = resp["urls"]["full"].as_str().unwrap_or("");

    let bytes = client.get(download_url).send().await?.bytes().await?;
    let dir = std::path::Path::new(&prefs.downloads_path);
    std::fs::create_dir_all(dir)?;
    let filename = format!("unsplash_{}.jpg", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    std::fs::write(dir.join(&filename), &bytes)?;

    Ok(())
}
