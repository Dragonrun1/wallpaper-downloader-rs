#![allow(non_snake_case)]
use crate::commands::*;
use crate::types::Prefs;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn WallpapersTab() -> impl IntoView {
    let prefs_signal = RwSignal::new::<Option<Prefs>>(None);
    let can_change = RwSignal::new(false);
    let wallpapers = RwSignal::new::<Vec<String>>(vec![]);
    let selected = RwSignal::new::<Option<String>>(None);

    spawn_local(async move {
        let p = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
        prefs_signal.set(Some(p));
        let c = invoke::<_, bool>("can_change_wallpaper", &[])
            .await
            .unwrap_or(false);
        can_change.set(c);
        let paths = invoke::<_, Vec<String>>("get_recent_wallpapers", &[("count", &5)])
            .await
            .unwrap_or_default();
        wallpapers.set(paths);
    });

    let on_delete = move |_| {
        if let Some(path) = selected.get() {
            spawn_local(async move {
                let _ = invoke::<_, ()>("delete_wallpapers", &[("paths", &[path])]).await;
                selected.set(None);
                let paths = invoke::<_, Vec<String>>("get_recent_wallpapers", &[("count", &5)])
                    .await
                    .unwrap_or_default();
                wallpapers.set(paths);
            });
        }
    };

    let on_favorite = move |_| {
        if let Some(path) = selected.get() {
            spawn_local(async move {
                let is_fav = path.contains("wd_fav_");
                let _ = invoke::<_, ()>(
                    "set_favorite",
                    &[("paths", &[path]), ("favorite", &!is_fav)],
                )
                .await;
                let paths = invoke::<_, Vec<String>>("get_recent_wallpapers", &[("count", &5)])
                    .await
                    .unwrap_or_default();
                wallpapers.set(paths);
            });
        }
    };

    let on_set_wallpaper = move |_| {
        if let Some(path) = selected.get() {
            spawn_local(async move {
                let _ = invoke::<_, ()>("set_wallpaper", &[("path", &path)]).await;
            });
        }
    };

    let on_manager = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("open_manager", &[]).await;
        });
    };

    let on_choose = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("open_choose", &[]).await;
        });
    };

    let on_move = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("move_favorite_wallpapers", &[]).await;
        });
    };

    let on_random = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("set_random_wallpaper", &[]).await;
        });
    };

    let on_next = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("set_next_wallpaper", &[]).await;
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
                    let c = &prefs.changer;
                    view! {
                        <div class="p-3 space-y-3 text-sm">
                            {/* Last wallpapers strip */}
                            <label class="block text-xs text-gray-500">"Last 5 wallpapers"</label>
                            <div class="flex gap-2 overflow-x-auto pb-1 h-[100px]">
                                <Show when=move || wallpapers.get().is_empty()>
                                    <div class="text-gray-400 self-center px-2">"No wallpapers yet"</div>
                                </Show>
                                <For
                                    each=move || wallpapers.get()
                                    key=move |p| p.clone()
                                    let:path
                                >
                                    {move || {
                                        let is_selected = selected.get().as_deref() == Some(path.as_str());
                                        view! {
                                            <img
                                                src=move || format!("asset://{}", path)
                                                alt=""
                                                class=move || {
                                                    if is_selected {
                                                        "w-[128px] h-[100px] object-cover rounded cursor-pointer border-2 border-blue-500"
                                                    } else {
                                                        "w-[128px] h-[100px] object-cover rounded cursor-pointer border-2 border-transparent hover:border-gray-300"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    selected.update(|s| {
                                                        *s = if s.as_deref() == Some(path.as_str()) { None } else { Some(path.clone()) };
                                                    });
                                                }
                                            />
                                        }
                                    }}
                                </For>
                            </div>

                            {/* Action buttons */}
                            <div class="flex gap-2">
                                <button class="btn" disabled=move || selected.get().is_none() on:click=on_delete title="Delete selected">
                                    "🗑 Delete"
                                </button>
                                <button class="btn" disabled=move || selected.get().is_none() on:click=on_favorite title="Toggle favorite">
                                    "⭐ Favorite"
                                </button>
                                <button class="btn" disabled=move || selected.get().is_none()
                                    on:click=move |_| {
                                        if let Some(p) = selected.get() {
                                            spawn_local(async move {
                                                let _ = invoke::<_, ()>("open_preview", &[("path", &p)]).await;
                                            });
                                        }
                                    }
                                    title="Preview"
                                >"👁 Preview"</button>
                                <Show when=move || can_change.get()>
                                    <button class="btn" disabled=move || selected.get().is_none() on:click=on_set_wallpaper title="Set as wallpaper">
                                        "🖥 Set"
                                    </button>
                                </Show>
                            </div>

                            <hr />

                            {/* Secondary actions */}
                            <div class="flex flex-wrap gap-2">
                                <button class="btn" on:click=on_manager>"Manage Wallpapers"</button>
                                <button class="btn" on:click=on_choose>"Choose from Disk"</button>
                                <Show when=move || d.move_favorite>
                                    <button class="btn" on:click=on_move>"Move Favorites"</button>
                                </Show>
                            </div>

                            {/* Quick wallpaper actions */}
                            <Show when=move || can_change.get()>
                                <>
                                    <hr />
                                    <div class="flex gap-2">
                                        <button class="btn" on:click=on_random>"🎲 Random"</button>
                                        <Show when=move || c.mode != "random">
                                            <button class="btn" on:click=on_next>"⏭ Next"</button>
                                        </Show>
                                    </div>
                                </>
                            </Show>
                        </div>
                    }
                }}
            </For>
        </Show>
    }
}
