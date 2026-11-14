#![allow(non_snake_case)]
use crate::types::Prefs;
use leptos::prelude::*;
use tauri_wasm::invoke;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ChangerTab() -> impl IntoView {
    let prefs_signal = RwSignal::new::<Option<Prefs>>(None);
    let can_change = RwSignal::new(false);

    let editing_interval = RwSignal::new(false);
    let interval = RwSignal::new(5u32);
    let units = RwSignal::new("minutes".to_string());
    let selected_folder = RwSignal::new::<Option<String>>(None);

    spawn_local(async move {
        let p = invoke::<Prefs>("get_prefs", &[]).await.unwrap();
        interval.set(p.changer.interval);
        units.set(p.changer.interval_units.clone());
        prefs_signal.set(Some(p));
        let c = invoke::<bool>("can_change_wallpaper", &[])
            .await
            .unwrap_or(false);
        can_change.set(c);
    });

    let save = move |updated: Prefs| {
        spawn_local(async move {
            let _ = invoke::<()>("save_prefs", &[("prefs", &updated)]).await;
            prefs_signal.set(Some(updated));
        });
    };

    let on_apply_interval = move |_| {
        let mut p = prefs_signal.get().unwrap();
        p.changer.interval = interval.get();
        p.changer.interval_units = units.get();
        editing_interval.set(false);
        let updated = p.clone();
        save(updated);
    };

    let on_set_mode = move |mode: String| {
        let mut p = prefs_signal.get().unwrap();
        p.changer.mode = mode;
        let updated = p.clone();
        save(updated);
    };

    let on_add_folder = move |_| {
        spawn_local(async move {
            let path = invoke::<String>("pick_folder", &[]).await;
            if let Ok(Some(dir)) = path {
                let _ = invoke::<_, ()>("add_changer_folder", &[("new_path", &dir)]).await;
                let updated = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
                prefs_signal.set(Some(updated));
            }
        });
    };

    let on_remove_folder = move |_| {
        if let Some(path) = selected_folder.get() {
            spawn_local(async move {
                let _ = invoke::<()>("remove_changer_folder", &[("path", &path)]).await;
                let updated = invoke::<_, Prefs>("get_prefs", &[]).await.unwrap();
                prefs_signal.set(Some(updated));
                selected_folder.set(None);
            });
        }
    };

    let on_toggle_multi = move |checked: bool| {
        let mut p = prefs_signal.get().unwrap();
        p.changer.multi_monitor = checked;
        let updated = p.clone();
        save(updated);
    };

    let on_random = move |_| {
        spawn_local(async move {
            let _ = invoke::<()>("set_random_wallpaper", &[]).await;
        });
    };

    let on_next = move |_| {
        spawn_local(async move {
            let _ = invoke::<()>("set_next_wallpaper", &[]).await;
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
                    let c = &prefs.changer;
                    view! {
                        <div class="p-3 space-y-4 text-sm">
                            {/* Interval */}
                            <div class="flex items-center gap-2">
                                <label class="w-[100px] shrink-0">"Change every"</label>
                                <Show
                                    when=move || editing_interval.get()
                                    fallback=move || view! {
                                        <span class="border rounded px-2 py-1 bg-gray-50">
                                            {move || if c.interval == 0 { "Disabled".to_string() } else { format!("{} {}", c.interval, c.interval_units) }}
                                        </span>
                                        <button class="btn" on:click=move |_| editing_interval.set(true)>"✎"</button>
                                    }
                                >
                                    <input type="number" min="0" class="w-16 border rounded px-2 py-1"
                                        value=move || interval.get().to_string()
                                        on:input=move |e| interval.set(e.value().parse().unwrap_or(0))
                                    />
                                    <select class="border rounded px-1 py-1"
                                        value=move || units.get()
                                        on:change=move |e| units.set(e.value())
                                    >
                                        <option value="seconds">"seconds"</option>
                                        <option value="minutes">"minutes"</option>
                                        <option value="hours">"hours"</option>
                                    </select>
                                    <button class="btn" on:click=on_apply_interval.clone()>"✓"</button>
                                    <button class="btn" on:click=move |_| editing_interval.set(false)>"✕"</button>
                                </Show>
                            </div>

                            {/* Mode */}
                            <div class="flex items-center gap-2">
                                <label class="w-[100px] shrink-0">"Mode"</label>
                                <select class="flex-1 border rounded px-2 py-1"
                                    value=move || c.mode.clone()
                                    on:change=move |e| on_set_mode(e.value())
                                >
                                    <option value="random">"Random"</option>
                                    <option value="sorted">"Sorted (date)"</option>
                                    <option value="reverse">"Reverse (date)"</option>
                                </select>
                                <Show when=move || c.mode != "random" && can_change.get()>
                                    <button class="btn" title="Restart from beginning" on:click=on_next.clone()>"↺"</button>
                                </Show>
                            </div>

                            <hr />

                            {/* Folders */}
                            <div class="flex items-start gap-2">
                                <label class="w-[100px] shrink-0 pt-2">"Directories"</label>
                                <div class="flex-1 border rounded h-[180px] overflow-y-auto p-1">
                                    <Show when=move || c.folders.is_empty()>
                                        <div class="text-gray-400 px-2 py-1">"No folders configured"</div>
                                    </Show>
                                    <For
                                        each=move || c.folders.clone()
                                        key=move |f| f.clone()
                                        let:folder
                                    >
                                        {move || {
                                            let is_selected = selected_folder.get().as_deref() == Some(folder.as_str());
                                            view! {
                                                <div
                                                    class=move || {
                                                        if selected_folder.get().as_deref() == Some(folder.as_str()) {
                                                            "px-2 py-1 cursor-pointer rounded bg-blue-100"
                                                        } else {
                                                            "px-2 py-1 cursor-pointer rounded hover:bg-gray-50"
                                                        }
                                                    }
                                                    on:click=move |_| {
                                                        selected_folder.update(|s| {
                                                            *s = if s.as_deref() == Some(folder.as_str()) { None } else { Some(folder.clone()) };
                                                        });
                                                    }
                                                    title=move || folder.clone()
                                                >{folder.clone()}</div>
                                            }
                                        }}
                                    </For>
                                </div>
                                <div class="flex flex-col gap-1">
                                    <button class="btn" on:click=on_add_folder title="Add directory">"+"</button>
                                    <button class="btn" on:click=on_remove_folder
                                        disabled=move || selected_folder.get().is_none()
                                        title="Remove selected directory">"−"</button>
                                </div>
                            </div>

                            {/* Multi-monitor */}
                            <div class="flex items-center gap-2">
                                <input type="checkbox"
                                    checked=move || c.multi_monitor
                                    on:change=move |e| on_toggle_multi(e.checked())
                                />
                                <label>"Multi-monitor (span)"</label>
                            </div>

                            <hr />

                            {/* Quick actions */}
                            <Show when=move || can_change.get()>
                                <div class="flex gap-2">
                                    <button class="btn" on:click=on_random>"🎲 Random"</button>
                                    <Show when=move || c.mode != "random">
                                        <button class="btn" on:click=on_next>"⏭ Next"</button>
                                    </Show>
                                </div>
                            </Show>
                        </div>
                    }
                }}
            </For>
        </Show>
    }
}
