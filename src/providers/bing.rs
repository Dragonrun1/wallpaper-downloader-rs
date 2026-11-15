use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use super::net::check_status;
use super::{
    Capabilities, Ctx, KeyUse, ProviderId, ProviderSettings, ResolutionFilter, ResolutionPolicy, Source,
    Wallpaper,
};

const ARCHIVE_URL: &str = "https://www.bing.com/HPImageArchive.aspx";
const SITE: &str = "https://www.bing.com";

pub struct Bing;

#[derive(Deserialize)]
struct Archive {
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    urlbase: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    copyright: String,
    #[serde(default)]
    copyrightlink: String,
}

#[async_trait]
impl Source for Bing {
    fn id(&self) -> ProviderId {
        ProviderId::Bing
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sorts: &[],
            keywords: false,
            api_key: KeyUse::None,
            key_label: "",
            secret_label: None,
            market: true,
            purity: false,
            help: "",
        }
    }

    /// Bing publishes one image a day, so a few checks a day is plenty.
    fn min_interval(&self, _settings: &ProviderSettings) -> Duration {
        Duration::from_secs(8 * 3600)
    }

    /// Bing's archive holds the last 8 days. Everything the library already
    /// has gets skipped, so this also covers "today's was already downloaded".
    async fn list(&self, ctx: &Ctx) -> Result<Vec<Wallpaper>> {
        let market = market(ctx.settings.market.as_deref());
        let archive: Archive = check_status(
            ctx.client
                .get(ARCHIVE_URL)
                .query(&[("format", "js"), ("idx", "0"), ("n", "8"), ("mkt", market.as_str())])
                .send()
                .await?,
        )?
            .json()
            .await?;

        let size = size_suffix(&ctx.resolution);

        Ok(archive
            .images
            .into_iter()
            .map(|image| {
                let mut w = Wallpaper::new(
                    ProviderId::Bing,
                    image_id(&image.urlbase),
                    format!("{SITE}{}_{size}.jpg", image.urlbase),
                );
                w.title = non_empty(&image.title);
                w.author = credit(&image.copyright);
                w.description = non_empty(&image.copyright);
                w.page_url = match image.copyrightlink.as_str() {
                    "" => None,
                    link if link.starts_with('/') => Some(format!("{SITE}{link}")),
                    link => Some(link.to_owned()),
                };
                w
            })
            .collect())
    }
}

/// Bing serves a fixed set of sizes. Anything bigger than Full HD gets UHD
/// (3840×2160); an exact policy asks for that exact size, which Bing may not have.
fn size_suffix(filter: &ResolutionFilter) -> String {
    let Some(t) = filter.largest() else { return "1920x1080".to_owned() };
    if filter.policy == ResolutionPolicy::Exact {
        return if (t.width, t.height) == (3840, 2160) {
            "UHD".to_owned()
        } else {
            format!("{}x{}", t.width, t.height)
        };
    }
    if t.width > 1920 || t.height > 1080 { "UHD" } else { "1920x1080" }.to_owned()
}

/// The configured market, else the system locale (e.g. "de-DE"), else "en-US".
fn market(configured: Option<&str>) -> String {
    configured
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
        .or_else(|| sys_locale::get_locale().filter(|l| l.contains('-')))
        .unwrap_or_else(|| "en-US".to_owned())
}

/// `/th?id=OHR.Name_EN-US123` and `/az/hprichbg/rb/Name_EN-US123` both
/// end in the image's own name.
fn image_id(urlbase: &str) -> String {
    urlbase.rsplit(['=', '/']).next().unwrap_or(urlbase).to_owned()
}

/// Best effort: Bing writes "Place (© Photographer/Agency)".
fn credit(copyright: &str) -> Option<String> {
    let start = copyright.find("(©")? + "(©".len();
    let end = copyright.rfind(')')?;
    non_empty(copyright.get(start..end)?.trim())
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_id_handles_both_url_styles() {
        assert_eq!(image_id("/th?id=OHR.Grand_EN-US845"), "OHR.Grand_EN-US845");
        assert_eq!(image_id("/az/hprichbg/rb/Sibelius_ZH-CN890"), "Sibelius_ZH-CN890");
    }

    #[test]
    fn size_suffix_choices() {
        use crate::providers::Resolution;
        let f = |policy, w, h| ResolutionFilter { policy, targets: vec![Resolution::new(w, h)] };

        assert_eq!(size_suffix(&f(ResolutionPolicy::AtLeast, 1920, 1080)), "1920x1080");
        assert_eq!(size_suffix(&f(ResolutionPolicy::AtLeast, 1920, 1200)), "UHD");
        assert_eq!(size_suffix(&f(ResolutionPolicy::AtLeast, 2560, 1440)), "UHD");
        assert_eq!(size_suffix(&f(ResolutionPolicy::Exact, 1366, 768)), "1366x768");
        assert_eq!(size_suffix(&f(ResolutionPolicy::Exact, 3840, 2160)), "UHD");
    }

    #[test]
    fn configured_market_wins() {
        assert_eq!(market(Some(" fr-FR ")), "fr-FR");
        assert!(market(None).contains('-')); // system locale or the en-US fallback
    }

    #[test]
    fn credit_extracts_photographer() {
        assert_eq!(
            credit("Guild houses, Brussels (© Richard Taylor/Sime/eStock Photo)").as_deref(),
            Some("Richard Taylor/Sime/eStock Photo")
        );
        assert_eq!(credit("No credit here"), None);
    }
}
