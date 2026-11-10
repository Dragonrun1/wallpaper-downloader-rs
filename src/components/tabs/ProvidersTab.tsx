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

    async function applyKeywords() {
        const updated = { ...prefs!, providers: { ...prefs!.providers, keywords } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
        setEditingKeywords(false);
    }

    async function applyForbidden() {
        const updated = { ...prefs!, providers: { ...prefs!.providers, forbidden_keywords: forbidden } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
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

    function toggleProvider(key: keyof typeof p, value: boolean) {
        const updated = { ...prefs!, providers: { ...prefs!.providers, [key]: value } };
        setPrefs(updated);
        cmd.savePrefs(updated);
    }

    function setProviderSearch(key: string, value: number) {
        const updated = { ...prefs!, providers: { ...prefs!.providers, [key]: value } };
        setPrefs(updated);
        cmd.savePrefs(updated);
    }

    async function togglePause() {
        if (paused) {
            await cmd.resumeHarvesting();
        } else {
            await cmd.pauseHarvesting();
        }
        setPaused(!paused);
    }

    const anyProviderEnabled =
        p.wallhaven || p.deviantart || p.bing ||
        p.social_wallpapering || p.wallpaper_fusion ||
        p.dual_monitor || p.unsplash;

    return (
        <div className="p-3 space-y-3 text-sm">
            {/* Keywords */}
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
                        <span className="flex-1 border rounded px-2 py-1 bg-gray-50">{p.keywords || "—"}</span>
                        <button className="btn" onClick={() => { setKeywords(p.keywords); setEditingKeywords(true); }}>✎</button>
                    </>
                )}
            </div>

            {/* Forbidden keywords */}
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
                    onChange={(e) => setProviderSearch("download_policy", +e.target.value)}
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
                    checked={p.wallhaven}
                    onChange={(v) => toggleProvider("wallhaven", v)}
                    searchType={p.wallhaven_search_type}
                    onSearchType={(v) => setProviderSearch("wallhaven_search_type", v)}
                    searchOptions={["Top", "Latest", "Relevant", "Trending", "Resolution"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="DeviantArt"
                    checked={p.deviantart}
                    onChange={(v) => toggleProvider("deviantart", v)}
                    searchType={p.deviantart_search_type}
                    onSearchType={(v) => setProviderSearch("deviantart_search_type", v)}
                    searchOptions={["Relevance", "Top Rated", "Latest", "Popular", "Date Added", "Newest"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Bing"
                    checked={p.bing}
                    onChange={(v) => toggleProvider("bing", v)}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Social Wallpapering"
                    checked={p.social_wallpapering}
                    onChange={(v) => toggleProvider("social_wallpapering", v)}
                    extra={
                        <label className="flex items-center gap-1 text-xs">
                            <input
                                type="checkbox"
                                checked={p.social_wallpapering_ignore_keywords}
                                onChange={(e) => toggleProvider("social_wallpapering_ignore_keywords", e.target.checked)}
                                disabled={!p.social_wallpapering}
                            />
                            Ignore keywords
                        </label>
                    }
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="WallpaperFusion"
                    checked={p.wallpaper_fusion}
                    onChange={(v) => toggleProvider("wallpaper_fusion", v)}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Dual Monitor"
                    checked={p.dual_monitor}
                    onChange={(v) => toggleProvider("dual_monitor", v)}
                    searchType={p.dual_monitor_search_type}
                    onSearchType={(v) => setProviderSearch("dual_monitor_search_type", v)}
                    searchOptions={["Top", "Latest", "Relevant"]}
                />
                <hr className="border-gray-200" />
                <ProviderRow
                    label="Unsplash"
                    checked={p.unsplash}
                    onChange={(v) => toggleProvider("unsplash", v)}
                    searchType={p.unsplash_search_type}
                    onSearchType={(v) => setProviderSearch("unsplash_search_type", v)}
                    searchOptions={["Relevant", "Latest"]}
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
    searchType?: number;
    onSearchType?: (v: number) => void;
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
                    value={searchType ?? 0}
                    onChange={(e) => onSearchType(+e.target.value)}
                    disabled={!checked}
                >
                    {searchOptions.map((opt, i) => (
                        <option key={i} value={i}>{opt}</option>
                    ))}
                </select>
            )}
            {extra}
        </div>
    );
}
