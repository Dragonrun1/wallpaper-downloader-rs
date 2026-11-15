//! Helpers shared by the providers that read web pages.

use std::{future::Future, time::Duration};

use anyhow::{Context, Result};
use scraper::{Html, Selector};
use url::Url;

use super::{net::check_status, Ctx};

/// How many listing pages one check may read while looking for something new.
pub const MAX_PAGES: u32 = 3;

pub async fn fetch_text(ctx: &Ctx, url: &str) -> Result<String> {
    let response = ctx.client.get(url).send().await.with_context(|| format!("could not reach {url}"))?;
    Ok(check_status(response)?.text().await?)
}

/// Resolve `href` against `base`, whether it is relative or absolute.
pub fn join(base: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    Url::parse(base).ok()?.join(href).ok().map(String::from)
}

/// A stable ID for a page: its path and query, e.g. `wallpaper_view.php-id=12`.
pub fn url_id(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else { return url.to_owned() };
    let mut id = parsed.path().trim_matches('/').replace('/', "_");
    if let Some(query) = parsed.query() {
        id.push('-');
        id.push_str(query);
    }
    if id.is_empty() {
        url.to_owned()
    } else {
        id
    }
}

/// The text of every element matching `css`, trimmed, blanks dropped.
pub fn texts(doc: &Html, css: &str) -> Vec<String> {
    let Ok(selector) = Selector::parse(css) else { return Vec::new() };
    doc.select(&selector)
        .map(|e| e.text().collect::<String>().trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect()
}

/// The `attr` attribute of every element matching `css`.
pub fn attrs(doc: &Html, css: &str, attr: &str) -> Vec<String> {
    let Ok(selector) = Selector::parse(css) else { return Vec::new() };
    doc.select(&selector)
        .filter_map(|e| e.value().attr(attr))
        .map(str::to_owned)
        .collect()
}

/// Read listing pages one after another, stopping at the first page that has
/// something not already in the library (or after `MAX_PAGES`). `fetch` returns
/// the detail-page links found on one page; `id_of` turns a link into the ID
/// the library knows it by.
pub async fn paged_links<F, Fut>(
    ctx: &Ctx,
    pause: Duration,
    first_page: u32,
    id_of: fn(&str) -> String,
    fetch: F,
) -> Result<Vec<String>>
where
    F: Fn(u32) -> Fut + Send,
    Fut: Future<Output = Result<Vec<String>>> + Send,
{
    let mut found = Vec::new();
    for n in 0..MAX_PAGES {
        if n > 0 {
            tokio::time::sleep(pause).await;
        }
        let links = fetch(first_page + n).await?;
        if links.is_empty() {
            break;
        }
        let fresh = links.iter().any(|link| !ctx.is_known(&id_of(link)));
        found.extend(links);
        if fresh {
            break;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn join_handles_relative_and_absolute_links() {
        let base = "http://example.com/";
        assert_eq!(join(base, "/a/b.php").as_deref(), Some("http://example.com/a/b.php"));
        assert_eq!(join(base, "a/b.php").as_deref(), Some("http://example.com/a/b.php"));
        assert_eq!(join(base, "https://other.org/x").as_deref(), Some("https://other.org/x"));
        assert_eq!(join(base, "  "), None);
    }

    #[test]
    fn url_ids_are_stable_and_filename_friendly() {
        assert_eq!(url_id("http://example.com/wallpaper/view.php?id=12"), "wallpaper_view.php-id=12");
        assert_eq!(url_id("http://example.com/wallpapers/cat/"), "wallpapers_cat");
        assert_eq!(url_id("http://example.com/"), "http://example.com/");
    }

    #[test]
    fn selectors_pick_text_and_attributes() {
        let doc = Html::parse_document(r#"<ul class="t"><li><a href="/x"> Space </a></li><li><a href="/y"> </a></li></ul>"#);
        assert_eq!(texts(&doc, "ul.t li a"), vec!["Space"]);
        assert_eq!(attrs(&doc, "ul.t li a", "href"), vec!["/x", "/y"]);
        assert!(texts(&doc, "!!not a selector").is_empty());
    }

    #[tokio::test]
    async fn paging_stops_at_the_first_page_with_something_new() {
        let mut ctx = crate::providers::test_ctx();
        ctx.known = Arc::new(|id| id == "old1" || id == "old2");
        let id_of: fn(&str) -> String = |link| link.to_owned();

        // page 1 is all known, page 2 has something new, page 3 must not be read
        let pages = std::sync::Mutex::new(Vec::new());
        let links = paged_links(&ctx, Duration::ZERO, 1, id_of, |page| {
            pages.lock().unwrap().push(page);
            async move {
                Ok(match page {
                    1 => vec!["old1".to_owned(), "old2".to_owned()],
                    2 => vec!["old1".to_owned(), "new".to_owned()],
                    _ => vec!["never".to_owned()],
                })
            }
        })
            .await
            .unwrap();
        assert_eq!(links, vec!["old1", "old2", "old1", "new"]);
        assert_eq!(*pages.lock().unwrap(), vec![1, 2]);
    }

    #[tokio::test]
    async fn paging_is_bounded_and_stops_on_an_empty_page() {
        let mut ctx = crate::providers::test_ctx();
        ctx.known = Arc::new(|_| true); // everything is old
        let id_of: fn(&str) -> String = |link| link.to_owned();

        let links = paged_links(&ctx, Duration::ZERO, 0, id_of, |page| async move { Ok(vec![format!("p{page}")]) })
            .await
            .unwrap();
        assert_eq!(links.len(), MAX_PAGES as usize);

        let links = paged_links(&ctx, Duration::ZERO, 0, id_of, |_| async { Ok(Vec::new()) }).await.unwrap();
        assert!(links.is_empty());
    }
}
