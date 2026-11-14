use std::{cell::RefCell, fs, time::SystemTime};

use display_info::DisplayInfo;
use more_wallpapers::{Mode, WallpaperBuilder};

use crate::config::{ChangeOrder, ChangerConfig};

const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp"];

struct Image {
    path: String,
    modified: SystemTime,
}

/// Remembers the last image used so sequential order continues where it left off.
#[derive(Default)]
pub struct Engine {
    last: RefCell<Option<String>>,
}

impl Engine {
    /// Returns the paths of the wallpapers that were set.
    pub fn change(&self, cfg: &ChangerConfig) -> Result<Vec<String>, String> {
        let images = scan(&cfg.directories);
        if images.is_empty() {
            return Err("No images found in the selected directories".into());
        }

        let builder = WallpaperBuilder::new().map_err(|e| e.to_string())?;
        let targets = target_screens(&builder, cfg.all_monitors)?;
        let picks = self.pick(images, cfg.order, targets.len());

        let mut index = 0usize;
        let mut used = Vec::new();
        builder
            .set_wallpapers(|screen| {
                let i = index;
                index += 1;
                match targets.iter().position(|&t| t == i) {
                    Some(n) => {
                        used.push(picks[n].clone());
                        (picks[n].clone(), Mode::default())
                    }
                    // Not a target: keep whatever it has now
                    None => {
                        let current = screen.wallpaper.as_ref().map(|p| p.to_string());
                        (current.unwrap_or_else(|| picks[0].clone()), screen.mode.unwrap_or_default())
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(used)
    }

    fn pick(&self, mut images: Vec<Image>, order: ChangeOrder, count: usize) -> Vec<String> {
        match order {
            ChangeOrder::OldestToNewest => images.sort_by_key(|i| i.modified),
            ChangeOrder::NewestToOldest => images.sort_by_key(|i| std::cmp::Reverse(i.modified)),
            ChangeOrder::Random => {}
        }

        let len = images.len();
        let last_idx = self
            .last
            .borrow()
            .as_ref()
            .and_then(|l| images.iter().position(|i| &i.path == l));

        let indices: Vec<usize> = match order {
            ChangeOrder::Random => {
                let mut idx: Vec<usize> = (0..len).collect();
                fastrand::shuffle(&mut idx);
                // Avoid repeating the previous image first when there is a choice
                if len > 1 && Some(idx[0]) == last_idx {
                    idx.swap(0, len - 1);
                }
                idx.into_iter().cycle().take(count).collect()
            }
            _ => {
                let start = last_idx.map_or(0, |i| i + 1);
                (0..count).map(|n| (start + n) % len).collect()
            }
        };

        let picks: Vec<String> = indices.iter().map(|&i| images[i].path.clone()).collect();
        *self.last.borrow_mut() = picks.last().cloned();
        picks
    }
}

/// Indices (into the builder's screen list) that should get a new wallpaper.
fn target_screens(builder: &WallpaperBuilder, all_monitors: bool) -> Result<Vec<usize>, String> {
    let screens = builder.screens();
    if screens.is_empty() {
        return Err("No screens detected".into());
    }
    if all_monitors {
        let active: Vec<usize> = (0..screens.len()).filter(|&i| screens[i].active).collect();
        return Ok(if active.is_empty() { vec![0] } else { active });
    }

    let primary_name = DisplayInfo::all()
        .ok()
        .and_then(|d| d.into_iter().find(|d| d.is_primary))
        .map(|d| d.name);
    let by_name = screens
        .iter()
        .position(|s| s.active && Some(&s.name) == primary_name.as_ref());
    let first_active = screens.iter().position(|s| s.active);
    Ok(vec![by_name.or(first_active).unwrap_or(0)])
}

/// Non-recursive scan; skips unreadable directories and non-UTF-8 paths.
fn scan(dirs: &[String]) -> Vec<Image> {
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)));
            let Some(path_str) = path.to_str() else { continue };
            let Ok(meta) = entry.metadata() else { continue };
            if !is_image || !meta.is_file() {
                continue;
            }
            out.push(Image {
                path: path_str.to_owned(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path); // overlapping directories
    out
}

