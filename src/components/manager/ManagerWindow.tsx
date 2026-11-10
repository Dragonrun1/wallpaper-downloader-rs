import { useState, useEffect, useCallback } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import * as cmd from "../../lib/commands";

const PAGE_SIZE = 24;

type Filter = "all" | "favorite" | "no_favorite";

export default function ManagerWindow() {
    const [filter, setFilter] = useState<Filter>("all");
    const [offset, setOffset] = useState(0);
    const [wallpapers, setWallpapers] = useState<string[]>([]);
    const [total, setTotal] = useState(0);
    const [selected, setSelected] = useState<Set<string>>(new Set());
    const [loading, setLoading] = useState(true);

    const window = getCurrentWindow();

    const refresh = useCallback(async () => {
        setLoading(true);
        let paths: string[];
        let count: number;

        if (filter === "all") {
            [paths, count] = await Promise.all([
                cmd.getAllWallpapers(offset, PAGE_SIZE),
                cmd.countAllWallpapers(),
            ]);
        } else if (filter === "favorite") {
            [paths, count] = await Promise.all([
                cmd.getFavoriteWallpapers(offset, PAGE_SIZE),
                cmd.countWallpapers(true),
            ]);
        } else {
            [paths, count] = await Promise.all([
                cmd.getNoFavoriteWallpapers(offset, PAGE_SIZE),
                cmd.countWallpapers(false),
            ]);
        }

        setWallpapers(paths);
        setTotal(count);
        setSelected(new Set());
        setLoading(false);
    }, [filter, offset]);

    useEffect(() => { refresh(); }, [refresh]);

    function toggleSelect(path: string) {
        setSelected((prev) => {
            const next = new Set(prev);
            if (next.has(path)) next.delete(path);
            else next.add(path);
            return next;
        });
    }

    function selectAll() {
        setSelected(new Set(wallpapers));
    }

    function clearSelection() {
        setSelected(new Set());
    }

    async function handleDelete() {
        if (selected.size === 0) return;
        if (!confirm(`Delete ${selected.size} wallpaper(s)?`)) return;
        await cmd.deleteWallpapers([...selected]);
        await refresh();
    }

    async function handleFavorite() {
        if (selected.size === 0) return;
        const isFav = filter === "favorite";
        await cmd.setFavorite([...selected], !isFav);
        await refresh();
    }

    async function handleSet() {
        if (selected.size !== 1) return;
        await cmd.setWallpaper([...selected][0]);
    }

    const totalPages = Math.ceil(total / PAGE_SIZE);
    const page = Math.floor(offset / PAGE_SIZE) + 1;

    return (
        <div className="flex flex-col h-screen p-3 gap-3">
            {/* Header: filter + actions */}
            <div className="flex items-center gap-2">
                <select
                    className="border rounded px-2 py-1 text-sm"
                    value={filter}
                    onChange={(e) => { setFilter(e.target.value as Filter); setOffset(0); }}
                >
                    <option value="all">All</option>
                    <option value="favorite">Favorites</option>
                    <option value="no_favorite">Non-favorites</option>
                </select>
                <span className="text-xs text-gray-500">{total} wallpapers</span>

                <div className="flex-1" />

                <button className="btn text-xs" onClick={selectAll}>Select All</button>
                <button className="btn text-xs" onClick={clearSelection} disabled={selected.size === 0}>Clear</button>
                <button className="btn text-xs" disabled={selected.size === 0} onClick={handleDelete}>
                    🗑 Delete
                </button>
                <button className="btn text-xs" disabled={selected.size === 0} onClick={handleFavorite}>
                    ⭐ {filter === "favorite" ? "Unfavorite" : "Favorite"}
                </button>
                <button className="btn text-xs" disabled={selected.size !== 1} onClick={handleSet}>
                    🖥 Set
                </button>
            </div>

            {/* Grid */}
            <div className="flex-1 overflow-y-auto">
                {loading ? (
                    <div className="flex items-center justify-center h-full text-gray-400">Loading…</div>
                ) : wallpapers.length === 0 ? (
                    <div className="flex items-center justify-center h-full text-gray-400">No wallpapers</div>
                ) : (
                    <div className="grid grid-cols-4 gap-2">
                        {wallpapers.map((path) => (
                            <div
                                key={path}
                                className={`relative rounded overflow-hidden cursor-pointer border-2 ${
                                    selected.has(path) ? "border-blue-500" : "border-transparent hover:border-gray-300"
                                }`}
                                onClick={() => toggleSelect(path)}
                                onDoubleClick={() => cmd.openPreview(path)}
                            >
                                <img
                                    src={convertFileSrc(path)}
                                    alt=""
                                    className="w-full h-[120px] object-cover"
                                    loading="lazy"
                                />
                                {selected.has(path) && (
                                    <div className="absolute top-1 right-1 w-5 h-5 bg-blue-500 rounded-full flex items-center justify-center text-white text-xs">✓</div>
                                )}
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Pagination */}
            <div className="flex items-center justify-center gap-2 text-sm">
                <button className="btn" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>
                    ← Prev
                </button>
                <span className="text-gray-500">Page {page} / {totalPages || 1}</span>
                <button
                    className="btn"
                    disabled={offset + PAGE_SIZE >= total}
                    onClick={() => setOffset(offset + PAGE_SIZE)}
                >
                    Next →
                </button>
            </div>
        </div>
    );
}
