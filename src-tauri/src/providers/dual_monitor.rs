use anyhow::Result;
use scraper::{Html, Selector};
use crate::prefs::Prefs;

const BASE_URL: &str = "http://www.dualmonitorbackgrounds.com";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    let mut page = 1;

    loop {
        let url = build_url(prefs, page);
        tracing::info!("DualMonitor: fetching {}", url);

        let html = client.get(&url).send().await?.text().await?;
        let document = Html::parse_document(&html);

        let links = get_thumbnail_links(&document, prefs);
        if links.is_empty() {
            tracing::info!("DualMonitor: no more wallpapers found");
            anyhow::bail!("DualMonitor: no new wallpapers found");
        }

        for link in &links {
            if download_wallpaper(&client, prefs, link).await? {
                return Ok(());
            }
        }

        page += 1;
    }
}

// ─── URL Building ───────────────────────────────────────────────────────────

fn build_url(prefs: &Prefs, page: usize) -> String {
    let keywords = &prefs.providers.wallhaven.keywords;

    if keywords.is_empty() {
        let sorting = sorting_param(prefs);
        format!("{}index.php?{}&latestImagesPage={}", BASE_URL, sorting, page)
    } else {
        let kw = keywords.replace(' ', "+AND+");
        format!("{}/page/search/{}/{}", BASE_URL, kw, page)
    }
}

fn sorting_param(prefs: &Prefs) -> &'static str {
    match prefs.providers.dual_monitor.search_type {
        crate::prefs::SearchType::Relevant => "sortby=date",
        crate::prefs::SearchType::Top => "sortby=rating",
        crate::prefs::SearchType::Random => "sortby=popularity",
        _ => "sortby=date",
    }
}

// ─── HTML Parsing ───────────────────────────────────────────────────────────

fn get_thumbnail_links(document: &Html, prefs: &Prefs) -> Vec<String> {
    let div_id = if prefs.providers.wallhaven.keywords.is_empty() {
        div_id_for_sorting(prefs)
    } else {
        "albumsSearch"
    };

    let selector = Selector::parse(&format!("div#{} a", div_id)).unwrap();
    document
        .select(&selector)
        .filter_map(|el| {
            let href = el.value().attr("href").unwrap_or("");
            if href.contains(".php") {
                Some(format!("{}{}", BASE_URL, &href[1..]))
            } else {
                None
            }
        })
        .collect()
}

fn div_id_for_sorting(prefs: &Prefs) -> &'static str {
    match prefs.providers.dual_monitor.search_type {
        crate::prefs::SearchType::Relevant => "latest",
        crate::prefs::SearchType::Top => "toprated",
        crate::prefs::SearchType::Random => "popular",
        _ => "latest",
    }
}

// ─── Single Wallpaper Download ──────────────────────────────────────────────

/// Returns true if a wallpaper was successfully downloaded
async fn download_wallpaper(
    client: &reqwest::Client,
    prefs: &Prefs,
    wallpaper_page_url: &str,
) -> Result<bool> {
    let detail_html = client
        .get(wallpaper_page_url)
        .send()
        .await?
        .text()
        .await?;
    let detail_doc = Html::parse_document(&detail_html);

    if has_forbidden_keyword(&detail_doc, prefs) {
        tracing::debug!("DualMonitor: skipping (forbidden keyword)");
        return Ok(false);
    }

    let full_url = extract_image_url(&detail_doc)?;
    let filename = derive_filename(&full_url);
    let filepath = prefs.downloads.path.join(&filename);

    if filepath.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(&prefs.downloads.path)?;
    let bytes = client.get(&full_url).send().await?.bytes().await?;
    std::fs::write(&filepath, &bytes)?;

    tracing::info!("DualMonitor: saved {}", filepath.display());
    Ok(true)
}

// ─── Keyword Filtering ──────────────────────────────────────────────────────

fn has_forbidden_keyword(doc: &Html, prefs: &Prefs) -> bool {
    let forbidden = prefs.harvester.avoid_keywords.split(';').filter(|s| !s.is_empty());
    if prefs.harvester.avoid_keywords.is_empty() {
        return false;
    }

    let tag_selector = Selector::parse("ul.taglist li a").unwrap();
    doc.select(&tag_selector).any(|tag| {
        let text: String = tag.text().collect::<String>().to_lowercase();
        forbidden.clone().any(|f| text.contains(&f.to_lowercase()))
    })
}

// ─── Image URL Extraction ───────────────────────────────────────────────────

fn extract_image_url(doc: &Html) -> Result<String> {
    let img_selector = Selector::parse("div#SingleImageContainer a").unwrap();
    let images: Vec<_> = doc.select(&img_selector).collect();

    let href = images
        .first()
        .and_then(|el| el.value().attr("href"))
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow::anyhow!("DualMonitor: no image URL found"))?;

    Ok(format!("{}{}", BASE_URL, &href[1..]))
}

// ─── Filename ───────────────────────────────────────────────────────────────

fn derive_filename(url: &str) -> String {
    let base = url.rsplit('/').next().unwrap_or("wallpaper.jpg");
    format!("wd_dm_{}", base)
}
