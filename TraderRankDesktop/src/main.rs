#![allow(non_snake_case)]

mod theme;
mod models;
mod analytics;
mod parser;
mod sample_data;
mod data_loader;
mod trade_matcher;
mod state;
mod settings_store;
mod components;
mod views;

use dioxus::prelude::*;
use theme::Theme;

const CSS: &str = include_str!("../assets/main.css");

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    Dashboard {},
    #[route("/timeline")]
    Timeline {},
    #[route("/visual")]
    VisualTimeline {},
    #[route("/trades")]
    Trades {},
    #[route("/analytics")]
    Analytics {},
    #[route("/settings")]
    Settings {},
}

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(
                    dioxus::desktop::tao::window::WindowBuilder::new()
                        .with_title("TraderRank")
                        .with_always_on_top(false)
                )
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let saved = settings_store::load_settings();
    let initial_theme = saved.as_ref().map(|(t, _)| *t).unwrap_or(Theme::Dark);

    let _theme = use_context_provider(|| Signal::new(initial_theme));

    let _state = use_context_provider(|| {
        let mut app_state = data_loader::load_app_state();
        // Apply saved R-configs if available
        if let Some((_, r_configs)) = saved {
            if !r_configs.is_empty() {
                for saved_r in &r_configs {
                    if let Some(existing) = app_state.r_configs.iter_mut().find(|c| c.week_start == saved_r.week_start) {
                        existing.r_value = saved_r.r_value;
                    }
                }
            }
        }
        Signal::new(app_state)
    });

    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    let theme = use_context::<Signal<Theme>>();
    let theme_str = theme.read().as_str();

    rsx! {
        style { "{CSS}" }
        div {
            class: "app-root",
            "data-theme": "{theme_str}",

            // Top navigation bar
            nav { class: "top-nav",
                div { class: "nav-brand",
                    span { class: "brand-icon", "\u{1F4C8}" }
                    span { class: "brand-text", "TraderRank" }
                }
                div { class: "nav-tabs",
                    Link { class: "nav-tab", to: Route::Dashboard {}, "Dashboard" }
                    Link { class: "nav-tab", to: Route::Timeline {}, "Timeline" }
                    Link { class: "nav-tab", to: Route::VisualTimeline {}, "Visual" }
                    Link { class: "nav-tab", to: Route::Trades {}, "Trades" }
                    Link { class: "nav-tab", to: Route::Analytics {}, "Analytics" }
                    Link { class: "nav-tab", to: Route::Settings {}, "Settings" }
                }
                div { class: "nav-right",
                    ThemeToggle {}
                }
            }

            // Main content
            div { class: "main-content",
                Outlet::<Route> {}
            }
        }
    }
}

#[component]
fn ThemeToggle() -> Element {
    let mut theme = use_context::<Signal<Theme>>();
    let state = use_context::<Signal<state::AppState>>();
    let current = *theme.read();
    let icon = match current {
        Theme::Dark => "\u{2600}\u{FE0F}",
        Theme::Light => "\u{1F319}",
    };

    rsx! {
        button {
            class: "theme-toggle",
            onclick: move |_| {
                let new_theme = current.toggle();
                theme.set(new_theme);
                let s = state.read();
                settings_store::save_settings(&new_theme, &s.r_configs);
            },
            "{icon}"
        }
    }
}

// Route components delegate to views
#[component]
fn Dashboard() -> Element {
    rsx! { views::dashboard::Dashboard {} }
}

#[component]
fn Timeline() -> Element {
    rsx! { views::timeline::Timeline {} }
}

#[component]
fn VisualTimeline() -> Element {
    rsx! { views::visual_timeline::VisualTimeline {} }
}

#[component]
fn Trades() -> Element {
    rsx! { views::trades::Trades {} }
}

#[component]
fn Analytics() -> Element {
    rsx! { views::analytics::Analytics {} }
}

#[component]
fn Settings() -> Element {
    rsx! { views::settings::Settings {} }
}
