use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use scraper::Html;

use super::scrape::{attrs, fetch_text, join, paged_links, url_id};
use super::{Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, Source, Wallpaper};

const BASE: &str = "http://socialwallpapering.com/";

pub struct SocialWallpapering;

fn listing_url(keywords: &str, page: u32) -> String {
    let words: Vec<String> = keywords
        .replace('"', "")
        .split_whitespace()
        .map(|w| url::form_urlencoded::byte_serialize(w.as_bytes()).collect())
        .collect();
    if words.is_empty() {
        format!("{BASE}wallpapers/page:{page}/")
    } else {
        format!("{BASE}wallpapers/page:{page}/search:{}/", words.join("+"))
    }
}

fn parse_listing(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    attrs(&doc, "li.wallpaper a", "href")
        .into_iter()
        .filter_map(|href| join(BASE, &href))
        .collect()
}

/// The wallpaper is the second image on its page (the first is site chrome).
fn parse_image(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let srcs = attrs(&doc, "img", "src");
    join(BASE, srcs.get(1)?)
}

#[async_trait]
impl Source for SocialWallpapering {
    fn id(&self) -> ProviderId {
        ProviderId::SocialWallpapering
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[],
            keywords: true,
            api_key: KeyUse::None,
            key_label: "",
            secret_label: None,
            market: false,
            purity: false,
            help: "",
        }
    }

    /// A small hobby site: be gentle.
    fn min_interval(&self, _settings: &ProviderSettings) -> Duration {
        Duration::from_secs(3600)
    }

    fn pause(&self) -> Duration {
        Duration::from_secs(1)
    }

    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let links = paged_links(ctx, self.pause(), 1, url_id, |page| async move {
            let html = fetch_text(ctx, &listing_url(&ctx.keywords, page)).await?;
            Ok(parse_listing(&html))
        })
            .await?;

        Ok(links
            .into_iter()
            .map(|link| {
                let mut w = Wallpaper::new(ProviderId::SocialWallpapering, url_id(&link), "");
                w.page_url = Some(link);
                w
            })
            .collect())
    }

    async fn details(&self, ctx: &Ctx, mut wallpaper: Wallpaper) -> Result<Wallpaper> {
        let page = wallpaper.page_url.clone().ok_or_else(|| anyhow!("no page URL"))?;
        wallpaper.image_url =
            parse_image(&fetch_text(ctx, &page).await?).ok_or_else(|| anyhow!("no image found on {page}"))?;
        Ok(wallpaper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_urls() {
        assert_eq!(listing_url("", 1), "http://socialwallpapering.com/wallpapers/page:1/");
        assert_eq!(
            listing_url("red \"fox\" run", 2),
            "http://socialwallpapering.com/wallpapers/page:2/search:red+fox+run/"
        );
    }

    #[test]
    fn links_and_the_second_image() {
        let listing = r#"<ul><li class="wallpaper"><a href="wallpaper/12/fox">f</a></li>
            <li class="other"><a href="nope">n</a></li></ul>"#;
        assert_eq!(parse_listing(listing), vec!["http://socialwallpapering.com/wallpaper/12/fox"]);

        let page = r#"<img src="logo.png"><img src="files/fox-1920x1080.jpg">"#;
        assert_eq!(
            parse_image(page).as_deref(),
            Some("http://socialwallpapering.com/files/fox-1920x1080.jpg")
        );
        assert_eq!(parse_image(r#"<img src="logo.png">"#), None);
    }
}
