//! What is in the download folder, joined with what the library knows about it.
//! The folder is the truth: files the library has never heard of (older
//! downloads, pictures you added yourself) are listed too, just without metadata.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::{
    library::{Entry, Library},
    providers::ProviderId,
};

const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp"];

#[derive(Debug, Clone)]
pub struct Item {
    pub file: String,
    pub path: PathBuf,
    /// What the library knows, if anything.
    pub entry: Option<Entry>,
    /// When it arrived: the library's date, else the file's modification time (unix seconds).
    pub added: u64,
    pub size: Option<(u32, u32)>,
}

impl Item {
    pub fn provider(&self) -> Option<ProviderId> {
        self.entry.as_ref().map(|e| e.wallpaper.provider)
    }

    pub fn favorite(&self) -> bool {
        self.entry.as_ref().is_some_and(|e| e.favorite)
    }

    /// The image's title, or its file name without the extension.
    pub fn title(&self) -> String {
        self.entry
            .as_ref()
            .and_then(|e| e.wallpaper.title.clone())
            .unwrap_or_else(|| {
                Path::new(&self.file)
                    .file_stem()
                    .map_or_else(|| self.file.clone(), |s| s.to_string_lossy().into_owned())
            })
    }
}

/// Every image in `dir`, newest first.
pub fn scan(dir: &Path, library: &Library) -> Vec<Item> {
    let known: HashMap<&str, &Entry> = library.visible().map(|e| (e.file.as_str(), e)).collect();
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };

    let mut items: Vec<Item> = entries
        .flatten()
        .filter_map(|dir_entry| {
            let path = dir_entry.path();
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)));
            let meta = dir_entry.metadata().ok().filter(|m| m.is_file())?;
            if !is_image {
                return None;
            }
            let file = path.file_name()?.to_str()?.to_owned();
            let entry = known.get(file.as_str()).map(|e| (*e).clone());

            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            let size = match entry.as_ref().and_then(|e| e.wallpaper.width.zip(e.wallpaper.height)) {
                Some(size) => Some(size),
                None => imagesize::size(&path).ok().map(|s| (s.width as u32, s.height as u32)),
            };
            let added = entry.as_ref().map_or(modified, |e| e.added);
            Some(Item { file, path, entry, added, size })
        })
        .collect();

    items.sort_by(|a, b| b.added.cmp(&a.added).then_with(|| a.file.cmp(&b.file)));
    items
}

/// Where an image came from, for filtering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Source {
    #[default]
    All,
    Provider(ProviderId),
    /// Files the library doesn't know.
    Other,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All sources",
            Self::Provider(id) => id.label(),
            Self::Other => "Other files",
        }
    }
}

/// The filter choices that make sense for these items: "All sources", the
/// providers that have something, alphabetically, and "Other files" if any.
pub fn sources_present(items: &[Item]) -> Vec<Source> {
    let mut providers: Vec<ProviderId> = items.iter().filter_map(Item::provider).collect();
    providers.sort_by_key(|id| id.label().to_lowercase());
    providers.dedup();

    let mut sources = vec![Source::All];
    sources.extend(providers.into_iter().map(Source::Provider));
    if items.iter().any(|i| i.entry.is_none()) {
        sources.push(Source::Other);
    }
    sources
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub search: String,
    pub favorites_only: bool,
    pub source: Source,
}

impl Filter {
    pub fn matches(&self, item: &Item) -> bool {
        if self.favorites_only && !item.favorite() {
            return false;
        }
        let source_ok = match self.source {
            Source::All => true,
            Source::Provider(id) => item.provider() == Some(id),
            Source::Other => item.entry.is_none(),
        };
        source_ok && self.matches_search(item)
    }

    fn matches_search(&self, item: &Item) -> bool {
        let needle = self.search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let mut haystack = vec![item.file.to_lowercase(), item.title().to_lowercase()];
        if let Some(entry) = &item.entry {
            haystack.push(entry.wallpaper.provider.label().to_lowercase());
            haystack.extend(entry.wallpaper.author.iter().map(|a| a.to_lowercase()));
            haystack.extend(entry.wallpaper.tags.iter().map(|t| t.to_lowercase()));
        }
        haystack.iter().any(|h| h.contains(&needle))
    }
}

pub fn filtered<'a>(items: &'a [Item], filter: &Filter) -> Vec<&'a Item> {
    items.iter().filter(|i| filter.matches(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Wallpaper;

    fn entry(provider: ProviderId, id: &str, file: &str, added: u64) -> Entry {
        let mut w = Wallpaper::new(provider, id, "https://example.invalid/x.jpg");
        w.title = Some(format!("Title {id}"));
        w.author = Some("Jane".into());
        w.tags = vec!["mountain".into(), "snow".into()];
        w.width = Some(1920);
        w.height = Some(1080);
        Entry { wallpaper: w, file: file.into(), added, favorite: false, blocked: false }
    }

    /// A temp folder with tiny files named like images.
    fn folder(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for f in files {
            fs::write(dir.join(f), b"not really an image").unwrap();
        }
        dir
    }

    /// Library dates are real timestamps; these are far enough ahead to sort
    /// after the modification time of files the tests create just now.
    const LATER: u64 = 4_000_000_000;

    fn library() -> Library {
        let mut lib = Library::default();
        lib.insert(entry(ProviderId::Wallhaven, "a", "wallhaven_a.jpg", LATER + 300));
        lib.insert(entry(ProviderId::Bing, "b", "bing_b.jpg", LATER + 100));
        let mut blocked = entry(ProviderId::Bing, "gone", "bing_gone.jpg", LATER + 50);
        blocked.blocked = true;
        lib.insert(blocked);
        lib
    }

    #[test]
    fn the_folder_is_the_truth_and_the_library_adds_detail() {
        let dir = folder("wd-gallery-scan", &["wallhaven_a.jpg", "bing_b.jpg", "mine.png", "notes.txt", "x.jpg.part"]);
        let items = scan(&dir, &library());

        let files: Vec<&str> = items.iter().map(|i| i.file.as_str()).collect();
        assert_eq!(files.len(), 3, "{files:?}"); // no .txt, no .part
        assert_eq!(files, vec!["wallhaven_a.jpg", "bing_b.jpg", "mine.png"]); // newest first

        let tracked = &items[0];
        assert_eq!(tracked.provider(), Some(ProviderId::Wallhaven));
        assert_eq!(tracked.title(), "Title a");
        assert_eq!(tracked.size, Some((1920, 1080)));

        let mine = items.iter().find(|i| i.file == "mine.png").unwrap();
        assert!(mine.entry.is_none());
        assert_eq!(mine.title(), "mine"); // file name without the extension
    }

    #[test]
    fn a_missing_folder_is_just_empty() {
        assert!(scan(Path::new("/definitely/not/here"), &library()).is_empty());
    }

    #[test]
    fn a_blocked_entrys_file_is_shown_as_an_untracked_file_not_hidden() {
        let dir = folder("wd-gallery-blocked", &["bing_gone.jpg"]);
        let items = scan(&dir, &library());
        assert_eq!(items.len(), 1);
        assert!(items[0].entry.is_none());
    }

    #[test]
    fn filters_combine() {
        let dir = folder("wd-gallery-filter", &["wallhaven_a.jpg", "bing_b.jpg", "mine.png"]);
        let mut lib = library();
        lib.set_favorite(ProviderId::Bing, "b", true);
        let items = scan(&dir, &lib);

        let by = |search: &str, favorites_only, source| {
            let filter = Filter { search: search.into(), favorites_only, source };
            filtered(&items, &filter).iter().map(|i| i.file.clone()).collect::<Vec<_>>()
        };
        assert_eq!(by("", false, Source::All).len(), 3);
        assert_eq!(by("", true, Source::All), vec!["bing_b.jpg"]);
        assert_eq!(by("", false, Source::Provider(ProviderId::Wallhaven)), vec!["wallhaven_a.jpg"]);
        assert_eq!(by("", false, Source::Other), vec!["mine.png"]);
        assert_eq!(by("SNOW", false, Source::All).len(), 2); // matches tags, case-insensitively
        assert_eq!(by("jane", false, Source::All).len(), 2); // author
        assert_eq!(by("mine", false, Source::All), vec!["mine.png"]); // file name
        assert_eq!(by("snow", true, Source::All), vec!["bing_b.jpg"]);
        assert!(by("nothing like this", false, Source::All).is_empty());
    }

    #[test]
    fn the_source_choices_only_list_what_is_there() {
        let dir = folder("wd-gallery-sources", &["wallhaven_a.jpg", "bing_b.jpg"]);
        let items = scan(&dir, &library());
        let labels: Vec<_> = sources_present(&items).iter().map(|s| s.label()).collect();
        assert_eq!(labels, vec!["All sources", "Bing Photo of the Day", "Wallhaven"]);

        let dir = folder("wd-gallery-sources2", &["mine.png"]);
        let labels: Vec<_> = sources_present(&scan(&dir, &library())).iter().map(|s| s.label()).collect();
        assert_eq!(labels, vec!["All sources", "Other files"]);
    }
}
