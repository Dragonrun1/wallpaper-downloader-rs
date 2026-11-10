export interface Prefs {
    providers: Providers;
    resolution: { width: number; height: number };
    harvester: { interval: number; interval_units: string };
    changer: {
        interval: number;
        interval_units: string;
        mode: string;
        folders: string[];
        multi_monitor: boolean;
        last_wallpaper: string | null;
    };
    downloads: {
        path: string;
        max_size: number;
        move_favorite: boolean;
        move_favorite_folder: string;
    };
    ui: {
        start_minimized: boolean;
        time_to_minimize: number;
        notifications: number;
        i18n: number;
    };
}

export interface Providers {
    wallhaven: boolean;
    wallhaven_search_type: number;
    deviantart: boolean;
    deviantart_search_type: number;
    bing: boolean;
    social_wallpapering: boolean;
    social_wallpapering_ignore_keywords: boolean;
    wallpaper_fusion: boolean;
    dual_monitor: boolean;
    dual_monitor_search_type: number;
    unsplash: boolean;
    unsplash_search_type: number;
    keywords: string;
    forbidden_keywords: string;
    download_policy: number;
}

export interface WallpaperInfo {
    path: string;
    width: number;
    height: number;
    provider: string;
    author: string;
    author_url: string;
    provider_url: string;
}
