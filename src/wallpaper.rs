use std::{cell::RefCell, collections::HashMap, fs, path::Path, time::SystemTime};

use display_info::DisplayInfo;
use more_wallpapers::{Mode, WallpaperBuilder};

use crate::config::{ChangeOrder, ChangerConfig, MonitorMode};
use crate::state::State;

const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp"];

struct Image {
    path: String,
    modified: SystemTime,
}

/// What the desktop looks like after a change.
pub struct Outcome {
    /// Screen name -> wallpaper path, for every screen the app has set.
    pub by_screen: HashMap<String, String>,
    /// One wallpaper shared by every screen, if that's what was applied.
    pub shared: Option<String>,
}

pub struct Engine {
    per_screen: bool,
    /// Last image used, so sequential order continues where it left off.
    last: RefCell<Option<String>>,
    /// What each screen currently shows, for screens that are left unchanged.
    applied: RefCell<HashMap<String, String>>,
}

impl Engine {
    pub fn new(state: State) -> Self {
        // If detection fails, assume per-screen works and let the real call report errors
        let per_screen = WallpaperBuilder::new()
            .map(|b| b.environment().support_various_wallpaper())
            .unwrap_or(true);
        Self {
            per_screen,
            last: RefCell::new(state.last),
            applied: RefCell::new(state.screens.into_iter().collect()),
        }
    }

    pub fn per_screen_supported(&self) -> bool {
        self.per_screen
    }

    fn save_state(&self) {
        let state = State {
            last: self.last.borrow().clone(),
            screens: self
                .applied
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        if let Err(e) = state.save() {
            eprintln!("Could not save state: {e}");
        }
    }

    /// Re-apply the saved wallpapers (e.g. at startup).
    pub fn restore(&self) -> Result<Outcome, String> {
        let usable: HashMap<String, String> = self
            .applied
            .borrow()
            .iter()
            .filter(|(_, path)| Path::new(path.as_str()).is_file())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let Some(fallback) = usable.values().next().cloned() else {
            return Err("No saved wallpapers to restore".into());
        };

        WallpaperBuilder::new()
            .map_err(|e| e.to_string())?
            .set_wallpapers(|screen| {
                let path = usable
                    .get(&screen.name)
                    .cloned()
                    .or_else(|| screen.wallpaper.as_ref().map(|p| p.to_string()))
                    .unwrap_or_else(|| fallback.clone());
                (path, screen.mode.unwrap_or_default())
            })
            .map_err(|e| e.to_string())?;

        Ok(Outcome {
            by_screen: usable,
            shared: None,
        })
    }

    pub fn change(&self, cfg: &ChangerConfig) -> Result<Outcome, String> {
        let images = scan(&cfg.directories);
        if images.is_empty() {
            return Err("No images found in the selected directories".into());
        }

        let monitor_mode = if self.per_screen {
            cfg.monitor_mode
        } else {
            MonitorMode::SameOnAll
        };

        let builder = WallpaperBuilder::new().map_err(|e| e.to_string())?;
        let targets = target_screens(&builder, monitor_mode)?;
        let pick_count = if monitor_mode == MonitorMode::DifferentPerMonitor {
            targets.len()
        } else {
            1
        };
        let picks = self.pick(images, cfg.order, pick_count);

        let mut index = 0usize;
        let mut assigned: Vec<(String, String)> = Vec::new();
        builder
            .set_wallpapers(|screen| {
                let target = targets.iter().position(|&t| t == index);
                index += 1;

                let (path, mode) = match target {
                    Some(n) => (picks[n % picks.len()].clone(), Mode::default()),
                    // Not changing this screen: reuse what it already shows
                    None => {
                        let current = screen
                            .wallpaper
                            .as_ref()
                            .map(|p| p.to_string())
                            .or_else(|| self.applied.borrow().get(&screen.name).cloned())
                            .unwrap_or_else(|| picks[0].clone());
                        (current, screen.mode.unwrap_or_default())
                    }
                };
                assigned.push((screen.name.clone(), path.clone()));
                (path, mode)
            })
            .map_err(|e| e.to_string())?;

        // Only remember it once the desktop accepted it
        self.applied.borrow_mut().extend(assigned);
        self.save_state();
        Ok(Outcome {
            by_screen: self.applied.borrow().clone(),
            shared: (monitor_mode == MonitorMode::SameOnAll).then(|| picks[0].clone()),
        })
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
fn target_screens(builder: &WallpaperBuilder, mode: MonitorMode) -> Result<Vec<usize>, String> {
    let screens = builder.screens();
    if screens.is_empty() {
        return Err("No screens detected".into());
    }

    let active: Vec<usize> = (0..screens.len()).filter(|&i| screens[i].active).collect();
    match mode {
        MonitorMode::DifferentPerMonitor | MonitorMode::SameOnAll => {
            Ok(if active.is_empty() { vec![0] } else { active })
        }
        MonitorMode::PrimaryOnly => {
            let primary_name = DisplayInfo::all()
                .ok()
                .and_then(|d| d.into_iter().find(|d| d.is_primary))
                .map(|d| d.name);
            let by_name = screens
                .iter()
                .position(|s| s.active && Some(&s.name) == primary_name.as_ref());
            Ok(vec![by_name.or(active.first().copied()).unwrap_or(0)])
        }
    }
}

/// Non-recursive scan; skips unreadable directories and non-UTF-8 paths.
fn scan(dirs: &[String]) -> Vec<Image> {
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)));
            let Some(path_str) = path.to_str() else {
                continue;
            };
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
