export default function HelpTab() {
    return (
        <div className="p-3 text-sm space-y-3">
            <h3 className="font-semibold">Tips</h3>

            <ul className="list-disc pl-5 space-y-1 text-gray-700">
                <li>
                    <strong>Keywords</strong> — Separate multiple keywords with <code>;</code>. Leave empty to download all wallpapers.
                </li>
                <li>
                    <strong>Avoid keywords</strong> — Wallpapers matching these will be skipped. Same <code>;</code> separator.
                </li>
                <li>
                    <strong>Unsplash policy</strong> — Maximum 1 download per hour. The app enforces this automatically.
                </li>
                <li>
                    <strong>Changer</strong> — Set interval to <code>0</code> to disable auto-changing.
                </li>
                <li>
                    <strong>Disk full</strong> — When the downloads folder exceeds the max size, oldest wallpapers are deleted randomly.
                </li>
                <li>
                    <strong>Move favorites</strong> — Favorited wallpapers are moved to the configured directory after each download cycle.
                </li>
                <li>
                    <strong>System tray</strong> — Minimize to tray for quick access to pause/resume, random wallpaper, and exit.
                </li>
            </ul>

            <hr />

            <h3 className="font-semibold">Troubleshooting</h3>
            <ul className="list-disc pl-5 space-y-1 text-gray-700">
                <li>
                    Wallpaper not changing? Check that your desktop environment is supported (see the Changer tab).
                </li>
                <li>
                    No downloads? Ensure at least one provider is enabled and the download timer is set.
                </li>
                <li>
                    Reset all settings from the Settings tab if behavior is unexpected.
                </li>
            </ul>
        </div>
    );
}
