#![allow(non_snake_case)]

mod theme;
mod models;
mod analytics;
mod parser;
mod sample_data;
mod data_loader;
mod trade_matcher;
mod flex_fetcher;
mod app_dirs;
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
                .with_background_color((12, 13, 20, 255)) // matches --bg-primary #0c0d14
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

    let _log = use_context_provider(|| Signal::new(Vec::<(String, String)>::new())); // (timestamp, message)

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
                    RefreshButton {}
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

#[derive(Clone, Copy, PartialEq)]
enum RefreshState {
    Idle,
    Fetching,
    Done,
    Error,
}

/// Helper: reload AppState preserving user R-configs
fn reload_app_state(state: &mut Signal<state::AppState>) {
    let mut new_state = data_loader::load_app_state();
    let old_configs = state.read().r_configs.clone();
    for saved_r in &old_configs {
        if let Some(existing) = new_state.r_configs.iter_mut().find(|c| c.week_start == saved_r.week_start) {
            existing.r_value = saved_r.r_value;
        }
    }
    state.set(new_state);
}

/// Push a timestamped message to the app log (visible in Settings)
pub fn log_message(log: &mut Signal<Vec<(String, String)>>, msg: &str) {
    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
    eprintln!("[{}] {}", ts, msg);
    log.write().push((ts, msg.to_string()));
    // Keep last 100 entries
    let len = log.read().len();
    if len > 100 {
        log.write().drain(..len - 100);
    }
}

#[component]
fn RefreshButton() -> Element {
    let mut state = use_context::<Signal<state::AppState>>();
    let mut app_log = use_context::<Signal<Vec<(String, String)>>>();
    let mut status = use_signal(|| RefreshState::Idle);
    let current = *status.read();

    let label = match current {
        RefreshState::Idle => "\u{21BB}",      // ↻
        RefreshState::Fetching => "\u{23F3}",  // ⏳
        RefreshState::Done => "\u{2705}",      // ✅
        RefreshState::Error => "\u{274C}",     // ❌
    };

    let btn_class = match current {
        RefreshState::Fetching => "refresh-btn fetching",
        _ => "refresh-btn",
    };

    rsx! {
        button {
            class: "{btn_class}",
            title: "Refresh trades from broker",
            disabled: current == RefreshState::Fetching,
            onclick: move |_| {
                let saved = settings_store::load_raw();
                let token = saved.as_ref().map(|s| s.flex_token.clone()).unwrap_or_default();
                let qid = saved.as_ref().map(|s| s.flex_query_id.clone()).unwrap_or_default();

                if token.is_empty() || qid.is_empty() {
                    log_message(&mut app_log, "Reloading local trade data...");
                    reload_app_state(&mut state);
                    log_message(&mut app_log, "Data reloaded from local imports.");
                    status.set(RefreshState::Done);
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        status.set(RefreshState::Idle);
                    });
                    return;
                }

                log_message(&mut app_log, "Fetching trades from IB Flex Web Service...");
                status.set(RefreshState::Fetching);
                spawn(async move {
                    match flex_fetcher::fetch_and_save(&token, &qid).await {
                        Ok(count) => {
                            log_message(&mut app_log, &format!("Fetched {} trades from IB. Reloading...", count));
                            reload_app_state(&mut state);
                            log_message(&mut app_log, "Data reloaded successfully.");
                            status.set(RefreshState::Done);
                        }
                        Err(e) => {
                            log_message(&mut app_log, &format!("ERROR: {}", e));
                            status.set(RefreshState::Error);
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    status.set(RefreshState::Idle);
                });
            },
            "{label}"
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
