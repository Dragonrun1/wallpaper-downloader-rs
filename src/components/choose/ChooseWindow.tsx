import { useState, useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as cmd from "../../lib/commands";

const PAGE_SIZE = 24;

export default function ChooseWindow() {
    const [dir, setDir] = useState<string | null>(null);
    const [files, setFiles] = useState<string[]>([]);
    const [offset, setOffset] = useState(0);
    const [total, setTotal] = useState(0);
    const [selected, setSelected] = useState<string | null>(null);

    async function browse() {
        const path = await cmd.pickFolder();
        if (!path) return;
        setDir(path);
        setOffset(0);
        setSelected(null);
        // For now, list image files in the picked directory
        // (You'd want a Rust command `list_images(dir, offset, limit)` for this)
        // Placeholder: just show the dir path
    }

    useEffect(() => {
        if (!dir) return;
        cmd.listImages(dir, offset, PAGE_SIZE).then((paths) => {
            setFiles(paths);
            setTotal(paths.length); // or use a separate count command
        });
    }, [dir, offset]);

    const totalPages = Math.ceil(total / PAGE_SIZE);

    return (
        <div className="flex flex-col h-screen p-3 gap-3">
            {/* Header */}
            <div className="flex items-center gap-2">
                <button className="btn" onClick={browse}>📁 Browse…</button>
                {dir && (
                    <span className="text-xs text-gray-500 truncate flex-1" title={dir}>{dir}</span>
                )}
                <div className="flex-1" />
                <button
                    className="btn"
                    disabled={!selected}
                    onClick={async () => {
                        if (selected) {
                            await cmd.setWallpaper(selected);
                            // Close this window after setting
                            const { getCurrentWindow } = await import("@tauri-apps/api/window");
                            await getCurrentWindow().close();
                        }
                    }}
                >
                    🖥 Set as Wallpaper
                </button>
            </div>

            {/* Grid */}
            <div className="flex-1 overflow-y-auto">
                {!dir ? (
                    <div className="flex items-center justify-center h-full text-gray-400 text-sm">
                        Pick a folder to browse images
                    </div>
                ) : files.length === 0 ? (
                    <div className="flex items-center justify-center h-full text-gray-400 text-sm">
                        No images found
                    </div>
                ) : (
                    <div className="grid grid-cols-4 gap-2">
                        {files.map((path) => (
                            <div
                                key={path}
                                className={`rounded overflow-hidden cursor-pointer border-2 ${
                                    selected === path ? "border-blue-500" : "border-transparent hover:border-gray-300"
                                }`}
                                onClick={() => setSelected(path)}
                            >
                                <img
                                    src={convertFileSrc(path)}
                                    alt=""
                                    className="w-full h-[120px] object-cover"
                                    loading="lazy"
                                />
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Pagination */}
            {total > 0 && (
                <div className="flex items-center justify-center gap-2 text-sm">
                    <button className="btn" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>
                        ← Prev
                    </button>
                    <span className="text-gray-500">Page {Math.floor(offset / PAGE_SIZE) + 1} / {totalPages}</span>
                    <button className="btn" disabled={offset + PAGE_SIZE >= total} onClick={() => setOffset(offset + PAGE_SIZE)}>
                        Next →
                    </button>
                </div>
            )}
        </div>
    );
}
