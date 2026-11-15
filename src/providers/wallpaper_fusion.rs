use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use scraper::Html;

use super::net::check_status;
use super::scrape::{attrs, fetch_text, join, paged_links, texts};
use super::{
    Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, ResolutionFilter, ResolutionPolicy, Sort, Source,
    Wallpaper,
};

const BASE: &str = "https://www.wallpaperfusion.com/";
const DOWNLOAD: &str = "https://www.wallpaperfusion.com/Download/";
const SEARCH: &str = "https://www.wallpaperfusion.com/Ajax/BuildSearchResultsHTML/";

pub struct WallpaperFusion;

fn sort_param(sort: Option<Sort>) -> &'static str {
    match sort {
        Some(Sort::Newest) => "4",
        Some(Sort::Top) | Some(Sort::Views) => "1",
        _ => "0",
    }
}

/// Listings are fetched with POST. Without a keyword it is the front page.
async fn fetch_listing(ctx: &Ctx, page: u32) -> Result<String> {
    let page_text = page.to_string();
    let request = if ctx.keywords.trim().is_empty() {
        ctx.client.post(BASE).form(&[("Page", page_text.as_str())])
    } else {
        let req_id = (page + 3).to_string();
        ctx.client.post(SEARCH).form(&[
            ("ReqID", req_id.as_str()),
            ("Page", page_text.as_str()),
            ("Sort", sort_param(ctx.settings.sort)),
            ("Text", ctx.keywords.as_str()),
            ("TagShow", ""),
            ("TagHide", ""),
            ("Mon", "0"),
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
    };
    Ok(check_status(request.send().await?)?.text().await?)
}

fn parse_listing(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    attrs(&doc, "a.ImageThumbBorder", "href")
        .into_iter()
        .filter_map(|href| join(BASE, &href))
        .collect()
}

fn parse_tags(html: &str) -> Vec<String> {
    texts(&Html::parse_document(html), "a.WPFTag")
}

/// The last part of the page URL is the wallpaper's number.
fn wallpaper_id(url: &str) -> String {
    url.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_owned()
}

/// Original size, or exactly the largest target when the policy is "exact".
fn download_url(id: &str, filter: &ResolutionFilter) -> String {
    match (filter.policy, filter.largest()) {
        (ResolutionPolicy::Exact, Some(t)) => format!("{DOWNLOAD}{id}/?W={}&H={}", t.width, t.height),
        _ => format!("{DOWNLOAD}{id}/?W=-1&H=-1"),
    }
}

#[async_trait]
impl Source for WallpaperFusion {
    fn id(&self) -> ProviderId {
        ProviderId::WallpaperFusion
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[Sort::Relevance, Sort::Top, Sort::Newest],
            keywords: true,
            api_key: KeyUse::None,
            key_label: "",
            secret_label: None,
            market: false,
            purity: false,
            help: "Sorting only applies when there is a keyword.",
        }
    }

    /// A small site: be gentle.
    fn min_interval(&self, _settings: &ProviderSettings) -> Duration {
        Duration::from_secs(3600)
    }

    fn pause(&self) -> Duration {
        Duration::from_secs(1)
    }

    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let links = paged_links(ctx, self.pause(), 0, wallpaper_id, |page| async move {
            let html = fetch_listing(ctx, page).await?;
            Ok(parse_listing(&html))
        })
            .await?;

        Ok(links
            .into_iter()
            .filter_map(|link| {
                let id = wallpaper_id(&link);
                (!id.is_empty()).then(|| {
                    let mut w = Wallpaper::new(ProviderId::WallpaperFusion, id, "");
                    w.page_url = Some(link);
                    w
                })
            })
            .collect())
    }

    async fn details(&self, ctx: &Ctx, mut wallpaper: Wallpaper) -> Result<Wallpaper> {
        let page = wallpaper.page_url.clone().ok_or_else(|| anyhow!("no page URL"))?;
        wallpaper.tags = parse_tags(&fetch_text(ctx, &page).await?);
        wallpaper.image_url = download_url(&wallpaper.id, &ctx.resolution);
        Ok(wallpaper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Resolution;

    #[test]
    fn listing_links_tags_and_ids() {
        let listing = r#"<a class="ImageThumbBorder" href="/Wallpaper/12345/"><img></a>
            <a class="ImageThumbBorder" href="https://www.wallpaperfusion.com/Wallpaper/678/"></a>
            <a class="other" href="/x/"></a>"#;
        let links = parse_listing(listing);
        assert_eq!(
            links,
            vec!["https://www.wallpaperfusion.com/Wallpaper/12345/", "https://www.wallpaperfusion.com/Wallpaper/678/"]
        );
        assert_eq!(wallpaper_id(&links[0]), "12345");
        assert_eq!(parse_tags(r#"<a class="WPFTag"> Mountains </a><a class="WPFTag">Snow</a>"#), vec!["Mountains", "Snow"]);
    }

    #[test]
    fn download_urls_follow_the_resolution_policy() {
        let filter = |policy| ResolutionFilter { policy, targets: vec![Resolution::new(1920, 1080), Resolution::new(2560, 1440)] };
        assert_eq!(download_url("9", &filter(ResolutionPolicy::AtLeast)), "https://www.wallpaperfusion.com/Download/9/?W=-1&H=-1");
        assert_eq!(download_url("9", &filter(ResolutionPolicy::Exact)), "https://www.wallpaperfusion.com/Download/9/?W=2560&H=1440");
    }

    #[test]
    fn sort_parameters() {
        assert_eq!(sort_param(None), "0");
        assert_eq!(sort_param(Some(Sort::Newest)), "4");
        assert_eq!(sort_param(Some(Sort::Top)), "1");
    }
}
