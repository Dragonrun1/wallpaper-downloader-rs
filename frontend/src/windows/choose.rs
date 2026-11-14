#![allow(non_snake_case)]
use crate::commands::*;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

const PAGE_SIZE: usize = 24;

#[component]
pub fn ChooseWindow() -> impl IntoView {
    let dir = RwSignal::new::<Option<String>>(None);
    let files = RwSignal::new::<Vec<String>>(vec![]);
    let total = RwSignal::new(0usize);
    let offset = RwSignal::new(0usize);
    let selected = RwSignal::new::<Option<String>>(None);

    let on_browse = move |_| {
        spawn_local(async move {
            let path = invoke::<_, String>("pick_folder", &[]).await;
            if let Ok(Some(d)) = path {
                dir.set(Some(d));
                offset.set(0);
                selected.set(None);
                let paths = invoke::<_, Vec<String>>(
                    "list_images",
                    &[("dir", &d), ("offset", &0usize), ("limit", &PAGE_SIZE)],
                )
                .await
                .unwrap_or_default();
                total.set(if paths.len() == PAGE_SIZE {
                    PAGE_SIZE + 1
                } else {
                    paths.len()
                });
                files.set(paths);
            }
        });
    };

    let on_set = move |_| {
        spawn_local(async move {
            if let Some(path) = selected.get() {
                let _ = invoke::<_, ()>("set_wallpaper", &[("path", &path)]).await;
                web_sys::window().and_then(|w| w.close());
            }
        });
    };

    let on_prev = move |_| {
        offset.update(|o| *o = o.saturating_sub(PAGE_SIZE));
        selected.set(None);
        spawn_local(async move {
            if let Some(d) = dir.get() {
                let o = offset.get();
                let paths = invoke::<_, Vec<String>>(
                    "list_images",
                    &[("dir", &d), ("offset", &o), ("limit", &PAGE_SIZE)],
                )
                .await
                .unwrap_or_default();
                total.set(if paths.len() == PAGE_SIZE {
                    o + PAGE_SIZE + 1
                } else {
                    o + paths.len()
                });
                files.set(paths);
            }
        });
    };

    let on_next = move |_| {
        offset.update(|o| *o = o + PAGE_SIZE);
        selected.set(None);
        spawn_local(async move {
            if let Some(d) = dir.get() {
                let o = offset.get();
                let paths = invoke::<_, Vec<String>>(
                    "list_images",
                    &[("dir", &d), ("offset", &o), ("limit", &PAGE_SIZE)],
                )
                .await
                .unwrap_or_default();
                total.set(if paths.len() == PAGE_SIZE {
                    o + PAGE_SIZE + 1
                } else {
                    o + paths.len()
                });
                files.set(paths);
            }
        });
    };

    view! {
        <div class="flex flex-col h-screen p-3 gap-3">
            {/* Header */}
            <div class="flex items-center gap-2">
                <button class="btn" on:click=on_browse>"📁 Browse…"</button>
                <Show when=move || dir.get().is_some()>
                    <span class="text-xs text-gray-500 truncate flex-1" title=move || dir.get().unwrap_or_default()>
                        {move || dir.get().unwrap_or_default()}
                    </span>
                </Show>
                <div class="flex-1" />
                <button class="btn" disabled=move || selected.get().is_none() on:click=on_set>
                    "🖥 Set as Wallpaper"
                </button>
            </div>

            {/* Grid */}
            <div class="flex-1 overflow-y-auto">
                <Show when=move || dir.get().is_none()>
                    <div class="flex items-center justify-center h-full text-gray-400 text-sm">
                        "Pick a folder to browse images"
                    </div>
                </Show>
                <Show when=move || dir.get().is_some() && files.get().is_empty()>
                    <div class="flex items-center justify-center h-full text-gray-400 text-sm">
                        "No images found"
                    </div>
                </Show>
                <Show when=move || dir.get().is_some() && !files.get().is_empty()>
                    <div class="grid grid-cols-4 gap-2">
                        <For
                            each=move || files.get()
                            key=move |p| p.clone()
                            let:path
                        >
                            {move || {
                                let is_selected = selected.get().as_deref() == Some(path.as_str());
                                view! {
                                    <div
                                        class=move || {
                                            if is_selected {
                                                "rounded overflow-hidden cursor-pointer border-2 border-blue-500"
                                            } else {
                                                "rounded overflow-hidden cursor-pointer border-2 border-transparent hover:border-gray-300"
                                            }
                                        }
                                        on:click=move |_| {
                                            selected.update(|s| {
                                                *s = if s.as_deref() == Some(path.as_str()) { None } else { Some(path.clone()) };
                                            });
                                        }
                                    >
                                        <img
                                            src=move || format!("asset://{}", path)
                                            alt=""
                                            class="w-full h-[120px] object-cover"
                                            loading="lazy"
                                        />
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
                        {move || format!("Page {}", offset.get() / PAGE_SIZE + 1)}
                    </span>
                    <button class="btn" disabled=move || offset.get() + PAGE_SIZE >= total.get() on:click=on_next>"Next →"</button>
                </div>
            </Show>
        </div>
    }
}
