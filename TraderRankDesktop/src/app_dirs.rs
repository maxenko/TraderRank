use std::path::PathBuf;

/// Standard app data directory: %LOCALAPPDATA%\TraderRank\
/// e.g. C:\Users\max\AppData\Local\TraderRank\
pub fn app_data_dir() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|local| PathBuf::from(local).join("TraderRank"))
}

/// Directory for IB Flex imported CSVs: %LOCALAPPDATA%\TraderRank\imports\
pub fn imports_dir() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join("imports"))
}

/// Path for app settings: %LOCALAPPDATA%\TraderRank\settings.json
pub fn settings_path() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join("settings.json"))
}
