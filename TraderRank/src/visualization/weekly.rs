use crate::models::TradingSummary;
use chrono::{Datelike, Weekday};
use colored::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

pub struct WeeklyRenderer;

impl WeeklyRenderer {
    pub fn render_weekly_analysis(summary: &TradingSummary) {
        if summary.weekly_summaries.is_empty() {
            return;
        }

        println!("\n{}", "📊 Weekly Trading Analysis".bold().cyan());
        println!("{}", "═".repeat(120));

        // Recent weeks summary table
        Self::render_recent_weeks_table(summary);

        // Weekly P&L trend chart
        Self::render_weekly_pnl_chart(summary);

        // Weekly win rate analysis
        Self::render_weekly_win_rates(summary);

        // Best and worst weeks
        Self::render_best_worst_weeks(summary);

        // Weekly commission impact
        Self::render_weekly_commission_impact(summary);

        // Current week detailed view
        Self::render_current_week_details(summary);
    }

    fn render_recent_weeks_table(summary: &TradingSummary) {
        println!("\n{}", "📅 Recent Weeks Performance".bold().yellow());
        println!("{}", "─".repeat(120));

        // Header
        println!("{:<12} {:<16} {:>8} {:>6} {:>12} {:>12} {:>10} {:>12} {:>12}",
            "Week", "Date Range", "Days", "Win%", "Commission", "Volume", "Trades", "Gross P&L", "Net P&L");
        println!("{}", "─".repeat(120));

        // Get last 6 weeks
        let recent_weeks: Vec<_> = summary.weekly_summaries
            .iter()
            .rev()
            .take(6)
            .rev()
            .collect();

        for week in recent_weeks {
            let date_range = format!("{} - {}",
                week.start_date.format("%m/%d"),
                week.end_date.format("%m/%d")
            );

            let net_pnl_str = Self::format_currency(week.realized_pnl);
            let gross_pnl_str = Self::format_currency(week.gross_pnl);
            let commission_str = Self::format_currency(week.total_commission);

            // Format with width first, then apply color
            let net_pnl_formatted = format!("{:>12}", net_pnl_str);
            let net_pnl_display = if week.realized_pnl > Decimal::ZERO {
                net_pnl_formatted.green().bold()
            } else if week.realized_pnl < Decimal::ZERO {
                net_pnl_formatted.red().bold()
            } else {
                net_pnl_formatted.yellow()
            };

            let gross_pnl_formatted = format!("{:>12}", gross_pnl_str);
            let gross_pnl_display = if week.gross_pnl > Decimal::ZERO {
                gross_pnl_formatted.green()
            } else if week.gross_pnl < Decimal::ZERO {
                gross_pnl_formatted.red()
            } else {
                gross_pnl_formatted.yellow()
            };

            println!("{:<12} {:<16} {:>8} {:>6.1}% {:>12} {:>12} {:>10} {} {}",
                format!("Week {}", week.week_number),
                date_range,
                week.trading_days,
                week.win_rate,
                commission_str,
                week.total_volume.round().to_i64().unwrap_or(0),
                week.total_trades,
                gross_pnl_display,
                net_pnl_display
            );
        }
    }

    fn render_weekly_pnl_chart(summary: &TradingSummary) {
        println!("\n{}", "📈 Weekly P&L Trend (Last 8 Weeks)".bold().cyan());

        let recent_weeks: Vec<_> = summary.weekly_summaries
            .iter()
            .rev()
            .take(8)
            .rev()
            .collect();

        if recent_weeks.is_empty() {
            return;
        }

        // Find max and min for scaling
        let max_pnl = recent_weeks.iter()
            .map(|w| w.realized_pnl)
            .max()
            .unwrap_or(Decimal::ZERO);
        let min_pnl = recent_weeks.iter()
            .map(|w| w.realized_pnl)
            .min()
            .unwrap_or(Decimal::ZERO);

        let range = max_pnl - min_pnl;
        let scale = if range != Decimal::ZERO {
            Decimal::from(40) / range
        } else {
            Decimal::from(1)
        };

        // Draw the chart
        let chart_height = 15;
        let zero_line = if min_pnl >= Decimal::ZERO {
            chart_height - 1
        } else if range == Decimal::ZERO {
            chart_height / 2  // Center line when all values are the same
        } else {
            let zero_pos = (max_pnl / range * Decimal::from(chart_height)).to_i32().unwrap_or(0) as usize;
            zero_pos.min(chart_height - 1)
        };

        // Create the chart grid
        let mut chart = vec![vec![' '; recent_weeks.len() * 8]; chart_height];

        // Draw zero line
        for col in 0..chart[0].len() {
            chart[zero_line][col] = '─';
        }

        // Plot weekly P&L bars
        for (week_idx, week) in recent_weeks.iter().enumerate() {
            let col = week_idx * 8 + 2;

            if week.realized_pnl != Decimal::ZERO {
                let bar_height = ((week.realized_pnl - min_pnl) * scale / Decimal::from(40) * Decimal::from(chart_height))
                    .abs()
                    .to_i32()
                    .unwrap_or(1) as usize;

                let bar_top = if week.realized_pnl > Decimal::ZERO {
                    zero_line.saturating_sub(bar_height)
                } else {
                    zero_line
                };

                for row in bar_top..=zero_line.min(bar_top + bar_height) {
                    if row < chart_height {
                        chart[row][col] = '█';
                        if col + 1 < chart[0].len() {
                            chart[row][col + 1] = '█';
                        }
                        if col + 2 < chart[0].len() {
                            chart[row][col + 2] = '█';
                        }
                    }
                }
            }
        }

        // Print the chart with values
        let max_label = format!("${:.0}", max_pnl);
        let min_label = format!("${:.0}", min_pnl);

        for (row_idx, row) in chart.iter().enumerate() {
            if row_idx == 0 {
                print!("{:>8} ┤", max_label);
            } else if row_idx == chart_height - 1 {
                print!("{:>8} ┤", min_label);
            } else if row_idx == zero_line {
                print!("{:>8} ┤", "$0");
            } else {
                print!("{:>8} ┤", "");
            }

            for &ch in row {
                if ch == '█' {
                    print!("{}", "█".cyan());
                } else if ch == '─' {
                    print!("{}", "─".bright_black());
                } else {
                    print!("{}", ch);
                }
            }
            println!();
        }

        // Print week labels
        print!("{:>8} └", "");
        for week in &recent_weeks {
            print!("──W{:02}───", week.week_number);
        }
        println!();

        // Print P&L values
        print!("{:>10}", "");
        for week in &recent_weeks {
            let pnl_str = format!("{:>7.0}", week.realized_pnl);
            if week.realized_pnl > Decimal::ZERO {
                print!("{} ", pnl_str.green());
            } else if week.realized_pnl < Decimal::ZERO {
                print!("{} ", pnl_str.red());
            } else {
                print!("{} ", pnl_str.yellow());
            }
        }
        println!();
    }

    fn render_weekly_win_rates(summary: &TradingSummary) {
        println!("\n{}", "🎯 Weekly Win Rate Analysis".bold().yellow());
        println!("{}", "─".repeat(80));

        let recent_weeks: Vec<_> = summary.weekly_summaries
            .iter()
            .rev()
            .take(8)
            .rev()
            .collect();

        for week in recent_weeks {
            let week_label = format!("W{:02}", week.week_number);
            let bar_width = (week.win_rate / 2.0) as usize;
            let bar = "█".repeat(bar_width);

            let color = if week.win_rate >= 60.0 {
                bar.green()
            } else if week.win_rate >= 50.0 {
                bar.yellow()
            } else {
                bar.red()
            };

            println!("{:<5} {} {:.1}% ({}/{})",
                week_label,
                color,
                week.win_rate,
                week.winning_trades,
                week.total_trades
            );
        }
    }

    fn render_best_worst_weeks(summary: &TradingSummary) {
        println!("\n{}", "🏆 Best & Worst Weekly Performance".bold().cyan());
        println!("{}", "─".repeat(80));

        if let Some(((year, week_num), pnl)) = summary.best_week {
            let pnl_str = Self::format_currency(pnl);
            println!("  {:<20} Week {} of {} ({})",
                "Best Week:".green().bold(),
                week_num,
                year,
                pnl_str.green().bold()
            );
        }

        if let Some(((year, week_num), pnl)) = summary.worst_week {
            let pnl_str = Self::format_currency(pnl);
            println!("  {:<20} Week {} of {} ({})",
                "Worst Week:".red().bold(),
                week_num,
                year,
                pnl_str.red().bold()
            );
        }

        // Calculate average weekly P&L
        if !summary.weekly_summaries.is_empty() {
            let avg_weekly_pnl: Decimal = summary.weekly_summaries.iter()
                .map(|w| w.realized_pnl)
                .sum::<Decimal>() / Decimal::from(summary.weekly_summaries.len());

            let avg_str = Self::format_currency(avg_weekly_pnl);
            let avg_display = if avg_weekly_pnl > Decimal::ZERO {
                avg_str.green().to_string()
            } else if avg_weekly_pnl < Decimal::ZERO {
                avg_str.red().to_string()
            } else {
                avg_str.yellow().to_string()
            };

            println!("  {:<20} {}",
                "Average Weekly P&L:".bright_white(),
                avg_display
            );
        }
    }

    fn render_weekly_commission_impact(summary: &TradingSummary) {
        println!("\n{}", "💸 Weekly Commission Impact".bold().yellow());
        println!("{}", "─".repeat(80));

        let recent_weeks: Vec<_> = summary.weekly_summaries
            .iter()
            .rev()
            .take(4)
            .rev()
            .collect();

        for week in recent_weeks {
            let week_label = format!("Week {}", week.week_number);
            let commission_pct = if week.gross_pnl != Decimal::ZERO {
                (week.total_commission.abs() / week.gross_pnl.abs() * Decimal::from(100))
                    .round_dp(1)
            } else {
                Decimal::ZERO
            };

            println!("{:<10} Gross: {:>10} | Comm: {:>8} | Net: {:>10} | Impact: {:>6}%",
                week_label,
                Self::format_currency(week.gross_pnl),
                Self::format_currency(week.total_commission),
                Self::format_currency(week.realized_pnl),
                commission_pct
            );
        }
    }

    fn render_current_week_details(summary: &TradingSummary) {
        if let Some(current_week) = summary.weekly_summaries.last() {
            println!("\n{}", "📊 Current Week Detailed Analysis".bold().cyan());
            println!("{}", "═".repeat(120));

            println!("  {:<20} Week {} ({} to {})",
                "Week:",
                current_week.week_number,
                current_week.start_date.format("%Y-%m-%d"),
                current_week.end_date.format("%Y-%m-%d")
            );

            println!("  {:<20} {} trading days ({} profitable)",
                "Trading Days:",
                current_week.trading_days,
                current_week.profitable_days
            );

            let pnl_str = Self::format_currency(current_week.realized_pnl);
            let pnl_display = if current_week.realized_pnl > Decimal::ZERO {
                pnl_str.green().bold().to_string()
            } else if current_week.realized_pnl < Decimal::ZERO {
                pnl_str.red().bold().to_string()
            } else {
                pnl_str.yellow().to_string()
            };

            println!("  {:<20} {}", "Net P&L:", pnl_display);
            println!("  {:<20} {}", "Gross P&L:", Self::format_currency(current_week.gross_pnl));
            println!("  {:<20} {} trades", "Total Trades:", current_week.total_trades);
            println!("  {:<20} {:.1}% ({} wins, {} losses)",
                "Win Rate:",
                current_week.win_rate,
                current_week.winning_trades,
                current_week.losing_trades
            );

            if current_week.profit_factor().is_some() {
                println!("  {:<20} {:.2}",
                    "Profit Factor:",
                    current_week.profit_factor().unwrap()
                );
            }

            println!("  {:<20} {}",
                "Average Win:",
                Self::format_currency(current_week.avg_win)
            );
            println!("  {:<20} {}",
                "Average Loss:",
                Self::format_currency(current_week.avg_loss)
            );

            // Daily breakdown for current week
            println!("\n  {}", "Daily Breakdown:".bold());
            for daily in &current_week.daily_summaries {
                let day_name = match daily.date.weekday() {
                    Weekday::Mon => "Monday",
                    Weekday::Tue => "Tuesday",
                    Weekday::Wed => "Wednesday",
                    Weekday::Thu => "Thursday",
                    Weekday::Fri => "Friday",
                    Weekday::Sat => "Saturday",
                    Weekday::Sun => "Sunday",
                };

                let pnl_str = Self::format_currency(daily.realized_pnl);
                let pnl_display = if daily.realized_pnl > Decimal::ZERO {
                    pnl_str.green().to_string()
                } else if daily.realized_pnl < Decimal::ZERO {
                    pnl_str.red().to_string()
                } else {
                    pnl_str.yellow().to_string()
                };

                println!("    {:<12} {:>3} trades | Win Rate: {:>5.1}% | {:>10}",
                    day_name,
                    daily.total_trades,
                    daily.win_rate,
                    pnl_display
                );
            }
        }
    }

    fn format_currency(amount: Decimal) -> String {
        if amount >= Decimal::ZERO {
            format!("${:.2}", amount)
        } else {
            format!("-${:.2}", amount.abs())
        }
    }
}