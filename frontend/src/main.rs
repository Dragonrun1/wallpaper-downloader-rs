use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod app;
mod tabs;
mod windows;

#[wasm_bindgen(inline_js = "export function get_window_label() { return window.__TAURI_INTERNALS__.metadata.currentWindow.label; }")]
extern "C" {
    fn get_window_label() -> String;
}

fn main() {
    let label = get_window_label();

    let view: AnyView = match label.as_str() {
        "manager" => view! { <windows::manager::ManagerWindow /> }.into_any(),
        "choose" => view! { <windows::choose::ChooseWindow /> }.into_any(),
        "preview" => view! { <windows::preview::PreviewWindow /> }.into_any(),
        _ => view! { <app::App /> }.into_any(),
    };

    mount_to_body(|| view);
}
