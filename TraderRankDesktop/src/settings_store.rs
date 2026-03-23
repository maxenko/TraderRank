use crate::state::WeeklyRConfig;
use crate::theme::Theme;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedSettings {
    pub theme: String,
    pub r_configs: Vec<PersistedRConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedRConfig {
    pub week_start: String,
    pub r_value: String,
}

fn settings_path() -> Option<PathBuf> {
    // Store next to the Data directory
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

pub fn save_settings(theme: &Theme, r_configs: &[WeeklyRConfig]) {
    let Some(path) = settings_path() else { return };

    let settings = PersistedSettings {
        theme: theme.as_str().to_string(),
        r_configs: r_configs
            .iter()
            .map(|c| PersistedRConfig {
                week_start: c.week_start.to_string(),
                r_value: c.r_value.to_string(),
            })
            .collect(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn load_settings() -> Option<(Theme, Vec<WeeklyRConfig>)> {
    let path = settings_path()?;
    let json = std::fs::read_to_string(&path).ok()?;
    let settings: PersistedSettings = serde_json::from_str(&json).ok()?;

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
