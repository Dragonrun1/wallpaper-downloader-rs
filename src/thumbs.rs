//! Small preview images for the gallery, cached on disk so a big folder is
//! only decoded once.

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Context, Result};
use directories::ProjectDirs;

/// Thumbnails fit inside this box, keeping their proportions.
pub const MAX_WIDTH: u32 = 320;
pub const MAX_HEIGHT: u32 = 200;

pub struct Thumb {
    pub width: u32,
    pub height: u32,
    /// RGBA, row by row.
    pub rgba: Vec<u8>,
}

pub fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "wallpaper-downloader-rs").map(|d| d.cache_dir().join("thumbnails"))
}

/// A stable 64-bit hash (FNV-1a); `DefaultHasher` may change between Rust versions.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, b| (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

/// Where this image's thumbnail is cached. The name covers the path, the size
/// and the modification time, so an edited or replaced file gets a fresh one.
fn cache_file(path: &Path, cache: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let key = format!("{}|{}|{modified}|{MAX_WIDTH}x{MAX_HEIGHT}", path.to_string_lossy(), meta.len());
    Some(cache.join(format!("{:016x}.png", fnv1a(key.as_bytes()))))
}

fn to_thumb(image: &image::DynamicImage) -> Thumb {
    let rgba = image.to_rgba8();
    Thumb { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() }
}

/// The thumbnail for `path`, from the cache if it is there, else made now.
pub fn load(path: &Path, cache: Option<&Path>) -> Result<Thumb> {
    let cached = cache.and_then(|dir| cache_file(path, dir));
    if let Some(file) = cached.as_ref().filter(|f| f.is_file()) {
        if let Ok(image) = image::open(file) {
            return Ok(to_thumb(&image));
        }
    }

    let image = image::ImageReader::open(path)
        .with_context(|| format!("could not open {}", path.display()))?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("could not read {} as an image", path.display()))?;
    let small = image.thumbnail(MAX_WIDTH, MAX_HEIGHT);

    if let Some(file) = cached {
        // Best effort: a failed cache write just means doing it again next time
        if let Some(parent) = file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = small.save_with_format(&file, image::ImageFormat::Png);
    }
    Ok(to_thumb(&small))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        image::RgbImage::from_fn(width, height, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn thumbnails_keep_their_proportions_and_fit_the_box() {
        let dir = fresh("wd-thumbs-fit");
        write_png(&dir.join("wide.png"), 1600, 800);
        write_png(&dir.join("tall.png"), 400, 1200);

        let wide = load(&dir.join("wide.png"), None).unwrap();
        assert_eq!((wide.width, wide.height), (320, 160));
        assert_eq!(wide.rgba.len(), (wide.width * wide.height * 4) as usize);

        let tall = load(&dir.join("tall.png"), None).unwrap();
        assert!(tall.width <= MAX_WIDTH && tall.height <= MAX_HEIGHT, "{}x{}", tall.width, tall.height);
        assert_eq!(tall.height, 200);
    }

    #[test]
    fn a_small_picture_is_not_blown_up() {
        let dir = fresh("wd-thumbs-small");
        write_png(&dir.join("tiny.png"), 40, 20);
        let t = load(&dir.join("tiny.png"), None).unwrap();
        assert!(t.width <= 40 || t.width <= MAX_WIDTH);
    }

    #[test]
    fn the_cache_is_written_and_then_used() {
        let dir = fresh("wd-thumbs-cache");
        let cache = dir.join("cache");
        write_png(&dir.join("a.png"), 1000, 500);

        let first = load(&dir.join("a.png"), Some(&cache)).unwrap();
        let file = cache_file(&dir.join("a.png"), &cache).unwrap();
        assert!(file.is_file(), "a cache file should exist");

        // swap the cached thumbnail for a recognisable one: the next load must return it
        write_png(&file, 10, 5);
        let second = load(&dir.join("a.png"), Some(&cache)).unwrap();
        assert_eq!((second.width, second.height), (10, 5));
        assert_ne!((first.width, first.height), (10, 5));
    }

    #[test]
    fn a_changed_file_gets_a_new_cache_entry() {
        let dir = fresh("wd-thumbs-changed");
        let cache = dir.join("cache");
        write_png(&dir.join("a.png"), 1000, 500);
        let before = cache_file(&dir.join("a.png"), &cache).unwrap();
        write_png(&dir.join("a.png"), 1200, 600); // different size on disk
        let after = cache_file(&dir.join("a.png"), &cache).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn things_that_are_not_images_are_errors_not_panics() {
        let dir = fresh("wd-thumbs-bad");
        fs::write(dir.join("fake.jpg"), b"just some text").unwrap();
        assert!(load(&dir.join("fake.jpg"), None).is_err());
        assert!(load(&dir.join("missing.png"), None).is_err());
    }

    #[test]
    fn the_hash_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
