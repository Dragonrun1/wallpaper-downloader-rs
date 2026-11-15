//! The shared pipeline: providers find images, this downloads and records them.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use tokio::task::JoinSet;

use super::{Ctx, ProviderId, ProvidersConfig, Source, Wallpaper};
use super::net::{check_status, rate_limit};
use super::schedule::now_secs;
use super::storage::{self, Cleanup};
use crate::library::{Entry, Library};

/// At most this many candidates are tried per check, so a provider whose
/// images keep getting rejected can't cause a flood of requests.
const MAX_ATTEMPTS: usize = 8;

/// A candidate we looked at and chose not to keep (not an error).
#[derive(Debug)]
struct Skipped(String);

impl std::fmt::Display for Skipped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Skipped {}

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

/// Check every enabled provider, ignoring the schedule.
pub async fn harvest_all(
    cfg: &ProvidersConfig,
    library: Arc<Mutex<Library>>,
) -> Result<HarvestReport> {
    harvest(cfg, library, &cfg.enabled()).await
}

/// Run the given providers in parallel, each fetching at most one new image,
/// then trim the downloads to the size limit. Which providers are allowed to
/// run right now is the caller's call (see `schedule`); this just does it.
/// The caller also decides when to save the library.
pub async fn harvest(
    cfg: &ProvidersConfig,
    library: Arc<Mutex<Library>>,
    providers: &[ProviderId],
) -> Result<HarvestReport> {
    let client = build_client()?;
    let dir = PathBuf::from(&cfg.download_dir);
    if cfg.download_dir.is_empty() {
        bail!("No download directory configured");
    }
    let keyword = pick_keyword(&cfg.keywords);

    let mut set = JoinSet::new();
    for &id in providers {
        let known: Arc<dyn Fn(&str) -> bool + Send + Sync> = {
            let library = library.clone();
            Arc::new(move |image_id| library.lock().unwrap().contains(id, image_id))
        };
        let ctx = Ctx {
            client: client.clone(),
            keywords: keyword.clone(),
            resolution: cfg.resolution.clone(),
            settings: cfg.settings_for(id),
            known,
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

    results.sort_by_key(|(id, _)| *id); // finishing order is random; keep output stable

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
    let mut attempts = 0;
    for candidate in candidates {
        if library.lock().unwrap().contains(provider, &candidate.id)
            || has_forbidden_tag(&candidate.tags, forbidden)
            || !size_fits(&ctx.resolution, &candidate)
        {
            continue;
        }

        if attempts >= MAX_ATTEMPTS {
            break;
        }
        if attempts > 0 {
            tokio::time::sleep(source.pause()).await;
        }
        attempts += 1;

        let wallpaper = match source.details(ctx, candidate).await {
            Ok(w) => w,
            Err(e) => {
                let e = e.context(format!("{}: details failed", provider.label()));
                if rate_limit(&e).is_some() {
                    return Err(e); // told to slow down: don't try the next candidate
                }
                last_error = Some(e);
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
            Err(e) if e.downcast_ref::<Skipped>().is_some() => continue,
            Err(e) => {
                let e = e.context(format!("{}: download of {} failed", provider.label(), wallpaper.id));
                if rate_limit(&e).is_some() {
                    return Err(e);
                }
                last_error = Some(e);
            }
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => Ok(None),
    }
}

async fn download(ctx: &Ctx, wallpaper: &Wallpaper, dir: &Path) -> Result<Entry> {
    let bytes = check_status(ctx.client.get(&wallpaper.image_url).send().await?)?
        .bytes()
        .await?;

    // The bytes decide the format, not the URL or headers, and this also
    // rejects HTML error pages that came back with a 200.
    let size = imagesize::blob_size(&bytes)
        .map_err(|e| anyhow!("downloaded data is not a readable image: {e}"))?;
    if !ctx.resolution.accepts(size.width as u32, size.height as u32) {
        return Err(anyhow::Error::new(Skipped(format!(
            "image is {}x{}, which doesn't fit the resolution settings",
            size.width, size.height
        ))));
    }
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

/// One keyword chosen at random for each run; empty means "no keyword"
/// (search everything).
fn pick_keyword(keywords: &[String]) -> String {
    let list: Vec<&str> = keywords.iter().map(|k| k.trim()).filter(|k| !k.is_empty()).collect();
    match list.len() {
        0 => String::new(),
        n => list[fastrand::usize(..n)].to_owned(),
    }
}

/// Skip candidates whose size is already known to be wrong, before spending
/// any requests on them. Unknown sizes pass; the download is checked again.
fn size_fits(filter: &super::ResolutionFilter, candidate: &Wallpaper) -> bool {
    match (candidate.width, candidate.height) {
        (Some(w), Some(h)) => filter.accepts(w, h),
        _ => true,
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
        // WD_LIVE=bing,unsplash picks providers (default: wallhaven,bing);
        // credentials come from WD_UNSPLASH_KEY, WD_DEVIANTART_ID and WD_DEVIANTART_SECRET.
        let wanted = std::env::var("WD_LIVE").unwrap_or_else(|_| "wallhaven,bing".into());
        for id in ProviderId::ALL {
            let on = wanted.split(',').any(|k| k.trim() == id.key());
            cfg.settings.entry(id.key().to_owned()).or_default().enabled = on;
        }
        let env = |name: &str| std::env::var(name).ok();
        cfg.settings.entry("unsplash".into()).or_default().api_key = env("WD_UNSPLASH_KEY");
        let deviantart = cfg.settings.entry("deviantart".into()).or_default();
        deviantart.api_key = env("WD_DEVIANTART_ID");
        deviantart.api_secret = env("WD_DEVIANTART_SECRET");

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

    /// Hands out three candidates, but its `details` always says "slow down".
    struct Throttled {
        details_calls: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Source for Throttled {
        fn id(&self) -> ProviderId {
            ProviderId::Wallhaven
        }
        fn capabilities(&self) -> super::super::Capabilities {
            super::super::Capabilities {
                sorts: &[],
                keywords: false,
                api_key: super::super::KeyUse::None,
                key_label: "",
                secret_label: None,
                market: false,
                purity: false,
                help: "",
            }
        }
        fn min_interval(&self, _: &super::super::ProviderSettings) -> Duration {
            Duration::from_secs(60)
        }
        async fn list(&self, _: &Ctx) -> Result<Vec<Wallpaper>> {
            Ok(["a", "b", "c"]
                .into_iter()
                .map(|id| Wallpaper::new(ProviderId::Wallhaven, id, "https://example.invalid/x.jpg"))
                .collect())
        }
        async fn details(&self, _: &Ctx, _: Wallpaper) -> Result<Wallpaper> {
            self.details_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(anyhow::Error::new(super::super::net::RateLimited {
                retry_after: Some(Duration::from_secs(90)),
            }))
        }
    }

    #[tokio::test]
    async fn a_rate_limit_stops_the_run_instead_of_trying_the_next_candidate() {
        let source = Throttled { details_calls: Default::default() };
        let ctx = super::super::test_ctx();
        let library = Mutex::new(Library::default());
        let dir = std::env::temp_dir().join("wd-ratelimit-test");

        let error = harvest_one(&source, &ctx, &library, &dir, &[]).await.unwrap_err();
        assert_eq!(
            rate_limit(&error).unwrap().retry_after,
            Some(Duration::from_secs(90))
        );
        assert_eq!(source.details_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn keyword_picking() {
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pick_keyword(&[]), "");
        assert_eq!(pick_keyword(&list(&["", "  "])), "");
        assert_eq!(pick_keyword(&list(&[" mountains "])), "mountains");
        let picked = pick_keyword(&list(&["a", "b", "c"]));
        assert!(["a", "b", "c"].contains(&picked.as_str()));
    }

    #[test]
    fn sanitize_keeps_file_names_safe() {
        assert_eq!(sanitize("OHR.Name_EN-US1"), "OHR.Name_EN-US1");
        assert_eq!(sanitize("a/b c"), "a_b_c");
    }
}
