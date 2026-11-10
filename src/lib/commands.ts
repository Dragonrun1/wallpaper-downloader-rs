import {invoke} from "@tauri-apps/api/core";

// Prefs
export const getPrefs = () => invoke<Prefs>("get_prefs");
export const savePrefs = (prefs: Prefs) => invoke("save_prefs", {prefs});
export const resetSettings = () => invoke("reset_settings");

// Harvester
export const pauseHarvesting = () => invoke("pause_harvesting");
export const resumeHarvesting = () => invoke("resume_harvesting");

// Wallpaper
export const setWallpaper = (path: string) => invoke("set_wallpaper", {path});
export const canChangeWallpaper = () => invoke<boolean>("can_change_wallpaper");
export const setRandomWallpaper = () => invoke("set_random_wallpaper");
export const setNextWallpaper = () => invoke("set_next_wallpaper");

// Disk
export const getDiskUsage = () => invoke<number>("get_disk_usage");

// Wallpapers list
export const getRecentWallpapers = (count: number) => invoke<string[]>("get_recent_wallpapers", {count});
export const getAllWallpapers = (offset: number, limit: number) => invoke<string[]>("get_all_wallpapers", {
    offset,
    limit
});
export const getFavoriteWallpapers = (offset: number, limit: number) => invoke<string[]>("get_favorite_wallpapers", {
    offset,
    limit
});
export const getNoFavoriteWallpapers = (offset: number, limit: number) => invoke<string[]>("get_no_favorite_wallpapers", {
    offset,
    limit
});
export const countWallpapers = (favorite: boolean) => invoke<number>("count_wallpapers", {favorite});
export const countAllWallpapers = () => invoke<number>("count_all_wallpapers");
export const getWallpaperInfo = (path: string) => invoke<WallpaperInfo>("get_wallpaper_info", {path});
export const listImages = (dir: string, offset: number, limit: number) => invoke<string[]>("list_images", { dir, offset, limit });

// Wallpaper actions
export const deleteWallpapers = (paths: string[]) => invoke("delete_wallpapers", {paths});
export const setFavorite = (paths: string[], favorite: boolean) => invoke("set_favorite", {paths, favorite});
export const moveFavoriteWallpapers = () => invoke("move_favorite_wallpapers");

// Folders
export const pickFolder = () => invoke<string | null>("pick_folder");
export const changeDownloadsDir = (newPath: string, deleteOld: boolean) => invoke("change_downloads_dir", {
    newPath,
    deleteOld
});
export const addChangerFolder = (path: string) => invoke("add_changer_folder", {path});
export const removeChangerFolder = (path: string) => invoke("remove_changer_folder", {path});
export const changeMoveDir = (newPath: string) => invoke("change_move_dir", {newPath});

// System
export const setAutostart = (enable: boolean) => invoke("set_autostart", {enable});
export const openDownloadsDir = () => invoke("open_downloads_dir");
export const copyToClipboard = (text: string) => invoke("copy_to_clipboard", {text});

// Secondary windows
export const openManager = () => invoke("open_manager");
export const openChoose = () => invoke("open_choose");
export const openPreview = (path: string) => invoke("open_preview", {path});
