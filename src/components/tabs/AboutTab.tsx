import { useState, useEffect } from "react";

export default function AboutTab() {
    const [version] = useState("0.1.0");

    function openLink(url: string) {
        // Use tauri-plugin-opener
        import("@tauri-apps/plugin-opener").then(({ openUrl }) => openUrl(url));
    }

    return (
        <div className="p-3 space-y-3 text-sm">
            {/* Version */}
            <div className="flex items-center gap-2">
                <label className="w-[90px] shrink-0">Version</label>
                <span className="font-mono">{version}</span>
            </div>

            <hr />

            {/* Developer */}
            <div className="flex items-center gap-2">
                <label className="w-[90px] shrink-0">Developer</label>
                <span>Eloy García Almadén</span>
            </div>

            {/* Icons */}
            <div className="flex items-center gap-2">
                <label className="w-[90px] shrink-0">Icons</label>
                <span>Jaime Álvarez; Dave Gandy</span>
                <button
                    className="btn text-blue-600"
                    onClick={() => openLink("https://www.flaticon.com/")}
                >
                    flaticon.com
                </button>
            </div>

            {/* Contributors */}
            <div className="flex items-center gap-2">
                <label className="w-[90px] shrink-0">Contributors</label>
                <span>Lee Burch; Guilherme Silva</span>
            </div>

            <hr />

            {/* Source code */}
            <div className="flex items-center gap-2">
                <label className="w-[90px] shrink-0">Source Code</label>
                <button
                    className="btn text-blue-600"
                    onClick={() => openLink("https://github.com/estoes/WallpaperDownloader")}
                >
                    github.com/estoes/WallpaperDownloader
                </button>
            </div>

            <hr />

            {/* License */}
            <div className="text-xs text-gray-500">
                GPL v3 — Free software, no warranty of any kind.
            </div>
        </div>
    );
}
