import { useState, useEffect } from "react";
import * as cmd from "../../lib/commands";
import type { Prefs } from "../../lib/commands";

export default function DownloadsTab() {
    const [prefs, setPrefs] = useState<Prefs | null>(null);
    const [diskUsage, setDiskUsage] = useState(0);

    useEffect(() => {
        cmd.getPrefs().then(setPrefs);
        refreshDisk();
        const interval = setInterval(refreshDisk, 10000);
        return () => clearInterval(interval);
    }, []);

    async function refreshDisk() {
        const usage = await cmd.getDiskUsage();
        setDiskUsage(usage);
    }

    async function handlePickDir() {
        const path = await cmd.pickFolder();
        if (!path) return;
        await cmd.changeDownloadsDir(path, false);
        const updated = await cmd.getPrefs();
        setPrefs(updated);
        refreshDisk();
    }

    async function handleCopy() {
        if (prefs) {
            await cmd.copyToClipboard(prefs.downloads.path);
        }
    }

    if (!prefs) return <div className="p-3 text-sm text-gray-500">Loading…</div>;

    const path = prefs.downloads.path;
    const showWarning = diskUsage >= 90;

    return (
        <div className="p-3 space-y-4 text-sm">
            {/* Directory path */}
            <div className="flex items-center gap-2">
                <label className="w-[120px] shrink-0">Downloads Dir</label>
                <span className="flex-1 border rounded px-2 py-1 bg-gray-50 truncate" title={path}>
          {path}
        </span>
                <button className="btn" onClick={handlePickDir} title="Change downloads directory">📁</button>
            </div>

            {/* Action buttons */}
            <div className="flex gap-2">
                <button className="btn" onClick={() => cmd.openDownloadsDir()} title="Open in file manager">
                    Open
                </button>
                <button className="btn" onClick={handleCopy} title="Copy path to clipboard">
                    Copy
                </button>
            </div>

            <hr />

            {/* Disk space */}
            <div className="flex items-center gap-2">
                <label className="w-[120px] shrink-0">Disk Space</label>
                <div className="flex-1 h-4 bg-gray-200 rounded overflow-hidden">
                    <div
                        className={`h-full transition-all ${diskUsage >= 90 ? "bg-red-500" : diskUsage >= 70 ? "bg-yellow-500" : "bg-green-500"}`}
                        style={{ width: `${diskUsage}%` }}
                    />
                </div>
                <span className="w-10 text-right">{diskUsage}%</span>
                {showWarning && (
                    <span className="text-red-500" title="Directory nearly full. Wallpapers will be removed randomly.">⚠</span>
                )}
            </div>

            {/* Move favorites */}
            {prefs.downloads.move_favorite && (
                <>
                    <hr />
                    <div className="flex items-center gap-2">
                        <label className="w-[120px] shrink-0">Move Favorites To</label>
                        <span className="flex-1 border rounded px-2 py-1 bg-gray-50 truncate" title={prefs.downloads.move_favorite_folder}>
              {prefs.downloads.move_favorite_folder || "—"}
            </span>
                        <button
                            className="btn"
                            onClick={async () => {
                                const dir = await cmd.pickFolder();
                                if (dir) await cmd.changeMoveDir(dir);
                            }}
                            title="Change move destination"
                        >
                            📁
                        </button>
                    </div>
                    <button className="btn" onClick={() => cmd.moveFavoriteWallpapers()}>
                        Move Favorites Now
                    </button>
                </>
            )}
        </div>
    );
}
