#![allow(non_snake_case)]
use crate::commands::*;
use leptos::prelude::*;
use std::collections::HashSet;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

const PAGE_SIZE: usize = 24;

#[derive(Clone, PartialEq)]
enum Filter {
    All,
    Favorite,
    NoFavorite,
}

#[component]
pub fn ManagerWindow() -> impl IntoView {
    let filter = RwSignal::new(Filter::All);
    let offset = RwSignal::new(0usize);
    let wallpapers = RwSignal::new::<Vec<String>>(vec![]);
    let total = RwSignal::new(0usize);
    let selected = RwSignal::new::<HashSet<String>>(HashSet::new());
    let loading = RwSignal::new(true);

    let refresh = move || {
        loading.set(true);
        spawn_local(async move {
            let f = filter.get();
            let o = offset.get();
            let (paths, count) = match f {
                Filter::All => {
                    let paths = invoke::<_, Vec<String>>(
                        "get_all_wallpapers",
                        &[("offset", &o), ("limit", &PAGE_SIZE)],
                    )
                    .await
                    .unwrap_or_default();
                    let count = invoke::<_, usize>("count_all_wallpapers", &[])
                        .await
                        .unwrap_or(0);
                    (paths, count)
                }
                Filter::Favorite => {
                    let paths = invoke::<_, Vec<String>>(
                        "get_favorite_wallpapers",
                        &[("offset", &o), ("limit", &PAGE_SIZE)],
                    )
                    .await
                    .unwrap_or_default();
                    let count = invoke::<_, usize>("count_wallpapers", &[("favorite", &true)])
                        .await
                        .unwrap_or(0);
                    (paths, count)
                }
                Filter::NoFavorite => {
                    let paths = invoke::<_, Vec<String>>(
                        "get_no_favorite_wallpapers",
                        &[("offset", &o), ("limit", &PAGE_SIZE)],
                    )
                    .await
                    .unwrap_or_default();
                    let count = invoke::<_, usize>("count_wallpapers", &[("favorite", &false)])
                        .await
                        .unwrap_or(0);
                    (paths, count)
                }
            };
            wallpapers.set(paths);
            total.set(count);
            selected.set(HashSet::new());
            loading.set(false);
        });
    };

    refresh();

    let on_filter_change = move |f: Filter| {
        filter.set(f);
        offset.set(0);
        refresh();
    };

    let on_toggle_select = move |path: String| {
        selected.update(|s| {
            if s.contains(&path) {
                s.remove(&path);
            } else {
                s.insert(path);
            }
        });
    };

    let on_select_all = move |_| {
        let current = wallpapers.get();
        selected.set(current.into_iter().collect());
    };

    let on_clear = move |_| {
        selected.set(HashSet::new());
    };

    let on_delete = move |_| {
        spawn_local(async move {
            let paths: Vec<String> = selected.get().into_iter().collect();
            if paths.is_empty() {
                return;
            }
            let _ = invoke::<_, ()>("delete_wallpapers", &[("paths", &paths)]).await;
            refresh();
        });
    };

    let on_favorite = move |_| {
        spawn_local(async move {
            let paths: Vec<String> = selected.get().into_iter().collect();
            if paths.is_empty() {
                return;
            }
            let is_fav = filter.get() == Filter::Favorite;
            let _ =
                invoke::<_, ()>("set_favorite", &[("paths", &paths), ("favorite", &!is_fav)]).await;
            refresh();
        });
    };

    let on_set = move |_| {
        spawn_local(async move {
            let sel = selected.get();
            if sel.len() != 1 {
                return;
            }
            let path = sel.iter().next().unwrap().clone();
            let _ = invoke::<_, ()>("set_wallpaper", &[("path", &path)]).await;
        });
    };

    let on_preview = move |path: String| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("open_preview", &[("path", &path)]).await;
        });
    };

    let on_prev = move |_| {
        offset.update(|o| *o = o.saturating_sub(PAGE_SIZE));
        selected.set(HashSet::new());
        refresh();
    };

    let on_next = move |_| {
        offset.update(|o| *o = o + PAGE_SIZE);
        selected.set(HashSet::new());
        refresh();
    };

    view! {
        <div class="flex flex-col h-screen p-3 gap-3">
            {/* Header */}
            <div class="flex items-center gap-2">
                <select class="border rounded px-2 py-1 text-sm"
                    value=move || match filter.get() {
                        Filter::All => "all".to_string(),
                        Filter::Favorite => "favorite".to_string(),
                        Filter::NoFavorite => "no_favorite".to_string(),
                    }
                    on:change=move |e| {
                        on_filter_change(match e.value().as_str() {
                            "favorite" => Filter::Favorite,
                            "no_favorite" => Filter::NoFavorite,
                            _ => Filter::All,
                        });
                    }
                >
                    <option value="all">"All"</option>
                    <option value="favorite">"Favorites"</option>
                    <option value="no_favorite">"Non-favorites"</option>
                </select>
                <span class="text-xs text-gray-500">
                    {move || format!("{} wallpapers", total.get())}
                </span>

                <div class="flex-1" />

                <button class="btn text-xs" on:click=on_select_all>"Select All"</button>
                <button class="btn text-xs" disabled=move || selected.get().is_empty() on:click=on_clear>"Clear"</button>
                <button class="btn text-xs" disabled=move || selected.get().is_empty() on:click=on_delete>"🗑 Delete"</button>
                <button class="btn text-xs" disabled=move || selected.get().is_empty() on:click=on_favorite>
                    {move || if filter.get() == Filter::Favorite { "Unfavorite".to_string() } else { "Favorite".to_string() }}
                </button>
                <button class="btn text-xs" disabled=move || selected.get().len() != 1 on:click=on_set>"🖥 Set"</button>
            </div>

            {/* Grid */}
            <div class="flex-1 overflow-y-auto">
                <Show when=move || loading.get()>
                    <div class="flex items-center justify-center h-full text-gray-400">"Loading…" </div>
                </Show>
                <Show when=move || !loading.get() && wallpapers.get().is_empty()>
                    <div class="flex items-center justify-center h-full text-gray-400">"No wallpapers"</div>
                </Show>
                <Show when=move || !loading.get() && !wallpapers.get().is_empty()>
                    <div class="grid grid-cols-4 gap-2">
                        <For
                            each=move || wallpapers.get()
                            key=move |p| p.clone()
                            let:path
                        >
                            {move || {
                                let is_selected = selected.get().contains(&path);
                                view! {
                                    <div
                                        class=move || {
                                            if is_selected {
                                                "relative rounded overflow-hidden cursor-pointer border-2 border-blue-500"
                                            } else {
                                                "relative rounded overflow-hidden cursor-pointer border-2 border-transparent hover:border-gray-300"
                                            }
                                        }
                                        on:click=move |_| on_toggle_select(path.clone())
                                        on:dblclick=move |_| on_preview(path.clone())
                                    >
                                        <img
                                            src=move || format!("asset://{}", path)
                                            alt=""
                                            class="w-full h-[120px] object-cover"
                                            loading="lazy"
                                        />
                                        <Show when=move || selected.get().contains(&path)>
                                            <div class="absolute top-1 right-1 w-5 h-5 bg-blue-500 rounded-full flex items-center justify-center text-white text-xs">"✓"</div>
                                        </Show>
                                    </div>
                                }
                            }}
                        </For>
                    </div>
                </Show>
            </div>

            {/* Pagination */}
            <Show when=move || total.get() > PAGE_SIZE>
                <div class="flex items-center justify-center gap-2 text-sm">
                    <button class="btn" disabled=move || offset.get() == 0 on:click=on_prev>"← Prev"</button>
                    <span class="text-gray-500">
                        {move || format!("Page {} / {}", offset.get() / PAGE_SIZE + 1, (total.get() + PAGE_SIZE - 1) / PAGE_SIZE)}
                    </span>
                    <button class="btn" disabled=move || offset.get() + PAGE_SIZE >= total.get() on:click=on_next>"Next →"</button>
                </div>
            </Show>
        </div>
    }
}
