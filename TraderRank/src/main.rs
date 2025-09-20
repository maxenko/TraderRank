mod models;
mod parser;
mod analytics;
mod persistence;
mod visualization;

use anyhow::Result;
use colored::*;
use parser::CsvParser;
use analytics::{TradingAnalytics, TimePatternAnalyzer};
use persistence::JsonStore;
use visualization::{TableRenderer, ChartRenderer, CalendarRenderer, WeeklyRenderer};
use std::collections::HashSet;

fn main() -> Result<()> {
    println!("{}", "\n🚀 TraderRank Analytics Engine Starting...".bold().cyan());

    let project_root = std::env::current_dir()?;
    let data_dir = project_root.parent().unwrap_or(&project_root).join("Data");
    let source_dir = data_dir.join("Source");

    if !source_dir.exists() {
        eprintln!("{}", "❌ Error: Data/Source directory not found!".red().bold());
        eprintln!("Please ensure the Data/Source directory exists with CSV files.");
        std::process::exit(1);
    }

    let store = JsonStore::new(data_dir.clone())?;

    println!("{}", "📂 Checking for new trade data...".yellow());

    let new_files = store.get_new_files(&source_dir)?;

    if new_files.is_empty() {
        println!("{}", "✅ All files already processed.".green());

        if let Some(processed_data) = store.load_processed_data()? {
            println!("{}", "📊 Loading cached analysis...".cyan());
            TableRenderer::render_summary(&processed_data.summary, 10);
            ChartRenderer::render_pnl_chart(&processed_data.summary.daily_summaries);

            // Add daily win rate chart
            ChartRenderer::render_daily_winrate_chart(&processed_data.summary.daily_summaries);

            if let Some(last_day) = processed_data.summary.daily_summaries.last() {
                ChartRenderer::render_hourly_distribution(last_day);
            }

            // Add weekly analysis
            WeeklyRenderer::render_weekly_analysis(&processed_data.summary);

            // Add calendar views
            CalendarRenderer::render_combined_calendars(&processed_data.summary);
        } else {
            println!("{}", "⚠️  No processed data found.".yellow());
        }
    } else {
        println!("{}", format!("🔍 Found {} new file(s) to process", new_files.len()).green());

        let mut unique_trades = HashSet::new();
        let mut duplicate_count = 0;

        if let Some(_processed_data) = store.load_processed_data()? {
            println!("{}", "📥 Loading existing trade history...".cyan());
            // Note: In a real implementation, we'd need to reconstruct trades from summaries
            // or store trades separately
        }

        for file in &new_files {
            println!("{}", format!("  📄 Processing: {}", file.file_name().unwrap().to_string_lossy()).white());
            let trades = CsvParser::parse_file(file)?;
            let file_trade_count = trades.len();

            for trade in trades {
                if !unique_trades.insert(trade) {
                    duplicate_count += 1;
                }
            }

            println!("{}", format!("     └─ {} trades found", file_trade_count).dimmed());
        }

        if duplicate_count > 0 {
            println!("{}", format!("⚠️  Found {} duplicate trade(s), filtering them out", duplicate_count).yellow());
        }

        let all_trades: Vec<_> = unique_trades.into_iter().collect();
        println!("{}", format!("✅ Processing {} unique trades (filtered {} duplicates)", all_trades.len(), duplicate_count).green());

        println!("{}", "🧮 Analyzing trading performance...".cyan());
        let summary = TradingAnalytics::analyze_trades(&all_trades);

        println!("{}", "💾 Saving analysis results...".yellow());
        store.mark_files_processed(new_files, summary.clone())?;
        store.save_daily_summary(&summary)?;

        println!("{}", "📊 Generating reports...".cyan());
        TableRenderer::render_summary(&summary, 10);
        ChartRenderer::render_pnl_chart(&summary.daily_summaries);

        // Add daily win rate chart - right after P&L charts for visibility
        ChartRenderer::render_daily_winrate_chart(&summary.daily_summaries);

        if let Some(last_day) = summary.daily_summaries.last() {
            ChartRenderer::render_hourly_distribution(last_day);
        }

        // Add weekly analysis
        WeeklyRenderer::render_weekly_analysis(&summary);

        // Add calendar views
        CalendarRenderer::render_combined_calendars(&summary);

        let periods = TimePatternAnalyzer::identify_best_trading_periods(&all_trades);
        println!("\n{}", "🎯 Best Trading Periods Analysis".bold().cyan());
        for (i, period) in periods.iter().take(3).enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            println!("{} {} ({:02}:00-{:02}:00): ${:.2} | Win Rate: {:.1}%",
                medal,
                period.name.bold(),
                period.start_hour,
                period.end_hour,
                period.total_pnl,
                period.win_rate
            );
        }
    }

    println!("\n{}", "✨ Analysis complete!".green().bold());
    Ok(())
}
