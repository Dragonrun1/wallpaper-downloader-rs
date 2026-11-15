use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use scraper::Html;

use super::scrape::{attrs, fetch_text, join, paged_links, texts, url_id};
use super::{Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, Sort, Source, Wallpaper};

const BASE: &str = "http://www.dualmonitorbackgrounds.com/";

pub struct DualMonitor;

/// The `sortby` parameter, and the id of the page section that lists the result.
fn sort_params(sort: Option<Sort>) -> (&'static str, &'static str) {
    match sort {
        Some(Sort::Top) => ("sortby=rating", "toprated"),
        Some(Sort::Views) => ("sortby=popularity", "popular"),
        _ => ("sortby=date", "latest"),
    }
}

fn listing_url(keywords: &str, sort: Option<Sort>, page: u32) -> String {
    if keywords.trim().is_empty() {
        format!("{BASE}index.php?{}&latestImagesPage={page}", sort_params(sort).0)
    } else {
        let words: Vec<String> = keywords
            .split_whitespace()
            .map(|w| url::form_urlencoded::byte_serialize(w.as_bytes()).collect())
            .collect();
        format!("{BASE}page/search/{}/{page}", words.join("+AND+"))
    }
}

fn listing_container(keywords: &str, sort: Option<Sort>) -> &'static str {
    if keywords.trim().is_empty() {
        sort_params(sort).1
    } else {
        "albumsSearch"
    }
}

fn parse_listing(html: &str, container: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    attrs(&doc, &format!("div#{container} a"), "href")
        .into_iter()
        .filter(|href| href.contains(".php"))
        .filter_map(|href| join(BASE, &href))
        .collect()
}

struct Detail {
    image: Option<String>,
    tags: Vec<String>,
}

fn parse_detail(html: &str) -> Detail {
    let doc = Html::parse_document(html);
    Detail {
        image: attrs(&doc, "div#SingleImageContainer a", "href")
            .into_iter()
            .find(|href| !href.trim().is_empty())
            .and_then(|href| join(BASE, &href)),
        tags: texts(&doc, "ul.taglist li a"),
    }
}

#[async_trait]
impl Source for DualMonitor {
    fn id(&self) -> ProviderId {
        ProviderId::DualMonitor
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[Sort::Newest, Sort::Top, Sort::Views],
            keywords: true,
            api_key: KeyUse::None,
            key_label: "",
            secret_label: None,
            market: false,
            purity: false,
            help: "Images made for two monitors side by side.",
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
        let sort = ctx.settings.sort;
        let container = listing_container(&ctx.keywords, sort);
        let links = paged_links(ctx, self.pause(), 1, url_id, |page| async move {
            let html = fetch_text(ctx, &listing_url(&ctx.keywords, sort, page)).await?;
            Ok(parse_listing(&html, container))
        })
            .await?;

        Ok(links
            .into_iter()
            .map(|link| {
                let mut w = Wallpaper::new(ProviderId::DualMonitor, url_id(&link), "");
                w.page_url = Some(link);
                w
            })
            .collect())
    }

    async fn details(&self, ctx: &Ctx, mut wallpaper: Wallpaper) -> Result<Wallpaper> {
        let page = wallpaper.page_url.clone().ok_or_else(|| anyhow!("no page URL"))?;
        let detail = parse_detail(&fetch_text(ctx, &page).await?);
        wallpaper.image_url = detail.image.ok_or_else(|| anyhow!("no image URL found on {page}"))?;
        wallpaper.tags = detail.tags;
        Ok(wallpaper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_for_each_kind_of_listing() {
        assert_eq!(
            listing_url("", None, 2),
            "http://www.dualmonitorbackgrounds.com/index.php?sortby=date&latestImagesPage=2"
        );
        assert!(listing_url("", Some(Sort::Top), 1).contains("sortby=rating"));
        assert_eq!(
            listing_url("deep space", None, 3),
            "http://www.dualmonitorbackgrounds.com/page/search/deep+AND+space/3"
        );
        assert_eq!(listing_container("", Some(Sort::Views)), "popular");
        assert_eq!(listing_container("space", Some(Sort::Views)), "albumsSearch");
    }

    #[test]
    fn listing_links_come_from_the_right_section() {
        let html = r#"
            <div id="latest"><a href="/wallpaper-a.php">a</a><a href="/about.html">x</a>
              <a href="wallpaper-b.php?id=2">b</a></div>
            <div id="toprated"><a href="/top.php">t</a></div>"#;
        assert_eq!(
            parse_listing(html, "latest"),
            vec![
                "http://www.dualmonitorbackgrounds.com/wallpaper-a.php",
                "http://www.dualmonitorbackgrounds.com/wallpaper-b.php?id=2",
            ]
        );
        assert_eq!(parse_listing(html, "toprated").len(), 1);
        assert!(parse_listing(html, "missing").is_empty());
    }

    #[test]
    fn detail_pages_give_the_image_and_tags() {
        let html = r#"<div id="SingleImageContainer"><a href="/images/full/abc.jpg"><img src="/t.jpg"></a></div>
            <ul class="taglist"><li><a>Space</a></li><li><a>Planets</a></li></ul>"#;
        let detail = parse_detail(html);
        assert_eq!(detail.image.as_deref(), Some("http://www.dualmonitorbackgrounds.com/images/full/abc.jpg"));
        assert_eq!(detail.tags, vec!["Space", "Planets"]);
        assert!(parse_detail("<p>nothing</p>").image.is_none());
    }
}
