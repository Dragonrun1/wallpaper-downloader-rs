import { useState, useEffect } from "react";
import * as cmd from "../../lib/commands";
import type { Prefs } from "../../lib/commands";

export default function ChangerTab() {
    const [prefs, setPrefs] = useState<Prefs | null>(null);
    const [canChange, setCanChange] = useState(false);

    // Edit states
    const [editingInterval, setEditingInterval] = useState(false);
    const [interval, setInterval] = useState(5);
    const [units, setUnits] = useState("minutes");
    const [selectedFolder, setSelectedFolder] = useState<string | null>(null);

    useEffect(() => {
        cmd.getPrefs().then((p) => {
            setPrefs(p);
            setInterval(p.changer.interval);
            setUnits(p.changer.interval_units);
        });
        cmd.canChangeWallpaper().then(setCanChange);
    }, []);

    if (!prefs) return <div className="p-3 text-sm text-gray-500">Loading…</div>;

    const c = prefs.changer;

    async function applyInterval() {
        const updated = { ...prefs!, changer: { ...prefs!.changer, interval, interval_units: units } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
        setEditingInterval(false);
    }

    async function setMode(mode: string) {
        const updated = { ...prefs!, changer: { ...prefs!.changer, mode } };
        await cmd.savePrefs(updated);
        setPrefs(updated);
    }

    async function handleAddFolder() {
        const path = await cmd.pickFolder();
        if (!path) return;
        await cmd.addChangerFolder(path);
        const updated = await cmd.getPrefs();
        setPrefs(updated);
    }

    async function handleRemoveFolder() {
        if (!selectedFolder) return;
        await cmd.removeChangerFolder(selectedFolder);
        const updated = await cmd.getPrefs();
        setPrefs(updated);
        setSelectedFolder(null);
    }

    return (
        <div className="p-3 space-y-4 text-sm">
            {/* Interval */}
            <div className="flex items-center gap-2">
                <label className="w-[100px] shrink-0">Change every</label>
                {editingInterval ? (
                    <>
                        <input
                            type="number"
                            min={0}
                            className="w-16 border rounded px-2 py-1"
                            value={interval}
                            onChange={(e) => setInterval(+e.target.value)}
                            autoFocus
                        />
                        <select
                            className="border rounded px-1 py-1"
                            value={units}
                            onChange={(e) => setUnits(e.target.value)}
                        >
                            <option value="seconds">seconds</option>
                            <option value="minutes">minutes</option>
                            <option value="hours">hours</option>
                        </select>
                        <button className="btn" onClick={applyInterval}>✓</button>
                        <button className="btn" onClick={() => setEditingInterval(false)}>✕</button>
                    </>
                ) : (
                    <>
            <span className="border rounded px-2 py-1 bg-gray-50">
              {c.interval === 0 ? "Disabled" : `${c.interval} ${c.interval_units}`}
            </span>
                        <button className="btn" onClick={() => setEditingInterval(true)}>✎</button>
                    </>
                )}
            </div>

            {/* Mode */}
            <div className="flex items-center gap-2">
                <label className="w-[100px] shrink-0">Mode</label>
                <select
                    className="flex-1 border rounded px-2 py-1"
                    value={c.mode}
                    onChange={(e) => setMode(e.target.value)}
                >
                    <option value="random">Random</option>
                    <option value="sorted">Sorted (date)</option>
                    <option value="reverse">Reverse (date)</option>
                </select>
                {c.mode !== "random" && canChange && (
                    <button className="btn" title="Restart from beginning" onClick={() => cmd.setNextWallpaper()}>
                        ↺
                    </button>
                )}
            </div>

            <hr />

            {/* Folders */}
            <div className="flex items-start gap-2">
                <label className="w-[100px] shrink-0 pt-2">Directories</label>
                <div className="flex-1 border rounded h-[180px] overflow-y-auto p-1">
                    {c.folders.length === 0 && (
                        <div className="text-gray-400 px-2 py-1">No folders configured</div>
                    )}
                    {c.folders.map((f) => (
                        <div
                            key={f}
                            className={`px-2 py-1 cursor-pointer rounded truncate ${selectedFolder === f ? "bg-blue-100" : "hover:bg-gray-50"}`}
                            onClick={() => setSelectedFolder(f === selectedFolder ? null : f)}
                            title={f}
                        >
                            {f}
                        </div>
                    ))}
                </div>
                <div className="flex flex-col gap-1">
                    <button className="btn" onClick={handleAddFolder} title="Add directory">+</button>
                    <button
                        className="btn"
                        onClick={handleRemoveFolder}
                        disabled={!selectedFolder}
                        title="Remove selected directory"
                    >
                        −
                    </button>
                </div>
            </div>

            {/* Multi-monitor */}
            <div className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={c.multi_monitor}
                    onChange={async (e) => {
                        const updated = { ...prefs!, changer: { ...prefs!.changer, multi_monitor: e.target.checked } };
                        await cmd.savePrefs(updated);
                        setPrefs(updated);
                    }}
                />
                <label>Multi-monitor (span)</label>
            </div>

            <hr />

            {/* Quick actions */}
            {canChange && (
                <div className="flex gap-2">
                    <button className="btn" onClick={() => cmd.setRandomWallpaper()}>
                        🎲 Random
                    </button>
                    {c.mode !== "random" && (
                        <button className="btn" onClick={() => cmd.setNextWallpaper()}>
                            ⏭ Next
                        </button>
                    )}
                </div>
            )}
        </div>
    );
}
