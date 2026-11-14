use serde::Serialize;
use crate::types::Prefs;

// ─── Opener plugin ──────────────────────────────────────────────────────────

#[wasm_bindgen::prelude::wasm_bindgen(inline_js = "export function openUrl(url) { window.__TAURI_INTERNALS__.invoke('plugin:opener|open_url', { url }); }")]
extern "C" {
    pub fn open_url(url: &str);
}

// ─── Args structs ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SavePrefsArgs { pub prefs: Prefs }

#[derive(Serialize)]
pub struct AddChangerFolderArgs { pub new_path: String }

#[derive(Serialize)]
pub struct RemoveChangerFolderArgs { pub path: String }

#[derive(Serialize)]
pub struct SetWallpaperArgs { pub path: String }

#[derive(Serialize)]
pub struct ChangeDownloadsDirArgs { pub new_path: String, pub delete_old: bool }

#[derive(Serialize)]
pub struct GetRecentWallpapersArgs { pub count: usize }

#[derive(Serialize)]
pub struct GetAllWallpapersArgs { pub offset: usize, pub limit: usize }

#[derive(Serialize)]
pub struct GetFavoriteWallpapersArgs { pub offset: usize, pub limit: usize }

#[derive(Serialize)]
pub struct GetNoFavoriteWallpapersArgs { pub offset: usize, pub limit: usize }

#[derive(Serialize)]
pub struct CountWallpapersArgs { pub favorite: bool }

#[derive(Serialize)]
pub struct GetWallpaperInfoArgs { pub path: String }

#[derive(Serialize)]
pub struct ListImagesArgs { pub dir: String, pub offset: usize, pub limit: usize }

#[derive(Serialize)]
pub struct DeleteWallpapersArgs { pub paths: Vec<String> }

#[derive(Serialize)]
pub struct SetFavoriteArgs { pub paths: Vec<String>, pub favorite: bool }

#[derive(Serialize)]
pub struct ChangeMoveDirArgs { pub new_path: String }

#[derive(Serialize)]
pub struct SetAutostartArgs { pub enable: bool }

#[derive(Serialize)]
pub struct CopyToClipboardArgs { pub text: String }

#[derive(Serialize)]
pub struct OpenPreviewArgs { pub path: String }

// ─── Helper functions ───────────────────────────────────────────────────────

pub async fn get_prefs() -> Result<Prefs, tauri_wasm::Error> {
    tauri_wasm::invoke("get_prefs").await
}

pub async fn save_prefs(prefs: Prefs) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&SavePrefsArgs { prefs })?;
    tauri_wasm::invoke("save_prefs").with_args(args).await
}

pub async fn reset_settings() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("reset_settings").await
}

pub async fn pause_harvesting() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("pause_harvesting").await
}

pub async fn resume_harvesting() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("resume_harvesting").await
}

pub async fn add_changer_folder(new_path: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&AddChangerFolderArgs { new_path })?;
    tauri_wasm::invoke("add_changer_folder").with_args(args).await
}

pub async fn remove_changer_folder(path: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&RemoveChangerFolderArgs { path })?;
    tauri_wasm::invoke("remove_changer_folder").with_args(args).await
}

pub async fn set_random_wallpaper() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("set_random_wallpaper").await
}

pub async fn set_next_wallpaper() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("set_next_wallpaper").await
}

pub async fn set_wallpaper(path: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&SetWallpaperArgs { path })?;
    tauri_wasm::invoke("set_wallpaper").with_args(args).await
}

pub async fn can_change_wallpaper() -> Result<bool, tauri_wasm::Error> {
    tauri_wasm::invoke("can_change_wallpaper").await
}

pub async fn get_disk_usage() -> Result<u8, tauri_wasm::Error> {
    tauri_wasm::invoke("get_disk_usage").await
}

pub async fn change_downloads_dir(new_path: String, delete_old: bool) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&ChangeDownloadsDirArgs { new_path, delete_old })?;
    tauri_wasm::invoke("change_downloads_dir").with_args(args).await
}

pub async fn open_downloads_dir() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("open_downloads_dir").await
}

pub async fn get_recent_wallpapers(count: usize) -> Result<Vec<String>, tauri_wasm::Error> {
    let args = tauri_wasm::args(&GetRecentWallpapersArgs { count })?;
    tauri_wasm::invoke("get_recent_wallpapers").with_args(args).await
}

pub async fn get_all_wallpapers(offset: usize, limit: usize) -> Result<Vec<String>, tauri_wasm::Error> {
    let args = tauri_wasm::args(&GetAllWallpapersArgs { offset, limit })?;
    tauri_wasm::invoke("get_all_wallpapers").with_args(args).await
}

pub async fn get_favorite_wallpapers(offset: usize, limit: usize) -> Result<Vec<String>, tauri_wasm::Error> {
    let args = tauri_wasm::args(&GetFavoriteWallpapersArgs { offset, limit })?;
    tauri_wasm::invoke("get_favorite_wallpapers").with_args(args).await
}

pub async fn get_no_favorite_wallpapers(offset: usize, limit: usize) -> Result<Vec<String>, tauri_wasm::Error> {
    let args = tauri_wasm::args(&GetNoFavoriteWallpapersArgs { offset, limit })?;
    tauri_wasm::invoke("get_no_favorite_wallpapers").with_args(args).await
}

pub async fn count_wallpapers(favorite: bool) -> Result<usize, tauri_wasm::Error> {
    let args = tauri_wasm::args(&CountWallpapersArgs { favorite })?;
    tauri_wasm::invoke("count_wallpapers").with_args(args).await
}

pub async fn count_all_wallpapers() -> Result<usize, tauri_wasm::Error> {
    tauri_wasm::invoke("count_all_wallpapers").await
}

pub async fn get_wallpaper_info(path: String) -> Result<crate::types::WallpaperInfo, tauri_wasm::Error> {
    let args = tauri_wasm::args(&GetWallpaperInfoArgs { path })?;
    tauri_wasm::invoke("get_wallpaper_info").with_args(args).await
}

pub async fn list_images(dir: String, offset: usize, limit: usize) -> Result<Vec<String>, tauri_wasm::Error> {
    let args = tauri_wasm::args(&ListImagesArgs { dir, offset, limit })?;
    tauri_wasm::invoke("list_images").with_args(args).await
}

pub async fn delete_wallpapers(paths: Vec<String>) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&DeleteWallpapersArgs { paths })?;
    tauri_wasm::invoke("delete_wallpapers").with_args(args).await
}

pub async fn set_favorite(paths: Vec<String>, favorite: bool) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&SetFavoriteArgs { paths, favorite })?;
    tauri_wasm::invoke("set_favorite").with_args(args).await
}

pub async fn move_favorite_wallpapers() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("move_favorite_wallpapers").await
}

pub async fn pick_folder() -> Result<Option<String>, tauri_wasm::Error> {
    tauri_wasm::invoke("pick_folder").await
}

pub async fn change_move_dir(new_path: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&ChangeMoveDirArgs { new_path })?;
    tauri_wasm::invoke("change_move_dir").with_args(args).await
}

pub async fn set_autostart(enable: bool) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&SetAutostartArgs { enable })?;
    tauri_wasm::invoke("set_autostart").with_args(args).await
}

pub async fn copy_to_clipboard(text: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&CopyToClipboardArgs { text })?;
    tauri_wasm::invoke("copy_to_clipboard").with_args(args).await
}

pub async fn open_manager() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("open_manager").await
}

pub async fn open_choose() -> Result<(), tauri_wasm::Error> {
    tauri_wasm::invoke("open_choose").await
}

pub async fn open_preview(path: String) -> Result<(), tauri_wasm::Error> {
    let args = tauri_wasm::args(&OpenPreviewArgs { path })?;
    tauri_wasm::invoke("open_preview").with_args(args).await
}
