use crate::models::{DailySummary, TradingSummary};
use colored::*;
use rust_decimal::Decimal;

pub struct TableRenderer;

impl TableRenderer {
    pub fn render_summary(summary: &TradingSummary, days: usize) {
        println!("\n{}", Self::create_header());

        let recent_summaries = summary.daily_summaries
            .iter()
            .rev()
            .take(days)
            .rev()
            .collect::<Vec<_>>();

        // Brief summaries for first n-1 days
        if recent_summaries.len() > 1 {
            Self::render_brief_summaries(&recent_summaries[..recent_summaries.len()-1]);
        }

        // Detailed summary for last day
        if let Some(last_day) = recent_summaries.last() {
            Self::render_detailed_summary(last_day);
            Self::render_time_analysis(last_day);
        }

        // Overall statistics
        Self::render_overall_stats(summary);
    }

    fn create_header() -> String {
        let title = "TraderRank Analytics Engine";
        let width = 60;
        format!("\n{}\n{:^width$}\n{}\n",
            "=".repeat(width),
            title,
            "=".repeat(width),
            width = width
        ).bold().cyan().to_string()
    }

    fn render_brief_summaries(summaries: &[&DailySummary]) {
        println!("\n{}", "📊 Recent Trading Days".bold().cyan());
        println!();

        // Simple format without box drawing - P&L moved to end
        println!("{:<12} {:>8} {:>8} {:>20} {:>12}",
            "Date", "Trades", "Win%", "Best/Worst", "P&L");
        println!("{}", "-".repeat(60));

        for summary in summaries {
            let date_str = summary.date.format("%Y-%m-%d").to_string();
            let pnl_str = Self::format_currency_plain(summary.realized_pnl);
            let pnl_display = if summary.realized_pnl > Decimal::ZERO {
                pnl_str.green().to_string()
            } else if summary.realized_pnl < Decimal::ZERO {
                pnl_str.red().to_string()
            } else {
                pnl_str
            };

            println!("{:<12} {:>8} {:>7.1}% {:>9.2}/{:<9.2} {:>12}",
                date_str,
                summary.total_trades,
                summary.win_rate,
                summary.largest_win,
                -summary.largest_loss.abs(),
                pnl_display);
        }
    }

    fn render_detailed_summary(summary: &DailySummary) {
        println!("\n{}", "🎯 Today's Detailed Report".bold().yellow());
        println!();

        // Simple two-column format
        let print_row = |label: &str, value: String| {
            println!("  {:<20} {}", label.bright_white(), value);
        };

        print_row("Date:", summary.date.format("%Y-%m-%d %H:%M").to_string());
        print_row("Total Trades:", summary.total_trades.to_string());
        print_row("Winning Trades:", format!("{} ({:.1}%)", summary.winning_trades, summary.win_rate));
        print_row("Losing Trades:", summary.losing_trades.to_string());

        let pnl_str = Self::format_currency_plain(summary.realized_pnl);
        let pnl_display = if summary.realized_pnl > Decimal::ZERO {
            pnl_str.green().bold().to_string()
        } else if summary.realized_pnl < Decimal::ZERO {
            pnl_str.red().bold().to_string()
        } else {
            pnl_str
        };
        print_row("Realized P&L:", pnl_display);
        print_row("Total $ Traded:", format!("${:.2}", summary.total_volume));
        print_row("Commission Paid:", format!("${:.2}", summary.total_commission));

        let avg_win_str = format!("${:.2}", summary.avg_win).green().to_string();
        print_row("Average Win:", avg_win_str);

        let avg_loss_str = format!("-${:.2}", summary.avg_loss.abs()).red().to_string();
        print_row("Average Loss:", avg_loss_str);

        let largest_win_str = format!("${:.2}", summary.largest_win).green().to_string();
        print_row("Largest Win:", largest_win_str);

        let largest_loss_str = format!("-${:.2}", summary.largest_loss.abs()).red().to_string();
        print_row("Largest Loss:", largest_loss_str);

        if let Some(pf) = summary.profit_factor() {
            print_row("Profit Factor:", format!("{:.2}", pf));
        }

        print_row("Symbols Traded:", summary.symbols_traded.len().to_string());
    }

    fn render_time_analysis(summary: &DailySummary) {
        if summary.time_slot_performance.is_empty() {
            return;
        }

        println!("\n{}", "⏰ Hourly Performance".bold().cyan());
        println!();

        println!("{:<15} {:>8} {:>8} {:>12}",
            "Hour", "Trades", "Win%", "P&L");
        println!("{}", "-".repeat(45));

        for slot in &summary.time_slot_performance {
            let hour_str = format!("{:02}:00-{:02}:00", slot.hour, slot.hour + 1);
            let pnl_str = Self::format_currency_plain(slot.pnl);
            let pnl_display = if slot.pnl > Decimal::ZERO {
                pnl_str.green().to_string()
            } else if slot.pnl < Decimal::ZERO {
                pnl_str.red().to_string()
            } else {
                pnl_str
            };

            println!("{:<15} {:>8} {:>7.1}% {:>12}",
                hour_str,
                slot.trades,
                slot.win_rate,
                pnl_display);
        }
    }

    fn render_overall_stats(summary: &TradingSummary) {
        println!("\n{}", "📈 Overall Performance".bold().green());
        println!();

        let print_row = |label: &str, value: String| {
            println!("  {:<20} {}", label.bright_white(), value);
        };

        print_row("Trading Period:", format!("{} to {}",
            summary.start_date.format("%Y-%m-%d"),
            summary.end_date.format("%Y-%m-%d")));

        let pnl_str = Self::format_currency_plain(summary.total_pnl);
        let pnl_display = if summary.total_pnl > Decimal::ZERO {
            pnl_str.green().bold().to_string()
        } else if summary.total_pnl < Decimal::ZERO {
            pnl_str.red().bold().to_string()
        } else {
            pnl_str
        };
        print_row("Total P&L:", pnl_display);
        print_row("Total $ Traded:", format!("${:.2}", summary.total_volume));
        print_row("Total Trades:", summary.total_trades.to_string());
        print_row("Overall Win Rate:", format!("{:.1}%", summary.overall_win_rate));

        if let Some((date, pnl)) = &summary.best_day {
            let pnl_str = Self::format_currency_plain(*pnl);
            let pnl_display = if *pnl > Decimal::ZERO {
                pnl_str.green().to_string()
            } else {
                pnl_str.red().to_string()
            };
            print_row("Best Day:", format!("{} ({})",
                date.format("%Y-%m-%d"),
                pnl_display));
        }

        if let Some((date, pnl)) = &summary.worst_day {
            let pnl_str = Self::format_currency_plain(*pnl);
            let pnl_display = if *pnl > Decimal::ZERO {
                pnl_str.green().to_string()
            } else {
                pnl_str.red().to_string()
            };
            print_row("Worst Day:", format!("{} ({})",
                date.format("%Y-%m-%d"),
                pnl_display));
        }

        if let Some((hour, pnl)) = &summary.most_profitable_hour {
            let pnl_str = Self::format_currency_plain(*pnl);
            let pnl_display = if *pnl > Decimal::ZERO {
                pnl_str.green().to_string()
            } else {
                pnl_str.red().to_string()
            };
            print_row("Best Hour:", format!("{:02}:00 ({})",
                hour,
                pnl_display));
        }
    }

    fn format_currency_plain(amount: Decimal) -> String {
        if amount >= Decimal::ZERO {
            format!("${:.2}", amount)
        } else {
            format!("-${:.2}", amount.abs())
        }
    }
}