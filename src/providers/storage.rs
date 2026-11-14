//! Keeps the app's downloads under a size limit.

use std::{collections::HashSet, fs, path::Path};

use super::ProviderId;
use crate::library::Library;

pub const MB: u64 = 1024 * 1024;

#[derive(Debug, Default)]
pub struct Cleanup {
    /// File names that were deleted to get under the limit.
    pub deleted: Vec<String>,
    pub freed_bytes: u64,
    /// What the app's downloads take up now.
    pub total_bytes: u64,
    /// Still over the limit, because everything left is a favorite or was just downloaded.
    pub over_limit: bool,
}

struct Candidate {
    added: u64,
    provider: ProviderId,
    id: String,
    file: String,
    favorite: bool,
    size: u64,
}

/// Delete the oldest downloads until the app's files fit in `max_bytes`
/// (`0` means unlimited).
///
/// Only files the library knows about count towards the limit or get deleted,
/// so pictures you put in the folder yourself are never touched. Favorites and
/// anything in `protect` (the images just downloaded) are kept. Deleted images
/// are blocked rather than forgotten, so a provider can't hand them straight back.
pub fn enforce_limit(
    library: &mut Library,
    dir: &Path,
    max_bytes: u64,
    protect: &HashSet<(ProviderId, String)>,
) -> Cleanup {
    let mut candidates: Vec<Candidate> = library
        .visible()
        .filter_map(|e| {
            let size = fs::metadata(dir.join(&e.file)).ok()?.len();
            Some(Candidate {
                added: e.added,
                provider: e.wallpaper.provider,
                id: e.wallpaper.id.clone(),
                file: e.file.clone(),
                favorite: e.favorite,
                size,
            })
        })
        .collect();

    let mut cleanup = Cleanup {
        total_bytes: candidates.iter().map(|c| c.size).sum(),
        ..Cleanup::default()
    };
    if max_bytes == 0 {
        return cleanup;
    }

    candidates.sort_by_key(|c| c.added); // oldest first
    for c in candidates {
        if cleanup.total_bytes <= max_bytes {
            break;
        }
        if c.favorite || protect.contains(&(c.provider, c.id.clone())) {
            continue;
        }
        match fs::remove_file(dir.join(&c.file)) {
            Ok(()) => {
                library.block(c.provider, &c.id);
                cleanup.total_bytes -= c.size;
                cleanup.freed_bytes += c.size;
                cleanup.deleted.push(c.file);
            }
            Err(e) => eprintln!("Could not delete {}: {e}", c.file),
        }
    }
    cleanup.over_limit = cleanup.total_bytes > max_bytes;
    cleanup
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{library::Entry, providers::Wallpaper};

    fn add(lib: &mut Library, dir: &Path, id: &str, added: u64, favorite: bool, size: usize) {
        let file = format!("{id}.jpg");
        fs::write(dir.join(&file), vec![0u8; size]).unwrap();
        lib.insert(Entry {
            wallpaper: Wallpaper::new(ProviderId::Wallhaven, id, "https://example.invalid/x.jpg"),
            file,
            added,
            favorite,
            blocked: false,
        });
    }

    fn fresh_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deletes_oldest_first_and_blocks_them() {
        let dir = fresh_dir("wd-storage-oldest");
        let mut lib = Library::default();
        add(&mut lib, &dir, "old", 1, false, 100);
        add(&mut lib, &dir, "mid", 2, false, 100);
        add(&mut lib, &dir, "new", 3, false, 100);

        let c = enforce_limit(&mut lib, &dir, 250, &HashSet::new());
        assert_eq!(c.deleted, vec!["old.jpg"]);
        assert_eq!((c.total_bytes, c.freed_bytes, c.over_limit), (200, 100, false));
        assert!(!dir.join("old.jpg").exists());
        assert!(lib.contains(ProviderId::Wallhaven, "old")); // blocked, not forgotten
        assert_eq!(lib.visible().count(), 2);
    }

    #[test]
    fn keeps_favorites_and_just_downloaded() {
        let dir = fresh_dir("wd-storage-protect");
        let mut lib = Library::default();
        add(&mut lib, &dir, "fav", 1, true, 100);
        add(&mut lib, &dir, "fresh", 2, false, 100);
        add(&mut lib, &dir, "other", 3, false, 100);

        let protect = HashSet::from([(ProviderId::Wallhaven, "fresh".to_string())]);
        let c = enforce_limit(&mut lib, &dir, 100, &protect);
        assert_eq!(c.deleted, vec!["other.jpg"]);
        assert!(c.over_limit); // 200 left, nothing else may go
        assert!(dir.join("fav.jpg").exists() && dir.join("fresh.jpg").exists());
    }

    #[test]
    fn zero_means_unlimited_and_other_files_are_ignored() {
        let dir = fresh_dir("wd-storage-unlimited");
        fs::write(dir.join("mine.png"), vec![0u8; 5_000]).unwrap(); // not in the library
        let mut lib = Library::default();
        add(&mut lib, &dir, "a", 1, false, 100);

        let c = enforce_limit(&mut lib, &dir, 0, &HashSet::new());
        assert!(c.deleted.is_empty());
        assert_eq!(c.total_bytes, 100); // hand-placed files don't count

        let c = enforce_limit(&mut lib, &dir, 1, &HashSet::new());
        assert_eq!(c.deleted, vec!["a.jpg"]);
        assert!(dir.join("mine.png").exists());
    }
}
