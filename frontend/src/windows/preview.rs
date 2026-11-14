#![allow(non_snake_case)]
use crate::commands::*;
use crate::types::WallpaperInfo;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn PreviewWindow() -> impl IntoView {
    let info = RwSignal::new::<Option<WallpaperInfo>>(None);
    let is_fav = RwSignal::new(false);

    spawn_local(async move {
        let path = get_path_param();
        if path.is_empty() {
            return;
        }
        if let Ok(i) = invoke::<_, WallpaperInfo>("get_wallpaper_info", &[("path", &path)]).await {
            is_fav.set(i.path.contains("wd_fav_"));
            info.set(Some(i));
        }
    });

    let on_set = move |_| {
        spawn_local(async move {
            if let Some(i) = info.get() {
                let _ = invoke::<_, ()>("set_wallpaper", &[("path", &i.path)]).await;
            }
        });
    };

    let on_favorite = move |_| {
        spawn_local(async move {
            if let Some(i) = info.get() {
                let fav = !is_fav.get();
                let _ = invoke::<_, ()>(
                    "set_favorite",
                    &[("paths", &[i.path.clone()]), ("favorite", &fav)],
                )
                .await;
                is_fav.set(fav);
            }
        });
    };

    let on_delete = move |_| {
        spawn_local(async move {
            if let Some(i) = info.get() {
                let _ = invoke::<_, ()>("delete_wallpapers", &[("paths", &[i.path])]).await;
                web_sys::window().and_then(|w| w.close());
            }
        });
    };

    let on_source = move |url: String| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("plugin:opener|open_url", &[("url", &url)]).await;
        });
    };

    view! {
        <div class="flex flex-col h-screen">
            <Show
                when=move || info.get().is_some()
                fallback=|| view! { <div class="flex-1 flex items-center justify-center text-gray-400">"Loading…" </div> }
            >
                <For
                    each=move || info.get()
                    key=move |i| i.path.clone()
                    let:i
                >
                    {move || {
                        view! {
                            <div class="flex-1 flex items-center justify-center bg-black overflow-hidden">
                                <img
                                    src=move || format!("asset://{}", i.path)
                                    alt=""
                                    class="max-w-full max-h-full object-contain"
                                />
                            </div>

                            <div class="border-t p-3 flex items-center gap-3 text-sm bg-gray-50">
                                <div class="flex-1 min-w-0">
                                    <div class="font-mono text-xs truncate" title=move || i.path.clone()>
                                        {i.path.clone()}
                                    </div>
                                    <div class="text-xs text-gray-500">
                                        {format!("{} × {}{}", i.width, i.height,
                                            if i.provider != "unknown" { format!(" · {}", i.provider) } else { String::new() })}
                                    </div>
                                </div>

                                <Show when=move || !i.provider_url.is_empty()>
                                    <button class="btn text-xs text-blue-600" on:click=move |_| on_source(i.provider_url.clone())>
                                        "Source ↗"
                                    </button>
                                </Show>

                                <button class="btn text-xs" on:click=on_favorite>
                                    {move || if is_fav.get() { "★ Unfavorite".to_string() } else { "☆ Favorite".to_string() }}
                                </button>

                                <button class="btn text-xs" on:click=on_set>"🖥 Set"</button>
                                <button class="btn text-xs text-red-600" on:click=on_delete>"🗑"</button>
                            </div>
                        }
                    }}
                </For>
            </Show>
        </div>
    }
}

// ─── Helper ──────────────────────────────────────────────────────────────────

fn get_path_param() -> String {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();

    let encoded = search.trim_start_matches("?path=");
    // Percent-decode
    let bytes: Vec<u8> = (0..encoded.len())
        .filter_map(|i| {
            if encoded.as_bytes()[i] == b'%' && i + 2 < encoded.len() {
                let hex = std::str::from_utf8(&encoded.as_bytes()[i + 1..i + 3]).unwrap_or("00");
                u8::from_str_radix(hex, 16).ok()
            } else {
                encoded.as_bytes()[i].to_string().parse().ok()
            }
        })
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}
