use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::Deserialize;

use super::{
    Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, ResolutionFilter, ResolutionPolicy, Sort,
    Source, Wallpaper,
};

const BASE_URL: &str = "https://wallhaven.cc/api/v1";

pub struct Wallhaven;

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    url: String,
    path: String,
    dimension_x: Option<u32>,
    dimension_y: Option<u32>,
    favorites: Option<u32>,
}

#[derive(Deserialize)]
struct DetailResponse {
    data: Detail,
}

#[derive(Deserialize)]
struct Detail {
    #[serde(default)]
    tags: Vec<Tag>,
    uploader: Option<Uploader>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[derive(Deserialize)]
struct Uploader {
    username: String,
}

fn get(ctx: &Ctx, url: &str) -> RequestBuilder {
    let request = ctx.client.get(url);
    match ctx.settings.api_key.as_deref().filter(|k| !k.is_empty()) {
        Some(key) => request.header("X-API-Key", key),
        None => request,
    }
}

/// `atleast=WxH` for a lower bound, or `resolutions=WxH,WxH` for exact sizes.
fn resolution_param(filter: &ResolutionFilter) -> Option<(&'static str, String)> {
    match filter.policy {
        ResolutionPolicy::Any => None,
        ResolutionPolicy::AtLeast => filter
            .minimum()
            .map(|m| ("atleast", format!("{}x{}", m.width, m.height))),
        ResolutionPolicy::Exact if filter.targets.is_empty() => None,
        ResolutionPolicy::Exact => Some((
            "resolutions",
            filter
                .targets
                .iter()
                .map(|t| format!("{}x{}", t.width, t.height))
                .collect::<Vec<_>>()
                .join(","),
        )),
    }
}

/// Wallhaven's three-digit purity code: SFW is always on, then sketchy, then NSFW.
/// NSFW is only available with an API key, so it is switched off without one.
fn purity(settings: &ProviderSettings) -> String {
    let has_key = settings.api_key.as_deref().is_some_and(|k| !k.is_empty());
    format!(
        "1{}{}",
        u8::from(settings.allow_sketchy),
        u8::from(settings.allow_nsfw && has_key)
    )
}

fn sorting(sort: Option<Sort>) -> &'static str {
    match sort.unwrap_or(Sort::Random) {
        Sort::Relevance => "relevance",
        Sort::Newest => "date_added",
        Sort::Top => "toplist",
        Sort::Views => "views",
        Sort::Favorites => "favorites",
        Sort::Random => "random",
    }
}

#[async_trait]
impl Source for Wallhaven {
    fn id(&self) -> ProviderId {
        ProviderId::Wallhaven
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[
                Sort::Relevance,
                Sort::Newest,
                Sort::Top,
                Sort::Views,
                Sort::Favorites,
                Sort::Random,
            ],
            keywords: true,
            api_key: KeyUse::Optional,
            market: false,
            purity: true,
        }
    }

    /// Conservative guesses, not published limits: a key probably earns a
    /// higher rate limit, so it gets a shorter minimum.
    fn min_interval(&self, settings: &ProviderSettings) -> Duration {
        let has_key = settings.api_key.as_deref().is_some_and(|k| !k.is_empty());
        Duration::from_secs(if has_key { 5 * 60 } else { 10 * 60 })
    }

    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let mut query = vec![
            ("q", ctx.keywords.clone()),
            ("sorting", sorting(ctx.settings.sort).to_owned()),
            ("purity", purity(&ctx.settings)),
        ];
        query.extend(resolution_param(&ctx.resolution));

        let response: SearchResponse = get(ctx, &format!("{BASE_URL}/search"))
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response
            .data
            .into_iter()
            .map(|item| {
                let mut w = Wallpaper::new(ProviderId::Wallhaven, item.id, item.path);
                w.page_url = Some(item.url);
                w.width = item.dimension_x;
                w.height = item.dimension_y;
                w.favorites = item.favorites;
                w
            })
            .collect())
    }

    /// Search results carry no tags or uploader, so fetch them per image.
    async fn details(&self, ctx: &Ctx, mut wallpaper: Wallpaper) -> Result<Wallpaper> {
        let response: DetailResponse = get(ctx, &format!("{BASE_URL}/w/{}", wallpaper.id))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        wallpaper.tags = response.data.tags.into_iter().map(|t| t.name).collect();
        wallpaper.author = response.data.uploader.map(|u| u.username);
        Ok(wallpaper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(sketchy: bool, nsfw: bool, key: Option<&str>) -> ProviderSettings {
        ProviderSettings {
            allow_sketchy: sketchy,
            allow_nsfw: nsfw,
            api_key: key.map(str::to_owned),
            ..Default::default()
        }
    }

    #[test]
    fn resolution_params() {
        use crate::providers::Resolution;
        let targets = vec![Resolution::new(1920, 1080), Resolution::new(2560, 1440)];
        let f = |policy| ResolutionFilter { policy, targets: targets.clone() };

        assert_eq!(resolution_param(&f(ResolutionPolicy::Any)), None);
        assert_eq!(
            resolution_param(&f(ResolutionPolicy::AtLeast)),
            Some(("atleast", "1920x1080".to_string()))
        );
        assert_eq!(
            resolution_param(&f(ResolutionPolicy::Exact)),
            Some(("resolutions", "1920x1080,2560x1440".to_string()))
        );
    }

    #[test]
    fn an_api_key_shortens_the_minimum_interval() {
        let none = Wallhaven.min_interval(&ProviderSettings::default());
        let keyed = Wallhaven.min_interval(&settings(false, false, Some("k")));
        assert!(keyed < none);
        assert_eq!(Wallhaven.min_interval(&settings(false, false, Some(""))), none);
    }

    #[test]
    fn purity_codes() {
        assert_eq!(purity(&settings(false, false, None)), "100");
        assert_eq!(purity(&settings(true, false, None)), "110");
        assert_eq!(purity(&settings(true, true, None)), "110"); // no key: NSFW ignored
        assert_eq!(purity(&settings(true, true, Some("k"))), "111");
        assert_eq!(purity(&settings(false, true, Some(""))), "100");
    }
}
