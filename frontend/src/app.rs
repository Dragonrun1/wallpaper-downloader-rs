use leptos::prelude::*;
use crate::tabs;

#[component]
pub fn App() -> impl IntoView {
    let active = RwSignal::new(0u8);

    let tab_names = vec!["Providers", "Settings", "Changer", "Downloads", "Wallpapers", "About", "Help"];

    view! {
        <div class="app">
            <div class="tab-bar">
                {tab_names.into_iter().enumerate().map(|(i, name)| {
                    let i = i as u8;
                    let active = active;
                    view! {
                        <button
                            class=move || if active.get() == i { "active" } else { "" }
                            on:click=move |_| active.set(i)
                        >{name}</button>
                    }
                }).collect_view()}
            </div>
            <div class="tab-content">
                {move || match active.get() {
                    0 => view! { <tabs::providers::ProvidersTab /> }.into_any(),
                    1 => view! { <tabs::settings::SettingsTab /> }.into_any(),
                    2 => view! { <tabs::changer::ChangerTab /> }.into_any(),
                    3 => view! { <tabs::downloads::DownloadsTab /> }.into_any(),
                    4 => view! { <tabs::wallpapers::WallpapersTab /> }.into_any(),
                    5 => view! { <tabs::about::AboutTab /> }.into_any(),
                    _ => view! { <tabs::help::HelpTab /> }.into_any(),
                }}
            </div>
        </div>
    }
}
