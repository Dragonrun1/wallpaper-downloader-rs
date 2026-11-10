use anyhow::Result;
use scraper::{Html, Selector};
use crate::prefs::Prefs;
use super::{build_client, download_and_save, has_forbidden_keyword};

const BASE_URL: &str = "https://www.wallpaperfusion.com/";
const DOWNLOAD_URL: &str = "https://www.wallpaperfusion.com/Download/";
const SEARCH_URL: &str = "https://www.wallpaperfusion.com/Ajax/BuildSearchResultsHTML/";
const ORIGINAL_PARAMS: &str = "?W=-1&H=-1";

pub async fn harvest(prefs: &Prefs) -> Result<()> {
    let client = build_client()?;
    let mut page: u32 = 0;

    loop {
        let html = fetch_listing(&client, prefs, page).await?;
        let document = Html::parse_document(&html);

        let links = get_wallpaper_links(&document);
        if links.is_empty() {
            anyhow::bail!("WallpaperFusion: no new wallpapers found");
        }

        for link in &links {
            if download_wallpaper(&client, prefs, link).await? {
                return Ok(());
            }
        }

        page += 1;
    }
}

async fn fetch_listing(client: &reqwest::Client, prefs: &Prefs, page: u32) -> Result<String> {
    let keywords = &prefs.providers.wallhaven.keywords;
    let page_str = page.to_string();
    let sort = prefs.providers.wallpaper_fusion.sort.as_param();
    let mon = prefs.providers.wallpaper_fusion.monitors.to_string();

    if keywords.is_empty() {
        let html = client
            .post(BASE_URL)
            .form(&[("Page", page_str.as_str())])
            .send()
            .await?
            .text()
            .await?;
        Ok(html)
    } else {
        let req_id = (page + 3).to_string();
        let html = client
            .post(SEARCH_URL)
            .form(&[
                ("ReqID", req_id.as_str()),
                ("Page", page_str.as_str()),
                ("Sort", sort),
                ("Text", keywords.as_str()),
                ("TagShow", ""),
                ("TagHide", ""),
                ("Mon", mon.as_str()),
                ("RateAll", "0"),
                ("RateUser", "undefined"),
                ("SearchWidth", "0"),
                ("SearchWidthMod", "0"),
                ("SearchHeight", "0"),
                ("SearchHeightMod", "0"),
                ("SubmittersMode", "undefined"),
                ("ArtistsMode", "undefined"),
                ("ShowFavs", "0"),
                ("TagSearchMode", "0"),
            ])
            .send()
            .await?
            .text()
            .await?;
        Ok(html)
    }
}

fn get_wallpaper_links(document: &Html) -> Vec<String> {
    let selector = Selector::parse("a.ImageThumbBorder").unwrap();
    document
        .select(&selector)
        .filter_map(|el| el.value().attr("href").map(|h| h.to_string()))
        .collect()
}

async fn download_wallpaper(
    client: &reqwest::Client,
    prefs: &Prefs,
    detail_url: &str,
) -> Result<bool> {
    let detail_html = client
        .get(detail_url)
        .send()
        .await?
        .text()
        .await?;
    let doc = Html::parse_document(&detail_html);

    if has_forbidden_keyword(&doc, "a.WPFTag", prefs) {
        return Ok(false);
    }

    let id = extract_wallpaper_id(detail_url);
    if id.is_empty() {
        return Ok(false);
    }

    let filename = format!("wd_wf-{id}.jpg");
    let download_url = build_download_url(prefs, id);

    download_and_save(client, prefs, &download_url, &filename).await
}

fn build_download_url(prefs: &Prefs, id: &str) -> String {
    match prefs.providers.download_policy {
        crate::prefs::DownloadPolicy::MatchResolution => {
            format!("{DOWNLOAD_URL}{id}/?W={}&H={}", prefs.resolution.width, prefs.resolution.height)
        }
        _ => {
            format!("{DOWNLOAD_URL}{id}/{ORIGINAL_PARAMS}")
        }
    }
}

fn extract_wallpaper_id(url: &str) -> &str {
    url.trim_end_matches('/').rsplit('/').next().unwrap_or("")
}
