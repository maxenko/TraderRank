use crate::models::DailySummary;
use colored::*;
use rust_decimal::prelude::*;

pub struct ChartRenderer;

impl ChartRenderer {
    pub fn render_pnl_chart(summaries: &[DailySummary]) {
        if summaries.is_empty() {
            return;
        }

        println!("\n{}", "📉 P&L Trend Chart".bold().cyan());

        // Create a simple ASCII line chart
        Self::render_ascii_line_chart(summaries);

        // Add a simple bar representation below
        Self::render_simple_bars(summaries);

        // Add gross P&L bars (without commissions)
        Self::render_gross_pnl_bars(summaries);
    }

    fn render_ascii_line_chart(summaries: &[DailySummary]) {
        if summaries.len() < 2 {
            return;
        }

        let height = 10;
        let width = 60;

        // Find min and max P&L
        let min_pnl = summaries.iter()
            .map(|s| s.realized_pnl)
            .min()
            .unwrap_or(Decimal::ZERO);
        let max_pnl = summaries.iter()
            .map(|s| s.realized_pnl)
            .max()
            .unwrap_or(Decimal::ZERO);

        let range = max_pnl - min_pnl;
        if range == Decimal::ZERO {
            return;
        }

        // Create the chart grid
        let mut chart = vec![vec![' '; width]; height];

        // Plot the points
        for (i, summary) in summaries.iter().enumerate() {
            let x = (i * (width - 1)) / (summaries.len() - 1);
            let normalized = ((summary.realized_pnl - min_pnl) / range * Decimal::from(height - 1))
                .to_usize()
                .unwrap_or(0);
            let y = height - 1 - normalized;

            if x < width && y < height {
                chart[y][x] = '●';

                // Connect points with lines (simple interpolation)
                if i > 0 {
                    let prev_summary = &summaries[i - 1];
                    let prev_x = ((i - 1) * (width - 1)) / (summaries.len() - 1);
                    let prev_normalized = ((prev_summary.realized_pnl - min_pnl) / range * Decimal::from(height - 1))
                        .to_usize()
                        .unwrap_or(0);
                    let prev_y = height - 1 - prev_normalized;

                    // Draw line between points
                    let steps = (x - prev_x).max(1);
                    for step in 1..steps {
                        let interp_x = prev_x + step;
                        let interp_y = if y >= prev_y {
                            prev_y + ((y - prev_y) * step) / steps
                        } else {
                            prev_y.saturating_sub(((prev_y - y) * step) / steps)
                        };
                        if interp_x < width && interp_y < height {
                            chart[interp_y][interp_x] = '─';
                        }
                    }
                }
            }
        }

        // Print the chart with axes
        let max_label = format!("${:.0}", max_pnl);
        let min_label = format!("${:.0}", min_pnl);

        println!("{:>8} ┤", max_label);
        for row in chart.iter() {
            print!("        │");
            for &cell in row.iter() {
                print!("{}", if cell == '●' {
                    cell.to_string().green().bold().to_string()
                } else if cell == '─' {
                    cell.to_string().cyan().to_string()
                } else {
                    cell.to_string()
                });
            }
            println!();
        }
        println!("{:>8} └{}─", min_label, "─".repeat(width));

        // X-axis labels (dates)
        print!("         ");
        for i in 0..5 {
            let idx = (i * (summaries.len() - 1)) / 4;
            if idx < summaries.len() {
                let date = summaries[idx].date.format("%m/%d").to_string();
                print!("{:<12}", date);
            }
        }
        println!();
    }

    pub fn render_hourly_distribution(summary: &DailySummary) {
        if summary.time_slot_performance.is_empty() {
            return;
        }

        println!("\n{}", "⏰ Hourly P&L Distribution".bold().cyan());

        let max_pnl = summary.time_slot_performance
            .iter()
            .map(|s| s.pnl.abs())
            .max()
            .unwrap_or(Decimal::ZERO);

        if max_pnl == Decimal::ZERO {
            return;
        }

        for slot in &summary.time_slot_performance {
            let hour_label = format!("{:02}:00", slot.hour);
            let bar_width = 40;
            let normalized = (slot.pnl.abs() / max_pnl * Decimal::from(bar_width))
                .to_usize()
                .unwrap_or(0);

            let bar = if slot.pnl > Decimal::ZERO {
                format!("{}", "█".repeat(normalized).green())
            } else {
                format!("{}", "█".repeat(normalized).red())
            };

            let pnl_str = format!("${:.2}", slot.pnl);
            println!("{} {} {}", hour_label.bright_white(), bar, pnl_str);
        }
    }

    fn render_simple_bars(summaries: &[DailySummary]) {
        println!("\n{}", "Daily P&L Bars (Net - After Commissions):".bold().cyan());

        let max_abs_pnl = summaries
            .iter()
            .map(|s| s.realized_pnl.abs())
            .max()
            .unwrap_or(Decimal::ZERO);

        if max_abs_pnl == Decimal::ZERO {
            return;
        }

        for summary in summaries.iter().rev().take(10).rev() {
            let date_str = summary.date.format("%m/%d").to_string();
            let bar_width = 30;
            let normalized = (summary.realized_pnl.abs() / max_abs_pnl * Decimal::from(bar_width))
                .to_usize()
                .unwrap_or(0);

            let bar = if summary.realized_pnl > Decimal::ZERO {
                format!("{:>30}", "█".repeat(normalized)).green()
            } else {
                format!("{:>30}", "█".repeat(normalized)).red()
            };

            let pnl_str = if summary.realized_pnl >= Decimal::ZERO {
                format!("▲${:.2}", summary.realized_pnl.abs()).green()
            } else {
                format!("▼${:.2}", summary.realized_pnl.abs()).red()
            };

            println!("{} {} {}", date_str.bright_white(), bar.to_string(), pnl_str);
        }
    }

    fn render_gross_pnl_bars(summaries: &[DailySummary]) {
        println!("\n{}", "Daily P&L Bars (Gross - Before Commissions):".bold().yellow());

        let max_abs_pnl = summaries
            .iter()
            .map(|s| s.gross_pnl.abs())
            .max()
            .unwrap_or(Decimal::ZERO);

        if max_abs_pnl == Decimal::ZERO {
            // If no gross P&L, show a message
            println!("No gross P&L data available");
            return;
        }

        for summary in summaries.iter().rev().take(10).rev() {
            let date_str = summary.date.format("%m/%d").to_string();
            let bar_width = 30;
            let normalized = (summary.gross_pnl.abs() / max_abs_pnl * Decimal::from(bar_width))
                .to_usize()
                .unwrap_or(0);

            let bar = if summary.gross_pnl > Decimal::ZERO {
                format!("{:>30}", "▓".repeat(normalized)).green()
            } else {
                format!("{:>30}", "▓".repeat(normalized)).red()
            };

            let gross_pnl_str = if summary.gross_pnl >= Decimal::ZERO {
                format!("▲${:.2}", summary.gross_pnl.abs()).green()
            } else {
                format!("▼${:.2}", summary.gross_pnl.abs()).red()
            };

            // Also show commission impact
            let commission_str = format!("(-${:.2})", summary.total_commission).yellow();

            println!("{} {} {} {}", date_str.bright_white(), bar.to_string(), gross_pnl_str, commission_str);
        }

        // Show total commission impact
        let total_commission: Decimal = summaries.iter().map(|s| s.total_commission).sum();
        let total_gross: Decimal = summaries.iter().map(|s| s.gross_pnl).sum();
        let total_net: Decimal = summaries.iter().map(|s| s.realized_pnl).sum();

        println!("\n{}", "Commission Impact Summary:".bold().yellow());
        println!("  Gross P&L: {}",
            if total_gross >= Decimal::ZERO {
                format!("${:.2}", total_gross).green()
            } else {
                format!("-${:.2}", total_gross.abs()).red()
            });
        println!("  Total Commissions: {}", format!("-${:.2}", total_commission).yellow());
        println!("  Net P&L: {}",
            if total_net >= Decimal::ZERO {
                format!("${:.2}", total_net).green()
            } else {
                format!("-${:.2}", total_net.abs()).red()
            });
        println!("  Commission % of Gross: {:.1}%",
            if total_gross != Decimal::ZERO {
                (total_commission / total_gross.abs() * Decimal::from(100)).to_f64().unwrap_or(0.0)
            } else {
                0.0
            });
    }

    pub fn render_daily_winrate_chart(summaries: &[DailySummary]) {
        if summaries.is_empty() {
            return;
        }

        println!("\n{}", "📊 Daily Win Rate - Last 10 Trading Days".bold().cyan());
        println!();

        // Take last 10 trading days (exclude weekends with no trades)
        let trading_days: Vec<&DailySummary> = summaries
            .iter()
            .filter(|s| s.total_trades > 0)  // Only include days with actual trades
            .collect();

        let last_10: Vec<&DailySummary> = trading_days
            .iter()
            .rev()
            .take(10)
            .rev()
            .copied()
            .collect();

        // Find max win rate for scaling
        let max_rate = 100.0;
        let chart_width = 50;

        for summary in &last_10 {
            let date_str = summary.date.format("%m/%d").to_string();

            // Calculate bar width
            let bar_width = ((summary.win_rate / max_rate) * chart_width as f64) as usize;

            // Create the bar
            let bar = if summary.total_trades > 0 {
                let bar_char = "█";
                let bar_str = bar_char.repeat(bar_width.max(1));

                // Color based on win rate
                if summary.win_rate >= 60.0 {
                    bar_str.green().bold().to_string()
                } else if summary.win_rate >= 50.0 {
                    bar_str.yellow().to_string()
                } else if summary.win_rate > 0.0 {
                    bar_str.red().to_string()
                } else {
                    "━".bright_black().to_string()
                }
            } else {
                "No trades".bright_black().to_string()
            };

            // Format the output
            let win_loss = if summary.total_trades > 0 {
                format!("({}/{})", summary.winning_trades, summary.losing_trades)
            } else {
                "(0/0)".to_string()
            };

            let rate_display = if summary.total_trades > 0 {
                format!("{:>5.1}%", summary.win_rate)
            } else {
                "  N/A".to_string()
            };

            // Color the percentage based on performance
            let rate_colored = if summary.win_rate >= 60.0 {
                rate_display.green().bold().to_string()
            } else if summary.win_rate >= 50.0 {
                rate_display.yellow().to_string()
            } else if summary.total_trades > 0 {
                rate_display.red().to_string()
            } else {
                rate_display.bright_black().to_string()
            };

            println!("{} {} {} {:>9}",
                date_str.bright_white(),
                bar,
                rate_colored,
                win_loss.bright_black()
            );
        }

        // Add legend
        println!();
        println!("{}", "Legend: ".bright_black());
        println!("  {} ≥60% win rate    {} 50-59% win rate    {} <50% win rate",
            "█".green().bold(),
            "█".yellow(),
            "█".red()
        );
    }

    pub fn render_winrate_progression(summaries: &[DailySummary]) {
        if summaries.is_empty() {
            return;
        }

        // Filter to only trading days
        let trading_days: Vec<&DailySummary> = summaries
            .iter()
            .filter(|s| s.total_trades > 0)
            .collect();

        if trading_days.len() < 2 {
            return;
        }

        println!("\n{}", "📈 Daily Win Rate Progression".bold().cyan());

        // Calculate cumulative win rate over time
        let mut cumulative_wins: u32 = 0;
        let mut cumulative_trades: u32 = 0;
        let mut progression: Vec<(String, f64, f64)> = Vec::new(); // (date, daily_rate, cumulative_rate)

        for summary in &trading_days {
            cumulative_wins += summary.winning_trades;
            cumulative_trades += summary.total_trades;

            let cumulative_rate = if cumulative_trades > 0 {
                (cumulative_wins as f64 / cumulative_trades as f64) * 100.0
            } else {
                0.0
            };

            progression.push((
                summary.date.format("%m/%d").to_string(),
                summary.win_rate,
                cumulative_rate,
            ));
        }

        // Render ASCII line chart for cumulative win rate
        Self::render_winrate_line_chart(&progression);

        // Show recent progression data
        println!("\n{}", "Recent Win Rate Trend:".bold().white());
        println!("{}", "─".repeat(70));
        println!("{:<10} {:>12} {:>15} {:>15} {:>12}",
            "Date".bright_white().bold(),
            "Daily".bright_white().bold(),
            "Cumulative".bright_white().bold(),
            "Trades".bright_white().bold(),
            "Running".bright_white().bold()
        );
        println!("{}", "─".repeat(70));

        let mut running_wins: u32 = 0;
        let mut running_total: u32 = 0;

        // Show last 10 days
        let start_idx = if trading_days.len() > 10 { trading_days.len() - 10 } else { 0 };
        for (i, summary) in trading_days.iter().enumerate().skip(start_idx) {
            running_wins += summary.winning_trades;
            running_total += summary.total_trades;

            let cumulative_rate = if running_total > 0 {
                (running_wins as f64 / running_total as f64) * 100.0
            } else {
                0.0
            };

            let date_str = summary.date.format("%m/%d").to_string();
            let daily_rate_str = format!("{:.1}%", summary.win_rate);
            let cumulative_str = format!("{:.1}%", progression[i].2);
            let trades_str = format!("{}/{}", summary.winning_trades, summary.total_trades);
            let running_str = format!("{}/{}", running_wins, running_total);

            // Color daily rate
            let daily_colored = if summary.win_rate >= 60.0 {
                daily_rate_str.green().bold()
            } else if summary.win_rate >= 50.0 {
                daily_rate_str.yellow()
            } else {
                daily_rate_str.red()
            };

            // Color cumulative rate
            let cumulative_colored = if cumulative_rate >= 50.0 {
                cumulative_str.green()
            } else {
                cumulative_str.red()
            };

            println!("{:<10} {:>12} {:>15} {:>15} {:>12}",
                date_str.bright_white(),
                daily_colored,
                cumulative_colored,
                trades_str.bright_black(),
                running_str.bright_black()
            );
        }

        // Summary statistics
        let final_cumulative = progression.last().map(|p| p.2).unwrap_or(0.0);
        let avg_daily: f64 = trading_days.iter().map(|s| s.win_rate).sum::<f64>() / trading_days.len() as f64;
        let best_day = trading_days.iter().max_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap());
        let worst_day = trading_days.iter().filter(|s| s.total_trades > 0).min_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap());

        println!("{}", "─".repeat(70));
        println!("\n{}", "Win Rate Statistics:".bold().white());

        let overall_colored = if final_cumulative >= 50.0 {
            format!("{:.1}%", final_cumulative).green().bold()
        } else {
            format!("{:.1}%", final_cumulative).red().bold()
        };
        println!("  Overall Win Rate:    {}", overall_colored);

        let avg_colored = if avg_daily >= 50.0 {
            format!("{:.1}%", avg_daily).green()
        } else {
            format!("{:.1}%", avg_daily).red()
        };
        println!("  Avg Daily Win Rate:  {}", avg_colored);

        if let Some(best) = best_day {
            println!("  Best Day:            {} ({:.1}%)",
                best.date.format("%m/%d").to_string().green(),
                best.win_rate);
        }
        if let Some(worst) = worst_day {
            println!("  Worst Day:           {} ({:.1}%)",
                worst.date.format("%m/%d").to_string().red(),
                worst.win_rate);
        }

        // Calculate win rate trend (improving or declining)
        if progression.len() >= 5 {
            let recent_5: f64 = progression.iter().rev().take(5).map(|p| p.1).sum::<f64>() / 5.0;
            let earlier_5: f64 = if progression.len() >= 10 {
                progression.iter().rev().skip(5).take(5).map(|p| p.1).sum::<f64>() / 5.0
            } else {
                progression.iter().take(progression.len() - 5).map(|p| p.1).sum::<f64>()
                    / (progression.len() - 5) as f64
            };

            let trend = recent_5 - earlier_5;
            let trend_str = if trend > 2.0 {
                format!("↑ Improving (+{:.1}%)", trend).green().bold()
            } else if trend < -2.0 {
                format!("↓ Declining ({:.1}%)", trend).red().bold()
            } else {
                format!("→ Stable ({:+.1}%)", trend).yellow()
            };
            println!("  Recent Trend:        {}", trend_str);
        }
    }

    fn render_winrate_line_chart(progression: &[(String, f64, f64)]) {
        if progression.len() < 2 {
            return;
        }

        let height = 10;
        let width = 60;

        // Fixed scale: 0-100%
        let display_max = 100.0;
        let range = 100.0;

        // Create the chart grid
        let mut chart = vec![vec![' '; width]; height];

        // Draw 50% reference line
        let fifty_percent_y = ((display_max - 50.0) / range * (height - 1) as f64) as usize;
        if fifty_percent_y < height {
            for x in 0..width {
                chart[fifty_percent_y][x] = '·';
            }
        }

        // Plot the DAILY win rate line (not cumulative)
        for (i, (_, daily_rate, _)) in progression.iter().enumerate() {
            let x = (i * (width - 1)) / (progression.len() - 1);
            let normalized = ((display_max - daily_rate) / range * (height - 1) as f64) as usize;
            let y = normalized.min(height - 1);

            if x < width && y < height {
                chart[y][x] = '●';

                // Connect points with lines
                if i > 0 {
                    let prev_daily = progression[i - 1].1;
                    let prev_x = ((i - 1) * (width - 1)) / (progression.len() - 1);
                    let prev_normalized = ((display_max - prev_daily) / range * (height - 1) as f64) as usize;
                    let prev_y = prev_normalized.min(height - 1);

                    let steps = (x - prev_x).max(1);
                    for step in 1..steps {
                        let interp_x = prev_x + step;
                        let interp_y = if y >= prev_y {
                            prev_y + ((y - prev_y) * step) / steps
                        } else {
                            prev_y.saturating_sub(((prev_y - y) * step) / steps)
                        };
                        if interp_x < width && interp_y < height && chart[interp_y][interp_x] == ' ' {
                            chart[interp_y][interp_x] = '─';
                        }
                    }
                }
            }
        }

        // Print the chart with axes
        println!("{:>8} ┤", "100%");
        for (row_idx, row) in chart.iter().enumerate() {
            // Show 50% marker on the left
            let is_fifty_line = row_idx == fifty_percent_y;
            if is_fifty_line {
                print!("{:>8} ┤", "50%".yellow());
            } else {
                print!("        │");
            }

            for &cell in row.iter() {
                let cell_str = match cell {
                    '●' => cell.to_string().cyan().bold().to_string(),
                    '─' => cell.to_string().cyan().to_string(),
                    '·' => cell.to_string().yellow().dimmed().to_string(),
                    _ => cell.to_string(),
                };
                print!("{}", cell_str);
            }
            println!();
        }
        println!("{:>8} └{}─", "0%", "─".repeat(width));

        // X-axis labels (dates)
        print!("         ");
        let label_count = 5.min(progression.len());
        for i in 0..label_count {
            let idx = (i * (progression.len() - 1)) / (label_count - 1).max(1);
            if idx < progression.len() {
                print!("{:<12}", progression[idx].0);
            }
        }
        println!();
    }
}