#![allow(non_snake_case)]
use crate::commands::*;
use crate::types::Prefs;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn SettingsTab() -> impl IntoView {
    let prefs_signal = RwSignal::new::<Option<Prefs>>(None);

    spawn_local(async move {
        let p = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
        prefs_signal.set(Some(p));
    });

    let editing_timer = RwSignal::new(false);
    let timer = RwSignal::new(5u32);
    let timer_units = RwSignal::new("minutes".to_string());
    let editing_size = RwSignal::new(false);
    let max_size = RwSignal::new(2u32);

    Effect::new(move |_| {
        if let Some(p) = prefs_signal.get() {
            timer.set(p.harvester.interval);
            timer_units.set(p.harvester.interval_units.clone());
            max_size.set(p.downloads.max_size_gb);
        }
    });

    let save = move |updated: Prefs| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
            prefs_signal.set(Some(updated));
        });
    };

    let on_apply_timer = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.harvester.interval = timer.get();
        p.harvester.interval_units = timer_units.get();
        editing_timer.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_apply_size = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.downloads.max_size_gb = max_size.get();
        editing_size.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_reset = move |_| {
        spawn_local(async move {
            let _ = invoke::<_, ()>("reset_settings", &[]).await;
            let p = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
            prefs_signal.set(Some(p));
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
                    let p = &prefs;
                    view! {
                        <div class="p-3 space-y-4 text-sm">
                            {/* Download timer */}
                            <div class="flex items-center gap-2">
                                <label class="w-[140px] shrink-0">"Download every"</label>
                                <Show
                                    when=move || editing_timer.get()
                                    fallback=move || view! {
                                        <span class="border rounded px-2 py-1 bg-gray-50">
                                            {format!("{} {}", p.harvester.interval, p.harvester.interval_units)}
                                        </span>
                                        <button class="btn" on:click=move |_| editing_timer.set(true)>"✎"</button>
                                    }
                                >
                                    <input type="number" min="1" class="w-16 border rounded px-2 py-1"
                                        value=move || timer.get().to_string()
                                        on:input=move |e| timer.set(e.value().parse().unwrap_or(1))
                                    />
                                    <select class="border rounded px-1 py-1"
                                        value=move || timer_units.get()
                                        on:change=move |e| timer_units.set(e.value())
                                    >
                                        <option value="seconds">"seconds"</option>
                                        <option value="minutes">"minutes"</option>
                                        <option value="hours">"hours"</option>
                                    </select>
                                    <button class="btn" on:click=on_apply_timer.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| editing_timer.set(false)>"✕"</button>
                                </Show>
                            </div>

                            {/* Max folder size */}
                            <div class="flex items-center gap-2">
                                <label class="w-[140px] shrink-0">"Max folder size"</label>
                                <Show
                                    when=move || editing_size.get()
                                    fallback=move || view! {
                                        <span class="border rounded px-2 py-1 bg-gray-50">
                                            {format!("{} GB", p.downloads.max_size_gb)}
                                        </span>
                                        <button class="btn" on:click=move |_| editing_size.set(true)>"✎"</button>
                                    }
                                >
                                    <input type="number" min="1" class="w-16 border rounded px-2 py-1"
                                        value=move || max_size.get().to_string()
                                        on:input=move |e| max_size.set(e.value().parse().unwrap_or(1))
                                    />
                                    <span class="text-gray-500">"GB"</span>
                                    <button class="btn" on:click=on_apply_size.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| editing_size.set(false)>"✕"</button>
                                </Show>
                            </div>

                            <hr />

                            {/* Move favorites */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.downloads.move_favorite
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.downloads.move_favorite = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label>"Move favorites after download"</label>
                            </div>
                            <Show when=move || p.downloads.move_favorite>
                                <div class="flex items-center gap-2 pl-6">
                                    <span class="flex-1 border rounded px-2 py-1 bg-gray-50 truncate">
                                        {p.downloads.move_favorite_folder.clone()}
                                    </span>
                                    <button class="btn" on:click=move |_| {
                                        spawn_local(async move {
                                            let path = invoke::<_, String>("pick_folder", &[]).await;
                                            if let Ok(Some(dir)) = path {
                                                let _ = invoke::<_, ()>("change_move_dir", &[("new_path", &dir)]).await;
                                                let updated = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
                                                prefs_signal.set(Some(updated));
                                            }
                                        });
                                    }>"📁"</button>
                                </div>
                            </Show>

                            <hr />

                            {/* Notifications */}
                            <div class="flex items-center gap-2">
                                <label class="w-[140px] shrink-0">"Notifications"</label>
                                <select class="border rounded px-2 py-1"
                                    value=move || p.ui.notifications.clone()
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.ui.notifications = e.value();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="none">"None"</option>
                                    <option value="minimal">"Minimal"</option>
                                    <option value="full">"Full"</option>
                                </select>
                            </div>

                            {/* Language */}
                            <div class="flex items-center gap-2">
                                <label class="w-[140px] shrink-0">"Language"</label>
                                <select class="border rounded px-2 py-1"
                                    value=move || p.ui.i18n.to_string()
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.ui.i18n = e.value().parse().unwrap_or(0);
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                >
                                    <option value="0">"English"</option>
                                    <option value="1">"Español"</option>
                                </select>
                            </div>

                            <hr />

                            {/* Start minimized */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || p.ui.start_minimized
                                    on:change=move |e| {
                                        let mut pr = prefs_signal.get().unwrap();
                                        pr.ui.start_minimized = e.checked();
                                        let updated = pr.clone();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                            prefs_signal.set(Some(updated));
                                        });
                                    }
                                />
                                <label>"Start minimized"</label>
                            </div>
                            <Show when=move || p.ui.start_minimized>
                                <div class="flex items-center gap-2 pl-6">
                                    <label class="text-gray-500">"Delay"</label>
                                    <select class="border rounded px-2 py-1"
                                        value=move || p.ui.time_to_minimize.to_string()
                                        on:change=move |e| {
                                            let mut pr = prefs_signal.get().unwrap();
                                            pr.ui.time_to_minimize = e.value().parse().unwrap_or(3);
                                            let updated = pr.clone();
                                            spawn_local(async move {
                                                let _ = invoke::<_, ()>("save_prefs", &[("prefs", &updated)]).await;
                                                prefs_signal.set(Some(updated));
                                            });
                                        }
                                    >
                                        {(1..=10u32).map(|i| {
                                            let val = i.to_string();
                                            view! {
                                                <option value=move || val.clone()>{format!("{}s", i)}</option>
                                            }
                                        }).collect_view()}
                                    </select>
                                </div>
                            </Show>

                            <hr />

                            {/* Autostart */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    on:change=move |e| {
                                        let enable = e.checked();
                                        spawn_local(async move {
                                            let _ = invoke::<_, ()>("set_autostart", &[("enable", &enable)]).await;
                                        });
                                    }
                                />
                                <label>"Start on boot"</label>
                            </div>

                            <hr />

                            {/* Reset */}
                            <button class="btn text-red-600 border-red-300" on:click=on_reset>
                                "Reset All Settings"
                            </button>
                        </div>
                    }
                }}
            </For>
        </Show>
    }
}
