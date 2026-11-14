#![allow(non_snake_case)]
use crate::commands::*;
use crate::types::Prefs;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ProvidersTab() -> impl IntoView {
    let prefs_signal = RwSignal::new::<Option<Prefs>>(None);

    spawn_local(async move {
        let p = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
        prefs_signal.set(Some(p));
    });

    let editing_keywords = RwSignal::new(false);
    let keywords = RwSignal::new(String::new());
    let editing_forbidden = RwSignal::new(false);
    let forbidden = RwSignal::new(String::new());
    let editing_resolution = RwSignal::new(false);
    let width = RwSignal::new(1920u32);
    let height = RwSignal::new(1080u32);
    let paused = RwSignal::new(false);

    Effect::new(move |_| {
        if let Some(p) = prefs_signal.get() {
            width.set(p.resolution.width);
            height.set(p.resolution.height);
        }
    });

    let save = move |updated: Prefs| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
            prefs_signal.set(Some(updated));
        });
    };

    let on_apply_keywords = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.providers.wallhaven.keywords = keywords.get();
        editing_keywords.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_apply_forbidden = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.providers.forbidden_keywords = forbidden.get();
        editing_forbidden.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_apply_resolution = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.resolution.width = width.get();
        p.resolution.height = height.get();
        editing_resolution.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_toggle_pause = move |_| {
        let is_paused = paused.get();
        spawn_local(async move {
            if is_paused {
                let _ = invoke::<_, ()>("resume_harvesting", &[]).await;
            } else {
                let _ = invoke::<_, ()>("pause_harvesting", &[]).await;
            }
        });
        paused.update(|p| *p = !*p);
    };

    view! {
        <Show when=move || prefs_signal.get().is_some() fallback=|| view! { <p>"Loading…" </p> }>
            <For
                each=move || prefs_signal.get()
                key=move |p| format!("{:?}", p)
                let:prefs
            >
                {move || {
                    let p = &prefs.providers;
                    let any_enabled = p.wallhaven.enabled || p.deviantart.enabled
                        || p.bing.enabled || p.social_wallpapering.enabled
                        || p.wallpaper_fusion.enabled || p.dual_monitor.enabled
                        || p.unsplash.enabled;

                    view! {
                        <div class="p-3 space-y-3 text-sm">
                            {/* Keywords */}
                            <div class="flex items-center gap-2">
                                <label class="w-[70px] shrink-0">"Keywords"</label>
                                <Show
                                    when=move || editing_keywords.get()
                                    fallback=move || view! {
                                        <span class="flex-1 border rounded px-2 py-1 bg-gray-50">
                                            {p.wallhaven.keywords.clone()}
                                        </span>
                                        <button class="btn"
                                            on:click=move |_| {
                                                keywords.set(p.wallhaven.keywords.clone());
                                                editing_keywords.set(true);
                                            }
                                        >"✎"</button>
                                    }
                                >
                                    <input
                                        class="flex-1 border rounded px-2 py-1"
                                        value=move || keywords.get()
                                        on:input=move |e| keywords.set(e.value())
                                    />
                                    <button class="btn" on:click=on_apply_keywords.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| editing_keywords.set(false)>"✕"</button>
                                </Show>
                            </div>

                            {/* Forbidden keywords */}
                            <div class="flex items-center gap-2">
                                <label class="w-[70px] shrink-0">"Avoid"</label>
                                <Show
                                    when=move || editing_forbidden.get()
                                    fallback=move || view! {
                                        <span class="flex-1 border rounded px-2 py-1 bg-gray-50">
                                            {p.forbidden_keywords.clone()}
                                        </span>
                                        <button class="btn"
                                            on:click=move |_| {
                                                forbidden.set(p.forbidden_keywords.clone());
                                                editing_forbidden.set(true);
                                            }
                                        >"✎"</button>
                                    }
                                >
                                    <input
                                        class="flex-1 border rounded px-2 py-1"
                                        value=move || forbidden.get()
                                        on:input=move |e| forbidden.set(e.value())
                                    />
                                    <button class="btn" on:click=on_apply_forbidden.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| editing_forbidden.set(false)>"✕"</button>
                                </Show>
                            </div>

                            {/* Resolution */}
                            <div class="flex items-center gap-2">
                                <label class="w-[70px] shrink-0">"Resolution"</label>
                                <Show
                                    when=move || editing_resolution.get()
                                    fallback=move || view! {
                                        <span class="border rounded px-2 py-1 bg-gray-50">
                                            {format!("{} × {}", prefs.resolution.width, prefs.resolution.height)}
                                        </span>
                                        <button class="btn" on:click=move |_| editing_resolution.set(true)>"✎"</button>
                                    }
                                >
                                    <input type="number" class="w-16 border rounded px-2 py-1"
                                        value=move || width.get().to_string()
                                        on:input=move |e| width.set(e.value().parse().unwrap_or(1920))
                                    />
                                    <span>"×"</span>
                                    <input type="number" class="w-16 border rounded px-2 py-1"
                                        value=move || height.get().to_string()
                                        on:input=move |e| height.set(e.value().parse().unwrap_or(1080))
                                    />
                                    <button class="btn" on:click=on_apply_resolution.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| {
                                        width.set(1920);
                                        height.set(1080);
                                    }>"↺"</button>
                                    <button class="btn" on:click=move |_| editing_resolution.set(false)>"✕"</button>
                                </Show>
                            </div>

                            {/* Download policy */}
                            <div class="flex items-center gap-2">
                                <label class="w-[70px] shrink-0">"Policy"</label>
                                <select class="flex-1 border rounded px-2 py-1"
                                    value=move || p.download_policy.to_string()
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.download_policy = e.value().parse().unwrap_or(0);
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="0">"Fill disk"</option>
                                    <option value="1">"Keep last N"</option>
                                    <option value="2">"Random delete"</option>
                                </select>
                            </div>

                            <hr />

                            {/* Wallhaven */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.wallhaven.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.wallhaven.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"Wallhaven"</label>
                                <select class="flex-1 border rounded px-1 py-0.5 text-xs"
                                    value=move || p.wallhaven.search_type.clone()
                                    disabled=move || !p.wallhaven.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.wallhaven.search_type = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="top">"top"</option>
                                    <option value="latest">"latest"</option>
                                    <option value="relevant">"relevant"</option>
                                    <option value="trending">"trending"</option>
                                    <option value="resolution">"resolution"</option>
                                </select>
                            </div>
                            <hr class="border-gray-200" />

                            {/* DeviantArt */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.deviantart.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.deviantart.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"DeviantArt"</label>
                                <select class="flex-1 border rounded px-1 py-0.5 text-xs"
                                    value=move || p.deviantart.search_type.clone()
                                    disabled=move || !p.deviantart.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.deviantart.search_type = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="relevance">"relevance"</option>
                                    <option value="top_rated">"top rated"</option>
                                    <option value="latest">"latest"</option>
                                    <option value="popular">"popular"</option>
                                    <option value="date_added">"date added"</option>
                                    <option value="newest">"newest"</option>
                                </select>
                            </div>
                            <hr class="border-gray-200" />

                            {/* Bing */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.bing.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.bing.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"Bing"</label>
                            </div>
                            <hr class="border-gray-200" />

                            {/* Social Wallpapering */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.social_wallpapering.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.social_wallpapering.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"Social Wallpapering"</label>
                                <label class="flex items-center gap-1 text-xs">
                                    <input type="checkbox"
                                        checked=move || p.social_wallpapering.ignore_keywords
                                        disabled=move || !p.social_wallpapering.enabled
                                        on:change=move |e| {
                                            let mut pr = prefs_signal.get().unwrap();
                                            pr.providers.social_wallpapering.ignore_keywords = e.checked();
                                            let updated = pr.clone();
                                            spawn_local(async move {
                                                let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                                prefs_signal.set(Some(updated));
                                            });
                                        }
                                    />
                                    "Ignore keywords"
                                </label>
                            </div>
                            <hr class="border-gray-200" />

                            {/* WallpaperFusion */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.wallpaper_fusion.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.wallpaper_fusion.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"WallpaperFusion"</label>
                                <select class="flex-1 border rounded px-1 py-0.5 text-xs"
                                    value=move || p.wallpaper_fusion.sort.clone()
                                    disabled=move || !p.wallpaper_fusion.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.wallpaper_fusion.sort = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="relevance">"relevance"</option>
                                    <option value="popular">"popular"</option>
                                    <option value="newest">"newest"</option>
                                </select>
                            </div>
                            <hr class="border-gray-200" />

                            {/* Dual Monitor */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.dual_monitor.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.dual_monitor.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"Dual Monitor"</label>
                                <select class="flex-1 border rounded px-1 py-0.5 text-xs"
                                    value=move || p.dual_monitor.search_type.clone()
                                    disabled=move || !p.dual_monitor.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.dual_monitor.search_type = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="top">"top"</option>
                                    <option value="latest">"latest"</option>
                                    <option value="relevant">"relevant"</option>
                                </select>
                            </div>
                            <hr class="border-gray-200" />

                            {/* Unsplash */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.unsplash.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.unsplash.enabled = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label class="w-[160px] shrink-0">"Unsplash"</label>
                                <select class="flex-1 border rounded px-1 py-0.5 text-xs"
                                    value=move || p.unsplash.search_type.clone()
                                    disabled=move || !p.unsplash.enabled
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.providers.unsplash.search_type = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="relevant">"relevant"</option>
                                    <option value="latest">"latest"</option>
                                </select>
                            </div>

                            {/* Pause / Resume */}
                            <div class="flex items-center gap-2 pt-1">
                                <button class="btn" on:click=on_toggle_pause disabled=move || !any_enabled>
                                    {move || if paused.get() { "▶ Resume".to_string() } else { "⏸ Pause".to_string() }}
                                </button>
                                <span
                                    class=move || {
                                        format!("w-3 h-3 rounded-full {}", if paused.get() { "bg-red-500" } else { "bg-green-500" })
                                    }
                                />
                            </div>
                        </div>
                    }
                }}
            </For>
        </Show>
    }
}
