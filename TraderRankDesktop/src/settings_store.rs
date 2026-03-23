use crate::state::WeeklyRConfig;
use crate::theme::Theme;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All persisted UI state — the entire app session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedSettings {
    // Theme
    #[serde(default)]
    pub theme: String,

    // R-unit configs
    #[serde(default)]
    pub r_configs: Vec<PersistedRConfig>,

    // Dashboard
    #[serde(default = "default_dashboard_range")]
    pub dashboard_range: String,

    // Timeline
    #[serde(default = "default_timeline_mode")]
    pub timeline_mode: String,
    #[serde(default = "default_max_entries")]
    pub timeline_max_entries: usize,
    #[serde(default = "default_sort_col_period")]
    pub timeline_sort_col: String,
    #[serde(default)]
    pub timeline_sort_asc: bool,

    // Trades
    #[serde(default = "default_max_entries")]
    pub trades_max_entries: usize,
    #[serde(default = "default_sort_col_time")]
    pub trades_sort_col: String,
    #[serde(default)]
    pub trades_sort_asc: bool,

    // Analytics
    #[serde(default)]
    pub analytics_tab: String,
    #[serde(default = "default_analytics_range")]
    pub analytics_range: String,

    // Visual timeline
    #[serde(default = "default_zoom")]
    pub vtl_zoom: f64,
    #[serde(default)]
    pub vtl_range_start: f64,
    #[serde(default = "default_one")]
    pub vtl_range_end: f64,

    // IB Flex Web Service
    #[serde(default)]
    pub flex_token: String,
    #[serde(default)]
    pub flex_query_id: String,
}

fn default_dashboard_range() -> String { "1M".to_string() }
fn default_timeline_mode() -> String { "Weekly".to_string() }
fn default_max_entries() -> usize { 100 }
fn default_sort_col_period() -> String { "period".to_string() }
fn default_sort_col_time() -> String { "time".to_string() }
fn default_analytics_range() -> String { "All".to_string() }
fn default_zoom() -> f64 { 1.0 }
fn default_one() -> f64 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRConfig {
    pub week_start: String,
    pub r_value: String,
}

fn settings_path() -> Option<PathBuf> {
    // Prefer standard user directory: %LOCALAPPDATA%\TraderRank\settings.json
    if let Some(user_path) = crate::app_dirs::settings_path() {
        // If the user-dir settings file already exists, use it
        if user_path.exists() {
            return Some(user_path);
        }
        // If no user-dir file yet, check for legacy file and migrate
        if let Some(legacy) = legacy_settings_path() {
            if legacy.exists() {
                // Migrate: copy old settings to user dir, then use user dir
                if let Some(parent) = user_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::copy(&legacy, &user_path);
                return Some(user_path);
            }
        }
        // No legacy file either — use user dir (will be created on first save)
        if let Some(parent) = user_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        return Some(user_path);
    }
    // Fallback: legacy paths (no LOCALAPPDATA available)
    legacy_settings_path()
}

/// Old settings path (relative to project Data/ dir). Kept for migration.
fn legacy_settings_path() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join("../Data"),
            cwd.join("Data"),
            cwd.join("../../Data"),
        ];
        for c in &candidates {
            if c.exists() {
                return Some(c.join("desktop_settings.json"));
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join("../../Data"),
                parent.join("../../../Data"),
            ];
            for c in &candidates {
                if c.exists() {
                    return Some(c.join("desktop_settings.json"));
                }
            }
        }
    }
    None
}

pub fn save_all(settings: &PersistedSettings) {
    let Some(path) = settings_path() else { return };
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn save_settings(theme: &Theme, r_configs: &[WeeklyRConfig]) {
    // Load existing, update theme + r_configs, save
    let mut settings = load_raw().unwrap_or_default();
    settings.theme = theme.as_str().to_string();
    settings.r_configs = r_configs
        .iter()
        .map(|c| PersistedRConfig {
            week_start: c.week_start.to_string(),
            r_value: c.r_value.to_string(),
        })
        .collect();
    save_all(&settings);
}

pub fn load_raw() -> Option<PersistedSettings> {
    let path = settings_path()?;
    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn load_settings() -> Option<(Theme, Vec<WeeklyRConfig>)> {
    let settings = load_raw()?;

    let theme = match settings.theme.as_str() {
        "light" => Theme::Light,
        _ => Theme::Dark,
    };

    let r_configs: Vec<WeeklyRConfig> = settings
        .r_configs
        .iter()
        .filter_map(|c| {
            let week_start = c.week_start.parse::<NaiveDate>().ok()?;
            let r_value = c.r_value.parse::<Decimal>().ok()?;
            Some(WeeklyRConfig { week_start, r_value })
        })
        .collect();

    Some((theme, r_configs))
}

/// Update a single field and persist. Loads current settings, applies the
/// mutation closure, then saves.
pub fn update<F: FnOnce(&mut PersistedSettings)>(f: F) {
    let mut settings = load_raw().unwrap_or_default();
    f(&mut settings);
    save_all(&settings);
}
