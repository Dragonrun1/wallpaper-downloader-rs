import { useState, useEffect } from "react";
import * as cmd from "../../lib/commands";
import type { Prefs } from "../../lib/commands";

export default function ProvidersTab() {
    const [prefs, setPrefs] = useState<Prefs | null>(null);

    // Edit states
    const [editingKeywords, setEditingKeywords] = useState(false);
    const [keywords, setKeywords] = useState("");
    const [editingForbidden, setEditingForbidden] = useState(false);
    const [forbidden, setForbidden] = useState("");
    const [editingResolution, setEditingResolution] = useState(false);
    const [width, setWidth] = useState(1920);
    const [height, setHeight] = useState(1080);

    // Runtime state
    const [paused, setPaused] = useState(false);

    useEffect(() => {
        cmd.getPrefs().then((p) => {
            setPrefs(p);
            setWidth(p.resolution.width);
            setHeight(p.resolution.height);
        });
    }, []);

    if (!prefs) return <div className="p-3 text-sm text-gray-500">Loading…</div>;

    const p = prefs.providers;

    // ─── Dot-path setter (handles nested objects) ───────────────────────────
    function setField(path: string, value: unknown) {
        const updated = structuredClone(prefs!);
        const parts = path.split(".");
        let obj: any = updated;
        for (let i = 0; i < parts.length - 1; i++) obj = obj[parts[i]];
        obj[parts[parts.length - 1]] = value;
        setPrefs(updated);
        cmd.savePrefs(updated);
    }

    // ─── Apply handlers ─────────────────────────────────────────────────────
    async function applyKeywords() {
        setField("providers.wallhaven.keywords", keywords);
        setEditingKeywords(false);
    }

    async function applyForbidden() {
        setField("providers.forbidden_keywords", forbidden);
        setEditingForbidden(false);
    }

    async function applyResolution() {
        const updated = { ...prefs!, resolution: { width, height } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
        setEditingResolution(false);
    }

    function resetResolution() {
        setWidth(1920);
        setHeight(1080);
    }

    async function togglePause() {
        if (paused) await cmd.resumeHarvesting();
        else await cmd.pauseHarvesting();
        setPaused(!paused);
    }

    const anyProviderEnabled =
        p.wallhaven.enabled || p.deviantart.enabled || p.bing.enabled ||
        p.social_wallpapering.enabled || p.wallpaper_fusion.enabled ||
        p.dual_monitor.enabled || p.unsplash.enabled;

    return (
        <div className="p-3 space-y-3 text-sm">
            {/* Keywords (Wallhaven-specific) */}
            <div className="flex items-center gap-2">
                <label className="w-[70px] shrink-0">Keywords</label>
                {editingKeywords ? (
                    <>
                        <input
                            className="flex-1 border rounded px-2 py-1"
                            value={keywords}
                            onChange={(e) => setKeywords(e.target.value)}
                            autoFocus
                        />
                        <button className="btn" onClick={applyKeywords}>✓</button>
                        <button className="btn" onClick={() => setEditingKeywords(false)}>✕</button>
                    </>
                ) : (
                    <>
                        <span className="flex-1 border rounded px-2 py-1 bg-gray-50">{p.wallhaven.keywords || "—"}</span>
                        <button className="btn" onClick={() => { setKeywords(p.wallhaven.keywords); setEditingKeywords(true); }}>✎</button>
                    </>
                )}
            </div>

            {/* Forbidden keywords (global) */}
            <div className="flex items-center gap-2">
                <label className="w-[70px] shrink-0">Avoid</label>
                {editingForbidden ? (
                    <>
                        <input
                            className="flex-1 border rounded px-2 py-1"
                            value={forbidden}
                            onChange={(e) => setForbidden(e.target.value)}
                            autoFocus
                        />
                        <button className="btn" onClick={applyForbidden}>✓</button>
                        <button className="btn" onClick={() => setEditingForbidden(false)}>✕</button>
                    </>
                ) : (
                    <>
                        <span className="flex-1 border rounded px-2 py-1 bg-gray-50">{p.forbidden_keywords || "—"}</span>
                        <button className="btn" onClick={() => { setForbidden(p.forbidden_keywords); setEditingForbidden(true); }}>✎</button>
                    </>
                )}
            </div>

            {/* Resolution */}
            <div className="flex items-center gap-2">
                <label className="w-[70px] shrink-0">Resolution</label>
                {editingResolution ? (
                    <>
                        <input type="number" className="w-16 border rounded px-2 py-1" value={width} onChange={(e) => setWidth(+e.target.value)} />
                        <span>×</span>
                        <input type="number" className="w-16 border rounded px-2 py-1" value={height} onChange={(e) => setHeight(+e.target.value)} />
                        <button className="btn" onClick={applyResolution}>✓</button>
                        <button className="btn" onClick={resetResolution}>↺</button>
                        <button className="btn" onClick={() => setEditingResolution(false)}>✕</button>
                    </>
                ) : (
                    <>
                        <span className="border rounded px-2 py-1 bg-gray-50">{prefs.resolution.width} × {prefs.resolution.height}</span>
                        <button className="btn" onClick={() => setEditingResolution(true)}>✎</button>
                    </>
                )}
            </div>

            {/* Download policy */}
            <div className="flex items-center gap-2">
                <label className="w-[70px] shrink-0">Policy</label>
                <select
                    className="flex-1 border rounded px-2 py-1"
                    value={p.download_policy}
                    onChange={(e) => setField("providers.download_policy", +e.target.value)}
                >
                    <option value={0}>Fill disk</option>
                    <option value={1}>Keep last N</option>
                    <option value={2}>Random delete</option>
                </select>
            </div>

            <hr />

            {/* Providers */}
            <div className="space-y-2">
                <ProviderRow
                    label="Wallhaven"
                    checked={p.wallhaven.enabled}
                    onChange={(v) => setField("providers.wallhaven.enabled", v)}
                    searchType={p.wallhaven.search_type}
                    onSearchType={(v) => setField("providers.wallhaven.search_type", v)}
                    searchOptions={["top", "latest", "relevant", "trending", "resolution"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="DeviantArt"
                    checked={p.deviantart.enabled}
                    onChange={(v) => setField("providers.deviantart.enabled", v)}
                    searchType={p.deviantart.search_type}
                    onSearchType={(v) => setField("providers.deviantart.search_type", v)}
                    searchOptions={["relevance", "top_rated", "latest", "popular", "date_added", "newest"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Bing"
                    checked={p.bing.enabled}
                    onChange={(v) => setField("providers.bing.enabled", v)}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Social Wallpapering"
                    checked={p.social_wallpapering.enabled}
                    onChange={(v) => setField("providers.social_wallpapering.enabled", v)}
                    extra={
                        <label className="flex items-center gap-1 text-xs">
                            <input
                                type="checkbox"
                                checked={p.social_wallpapering.ignore_keywords}
                                onChange={(e) => setField("providers.social_wallpapering.ignore_keywords", e.target.checked)}
                                disabled={!p.social_wallpapering.enabled}
                            />
                            Ignore keywords
                        </label>
                    }
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="WallpaperFusion"
                    checked={p.wallpaper_fusion.enabled}
                    onChange={(v) => setField("providers.wallpaper_fusion.enabled", v)}
                    searchType={p.wallpaper_fusion.sort}
                    onSearchType={(v) => setField("providers.wallpaper_fusion.sort", v)}
                    searchOptions={["relevance", "popular", "newest"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Dual Monitor"
                    checked={p.dual_monitor.enabled}
                    onChange={(v) => setField("providers.dual_monitor.enabled", v)}
                    searchType={p.dual_monitor.search_type}
                    onSearchType={(v) => setField("providers.dual_monitor.search_type", v)}
                    searchOptions={["top", "latest", "relevant"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Unsplash"
                    checked={p.unsplash.enabled}
                    onChange={(v) => setField("providers.unsplash.enabled", v)}
                    searchType={p.unsplash.search_type}
                    onSearchType={(v) => setField("providers.unsplash.search_type", v)}
                    searchOptions={["relevant", "latest"]}
                />
            </div>

            {/* Pause / Resume + status */}
            <div className="flex items-center gap-2 pt-1">
                <button
                    className="btn"
                    onClick={togglePause}
                    disabled={!anyProviderEnabled}
                    title={paused ? "Resume harvesting" : "Pause harvesting"}
                >
                    {paused ? "▶ Resume" : "⏸ Pause"}
                </button>
                <span className={`w-3 h-3 rounded-full ${paused ? "bg-red-500" : "bg-green-500"}`} />
            </div>
        </div>
    );
}

// ─── Sub-component ───────────────────────────────────────────────────────────

function ProviderRow({
                         label,
                         checked,
                         onChange,
                         searchType,
                         onSearchType,
                         searchOptions,
                         extra,
                     }: {
    label: string;
    checked: boolean;
    onChange: (v: boolean) => void;
    searchType?: string;
    onSearchType?: (v: string) => void;
    searchOptions?: string[];
    extra?: React.ReactNode;
}) {
    return (
        <div className="flex items-center gap-2">
            <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
            <label className="w-[160px] shrink-0">{label}</label>
            {searchOptions && onSearchType && (
                <select
                    className="flex-1 border rounded px-1 py-0.5 text-xs"
                    value={searchType ?? ""}
                    onChange={(e) => onSearchType(e.target.value)}
                    disabled={!checked}
                >
                    {searchOptions.map((opt) => (
                        <option key={opt} value={opt}>{opt.replace("_", " ")}</option>
                    ))}
                </select>
            )}
            {extra}
        </div>
    );
}
