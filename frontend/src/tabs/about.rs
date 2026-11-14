#![allow(non_snake_case)]
use leptos::prelude::*;
use tauri_wasm::invoke;

#[component]
pub fn AboutTab() -> impl IntoView {
    // In about.rs (or anywhere you need to open a URL)
    let on_repo = move |_| {
        spawn_local(async move {
            let _ = invoke::<()>(
                "plugin:opener|open_url",
                &[("url", "https://github.com/estoes/WallpaperDownloader")],
            )
            .await;
        });
    };

    let on_icons = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("open_external", &[("url", "https://www.flaticon.com/")]).await;
        });
    };

    view! {
        <div class="p-3 space-y-3 text-sm">
            <div class="flex items-center gap-2">
                <label class="w-[90px] shrink-0">"Version"</label>
                <span class="font-mono">"0.1.0"</span>
            </div>

            <hr />

            <div class="flex items-center gap-2">
                <label class="w-[90px] shrink-0">"Developer"</label>
                <span>"Eloy García Almadén"</span>
            </div>

            <div class="flex items-center gap-2">
                <label class="w-[90px] shrink-0">"Icons"</label>
                <span>"Jaime Álvarez; Dave Gandy"</span>
                <button class="btn text-blue-600" on:click=on_icons>"flaticon.com"</button>
            </div>

            <div class="flex items-center gap-2">
                <label class="w-[90px] shrink-0">"Contributors"</label>
                <span>"Lee Burch; Guilherme Silva"</span>
            </div>

            <hr />

            <div class="flex items-center gap-2">
                <label class="w-[90px] shrink-0">"Source Code"</label>
                <button class="btn text-blue-600" on:click=on_repo>
                    "github.com/estoes/WallpaperDownloader"
                </button>
            </div>

            <hr />

            <div class="text-xs text-gray-500">
                "GPL v3 — Free software, no warranty of any kind."
            </div>
        </div>
    }
}
