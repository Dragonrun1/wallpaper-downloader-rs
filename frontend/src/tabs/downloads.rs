#![allow(non_snake_case)]
use crate::commands::*;
use crate::types::Prefs;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn DownloadsTab() -> impl IntoView {
    let prefs_signal = RwSignal::new::<Option<Prefs>>(None);
    let disk_usage = RwSignal::new(0u8);

    spawn_local(async move {
        let p = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
        prefs_signal.set(Some(p));
        let usage = invoke::<_, u8>("get_disk_usage", &[]).await.unwrap_or(0);
        disk_usage.set(usage);
    });

    set_interval(
        move || {
            spawn_local(async move {
                let usage = invoke::<_, u8>("get_disk_usage", &[]).await.unwrap_or(0);
                disk_usage.set(usage);
            });
        },
        10_000,
    );

    let on_pick_dir = move |_| {
        spawn_local(async move {
            let path = invoke::<_, String>("pick_folder", &[]).await;
            if let Ok(Some(dir)) = path {
                let _ = invoke::<_, ()>(
                    "change_downloads_dir",
                    &[("new_path", &dir), ("delete_old", &false)],
                )
                .await;
                let updated = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
                prefs_signal.set(Some(updated));
            }
        });
    };

    let on_open = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("open_downloads_dir", &[]).await;
        });
    };

    let on_copy = move |_| {
        spawn_local(async move {
            if let Some(p) = prefs_signal.get() {
                let _ = invoke::<_, ()>("copy_to_clipboard", &[("text", &p.downloads.path)]).await;
            }
        });
    };

    let on_pick_move_dir = move |_| {
        spawn_local(async move {
            let path = invoke::<_, String>("pick_folder", &[]).await;
            if let Ok(Some(dir)) = path {
                let _ = invoke::<_, ()>("change_move_dir", &[("new_path", &dir)]).await;
                let updated = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
                prefs_signal.set(Some(updated));
            }
        });
    };

    let on_move_favorites = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("move_favorite_wallpapers", &[]).await;
        });
    };

    view! {
        <Show when=move || prefs_signal.get().is_some() fallback=|| view! { <p>"Loading…" </p> }>
            <For
                each=move || prefs_signal.get()
                key=move |p| format!("{:?}", p)
                let:prefs
            >
                {move || {
                    let d = &prefs.downloads;
                    view! {
                        <div class="p-3 space-y-4 text-sm">
                            {/* Directory path */}
                            <div class="flex items-center gap-2">
                                <label class="w-[120px] shrink-0">"Downloads Dir"</label>
                                <span class="flex-1 border rounded px-2 py-1 bg-gray-50 truncate" title=move || d.path.clone()>
                                    {d.path.clone()}
                                </span>
                                <button class="btn" on:click=on_pick_dir title="Change downloads directory">"📁"</button>
                            </div>

                            {/* Action buttons */}
                            <div class="flex gap-2">
                                <button class="btn" on:click=on_open title="Open in file manager">"Open"</button>
                                <button class="btn" on:click=on_copy title="Copy path to clipboard">"Copy"</button>
                            </div>

                            <hr />

                            {/* Disk space */}
                            <div class="flex items-center gap-2">
                                <label class="w-[120px] shrink-0">"Disk Space"</label>
                                <div class="flex-1 h-4 bg-gray-200 rounded overflow-hidden">
                                    <div
                                        class=move || {
                                            let u = disk_usage.get();
                                            if u >= 90 { "h-full bg-red-500" }
                                            else if u >= 70 { "h-full bg-yellow-500" }
                                            else { "h-full bg-green-500" }
                                        }
                                        style=move || format!("width: {}%", disk_usage.get())
                                    />
                                </div>
                                <span class="w-10 text-right">
                                    {move || format!("{}%", disk_usage.get())}
                                </span>
                                <Show when=move || disk_usage.get() >= 90>
                                    <span class="text-red-500" title="Directory nearly full">"⚠"</span>
                                </Show>
                            </div>

                            {/* Move favorites */}
                            <Show when=move || d.move_favorite>
                                <>
                                    <hr />
                                    <div class="flex items-center gap-2">
                                        <label class="w-[120px] shrink-0">"Move Favorites To"</label>
                                        <span class="flex-1 border rounded px-2 py-1 bg-gray-50 truncate" title=move || d.move_favorite_folder.clone()>
                                            {d.move_favorite_folder.clone()}
                                        </span>
                                        <button class="btn" on:click=on_pick_move_dir title="Change move destination">"📁"</button>
                                    </div>
                                    <button class="btn" on:click=on_move_favorites>
                                        "Move Favorites Now"
                                    </button>
                                </>
                            </Show>
                        </div>
                    }
                }}
            </For>
        </Show>
    }
}
