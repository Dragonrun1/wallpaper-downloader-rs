#![allow(non_snake_case)]
use leptos::prelude::*;

#[component]
pub fn HelpTab() -> impl IntoView {
    view! {
        <div class="p-3 text-sm space-y-3">
            <h3 class="font-semibold">"Tips"</h3>

            <ul class="list-disc pl-5 space-y-1 text-gray-700">
                <li>"Keywords — Separate multiple keywords with ;. Leave empty to download all wallpapers."</li>
                <li>"Avoid keywords — Wallpapers matching these will be skipped. Same ; separator."</li>
                <li>"Unsplash policy — Maximum 1 download per hour. The app enforces this automatically."</li>
                <li>"Changer — Set interval to 0 to disable auto-changing."</li>
                <li>"Disk full — When the downloads folder exceeds the max size, oldest wallpapers are deleted randomly."</li>
                <li>"Move favorites — Favorited wallpapers are moved to the configured directory after each download cycle."</li>
                <li>"System tray — Minimize to tray for quick access to pause/resume, random wallpaper, and exit."</li>
            </ul>

            <hr />

            <h3 class="font-semibold">"Troubleshooting"</h3>
            <ul class="list-disc pl-5 space-y-1 text-gray-700">
                <li>"Wallpaper not changing? Check that your desktop environment is supported (see the Changer tab)."</li>
                <li>"No downloads? Ensure at least one provider is enabled and the download timer is set."</li>
                <li>"Reset all settings from the Settings tab if behavior is unexpected."</li>
            </ul>
        </div>
    }
}
