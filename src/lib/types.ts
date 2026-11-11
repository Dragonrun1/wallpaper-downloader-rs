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
        i18n: number;
        notifications: "none" | "minimal" | "full";
        start_minimized: boolean;
        time_to_minimize: number;
        system_tray_icon: boolean;
    };
}

export interface Providers {
    bing: boolean;
    deviantart: boolean;
    deviantart_search_type: number;
    download_policy: number;
    dual_monitor: boolean;
    dual_monitor_search_type: number;
    forbidden_keywords: string;
    keywords: string;
    social_wallpapering: boolean;
    social_wallpapering_ignore_keywords: boolean;
    unsplash: boolean;
    unsplash_search_type: number;
    wallhaven: boolean;
    wallhaven_search_type: number;
    wallpaper_fusion: boolean;
}

export interface WallpaperInfo {
    author: string;
    author_url: string;
    height: number;
    path: string;
    provider: string;
    provider_url: string;
    width: number;
}
