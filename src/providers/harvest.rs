//! The shared pipeline: providers find images, this downloads and records them.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use display_info::DisplayInfo;
use tokio::task::JoinSet;

use super::{Ctx, ProviderId, ProvidersConfig, Source, Wallpaper};
use super::storage::{self, Cleanup};
use crate::library::{Entry, Library};

pub type HarvestResult = (ProviderId, Result<Option<Entry>>);

pub struct HarvestReport {
    pub results: Vec<HarvestResult>,
    pub cleanup: Cleanup,
}

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?)
}

/// Run every enabled provider in parallel, each fetching at most one new image,
/// then trim the downloads to the size limit.
/// The caller decides when to save the library.
pub async fn harvest_all(
    cfg: &ProvidersConfig,
    library: Arc<Mutex<Library>>,
) -> Result<HarvestReport> {
    let client = build_client()?;
    let dir = PathBuf::from(&cfg.download_dir);
    if cfg.download_dir.is_empty() {
        bail!("No download directory configured");
    }
    let resolution = Some(largest_screen());
    let keyword = pick_keyword(&cfg.keywords);

    let mut set = JoinSet::new();
    for id in cfg.enabled() {
        let ctx = Ctx {
            client: client.clone(),
            keywords: keyword.clone(),
            min_resolution: resolution,
            settings: cfg.settings_for(id),
        };
        let (library, dir, forbidden) = (library.clone(), dir.clone(), cfg.forbidden_tags.clone());
        set.spawn(async move {
            let source = id.source();
            (id, harvest_one(source.as_ref(), &ctx, &library, &dir, &forbidden).await)
        });
    }

    let mut results = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("Provider task failed: {e}"),
        }
    }

    // Never trim away what this very run just fetched
    let protect: HashSet<(ProviderId, String)> = results
        .iter()
        .filter_map(|(_, r)| r.as_ref().ok()?.as_ref())
        .map(|e| (e.wallpaper.provider, e.wallpaper.id.clone()))
        .collect();
    let cleanup = storage::enforce_limit(
        &mut library.lock().unwrap(),
        &dir,
        cfg.max_folder_size_mb * storage::MB,
        &protect,
    );

    Ok(HarvestReport { results, cleanup })
}

/// Download the first candidate that is new and allowed.
/// `Ok(None)` means there was nothing new.
pub async fn harvest_one(
    source: &dyn Source,
    ctx: &Ctx,
    library: &Mutex<Library>,
    dir: &Path,
    forbidden: &[String],
) -> Result<Option<Entry>> {
    let provider = source.id();
    let candidates = source
        .list(ctx)
        .await
        .with_context(|| format!("{}: could not list wallpapers", provider.label()))?;

    let mut last_error = None;
    for candidate in candidates {
        if library.lock().unwrap().contains(provider, &candidate.id)
            || has_forbidden_tag(&candidate.tags, forbidden)
        {
            continue;
        }

        let wallpaper = match source.details(ctx, candidate).await {
            Ok(w) => w,
            Err(e) => {
                last_error = Some(e.context(format!("{}: details failed", provider.label())));
                continue;
            }
        };
        if has_forbidden_tag(&wallpaper.tags, forbidden) {
            continue;
        }

        match download(ctx, &wallpaper, dir).await {
            Ok(entry) => {
                library.lock().unwrap().insert(entry.clone());
                return Ok(Some(entry));
            }
            Err(e) => {
                last_error = Some(e.context(format!("{}: download of {} failed", provider.label(), wallpaper.id)));
            }
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => Ok(None),
    }
}

async fn download(ctx: &Ctx, wallpaper: &Wallpaper, dir: &Path) -> Result<Entry> {
    let bytes = ctx
        .client
        .get(&wallpaper.image_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // The bytes decide the format, not the URL or headers, and this also
    // rejects HTML error pages that came back with a 200.
    let size = imagesize::blob_size(&bytes)
        .map_err(|e| anyhow!("downloaded data is not a readable image: {e}"))?;
    let ext = match imagesize::image_type(&bytes) {
        Ok(imagesize::ImageType::Jpeg) => "jpg",
        Ok(imagesize::ImageType::Png) => "png",
        Ok(imagesize::ImageType::Webp) => "webp",
        Ok(imagesize::ImageType::Bmp) => "bmp",
        other => bail!("unsupported image format: {other:?}"),
    };

    let file = format!("{}_{}.{ext}", wallpaper.provider.key(), sanitize(&wallpaper.id));
    tokio::fs::create_dir_all(dir).await?;
    let tmp = dir.join(format!("{file}.part"));
    tokio::fs::write(&tmp, &bytes).await?;
    tokio::fs::rename(&tmp, dir.join(&file)).await?;

    let mut wallpaper = wallpaper.clone();
    wallpaper.width = Some(size.width as u32); // measured, so more trustworthy than the API
    wallpaper.height = Some(size.height as u32);
    Ok(Entry { wallpaper, file, added: now_secs(), favorite: false, blocked: false })
}

/// Keywords are separated by `;`. One is chosen at random for each run, and
/// empty means "no keyword" (search everything).
fn pick_keyword(keywords: &str) -> String {
    let list: Vec<&str> = keywords.split(';').map(str::trim).filter(|k| !k.is_empty()).collect();
    match list.len() {
        0 => String::new(),
        n => list[fastrand::usize(..n)].to_owned(),
    }
}

fn has_forbidden_tag(tags: &[String], forbidden: &[String]) -> bool {
    forbidden
        .iter()
        .map(|f| f.trim().to_lowercase())
        .filter(|f| !f.is_empty())
        .any(|f| tags.iter().any(|t| t.to_lowercase().contains(&f)))
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Biggest connected screen, so we ask providers for images that fit it.
fn largest_screen() -> (u32, u32) {
    DisplayInfo::all()
        .ok()
        .and_then(|d| Some((d.iter().map(|d| d.width).max()?, d.iter().map(|d| d.height).max()?)))
        .unwrap_or((1920, 1080))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hits the real services. Run with:
    ///   cargo test live_harvest -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "uses the network"]
    async fn live_harvest() {
        let mut cfg = ProvidersConfig::default();
        cfg.download_dir = std::env::temp_dir()
            .join("wallpaper-downloader-test")
            .to_string_lossy()
            .into_owned();
        cfg.settings.get_mut(ProviderId::Bing.key()).unwrap().enabled = true;

        let library = Arc::new(Mutex::new(Library::default()));
        let report = harvest_all(&cfg, library.clone()).await.unwrap();
        println!("cleanup: {:?}", report.cleanup);
        for (id, result) in report.results {
            match result {
                Ok(Some(e)) => println!("{}: saved {} ({:?})", id.label(), e.file, e.wallpaper),
                Ok(None) => println!("{}: nothing new", id.label()),
                Err(e) => println!("{}: ERROR {e:#}", id.label()),
            }
        }
        println!("download dir: {}", cfg.download_dir);
    }

    #[test]
    fn forbidden_tags_match_case_insensitively() {
        let tags = vec!["Landscape".to_string(), "Night Sky".to_string()];
        assert!(has_forbidden_tag(&tags, &["night".into()]));
        assert!(!has_forbidden_tag(&tags, &["portrait".into(), "  ".into()]));
    }

    #[test]
    fn keyword_picking() {
        assert_eq!(pick_keyword(""), "");
        assert_eq!(pick_keyword(" ; ;"), "");
        assert_eq!(pick_keyword(" mountains "), "mountains");
        let picked = pick_keyword("a; b ;c");
        assert!(["a", "b", "c"].contains(&picked.as_str()));
    }

    #[test]
    fn sanitize_keeps_file_names_safe() {
        assert_eq!(sanitize("OHR.Name_EN-US1"), "OHR.Name_EN-US1");
        assert_eq!(sanitize("a/b c"), "a_b_c");
    }
}
