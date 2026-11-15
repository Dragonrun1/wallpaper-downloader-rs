use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

use super::net::check_status;
use super::{Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, Source, Wallpaper};

const API: &str = "https://api.unsplash.com";
const KEY_ENV: &str = "UNSPLASH_ACCESS_KEY";

pub struct Unsplash;

#[derive(Deserialize)]
struct Photo {
    id: String,
    width: Option<u32>,
    height: Option<u32>,
    description: Option<String>,
    alt_description: Option<String>,
    urls: Urls,
    links: Option<Links>,
    user: Option<User>,
    likes: Option<u32>,
    #[serde(default)]
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct Urls {
    full: String,
}

#[derive(Deserialize)]
struct Links {
    html: Option<String>,
}

#[derive(Deserialize)]
struct User {
    name: Option<String>,
}

#[derive(Deserialize)]
struct Tag {
    title: Option<String>,
}

/// The access key from the settings, else from `UNSPLASH_ACCESS_KEY`.
fn access_key(settings: &ProviderSettings, env: Option<String>) -> Option<String> {
    settings
        .api_key
        .clone()
        .or(env)
        .map(|k| k.trim().to_owned())
        .filter(|k| !k.is_empty())
}

fn key_for(settings: &ProviderSettings) -> Result<String> {
    access_key(settings, std::env::var(KEY_ENV).ok()).ok_or_else(|| {
        anyhow!("Unsplash needs an access key: create an app at unsplash.com/developers and enter its Access Key")
    })
}

fn parse_photos(json: &str) -> Result<Vec<Wallpaper>> {
    let photos: Vec<Photo> = serde_json::from_str(json)?;
    Ok(photos
        .into_iter()
        .map(|p| {
            let mut w = Wallpaper::new(ProviderId::Unsplash, p.id, p.urls.full);
            w.width = p.width;
            w.height = p.height;
            w.title = p.description.or(p.alt_description).filter(|t| !t.trim().is_empty());
            w.author = p.user.and_then(|u| u.name);
            w.page_url = p.links.and_then(|l| l.html);
            w.favorites = p.likes;
            w.tags = p.tags.into_iter().filter_map(|t| t.title).collect();
            w
        })
        .collect())
}

#[async_trait]
impl Source for Unsplash {
    fn id(&self) -> ProviderId {
        ProviderId::Unsplash
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[],
            keywords: true,
            api_key: KeyUse::Required,
            key_label: "Access key",
            secret_label: None,
            market: false,
            purity: false,
            help: "Needs a free app from unsplash.com/developers. New apps are limited to 50 requests an hour.",
        }
    }

    /// A check costs two requests (the list, then the download ping) out of
    /// 50 an hour, so this leaves plenty of room.
    fn min_interval(&self, _settings: &ProviderSettings) -> Duration {
        Duration::from_secs(5 * 60)
    }

    /// Ten random landscape photos in one request, so already-downloaded ones
    /// can be skipped without spending more of the hourly quota.
    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let key = key_for(&ctx.settings)?;
        let mut query = vec![("count", "10".to_owned()), ("orientation", "landscape".to_owned())];
        if !ctx.keywords.trim().is_empty() {
            query.push(("query", ctx.keywords.clone()));
        }
        let response = ctx
            .client
            .get(format!("{API}/photos/random"))
            .header(AUTHORIZATION, format!("Client-ID {key}"))
            .query(&query)
            .send()
            .await?;
        parse_photos(&check_status(response)?.text().await?)
    }

    /// Unsplash's guidelines ask apps to report each download.
    async fn details(&self, ctx: &Ctx, wallpaper: Wallpaper) -> Result<Wallpaper> {
        let key = key_for(&ctx.settings)?;
        let response = ctx
            .client
            .get(format!("{API}/photos/{}/download", wallpaper.id))
            .header(AUTHORIZATION, format!("Client-ID {key}"))
            .send()
            .await?;
        check_status(response)?;
        Ok(wallpaper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
        {"id":"abc123","width":6000,"height":4000,"description":"A misty lake","alt_description":"lake at dawn",
         "urls":{"raw":"https://images.unsplash.com/photo-1?ixid=x","full":"https://images.unsplash.com/photo-1?fm=jpg&ixid=x"},
         "links":{"html":"https://unsplash.com/photos/abc123","download_location":"https://api.unsplash.com/photos/abc123/download"},
         "user":{"name":"Jane Photographer","username":"jane"},"likes":42,"tags":[{"title":"lake"},{"title":"mist"}]},
        {"id":"def456","width":4000,"height":6000,"description":null,"alt_description":"a tall tree",
         "urls":{"full":"https://images.unsplash.com/photo-2"},"user":null}
    ]"#;

    #[test]
    fn photos_become_wallpapers_with_credit() {
        let photos = parse_photos(SAMPLE).unwrap();
        assert_eq!(photos.len(), 2);

        let first = &photos[0];
        assert_eq!(first.id, "abc123");
        assert_eq!(first.image_url, "https://images.unsplash.com/photo-1?fm=jpg&ixid=x");
        assert_eq!((first.width, first.height), (Some(6000), Some(4000)));
        assert_eq!(first.title.as_deref(), Some("A misty lake"));
        assert_eq!(first.author.as_deref(), Some("Jane Photographer"));
        assert_eq!(first.page_url.as_deref(), Some("https://unsplash.com/photos/abc123"));
        assert_eq!(first.tags, vec!["lake", "mist"]);
        assert_eq!(first.favorites, Some(42));

        let second = &photos[1];
        assert_eq!(second.title.as_deref(), Some("a tall tree")); // falls back to the alt text
        assert_eq!(second.author, None);
        assert!(second.tags.is_empty());
    }

    #[test]
    fn a_single_object_or_garbage_is_an_error_not_a_panic() {
        assert!(parse_photos(r#"{"errors":["Unauthorized"]}"#).is_err());
        assert!(parse_photos("not json").is_err());
        assert!(parse_photos("[]").unwrap().is_empty());
    }

    #[test]
    fn the_key_comes_from_settings_then_the_environment() {
        let with = ProviderSettings { api_key: Some(" k1 ".into()), ..Default::default() };
        let without = ProviderSettings::default();
        assert_eq!(access_key(&with, Some("k2".into())).as_deref(), Some("k1"));
        assert_eq!(access_key(&without, Some("k2".into())).as_deref(), Some("k2"));
        assert_eq!(access_key(&without, None), None);
        assert_eq!(access_key(&ProviderSettings { api_key: Some("  ".into()), ..Default::default() }, None), None);
    }

    #[tokio::test]
    async fn without_a_key_it_says_what_to_do_and_makes_no_request() {
        if std::env::var(KEY_ENV).is_ok() {
            return; // a real key in the environment would make this a live call
        }
        let error = Unsplash.list(&crate::providers::test_ctx()).await.unwrap_err();
        assert!(error.to_string().contains("access key"));
    }
}
