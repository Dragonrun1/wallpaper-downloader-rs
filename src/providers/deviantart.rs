use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;

use super::net::check_status_with_body;
use super::{Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, Sort, Source, Wallpaper};

const TOKEN_URL: &str = "https://www.deviantart.com/oauth2/token";
const API: &str = "https://www.deviantart.com/api/v1/oauth2";

pub struct DeviantArt;

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

#[derive(Deserialize)]
struct Browse {
    #[serde(default)]
    results: Vec<Deviation>,
}

#[derive(Deserialize)]
struct Deviation {
    deviationid: String,
    url: Option<String>,
    title: Option<String>,
    #[serde(default)]
    is_mature: bool,
    author: Option<Author>,
    content: Option<Content>,
}

#[derive(Deserialize)]
struct Author {
    username: Option<String>,
}

#[derive(Deserialize)]
struct Content {
    src: String,
    width: Option<u32>,
    height: Option<u32>,
}

/// DeviantArt removed its "popular" and "newest" browse endpoints, so we browse
/// a tag instead. Tags are single words, so a keyword has its spaces removed;
/// with none, the "wallpaper" tag is used.
fn tag_for(keywords: &str) -> String {
    let tag: String = keywords.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect();
    if tag.is_empty() {
        "wallpaper".to_owned()
    } else {
        tag
    }
}

/// `mode` picks popular or newest first. It isn't in DeviantArt's own reference,
/// but another client library documents it for this endpoint, and an unknown
/// parameter is simply ignored.
fn browse_query(keywords: &str, sort: Option<Sort>) -> Vec<(&'static str, String)> {
    let mode = if sort == Some(Sort::Newest) { "newest" } else { "popular" };
    vec![
        ("tag", tag_for(keywords)),
        ("mode", mode.to_owned()),
        ("limit", "24".to_owned()),
        ("mature_content", "false".to_owned()),
    ]
}

fn credentials(settings: &ProviderSettings) -> Result<(String, String)> {
    let clean = |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    match (clean(&settings.api_key), clean(&settings.api_secret)) {
        (Some(id), Some(secret)) => Ok((id, secret)),
        _ => Err(anyhow!(
            "DeviantArt needs a client ID and secret: register an app at deviantart.com/developers"
        )),
    }
}

/// Deviations without an image (journals, literature) and mature ones are dropped.
fn parse_browse(json: &str) -> Result<Vec<Wallpaper>> {
    let browse: Browse = serde_json::from_str(json)?;
    Ok(browse
        .results
        .into_iter()
        .filter(|d| !d.is_mature)
        .filter_map(|d| {
            let content = d.content?;
            let mut w = Wallpaper::new(ProviderId::DeviantArt, d.deviationid, content.src);
            w.width = content.width;
            w.height = content.height;
            w.title = d.title.filter(|t| !t.trim().is_empty());
            w.author = d.author.and_then(|a| a.username);
            w.page_url = d.url;
            Some(w)
        })
        .collect())
}

#[async_trait]
impl Source for DeviantArt {
    fn id(&self) -> ProviderId {
        ProviderId::DeviantArt
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[Sort::Top, Sort::Newest],
            keywords: true,
            api_key: KeyUse::Required,
            key_label: "Client ID",
            secret_label: Some("Client secret"),
            market: false,
            purity: false,
            help: "Needs a free app from deviantart.com/developers. Browses a tag: your keyword with spaces removed, or \"wallpaper\".",
        }
    }

    /// Each check costs two requests: a token, then the listing.
    fn min_interval(&self, _settings: &ProviderSettings) -> Duration {
        Duration::from_secs(15 * 60)
    }

    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let (client_id, client_secret) = credentials(&ctx.settings)?;

        let response = ctx
            .client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
            ])
            .send()
            .await?;
        let token: Token = check_status_with_body(response).await?.json().await?;

        let response = ctx
            .client
            .get(format!("{API}/browse/tags"))
            .bearer_auth(&token.access_token)
            .query(&browse_query(&ctx.keywords, ctx.settings.sort))
            .send()
            .await?;
        parse_browse(&check_status_with_body(response).await?.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"has_more":true,"next_offset":24,"results":[
        {"deviationid":"AAA-111","url":"https://www.deviantart.com/artist/art/Sunset-1","title":"Sunset",
         "is_mature":false,"author":{"username":"artist"},
         "content":{"src":"https://images-wixmp.example/sunset.jpg","width":3840,"height":2160,"filesize":123}},
        {"deviationid":"BBB-222","title":"A story","is_mature":false,"author":{"username":"writer"}},
        {"deviationid":"CCC-333","title":"Spicy","is_mature":true,
         "content":{"src":"https://images-wixmp.example/spicy.jpg","width":1920,"height":1080}}
    ]}"#;

    #[test]
    fn only_non_mature_deviations_with_an_image_are_kept() {
        let found = parse_browse(SAMPLE).unwrap();
        assert_eq!(found.len(), 1);
        let w = &found[0];
        assert_eq!(w.id, "AAA-111");
        assert_eq!(w.image_url, "https://images-wixmp.example/sunset.jpg");
        assert_eq!((w.width, w.height), (Some(3840), Some(2160)));
        assert_eq!(w.title.as_deref(), Some("Sunset"));
        assert_eq!(w.author.as_deref(), Some("artist"));
        assert_eq!(w.page_url.as_deref(), Some("https://www.deviantart.com/artist/art/Sunset-1"));
    }

    #[test]
    fn empty_and_malformed_responses() {
        assert!(parse_browse(r#"{"results":[]}"#).unwrap().is_empty());
        assert!(parse_browse("{}").unwrap().is_empty());
        assert!(parse_browse("nope").is_err());
    }

    #[test]
    fn keywords_become_a_single_tag() {
        assert_eq!(tag_for(""), "wallpaper");
        assert_eq!(tag_for("   "), "wallpaper");
        assert_eq!(tag_for("Deep Space"), "deepspace");
        assert_eq!(tag_for("stars"), "stars");
    }

    #[test]
    fn the_browse_query_asks_for_a_tag_in_the_chosen_order() {
        let get = |q: &[(&'static str, String)], key: &str| q.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone());
        let popular = browse_query("stars", None);
        assert_eq!(get(&popular, "tag").as_deref(), Some("stars"));
        assert_eq!(get(&popular, "mode").as_deref(), Some("popular"));
        assert_eq!(get(&popular, "limit").as_deref(), Some("24"));
        assert_eq!(get(&popular, "mature_content").as_deref(), Some("false"));
        assert_eq!(get(&browse_query("", Some(Sort::Newest)), "mode").as_deref(), Some("newest"));
        assert_eq!(get(&browse_query("", Some(Sort::Top)), "mode").as_deref(), Some("popular"));
        assert_eq!(get(&popular, "q"), None); // the old search parameter is gone
    }

    #[test]
    fn both_credentials_are_required() {
        let s = |id: Option<&str>, secret: Option<&str>| ProviderSettings {
            api_key: id.map(str::to_owned),
            api_secret: secret.map(str::to_owned),
            ..Default::default()
        };
        assert!(credentials(&s(Some("id"), Some("secret"))).is_ok());
        assert!(credentials(&s(Some("id"), None)).is_err());
        assert!(credentials(&s(None, Some("secret"))).is_err());
        assert!(credentials(&s(Some(" "), Some("secret"))).is_err());
    }

    #[tokio::test]
    async fn without_credentials_it_says_what_to_do() {
        let error = DeviantArt.list(&crate::providers::test_ctx()).await.unwrap_err();
        assert!(error.to_string().contains("client ID and secret"));
    }
}
