import { useState, useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import * as cmd from "../../lib/commands";
import type { WallpaperInfo } from "../../lib/commands";

export default function PreviewWindow() {
    const [info, setInfo] = useState<WallpaperInfo | null>(null);
    const [isFav, setIsFav] = useState(false);

    useEffect(() => {
        // Path is passed as a query param: index.html?path=...
        const params = new URLSearchParams(window.location.search);
        const path = params.get("path");
        if (!path) return;

        cmd.getWallpaperInfo(path).then((i) => {
            setInfo(i);
            setIsFav(path.includes("wd_fav_"));
        });
    }, []);

    async function handleSet() {
        if (!info) return;
        await cmd.setWallpaper(info.path);
    }

    async function handleFavorite() {
        if (!info) return;
        await cmd.setFavorite([info.path], !isFav);
        setIsFav(!isFav);
    }

    async function handleDelete() {
        if (!info) return;
        if (!confirm("Delete this wallpaper?")) return;
        await cmd.deleteWallpapers([info.path]);
        await getCurrentWindow().close();
    }

    async function handleOpenLink(url: string) {
        const { openUrl } = await import("@tauri-apps/plugin-opener");
        await openUrl(url);
    }

    if (!info) return <div className="flex items-center justify-center h-screen text-gray-400">Loading…</div>;

    return (
        <div className="flex flex-col h-screen">
            {/* Image */}
            <div className="flex-1 flex items-center justify-center bg-black overflow-hidden">
                <img
                    src={convertFileSrc(info.path)}
                    alt=""
                    className="max-w-full max-h-full object-contain"
                />
            </div>

            {/* Info bar */}
            <div className="border-t p-3 flex items-center gap-4 text-sm bg-gray-50">
                <div className="flex-1 min-w-0">
                    <div className="font-mono text-xs truncate" title={info.path}>{info.path}</div>
                    <div className="text-xs text-gray-500">
                        {info.width} × {info.height}
                        {info.provider && <span> · {info.provider}</span>}
                    </div>
                </div>

                {info.provider_url && (
                    <button className="btn text-xs text-blue-600" onClick={() => handleOpenLink(info.provider_url)}>
                        Source ↗
                    </button>
                )}
                <button className="btn text-xs" onClick={handleFavorite}>
                    {isFav ? "★ Unfavorite" : "☆ Favorite"}
                </button>
                <button className="btn text-xs" onClick={handleSet}>🖥 Set</button>
                <button className="btn text-xs text-red-600" onClick={handleDelete}>🗑</button>
            </div>
        </div>
    );
}
