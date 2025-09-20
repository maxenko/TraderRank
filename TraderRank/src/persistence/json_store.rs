use crate::models::TradingSummary;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedData {
    pub last_processed: DateTime<Utc>,
    pub processed_files: Vec<String>,
    pub summary: TradingSummary,
}

pub struct JsonStore {
    data_dir: PathBuf,
}

impl JsonStore {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)
                .with_context(|| format!("Failed to create data directory: {:?}", data_dir))?;
        }

        let summaries_dir = data_dir.join("Summaries");
        if !summaries_dir.exists() {
            fs::create_dir_all(&summaries_dir)
                .with_context(|| format!("Failed to create summaries directory: {:?}", summaries_dir))?;
        }

        Ok(Self { data_dir })
    }

    pub fn load_processed_data(&self) -> Result<Option<ProcessedData>> {
        let file_path = self.get_processed_data_path();

        if !file_path.exists() {
            return Ok(None);
        }

        let json_str = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read processed data from {:?}", file_path))?;

        let data: ProcessedData = serde_json::from_str(&json_str)
            .with_context(|| "Failed to deserialize processed data")?;

        Ok(Some(data))
    }

    pub fn save_processed_data(&self, data: &ProcessedData) -> Result<()> {
        let file_path = self.get_processed_data_path();

        let json_str = serde_json::to_string_pretty(data)
            .with_context(|| "Failed to serialize processed data")?;

        fs::write(&file_path, json_str)
            .with_context(|| format!("Failed to write processed data to {:?}", file_path))?;

        Ok(())
    }

    pub fn save_daily_summary(&self, summary: &TradingSummary) -> Result<()> {
        let summaries_dir = self.data_dir.join("Summaries");
        let file_name = format!("summary_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
        let file_path = summaries_dir.join(file_name);

        let json_str = serde_json::to_string_pretty(summary)
            .with_context(|| "Failed to serialize trading summary")?;

        fs::write(&file_path, json_str)
            .with_context(|| format!("Failed to write summary to {:?}", file_path))?;

        Ok(())
    }

    pub fn get_new_files(&self, source_dir: &Path) -> Result<Vec<PathBuf>> {
        let processed_data = self.load_processed_data()?;

        let processed_files = processed_data
            .map(|d| d.processed_files)
            .unwrap_or_default();

        let mut new_files = Vec::new();

        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if !processed_files.contains(&file_name) {
                    new_files.push(path);
                }
            }
        }

        Ok(new_files)
    }

    fn get_processed_data_path(&self) -> PathBuf {
        self.data_dir.join("processed_data.json")
    }

    pub fn mark_files_processed(&self, files: Vec<PathBuf>, summary: TradingSummary) -> Result<()> {
        let mut processed_files = self.load_processed_data()?
            .map(|d| d.processed_files)
            .unwrap_or_default();

        for file in files {
            if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                if !processed_files.contains(&name.to_string()) {
                    processed_files.push(name.to_string());
                }
            }
        }

        let data = ProcessedData {
            last_processed: Utc::now(),
            processed_files,
            summary,
        };

        self.save_processed_data(&data)?;
        Ok(())
    }
}