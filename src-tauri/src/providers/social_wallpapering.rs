use super::{build_client, download_and_save};
use crate::prefs::Prefs;
use anyhow::Result;
use scraper::{Html, Selector};

const BASE_URL: &str = "http://socialwallpapering.com/";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;
    let mut page = 1;

    loop {
        let url = build_url(prefs, page);
        tracing::info!("SocialWallpapering: fetching {}", url);

        let html = client.get(&url).send().await?.text().await?;
        let document = Html::parse_document(&html);

        let links = get_wallpaper_links(&document);
        if links.is_empty() {
            anyhow::bail!("SocialWallpapering: no new wallpapers found");
        }

        for link in &links {
            if download_wallpaper(&client, prefs, link).await? {
                return Ok(());
            }
        }

        page += 1;
    }
}

fn build_url(prefs: &Prefs, page: usize) -> String {
    let use_keywords = !prefs.providers.wallhaven.keywords.is_empty()
        && !prefs.providers.social_wallpapering.ignore_keywords;

    if use_keywords {
        let kw = prefs
            .providers
            .wallhaven
            .keywords
            .replace(' ', "+")
            .replace('"', "");
        format!("{BASE_URL}wallpapers/page:{page}/search:{kw}/")
    } else {
        format!("{BASE_URL}wallpapers/page:{page}/")
    }
}

fn get_wallpaper_links(document: &Html) -> Vec<String> {
    let li_selector = Selector::parse("li.wallpaper a").unwrap();
    document
        .select(&li_selector)
        .filter_map(|el| {
            el.value()
                .attr("href")
                .map(|href| format!("{BASE_URL}{href}"))
        })
        .collect()
}

async fn download_wallpaper(
    client: &reqwest::Client,
    prefs: &Prefs,
    detail_url: &str,
) -> Result<bool> {
    let detail_html = client.get(detail_url).send().await?.text().await?;
    let doc = Html::parse_document(&detail_html);

    let full_url = extract_image_url(&doc)?;
    let base = full_url.rsplit('/').next().unwrap_or("wallpaper.jpg");
    let name = base.split('-').next().unwrap_or(base);
    let filename = format!("wd_sw_{name}.jpg");

    download_and_save(client, prefs, &full_url, &filename).await
}

fn extract_image_url(doc: &Html) -> Result<String> {
    let img_selector = Selector::parse("img").unwrap();
    let images: Vec<_> = doc.select(&img_selector).collect();

    let src = images
        .get(1)
        .and_then(|el| el.value().attr("src"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("SocialWallpapering: no image found"))?;

    Ok(format!("{BASE_URL}{src}"))
}
