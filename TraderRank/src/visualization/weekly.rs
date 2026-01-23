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

    /// Renders a 6-month summary for longer-term perspective
    pub fn render_six_month_summary(summary: &TradingSummary) {
        if summary.monthly_summaries.is_empty() {
            return;
        }

        println!("\n{}", "📈 6-Month Performance Summary".bold().magenta());
        println!("{}", "═".repeat(120));

        // Get last 6 months
        let recent_months: Vec<_> = summary.monthly_summaries
            .iter()
            .rev()
            .take(6)
            .rev()
            .collect();

        if recent_months.is_empty() {
            println!("  No monthly data available.");
            return;
        }

        // Table header
        println!("\n{}", "📅 Monthly Performance Breakdown".bold().yellow());
        println!("{}", "─".repeat(120));
        println!("{:<12} {:>8} {:>6} {:>12} {:>12} {:>10} {:>10} {:>14} {:>14}",
            "Month", "Days", "Win%", "Commission", "Volume", "Trades", "Avg/Day", "Gross P&L", "Net P&L");
        println!("{}", "─".repeat(120));

        let mut total_pnl = Decimal::ZERO;
        let mut total_gross = Decimal::ZERO;
        let mut total_commission = Decimal::ZERO;
        let mut total_trades: u32 = 0;
        let mut total_days: u32 = 0;
        let mut total_wins: u32 = 0;

        for month in &recent_months {
            let month_label = format!("{} '{:02}", &month.month_name[..3], month.year % 100);

            let net_pnl_str = Self::format_currency(month.realized_pnl);
            let gross_pnl_str = Self::format_currency(month.gross_pnl);
            let commission_str = Self::format_currency(month.total_commission);
            let avg_daily_str = Self::format_currency(month.avg_daily_pnl);

            // Format with width first, then apply color
            let net_pnl_formatted = format!("{:>14}", net_pnl_str);
            let net_pnl_display = if month.realized_pnl > Decimal::ZERO {
                net_pnl_formatted.green().bold()
            } else if month.realized_pnl < Decimal::ZERO {
                net_pnl_formatted.red().bold()
            } else {
                net_pnl_formatted.yellow()
            };

            let gross_pnl_formatted = format!("{:>14}", gross_pnl_str);
            let gross_pnl_display = if month.gross_pnl > Decimal::ZERO {
                gross_pnl_formatted.green()
            } else if month.gross_pnl < Decimal::ZERO {
                gross_pnl_formatted.red()
            } else {
                gross_pnl_formatted.yellow()
            };

            let avg_daily_formatted = format!("{:>10}", avg_daily_str);
            let avg_daily_display = if month.avg_daily_pnl > Decimal::ZERO {
                avg_daily_formatted.green()
            } else if month.avg_daily_pnl < Decimal::ZERO {
                avg_daily_formatted.red()
            } else {
                avg_daily_formatted.normal()
            };

            println!("{:<12} {:>8} {:>6.1}% {:>12} {:>12} {:>10} {} {} {}",
                month_label,
                month.trading_days,
                month.win_rate,
                commission_str,
                month.total_volume.round().to_i64().unwrap_or(0),
                month.total_trades,
                avg_daily_display,
                gross_pnl_display,
                net_pnl_display
            );

            total_pnl += month.realized_pnl;
            total_gross += month.gross_pnl;
            total_commission += month.total_commission;
            total_trades += month.total_trades;
            total_days += month.trading_days;
            total_wins += month.winning_trades;
        }

        // Print totals
        println!("{}", "─".repeat(120));
        let total_win_rate = if total_trades > 0 {
            (total_wins as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };
        let avg_daily_total = if total_days > 0 {
            total_pnl / Decimal::from(total_days)
        } else {
            Decimal::ZERO
        };

        let total_net_formatted = format!("{:>14}", Self::format_currency(total_pnl));
        let total_net_display = if total_pnl > Decimal::ZERO {
            total_net_formatted.green().bold()
        } else if total_pnl < Decimal::ZERO {
            total_net_formatted.red().bold()
        } else {
            total_net_formatted.yellow()
        };

        let total_gross_formatted = format!("{:>14}", Self::format_currency(total_gross));
        let total_gross_display = if total_gross > Decimal::ZERO {
            total_gross_formatted.green()
        } else {
            total_gross_formatted.red()
        };

        let avg_daily_formatted = format!("{:>10}", Self::format_currency(avg_daily_total));
        let avg_daily_display = if avg_daily_total > Decimal::ZERO {
            avg_daily_formatted.green()
        } else if avg_daily_total < Decimal::ZERO {
            avg_daily_formatted.red()
        } else {
            avg_daily_formatted.normal()
        };

        println!("{:<12} {:>8} {:>6.1}% {:>12} {:>12} {:>10} {} {} {}",
            "TOTAL".bold(),
            total_days,
            total_win_rate,
            Self::format_currency(total_commission),
            "-",
            total_trades,
            avg_daily_display,
            total_gross_display,
            total_net_display
        );

        // Monthly P&L trend chart
        Self::render_monthly_pnl_chart(&recent_months);

        // Best and worst months
        Self::render_best_worst_months(summary);

        // Monthly consistency analysis
        Self::render_monthly_consistency(&recent_months);
    }

    fn render_monthly_pnl_chart(months: &[&crate::models::MonthlySummary]) {
        println!("\n{}", "📊 Monthly P&L Trend".bold().cyan());

        if months.is_empty() {
            return;
        }

        // Find max and min for scaling
        let max_pnl = months.iter()
            .map(|m| m.realized_pnl)
            .max()
            .unwrap_or(Decimal::ZERO);
        let min_pnl = months.iter()
            .map(|m| m.realized_pnl)
            .min()
            .unwrap_or(Decimal::ZERO);

        let range = max_pnl - min_pnl;
        let chart_width = 50;

        for month in months {
            let month_label = format!("{} '{:02}", &month.month_name[..3], month.year % 100);
            let pnl = month.realized_pnl;

            let bar_width = if range != Decimal::ZERO {
                let normalized = if pnl >= Decimal::ZERO {
                    (pnl / max_pnl.max(min_pnl.abs()) * Decimal::from(chart_width / 2))
                        .to_i32().unwrap_or(0).abs() as usize
                } else {
                    (pnl.abs() / max_pnl.max(min_pnl.abs()) * Decimal::from(chart_width / 2))
                        .to_i32().unwrap_or(0).abs() as usize
                };
                normalized.min(chart_width / 2)
            } else {
                0
            };

            let pnl_str = Self::format_currency(pnl);

            if pnl >= Decimal::ZERO {
                let padding = " ".repeat(chart_width / 2);
                let bar = "█".repeat(bar_width);
                println!("{:<10} {}│{} {:>12}",
                    month_label,
                    padding,
                    bar.green(),
                    pnl_str.green()
                );
            } else {
                let padding = " ".repeat((chart_width / 2).saturating_sub(bar_width));
                let bar = "█".repeat(bar_width);
                println!("{:<10} {}{}│ {:>12}",
                    month_label,
                    padding,
                    bar.red(),
                    pnl_str.red()
                );
            }
        }

        // Print scale line
        let scale_line = format!("{:<10} {}│",
            "",
            " ".repeat(chart_width / 2)
        );
        println!("{}", scale_line.bright_black());
    }

    fn render_best_worst_months(summary: &TradingSummary) {
        println!("\n{}", "🏆 Best & Worst Monthly Performance".bold().cyan());
        println!("{}", "─".repeat(80));

        if let Some(((year, month), pnl)) = summary.best_month {
            let month_name = match month {
                1 => "January", 2 => "February", 3 => "March",
                4 => "April", 5 => "May", 6 => "June",
                7 => "July", 8 => "August", 9 => "September",
                10 => "October", 11 => "November", 12 => "December",
                _ => "Unknown",
            };
            let pnl_str = Self::format_currency(pnl);
            println!("  {:<20} {} {} ({})",
                "Best Month:".green().bold(),
                month_name,
                year,
                pnl_str.green().bold()
            );
        }

        if let Some(((year, month), pnl)) = summary.worst_month {
            let month_name = match month {
                1 => "January", 2 => "February", 3 => "March",
                4 => "April", 5 => "May", 6 => "June",
                7 => "July", 8 => "August", 9 => "September",
                10 => "October", 11 => "November", 12 => "December",
                _ => "Unknown",
            };
            let pnl_str = Self::format_currency(pnl);
            println!("  {:<20} {} {} ({})",
                "Worst Month:".red().bold(),
                month_name,
                year,
                pnl_str.red().bold()
            );
        }

        // Calculate average monthly P&L
        if !summary.monthly_summaries.is_empty() {
            let avg_monthly_pnl: Decimal = summary.monthly_summaries.iter()
                .map(|m| m.realized_pnl)
                .sum::<Decimal>() / Decimal::from(summary.monthly_summaries.len());

            let avg_str = Self::format_currency(avg_monthly_pnl);
            let avg_display = if avg_monthly_pnl > Decimal::ZERO {
                avg_str.green().to_string()
            } else if avg_monthly_pnl < Decimal::ZERO {
                avg_str.red().to_string()
            } else {
                avg_str.yellow().to_string()
            };

            println!("  {:<20} {}",
                "Average Monthly P&L:".bright_white(),
                avg_display
            );
        }
    }

    fn render_monthly_consistency(months: &[&crate::models::MonthlySummary]) {
        println!("\n{}", "📈 Monthly Consistency Analysis".bold().yellow());
        println!("{}", "─".repeat(80));

        let profitable_months = months.iter().filter(|m| m.realized_pnl > Decimal::ZERO).count();
        let total_months = months.len();

        let consistency_rate = if total_months > 0 {
            (profitable_months as f64 / total_months as f64) * 100.0
        } else {
            0.0
        };

        let consistency_display = if consistency_rate >= 70.0 {
            format!("{:.1}%", consistency_rate).green().bold()
        } else if consistency_rate >= 50.0 {
            format!("{:.1}%", consistency_rate).yellow()
        } else {
            format!("{:.1}%", consistency_rate).red()
        };

        println!("  {:<25} {} ({}/{} months profitable)",
            "Monthly Win Rate:".bright_white(),
            consistency_display,
            profitable_months,
            total_months
        );

        // Calculate win streak
        let mut current_streak = 0;
        let mut max_win_streak = 0;
        let mut max_loss_streak = 0;
        let mut current_loss_streak = 0;

        for month in months.iter() {
            if month.realized_pnl > Decimal::ZERO {
                current_streak += 1;
                max_win_streak = max_win_streak.max(current_streak);
                current_loss_streak = 0;
            } else {
                current_loss_streak += 1;
                max_loss_streak = max_loss_streak.max(current_loss_streak);
                current_streak = 0;
            }
        }

        println!("  {:<25} {} months",
            "Longest Win Streak:".bright_white(),
            max_win_streak.to_string().green()
        );

        println!("  {:<25} {} months",
            "Longest Loss Streak:".bright_white(),
            max_loss_streak.to_string().red()
        );

        // Calculate month-over-month growth
        if months.len() >= 2 {
            let mut improvements = 0;
            for i in 1..months.len() {
                if months[i].realized_pnl > months[i-1].realized_pnl {
                    improvements += 1;
                }
            }
            let improvement_rate = (improvements as f64 / (months.len() - 1) as f64) * 100.0;
            println!("  {:<25} {:.1}% of months improved over prior",
                "Growth Trend:".bright_white(),
                improvement_rate
            );
        }
    }
}