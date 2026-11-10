import { useState, useEffect } from "react";
import * as cmd from "../../lib/commands";
import type { Prefs } from "../../lib/commands";

export default function SettingsTab() {
    const [prefs, setPrefs] = useState<Prefs | null>(null);

    // Edit states
    const [editingTimer, setEditingTimer] = useState(false);
    const [timer, setTimer] = useState(5);
    const [timerUnits, setTimerUnits] = useState("minutes");

    const [editingSize, setEditingSize] = useState(false);
    const [maxSize, setMaxSize] = useState(2);

    useEffect(() => {
        cmd.getPrefs().then((p) => {
            setPrefs(p);
            setTimer(p.harvester.interval);
            setTimerUnits(p.harvester.interval_units);
            setMaxSize(p.downloads.max_size);
        });
    }, []);

    if (!prefs) return <div className="p-3 text-sm text-gray-500">Loading…</div>;

    async function applyTimer() {
        const updated = { ...prefs!, harvester: { ...prefs!.harvester, interval: timer, interval_units: timerUnits } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
        setEditingTimer(false);
    }

    async function applySize() {
        const updated = { ...prefs!, downloads: { ...prefs!.downloads, max_size: maxSize } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
        setEditingSize(false);
    }

    function setField(path: string, value: unknown) {
        const updated = structuredClone(prefs!);
        const parts = path.split(".");
        let obj: any = updated;
        for (let i = 0; i < parts.length - 1; i++) obj = obj[parts[i]];
        obj[parts[parts.length - 1]] = value;
        setPrefs(updated);
        cmd.savePrefs(updated);
    }

    async function handleReset() {
        if (!confirm("Reset all settings to defaults?")) return;
        await cmd.resetSettings();
        const updated = await cmd.getPrefs();
        setPrefs(updated);
    }

    return (
        <div className="p-3 space-y-4 text-sm">
            {/* Download timer */}
            <div className="flex items-center gap-2">
                <label className="w-[140px] shrink-0">Download every</label>
                {editingTimer ? (
                    <>
                        <input
                            type="number"
                            min={1}
                            className="w-16 border rounded px-2 py-1"
                            value={timer}
                            onChange={(e) => setTimer(+e.target.value)}
                            autoFocus
                        />
                        <select
                            className="border rounded px-1 py-1"
                            value={timerUnits}
                            onChange={(e) => setTimerUnits(e.target.value)}
                        >
                            <option value="seconds">seconds</option>
                            <option value="minutes">minutes</option>
                            <option value="hours">hours</option>
                        </select>
                        <button className="btn" onClick={applyTimer}>✓</button>
                        <button className="btn" onClick={() => setEditingTimer(false)}>✕</button>
                    </>
                ) : (
                    <>
            <span className="border rounded px-2 py-1 bg-gray-50">
              {prefs.harvester.interval} {prefs.harvester.interval_units}
            </span>
                        <button className="btn" onClick={() => setEditingTimer(true)}>✎</button>
                    </>
                )}
            </div>

            {/* Max folder size */}
            <div className="flex items-center gap-2">
                <label className="w-[140px] shrink-0">Max folder size</label>
                {editingSize ? (
                    <>
                        <input
                            type="number"
                            min={1}
                            className="w-16 border rounded px-2 py-1"
                            value={maxSize}
                            onChange={(e) => setMaxSize(+e.target.value)}
                            autoFocus
                        />
                        <span className="text-gray-500">GB</span>
                        <button className="btn" onClick={applySize}>✓</button>
                        <button className="btn" onClick={() => setEditingSize(false)}>✕</button>
                    </>
                ) : (
                    <>
            <span className="border rounded px-2 py-1 bg-gray-50">
              {prefs.downloads.max_size} GB
            </span>
                        <button className="btn" onClick={() => setEditingSize(true)}>✎</button>
                    </>
                )}
            </div>

            <hr />

            {/* Move favorites */}
            <div className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={prefs.downloads.move_favorite}
                    onChange={(e) => setField("downloads.move_favorite", e.target.checked)}
                />
                <label>Move favorites after download</label>
            </div>
            {prefs.downloads.move_favorite && (
                <div className="flex items-center gap-2 pl-6">
          <span className="flex-1 border rounded px-2 py-1 bg-gray-50 truncate" title={prefs.downloads.move_favorite_folder}>
            {prefs.downloads.move_favorite_folder || "—"}
          </span>
                    <button
                        className="btn"
                        onClick={async () => {
                            const dir = await cmd.pickFolder();
                            if (dir) {
                                await cmd.changeMoveDir(dir);
                                const updated = await cmd.getPrefs();
                                setPrefs(updated);
                            }
                        }}
                    >
                        📁
                    </button>
                </div>
            )}

            <hr />

            {/* Notifications */}
            <div className="flex items-center gap-2">
                <label className="w-[140px] shrink-0">Notifications</label>
                <select
                    className="border rounded px-2 py-1"
                    value={prefs.ui.notifications}
                    onChange={(e) => setField("ui.notifications", +e.target.value)}
                >
                    <option value={0}>None</option>
                    <option value={1}>Important only</option>
                    <option value={2}>All</option>
                </select>
            </div>

            {/* i18n */}
            <div className="flex items-center gap-2">
                <label className="w-[140px] shrink-0">Language</label>
                <select
                    className="border rounded px-2 py-1"
                    value={prefs.ui.i18n}
                    onChange={(e) => setField("ui.i18n", +e.target.value)}
                >
                    <option value={0}>English</option>
                    <option value={1}>Español</option>
                </select>
            </div>

            <hr />

            {/* Start minimized */}
            <div className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={prefs.ui.start_minimized}
                    onChange={(e) => setField("ui.start_minimized", e.target.checked)}
                />
                <label>Start minimized</label>
            </div>
            {prefs.ui.start_minimized && (
                <div className="flex items-center gap-2 pl-6">
                    <label className="text-gray-500">Delay</label>
                    <select
                        className="border rounded px-2 py-1"
                        value={prefs.ui.time_to_minimize}
                        onChange={(e) => setField("ui.time_to_minimize", +e.target.value)}
                    >
                        {Array.from({ length: 10 }, (_, i) => (
                            <option key={i + 1} value={i + 1}>{i + 1}s</option>
                        ))}
                    </select>
                </div>
            )}

            <hr />

            {/* Autostart */}
            <div className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={false}
                    onChange={async (e) => {
                        await cmd.setAutostart(e.target.checked);
                    }}
                    ref={(el) => {
                        if (el) {
                            // Check on mount via a future command or store in prefs
                        }
                    }}
                />
                <label>Start on boot</label>
            </div>

            <hr />

            {/* Reset */}
            <button className="btn text-red-600 border-red-300" onClick={handleReset}>
                Reset All Settings
            </button>
        </div>
    );
}
