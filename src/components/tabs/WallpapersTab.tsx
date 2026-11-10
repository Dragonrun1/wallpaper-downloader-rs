import { useState, useEffect, useCallback } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as cmd from "../../lib/commands";
import type { Prefs } from "../../lib/commands";

export default function WallpapersTab() {
    const [prefs, setPrefs] = useState<Prefs | null>(null);
    const [canChange, setCanChange] = useState(false);
    const [wallpapers, setWallpapers] = useState<string[]>([]);
    const [selected, setSelected] = useState<string | null>(null);

    const refresh = useCallback(async () => {
        const paths = await cmd.getRecentWallpapers(5);
        setWallpapers(paths);
        setSelected(null);
    }, []);

    useEffect(() => {
        cmd.getPrefs().then(setPrefs);
        cmd.canChangeWallpaper().then(setCanChange);
        refresh();
    }, []);

    async function handleDelete() {
        if (!selected) return;
        await cmd.deleteWallpapers([selected]);
        await refresh();
    }

    async function handleFavorite() {
        if (!selected) return;
        const isFav = selected.includes("wd_fav_");
        await cmd.setFavorite([selected], !isFav);
        await refresh();
    }

    async function handleSetWallpaper() {
        if (!selected) return;
        await cmd.setWallpaper(selected);
    }

    async function handleMove() {
        await cmd.moveFavoriteWallpapers();
        await refresh();
    }

    if (!prefs) return <div className="p-3 text-sm text-gray-500">Loading…</div>;

    return (
        <div className="p-3 space-y-3 text-sm">
            {/* Last wallpapers strip */}
            <label className="block text-xs text-gray-500">Last 5 wallpapers</label>
            <div className="flex gap-2 overflow-x-auto pb-1 h-[100px]">
                {wallpapers.length === 0 && (
                    <div className="text-gray-400 self-center px-2">No wallpapers yet</div>
                )}
                {wallpapers.map((path) => (
                    <img
                        key={path}
                        src={convertFileSrc(path)}
                        alt=""
                        className={`w-[128px] h-[100px] object-cover rounded cursor-pointer border-2 ${
                            selected === path ? "border-blue-500" : "border-transparent hover:border-gray-300"
                        }`}
                        onClick={() => setSelected(selected === path ? null : path)}
                    />
                ))}
            </div>

            {/* Action buttons */}
            <div className="flex gap-2">
                <button className="btn" disabled={!selected} onClick={handleDelete} title="Delete selected">
                    🗑 Delete
                </button>
                <button className="btn" disabled={!selected} onClick={handleFavorite} title="Toggle favorite">
                    ⭐ Favorite
                </button>
                <button className="btn" disabled={!selected} onClick={() => cmd.openPreview(selected!)} title="Preview">
                    👁 Preview
                </button>
                {canChange && (
                    <button className="btn" disabled={!selected} onClick={handleSetWallpaper} title="Set as wallpaper">
                        🖥 Set
                    </button>
                )}
            </div>

            <hr />

            {/* Secondary actions */}
            <div className="flex flex-wrap gap-2">
                <button className="btn" onClick={() => cmd.openManager()}>
                    Manage Wallpapers
                </button>
                <button className="btn" onClick={() => cmd.openChoose()}>
                    Choose from Disk
                </button>
                {prefs.downloads.move_favorite && (
                    <button className="btn" onClick={handleMove}>
                        Move Favorites
                    </button>
                )}
            </div>

            {/* Quick wallpaper actions */}
            {canChange && (
                <>
                    <hr />
                    <div className="flex gap-2">
                        <button className="btn" onClick={() => cmd.setRandomWallpaper()}>
                            🎲 Random
                        </button>
                        {prefs.changer.mode !== "random" && (
                            <button className="btn" onClick={() => cmd.setNextWallpaper()}>
                                ⏭ Next
                            </button>
                        )}
                    </div>
                </>
            )}
        </div>
    );
}
