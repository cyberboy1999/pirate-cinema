#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use dioxus::prelude::*;
#[path = "../src/rutube_prototype.rs"]
mod rutube_prototype;
fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_menu(None).with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Pirate Cinema — Rutube · локальный прототип")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1180., 820.)),
            ),
        )
        .launch(App);
}
#[component]
fn App() -> Element {
    rsx! { style { {":root{background:#080808;color:#eee}body{margin:0;padding:30px}"} } rutube_prototype::RutubePage { active: true } }
}
