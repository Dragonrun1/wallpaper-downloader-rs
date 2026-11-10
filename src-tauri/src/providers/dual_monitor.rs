use super::{build_client, download_and_save, has_forbidden_keyword};
use crate::prefs::Prefs;
use anyhow::Result;
use scraper::{Html, Selector};

const BASE_URL: &str = "http://www.dualmonitorbackgrounds.com";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;
    let mut page = 1;

    loop {
        let url = build_url(prefs, page);
        tracing::info!("DualMonitor: fetching {}", url);

        let html = client.get(&url).send().await?.text().await?;
        let document = Html::parse_document(&html);

        let links = get_thumbnail_links(&document, prefs);
        if links.is_empty() {
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

fn build_url(prefs: &Prefs, page: usize) -> String {
    let keywords = &prefs.providers.wallhaven.keywords;

    if keywords.is_empty() {
        let sorting = sorting_param(prefs);
        format!(
            "{}index.php?{}&latestImagesPage={}",
            BASE_URL, sorting, page
        )
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

async fn download_wallpaper(
    client: &reqwest::Client,
    prefs: &Prefs,
    wallpaper_page_url: &str,
) -> Result<bool> {
    let detail_html = client.get(wallpaper_page_url).send().await?.text().await?;
    let doc = Html::parse_document(&detail_html);

    if has_forbidden_keyword(&doc, "ul.taglist li a", prefs) {
        return Ok(false);
    }

    let full_url = extract_image_url(&doc)?;
    let base = full_url.rsplit('/').next().unwrap_or("wallpaper.jpg");
    let filename = format!("wd_dm_{base}");

    download_and_save(client, prefs, &full_url, &filename).await
}

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
