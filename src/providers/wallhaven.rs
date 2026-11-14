use anyhow::Result;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::Deserialize;

use super::{Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, Sort, Source, Wallpaper};

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

    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let mut query = vec![
            ("q", ctx.keywords.clone()),
            ("sorting", sorting(ctx.settings.sort).to_owned()),
            ("purity", purity(&ctx.settings)),
        ];
        if let Some((w, h)) = ctx.min_resolution {
            query.push(("atleast", format!("{w}x{h}")));
        }

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
    fn purity_codes() {
        assert_eq!(purity(&settings(false, false, None)), "100");
        assert_eq!(purity(&settings(true, false, None)), "110");
        assert_eq!(purity(&settings(true, true, None)), "110"); // no key: NSFW ignored
        assert_eq!(purity(&settings(true, true, Some("k"))), "111");
        assert_eq!(purity(&settings(false, true, Some(""))), "100");
    }
}
