//! The Wallpapers tab: a grid of what is in the download folder, with the
//! library's details alongside, and the actions on a chosen picture.

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel, Weak};

use crate::{
    gallery::{self, Filter, Item, Source},
    library::Library,
    saver::Saver,
    screens::Screens,
    thumbs,
    wallpaper::Engine,
    GalleryItem, MainWindow, WallpapersState,
};

/// How many threads make thumbnails. Few, because decoding a large picture takes a lot of memory.
const THUMB_WORKERS: usize = 2;

thread_local! {
    /// Lets work finished on other threads find the gallery again on the UI thread.
    static GALLERY: RefCell<Option<Rc<Gallery>>> = const { RefCell::new(None) };
}

struct Gallery {
    window: Weak<MainWindow>,
    saver: Rc<Saver>,
    library: Arc<Mutex<Library>>,
    engine: Rc<Engine>,
    screens: Rc<Screens>,
    /// Everything in the folder.
    items: RefCell<Vec<Item>>,
    /// What passed the filters, in the order shown.
    visible: RefCell<Vec<Item>>,
    sources: RefCell<Vec<Source>>,
    model: Rc<VecModel<GalleryItem>>,
    source_options: Rc<VecModel<SharedString>>,
    thumbs: RefCell<HashMap<String, Image>>,
    selected_file: RefCell<Option<String>>,
}

pub fn setup(
    window: &MainWindow,
    saver: &Rc<Saver>,
    library: Arc<Mutex<Library>>,
    engine: Rc<Engine>,
    screens: Rc<Screens>,
) {
    let gallery = Rc::new(Gallery {
        window: window.as_weak(),
        saver: saver.clone(),
        library,
        engine,
        screens,
        items: RefCell::default(),
        visible: RefCell::default(),
        sources: RefCell::default(),
        model: Rc::default(),
        source_options: Rc::default(),
        thumbs: RefCell::default(),
        selected_file: RefCell::default(),
    });

    let state = window.global::<WallpapersState>();
    state.set_items(ModelRc::from(gallery.model.clone()));
    state.set_source_options(ModelRc::from(gallery.source_options.clone()));
    wire(&state, &gallery);

    GALLERY.with(|cell| *cell.borrow_mut() = Some(gallery.clone()));
    gallery.refresh();
}

/// Look at the folder again. Safe to call from the UI thread whenever something may have changed.
pub fn refresh_gallery() {
    GALLERY.with(|cell| {
        if let Some(gallery) = cell.borrow().as_ref() {
            gallery.refresh();
        }
    });
}

fn wire(state: &WallpapersState<'_>, gallery: &Rc<Gallery>) {
    let g = gallery.clone();
    state.on_refresh(move || g.refresh());
    let g = gallery.clone();
    state.on_filters_changed(move || g.apply_filters());
    let g = gallery.clone();
    state.on_select(move |i| g.select(i));
    let g = gallery.clone();
    state.on_toggle_favorite(move |i| g.toggle_favorite(i));
    let g = gallery.clone();
    state.on_set_wallpaper(move |i, primary_only| g.set_wallpaper(i, primary_only));
    let g = gallery.clone();
    state.on_delete_item(move |i| g.delete(i));
    let g = gallery.clone();
    state.on_open_page(move |i| g.open_link(i, |item| item.entry.as_ref()?.wallpaper.page_url.clone()));
    let g = gallery.clone();
    state.on_open_author(move |i| g.open_link(i, |item| item.entry.as_ref()?.wallpaper.author_url.clone()));
    let g = gallery.clone();
    state.on_open_folder(move || g.open_folder());
}

/// A thumbnail finished on a worker thread.
fn thumb_ready(file: String, buffer: SharedPixelBuffer<Rgba8Pixel>) {
    GALLERY.with(|cell| {
        if let Some(gallery) = cell.borrow().as_ref() {
            gallery.accept_thumb(file, Image::from_rgba8(buffer));
        }
    });
}

fn size_text(item: &Item) -> String {
    item.size.map_or_else(String::new, |(w, h)| format!("{w}×{h}"))
}

impl Gallery {
    fn state<'a>(&self, window: &'a MainWindow) -> WallpapersState<'a> {
        window.global::<WallpapersState>()
    }

    fn download_dir(&self) -> PathBuf {
        PathBuf::from(self.saver.providers().download_dir)
    }

    fn say(&self, text: impl Into<SharedString>) {
        if let Some(window) = self.window.upgrade() {
            self.state(&window).set_status(text.into());
        }
    }

    fn row_for(&self, item: &Item) -> GalleryItem {
        let thumbs = self.thumbs.borrow();
        let wallpaper = item.entry.as_ref().map(|e| &e.wallpaper);
        GalleryItem {
            key: item.file.as_str().into(),
            thumb: thumbs.get(&item.file).cloned().unwrap_or_default(),
            has_thumb: thumbs.contains_key(&item.file),
            title: item.title().into(),
            source: item.provider().map_or("Other file", |id| id.label()).into(),
            size: size_text(item).into(),
            author: wallpaper.and_then(|w| w.author.clone()).unwrap_or_default().into(),
            tags: wallpaper.map(|w| w.tags.join(", ")).unwrap_or_default().into(),
            favorite: item.favorite(),
            tracked: item.entry.is_some(),
            has_page: wallpaper.is_some_and(|w| w.page_url.is_some()),
            has_author_link: wallpaper.is_some_and(|w| w.author_url.is_some()),
        }
    }

    /// Re-read the folder, then redo the filters and make any missing thumbnails.
    fn refresh(&self) {
        let dir = self.download_dir();
        let items = gallery::scan(&dir, &self.library.lock().unwrap());

        // Forget thumbnails of files that are gone
        self.thumbs.borrow_mut().retain(|file, _| items.iter().any(|i| &i.file == file));
        let missing: Vec<(String, PathBuf)> = {
            let have = self.thumbs.borrow();
            items.iter().filter(|i| !have.contains_key(&i.file)).map(|i| (i.file.clone(), i.path.clone())).collect()
        };

        *self.items.borrow_mut() = items;
        self.rebuild_sources();
        self.apply_filters();
        self.spawn_thumbnails(missing);
    }

    /// The source drop-down only lists what is actually in the folder.
    fn rebuild_sources(&self) {
        let current = self.current_filter().source;
        let sources = gallery::sources_present(&self.items.borrow());
        let index = sources.iter().position(|s| *s == current).unwrap_or(0);

        self.source_options.set_vec(sources.iter().map(|s| SharedString::from(s.label())).collect::<Vec<_>>());
        *self.sources.borrow_mut() = sources;
        if let Some(window) = self.window.upgrade() {
            self.state(&window).set_source_index(index as i32);
        }
    }

    fn current_filter(&self) -> Filter {
        let Some(window) = self.window.upgrade() else { return Filter::default() };
        let state = self.state(&window);
        let source = usize::try_from(state.get_source_index())
            .ok()
            .and_then(|i| self.sources.borrow().get(i).copied())
            .unwrap_or_default();
        Filter { search: state.get_search().to_string(), favorites_only: state.get_favorites_only(), source }
    }

    /// Rebuild what is shown from `items` and the filter controls.
    fn apply_filters(&self) {
        let filter = self.current_filter();
        let visible: Vec<Item> = gallery::filtered(&self.items.borrow(), &filter).into_iter().cloned().collect();
        self.model.set_vec(visible.iter().map(|i| self.row_for(i)).collect::<Vec<_>>());

        // Keep the same picture selected if it is still there
        let selected = self.selected_file.borrow().clone();
        let index = selected.and_then(|file| visible.iter().position(|i| i.file == file));
        if index.is_none() {
            *self.selected_file.borrow_mut() = None;
        }

        let total = self.items.borrow().len();
        let favorites = self.items.borrow().iter().filter(|i| i.favorite()).count();
        let mut summary = format!("{total} wallpapers, {favorites} favorites");
        if visible.len() != total {
            summary = format!("Showing {} of {summary}", visible.len());
        }

        *self.visible.borrow_mut() = visible;
        if let Some(window) = self.window.upgrade() {
            let state = self.state(&window);
            state.set_selected(index.map_or(-1, |i| i as i32));
            state.set_summary(summary.into());
        }
    }

    fn spawn_thumbnails(&self, jobs: Vec<(String, PathBuf)>) {
        let cache = thumbs::cache_dir();
        let workers = THUMB_WORKERS.min(jobs.len());
        for worker in 0..workers {
            let mine: Vec<(String, PathBuf)> = jobs.iter().skip(worker).step_by(workers).cloned().collect();
            let cache = cache.clone();
            std::thread::spawn(move || {
                for (file, path) in mine {
                    let Ok(thumb) = thumbs::load(&path, cache.as_deref()) else { continue };
                    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&thumb.rgba, thumb.width, thumb.height);
                    let _ = slint::invoke_from_event_loop(move || thumb_ready(file, buffer));
                }
            });
        }
    }

    /// Show a thumbnail as soon as it exists.
    fn accept_thumb(&self, file: String, image: Image) {
        let row = self.visible.borrow().iter().position(|i| i.file == file);
        self.thumbs.borrow_mut().insert(file, image);
        if let Some(i) = row {
            if let Some(item) = self.visible.borrow().get(i) {
                self.model.set_row_data(i, self.row_for(item));
            }
        }
    }

    fn item_at(&self, index: i32) -> Option<Item> {
        usize::try_from(index).ok().and_then(|i| self.visible.borrow().get(i).cloned())
    }

    fn select(&self, index: i32) {
        *self.selected_file.borrow_mut() = self.item_at(index).map(|i| i.file);
        if let Some(window) = self.window.upgrade() {
            self.state(&window).set_selected(index);
        }
    }

    fn toggle_favorite(&self, index: i32) {
        let Some(item) = self.item_at(index) else { return };
        let Some(entry) = item.entry else { return };
        let (provider, id) = (entry.wallpaper.provider, entry.wallpaper.id.clone());
        {
            let mut library = self.library.lock().unwrap();
            library.set_favorite(provider, &id, !entry.favorite);
            if let Err(e) = library.save() {
                eprintln!("Gallery: could not save the library: {e}");
            }
        }
        if let Some(stored) = self.items.borrow_mut().iter_mut().find(|i| i.file == item.file) {
            if let Some(e) = stored.entry.as_mut() {
                e.favorite = !entry.favorite;
            }
        }
        self.apply_filters();
    }

    fn set_wallpaper(&self, index: i32, primary_only: bool) {
        let Some(item) = self.item_at(index) else { return };
        let Some(path) = item.path.to_str() else {
            self.say("That file's path can't be used.");
            return;
        };
        match self.engine.set_image(path, primary_only) {
            Ok(outcome) => {
                self.screens.set_outcome(&outcome);
                let scope = if primary_only { "the primary screen" } else { "all screens" };
                self.say(format!("Set {} on {scope}.", item.title()));
            }
            Err(e) => self.say(format!("Could not set the wallpaper: {e}")),
        }
    }

    /// Delete the file. If the app downloaded it, also make sure it never comes back.
    fn delete(&self, index: i32) {
        let Some(item) = self.item_at(index) else { return };
        if let Err(e) = fs::remove_file(&item.path) {
            self.say(format!("Could not delete {}: {e}", item.file));
            return;
        }
        if let Some(entry) = &item.entry {
            let mut library = self.library.lock().unwrap();
            library.block(entry.wallpaper.provider, &entry.wallpaper.id);
            if let Err(e) = library.save() {
                eprintln!("Gallery: could not save the library: {e}");
            }
        }
        self.items.borrow_mut().retain(|i| i.file != item.file);
        self.thumbs.borrow_mut().remove(&item.file);
        *self.selected_file.borrow_mut() = None;
        self.rebuild_sources();
        self.apply_filters();
        self.say(format!("Deleted {}.", item.title()));
    }

    fn open_link(&self, index: i32, link: impl Fn(&Item) -> Option<String>) {
        let Some(url) = self.item_at(index).and_then(|item| link(&item)) else { return };
        if let Err(e) = open::that(&url) {
            self.say(format!("Could not open {url}: {e}"));
        }
    }

    fn open_folder(&self) {
        let dir = self.download_dir();
        if let Err(e) = open::that(&dir) {
            self.say(format!("Could not open {}: {e}", dir.display()));
        }
    }
}
