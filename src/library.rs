use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::providers::{ProviderId, Wallpaper};

/// A downloaded image: its metadata plus where it lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    #[serde(flatten)]
    pub wallpaper: Wallpaper,
    /// File name inside the download directory.
    pub file: String,
    /// Unix seconds.
    pub added: u64,
    #[serde(default)]
    pub favorite: bool,
    /// The user got rid of this image: keep the record so it is never fetched again.
    #[serde(default)]
    pub blocked: bool,
}

/// Index of everything downloaded, so providers never fetch the same image
/// twice and the Wallpapers tab can show titles, authors and tags.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    /// The download folder these entries describe.
    dir: Option<String>,
    entries: BTreeMap<String, Entry>,
}

impl Library {
    fn key(provider: ProviderId, id: &str) -> String {
        format!("{}:{id}", provider.key())
    }

    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "wallpaper-downloader-rs")
            .map(|d| d.data_dir().join("library.json"))
    }

    /// Never fails: missing or unreadable means an empty library.
    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                eprintln!("Ignoring invalid library {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(io::Error::other)?)?;
        fs::rename(&tmp, &path)
    }

    pub fn contains(&self, provider: ProviderId, id: &str) -> bool {
        self.entries.contains_key(&Self::key(provider, id))
    }

    pub fn insert(&mut self, entry: Entry) {
        let key = Self::key(entry.wallpaper.provider, &entry.wallpaper.id);
        self.entries.insert(key, entry);
    }

    /// Everything not blocked: what the Wallpapers tab should show.
    pub fn visible(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values().filter(|e| !e.blocked)
    }

    pub fn set_favorite(&mut self, provider: ProviderId, id: &str, favorite: bool) {
        if let Some(e) = self.entries.get_mut(&Self::key(provider, id)) {
            e.favorite = favorite;
        }
    }

    /// Hide an image for good and make sure it is never downloaded again.
    pub fn block(&mut self, provider: ProviderId, id: &str) {
        if let Some(e) = self.entries.get_mut(&Self::key(provider, id)) {
            e.blocked = true;
        }
    }

    /// Forget an image completely, so it *can* be downloaded again
    /// (e.g. when the app itself deletes files to stay under a size limit).
    pub fn remove(&mut self, provider: ProviderId, id: &str) {
        self.entries.remove(&Self::key(provider, id));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// A library describes one download folder. If the folder changed, the old
    /// entries describe files somewhere else, so drop them (nothing on disk is
    /// touched) and start fresh. Returns true if that happened.
    pub fn bind_dir(&mut self, dir: &Path) -> bool {
        let key = dir.to_string_lossy().into_owned();
        match self.dir.as_deref() {
            Some(current) if current != key => {
                self.entries.clear();
                self.dir = Some(key);
                true
            }
            Some(_) => false,
            None => {
                self.dir = Some(key);
                false
            }
        }
    }

    /// Entries whose file has vanished (the user deleted it) become blocked,
    /// so the image isn't downloaded again. Returns how many were newly blocked.
    /// Does nothing if the directory itself is missing, so an unplugged drive
    /// can't block everything.
    pub fn prune(&mut self, dir: &Path) -> usize {
        if !dir.is_dir() {
            return 0;
        }
        let mut blocked = 0;
        for e in self.entries.values_mut() {
            if !e.blocked && !dir.join(&e.file).is_file() {
                e.blocked = true;
                blocked += 1;
            }
        }
        blocked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, file: &str) -> Entry {
        Entry {
            wallpaper: Wallpaper::new(ProviderId::Wallhaven, id, "https://example.invalid/x.jpg"),
            file: file.to_owned(),
            added: 0,
            favorite: false,
            blocked: false,
        }
    }

    #[test]
    fn prune_blocks_missing_files_but_keeps_the_record() {
        let dir = std::env::temp_dir().join("wd-library-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("present.jpg"), b"x").unwrap();

        let mut lib = Library::default();
        lib.insert(entry("a", "present.jpg"));
        lib.insert(entry("b", "gone.jpg"));

        assert_eq!(lib.prune(&dir), 1);
        assert!(lib.contains(ProviderId::Wallhaven, "b")); // still known: never re-downloaded
        assert_eq!(lib.visible().count(), 1);
        assert_eq!(lib.prune(&dir), 0); // idempotent
    }

    #[test]
    fn a_changed_folder_starts_a_fresh_library_instead_of_blocking_everything() {
        let mut lib = Library::default();
        assert!(!lib.bind_dir(Path::new("/pictures/a"))); // first folder: just remembered
        lib.insert(entry("x", "x.jpg"));
        assert!(!lib.bind_dir(Path::new("/pictures/a"))); // same folder: nothing happens
        assert_eq!(lib.visible().count(), 1);

        assert!(lib.bind_dir(Path::new("/pictures/b"))); // different folder
        assert_eq!(lib.len(), 0);
        assert!(!lib.contains(ProviderId::Wallhaven, "x")); // not blocked, just not tracked here
    }

    #[test]
    fn prune_ignores_a_missing_directory() {
        let mut lib = Library::default();
        lib.insert(entry("a", "present.jpg"));
        assert_eq!(lib.prune(Path::new("/definitely/not/here")), 0);
        assert_eq!(lib.visible().count(), 1);
    }

    #[test]
    fn remove_allows_a_second_download_but_block_does_not() {
        let mut lib = Library::default();
        lib.insert(entry("a", "a.jpg"));
        lib.insert(entry("b", "b.jpg"));
        lib.remove(ProviderId::Wallhaven, "a");
        lib.block(ProviderId::Wallhaven, "b");
        assert!(!lib.contains(ProviderId::Wallhaven, "a"));
        assert!(lib.contains(ProviderId::Wallhaven, "b"));
        lib.set_favorite(ProviderId::Wallhaven, "b", true);
        assert!(lib.entries.values().all(|e| e.blocked && e.favorite));
    }
}
