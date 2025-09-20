use crate::models::{DailySummary, TradingSummary};
use chrono::{Datelike, NaiveDate, Utc, Weekday};
use colored::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct CalendarRenderer;

impl CalendarRenderer {
    pub fn render_combined_calendars(summary: &TradingSummary) {
        // Get current month or the month of the last trade
        let current_date = if let Some(last_summary) = summary.daily_summaries.last() {
            last_summary.date
        } else {
            Utc::now()
        };

        let year = current_date.year();
        let month = current_date.month();

        // Create a map of date to daily summary for quick lookup
        let mut daily_map: HashMap<NaiveDate, &DailySummary> = HashMap::new();
        for daily in &summary.daily_summaries {
            let date = daily.date.date_naive();
            if date.year() == year && date.month() == month {
                daily_map.insert(date, daily);
            }
        }

        // Calculate monthly statistics
        let month_summaries: Vec<&DailySummary> = summary.daily_summaries
            .iter()
            .filter(|s| s.date.year() == year && s.date.month() == month)
            .collect();

        let monthly_pnl: Decimal = month_summaries.iter().map(|s| s.realized_pnl).sum();
        let monthly_gross_pnl: Decimal = month_summaries.iter().map(|s| s.gross_pnl).sum();
        let monthly_commission: Decimal = month_summaries.iter().map(|s| s.total_commission).sum();

        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };

        // Print header
        println!("\n{}", "📅 Monthly Calendar Comparison".bold().cyan());
        println!();

        // Print month/year centered over both calendars
        println!("{:^120}", format!("{} {}", month_name, year).bold().bright_yellow());
        println!();

        // Print calendar titles side by side
        print!("{:^58}", "Net P&L (After Commissions)".bright_cyan());
        print!("  ");
        println!("{:^58}", "Gross P&L (Before Commissions)".bright_yellow());
        println!();

        // Print day headers for both calendars
        let day_header = format!("{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}",
            "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat");
        print!("{}", day_header);
        print!("    ");
        println!("{}", day_header);

        print!("{}", "─".repeat(56));
        print!("    ");
        println!("{}", "─".repeat(56));

        // Get the first day of the month and its weekday
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first_day.weekday();

        // Calculate padding for the first week
        let padding = match first_weekday {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        };

        // Get the last day of the month
        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap().pred_opt().unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap().pred_opt().unwrap()
        };

        // Build calendar weeks
        let mut weeks: Vec<Vec<Option<u32>>> = Vec::new();
        let mut current_week: Vec<Option<u32>> = Vec::new();

        // Add padding for first week
        for _ in 0..padding {
            current_week.push(None);
        }

        // Add all days
        for day in 1..=last_day.day() {
            current_week.push(Some(day));
            if current_week.len() == 7 {
                weeks.push(current_week.clone());
                current_week.clear();
            }
        }

        // Add last partial week if any
        if !current_week.is_empty() {
            while current_week.len() < 7 {
                current_week.push(None);
            }
            weeks.push(current_week);
        }

        // Print calendar weeks side by side
        for week in &weeks {
            // Print day numbers for both calendars
            for day_opt in week {
                match day_opt {
                    Some(day) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;
                        if is_weekend {
                            print!("{:^8}", day.to_string().bright_black());
                        } else {
                            print!("{:^8}", day);
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }

            print!("    "); // Spacing between calendars

            // Print day numbers again for right calendar
            for day_opt in week {
                match day_opt {
                    Some(day) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;
                        if is_weekend {
                            print!("{:^8}", day.to_string().bright_black());
                        } else {
                            print!("{:^8}", day);
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();

            // Print P&L values for both calendars
            // Net P&L (left calendar)
            for day_opt in week {
                match day_opt {
                    Some(day) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;

                        if is_weekend {
                            print!("{:^8}", "·".bright_black());
                        } else if let Some(summary) = daily_map.get(&date) {
                            let pnl = summary.realized_pnl;
                            if pnl > Decimal::ZERO {
                                print!("{:^8}", format!("+${:.0}", pnl).green().bold());
                            } else if pnl < Decimal::ZERO {
                                print!("{:^8}", format!("-${:.0}", pnl.abs()).red().bold());
                            } else {
                                print!("{:^8}", "$0".yellow());
                            }
                        } else {
                            print!("{:^8}", "-");
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }

            print!("    "); // Spacing between calendars

            // Gross P&L (right calendar)
            for day_opt in week {
                match day_opt {
                    Some(day) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;

                        if is_weekend {
                            print!("{:^8}", "·".bright_black());
                        } else if let Some(summary) = daily_map.get(&date) {
                            let gross_pnl = summary.gross_pnl;
                            if gross_pnl > Decimal::ZERO {
                                print!("{:^8}", format!("+${:.0}", gross_pnl).green().bold());
                            } else if gross_pnl < Decimal::ZERO {
                                print!("{:^8}", format!("-${:.0}", gross_pnl.abs()).red().bold());
                            } else {
                                print!("{:^8}", "$0".yellow());
                            }
                        } else {
                            print!("{:^8}", "-");
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();
            println!(); // Empty line between weeks
        }

        // Print comparison summary
        println!("{}", "─".repeat(120));
        println!("\n{}", "📊 Commission Impact Summary".bold().yellow());
        println!("{}", "─".repeat(120));

        let print_comparison = |label: &str, net: Decimal, gross: Decimal| {
            let net_str = Self::format_currency(net);
            let gross_str = Self::format_currency(gross);
            let diff = net - gross;
            let diff_str = Self::format_currency(diff.abs());

            let net_display = if net > Decimal::ZERO {
                net_str.green().bold().to_string()
            } else if net < Decimal::ZERO {
                net_str.red().bold().to_string()
            } else {
                net_str
            };

            let gross_display = if gross > Decimal::ZERO {
                gross_str.green().bold().to_string()
            } else if gross < Decimal::ZERO {
                gross_str.red().bold().to_string()
            } else {
                gross_str
            };

            println!("  {:<20} Net: {:<15} Gross: {:<15} Commission Impact: {}",
                label.bright_white(), net_display, gross_display, diff_str.bright_yellow());
        };

        print_comparison("Month P&L:", monthly_pnl, monthly_gross_pnl);
        println!("  {:<20} ${:.2}", "Total Commissions:".bright_white(), monthly_commission.abs());

        if monthly_gross_pnl != Decimal::ZERO {
            let commission_pct = (monthly_commission.abs() / monthly_gross_pnl.abs()) * Decimal::from(100);
            println!("  {:<20} {:.1}%", "Commission % of Gross:".bright_white(), commission_pct);
        }
    }

    #[allow(dead_code)]
    pub fn render_monthly_calendar(summary: &TradingSummary) {
        // Get current month or the month of the last trade
        let current_date = if let Some(last_summary) = summary.daily_summaries.last() {
            last_summary.date
        } else {
            Utc::now()
        };

        let year = current_date.year();
        let month = current_date.month();

        // Create a map of date to daily summary for quick lookup
        let mut daily_map: HashMap<NaiveDate, &DailySummary> = HashMap::new();
        for daily in &summary.daily_summaries {
            let date = daily.date.date_naive();
            if date.year() == year && date.month() == month {
                daily_map.insert(date, daily);
            }
        }

        // Calculate monthly statistics
        let month_summaries: Vec<&DailySummary> = summary.daily_summaries
            .iter()
            .filter(|s| s.date.year() == year && s.date.month() == month)
            .collect();

        let monthly_pnl: Decimal = month_summaries.iter().map(|s| s.realized_pnl).sum();
        let monthly_trades: u32 = month_summaries.iter().map(|s| s.total_trades).sum();
        let monthly_volume: Decimal = month_summaries.iter().map(|s| s.total_volume).sum();
        let monthly_commission: Decimal = month_summaries.iter().map(|s| s.total_commission).sum();

        let monthly_wins: u32 = month_summaries.iter().map(|s| s.winning_trades).sum();
        let monthly_win_rate = if monthly_trades > 0 {
            (monthly_wins as f64) / (monthly_trades as f64) * 100.0
        } else {
            0.0
        };

        // Print calendar header
        println!("\n{}", "📅 Monthly Calendar View".bold().cyan());
        println!();

        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };

        println!("{:^56}", format!("{} {}", month_name, year).bold().bright_yellow());
        println!("{:^56}", "Daily P&L shown below each date".bright_black());
        println!();

        // Print day headers with better spacing
        println!("{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}",
            "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"
        );
        println!("{}", "─".repeat(56));

        // Get the first day of the month and its weekday
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first_day.weekday();

        // Calculate padding for the first week
        let padding = match first_weekday {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        };

        // Get the last day of the month
        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap().pred_opt().unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap().pred_opt().unwrap()
        };

        // Build calendar in memory first for better formatting
        let mut weeks: Vec<Vec<Option<(u32, Option<Decimal>)>>> = Vec::new();
        let mut current_week: Vec<Option<(u32, Option<Decimal>)>> = Vec::new();

        // Add padding for first week
        for _ in 0..padding {
            current_week.push(None);
        }

        // Add all days
        for day in 1..=last_day.day() {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let pnl = daily_map.get(&date).map(|d| d.realized_pnl);
            current_week.push(Some((day, pnl)));

            if current_week.len() == 7 {
                weeks.push(current_week.clone());
                current_week.clear();
            }
        }

        // Add last partial week if any
        if !current_week.is_empty() {
            while current_week.len() < 7 {
                current_week.push(None);
            }
            weeks.push(current_week);
        }

        // Print calendar weeks
        for week in weeks {
            // Print day numbers
            for (_idx, day_opt) in week.iter().enumerate() {
                match day_opt {
                    Some((day, _)) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;
                        if is_weekend {
                            print!("{:^8}", day.to_string().bright_black());
                        } else {
                            print!("{:^8}", day);
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();

            // Print P&L values
            for day_opt in &week {
                match day_opt {
                    Some((day, pnl)) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;

                        if is_weekend {
                            // Weekends always shown as gray dots regardless of P&L
                            print!("{:^8}", "·".bright_black());
                        } else if let Some(pnl_val) = pnl {
                            // Weekday with trades
                            if *pnl_val > Decimal::ZERO {
                                print!("{:^8}", format!("+${:.0}", pnl_val).green().bold());
                            } else if *pnl_val < Decimal::ZERO {
                                print!("{:^8}", format!("-${:.0}", pnl_val.abs()).red().bold());
                            } else {
                                print!("{:^8}", "$0".yellow());
                            }
                        } else {
                            // Weekday with no trades
                            print!("{:^8}", "-");
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();
            println!(); // Empty line between weeks
        }

        // Print monthly summary
        println!("{}", "─".repeat(56));
        println!("\n{}", "📊 Monthly Summary".bold().yellow());
        println!("{}", "─".repeat(56));

        let print_row = |label: &str, value: String| {
            println!("  {:<20} {}", label.bright_white(), value);
        };

        let pnl_str = Self::format_currency(monthly_pnl);
        let pnl_display = if monthly_pnl > Decimal::ZERO {
            pnl_str.green().bold().to_string()
        } else if monthly_pnl < Decimal::ZERO {
            pnl_str.red().bold().to_string()
        } else {
            pnl_str
        };

        print_row("Month P&L:", pnl_display);
        print_row("Total Trades:", monthly_trades.to_string());
        print_row("Win Rate:", format!("{:.1}%", monthly_win_rate));
        print_row("Volume Traded:", format!("${:.2}", monthly_volume));
        print_row("Commissions:", format!("${:.2}", monthly_commission));

        // Trading days calculation
        let trading_days = month_summaries.len();
        let profitable_days = month_summaries.iter()
            .filter(|s| s.realized_pnl > Decimal::ZERO)
            .count();

        print_row("Trading Days:", format!("{} ({} profitable)", trading_days, profitable_days));

        if trading_days > 0 {
            let avg_daily_pnl = monthly_pnl / Decimal::from(trading_days);
            let avg_pnl_str = Self::format_currency(avg_daily_pnl);
            let avg_display = if avg_daily_pnl > Decimal::ZERO {
                avg_pnl_str.green().to_string()
            } else if avg_daily_pnl < Decimal::ZERO {
                avg_pnl_str.red().to_string()
            } else {
                avg_pnl_str
            };
            print_row("Avg Daily P&L:", avg_display);
        }

        // Best and worst days of the month
        if let Some(best_day) = month_summaries.iter().max_by_key(|s| s.realized_pnl) {
            let best_str = Self::format_currency(best_day.realized_pnl);
            let best_display = if best_day.realized_pnl > Decimal::ZERO {
                best_str.green().to_string()
            } else {
                best_str.red().to_string()
            };
            print_row("Best Day:", format!("{} ({})",
                best_day.date.format("%m/%d"),
                best_display));
        }

        if let Some(worst_day) = month_summaries.iter().min_by_key(|s| s.realized_pnl) {
            let worst_str = Self::format_currency(worst_day.realized_pnl);
            let worst_display = if worst_day.realized_pnl > Decimal::ZERO {
                worst_str.green().to_string()
            } else {
                worst_str.red().to_string()
            };
            print_row("Worst Day:", format!("{} ({})",
                worst_day.date.format("%m/%d"),
                worst_display));
        }
    }

    #[allow(dead_code)]
    pub fn render_gross_pnl_calendar(summary: &TradingSummary) {
        // Get current month or the month of the last trade
        let current_date = if let Some(last_summary) = summary.daily_summaries.last() {
            last_summary.date
        } else {
            Utc::now()
        };

        let year = current_date.year();
        let month = current_date.month();

        // Create a map of date to daily summary for quick lookup
        let mut daily_map: HashMap<NaiveDate, &DailySummary> = HashMap::new();
        for daily in &summary.daily_summaries {
            let date = daily.date.date_naive();
            if date.year() == year && date.month() == month {
                daily_map.insert(date, daily);
            }
        }

        // Calculate monthly statistics for gross P&L
        let month_summaries: Vec<&DailySummary> = summary.daily_summaries
            .iter()
            .filter(|s| s.date.year() == year && s.date.month() == month)
            .collect();

        let monthly_gross_pnl: Decimal = month_summaries.iter().map(|s| s.gross_pnl).sum();
        let monthly_trades: u32 = month_summaries.iter().map(|s| s.total_trades).sum();
        let monthly_volume: Decimal = month_summaries.iter().map(|s| s.total_volume).sum();

        let monthly_wins: u32 = month_summaries.iter().map(|s| s.winning_trades).sum();
        let monthly_win_rate = if monthly_trades > 0 {
            (monthly_wins as f64) / (monthly_trades as f64) * 100.0
        } else {
            0.0
        };

        // Print calendar header
        println!("\n{}", "📅 Monthly Calendar View - Gross P&L (Before Commissions)".bold().yellow());
        println!();

        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };

        println!("{:^56}", format!("{} {}", month_name, year).bold().bright_yellow());
        println!("{:^56}", "Daily Gross P&L shown below each date".bright_black());
        println!();

        // Print day headers with better spacing
        println!("{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}{:^8}",
            "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"
        );
        println!("{}", "─".repeat(56));

        // Get the first day of the month and its weekday
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let first_weekday = first_day.weekday();

        // Calculate padding for the first week
        let padding = match first_weekday {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        };

        // Get the last day of the month
        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap().pred_opt().unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap().pred_opt().unwrap()
        };

        // Build calendar in memory first for better formatting
        let mut weeks: Vec<Vec<Option<(u32, Option<Decimal>)>>> = Vec::new();
        let mut current_week: Vec<Option<(u32, Option<Decimal>)>> = Vec::new();

        // Add padding for first week
        for _ in 0..padding {
            current_week.push(None);
        }

        // Add all days
        for day in 1..=last_day.day() {
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let gross_pnl = daily_map.get(&date).map(|d| d.gross_pnl);
            current_week.push(Some((day, gross_pnl)));

            if current_week.len() == 7 {
                weeks.push(current_week.clone());
                current_week.clear();
            }
        }

        // Add last partial week if any
        if !current_week.is_empty() {
            while current_week.len() < 7 {
                current_week.push(None);
            }
            weeks.push(current_week);
        }

        // Print calendar weeks
        for week in weeks {
            // Print day numbers
            for day_opt in &week {
                match day_opt {
                    Some((day, _)) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;
                        if is_weekend {
                            print!("{:^8}", day.to_string().bright_black());
                        } else {
                            print!("{:^8}", day);
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();

            // Print Gross P&L values
            for day_opt in &week {
                match day_opt {
                    Some((day, gross_pnl)) => {
                        let date = NaiveDate::from_ymd_opt(year, month, *day).unwrap();
                        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;

                        if is_weekend {
                            // Weekends always shown as gray dots regardless of P&L
                            print!("{:^8}", "·".bright_black());
                        } else if let Some(pnl_val) = gross_pnl {
                            // Weekday with trades
                            if *pnl_val > Decimal::ZERO {
                                print!("{:^8}", format!("+${:.0}", pnl_val).green().bold());
                            } else if *pnl_val < Decimal::ZERO {
                                print!("{:^8}", format!("-${:.0}", pnl_val.abs()).red().bold());
                            } else {
                                print!("{:^8}", "$0".yellow());
                            }
                        } else {
                            // Weekday with no trades
                            print!("{:^8}", "-");
                        }
                    }
                    None => print!("{:^8}", ""),
                }
            }
            println!();
            println!(); // Empty line between weeks
        }

        // Print monthly summary for gross P&L
        println!("{}", "─".repeat(56));
        println!("\n{}", "📊 Monthly Gross P&L Summary".bold().yellow());
        println!("{}", "─".repeat(56));

        let print_row = |label: &str, value: String| {
            println!("  {:<20} {}", label.bright_white(), value);
        };

        let gross_pnl_str = Self::format_currency(monthly_gross_pnl);
        let gross_pnl_display = if monthly_gross_pnl > Decimal::ZERO {
            gross_pnl_str.green().bold().to_string()
        } else if monthly_gross_pnl < Decimal::ZERO {
            gross_pnl_str.red().bold().to_string()
        } else {
            gross_pnl_str
        };

        print_row("Gross P&L:", gross_pnl_display);
        print_row("Total Trades:", monthly_trades.to_string());
        print_row("Win Rate:", format!("{:.1}%", monthly_win_rate));
        print_row("Volume Traded:", format!("${:.2}", monthly_volume));

        // Trading days calculation
        let trading_days = month_summaries.len();
        let profitable_days = month_summaries.iter()
            .filter(|s| s.gross_pnl > Decimal::ZERO)
            .count();

        print_row("Trading Days:", format!("{} ({} gross profitable)", trading_days, profitable_days));

        if trading_days > 0 {
            let avg_daily_gross = monthly_gross_pnl / Decimal::from(trading_days);
            let avg_gross_str = Self::format_currency(avg_daily_gross);
            let avg_display = if avg_daily_gross > Decimal::ZERO {
                avg_gross_str.green().to_string()
            } else if avg_daily_gross < Decimal::ZERO {
                avg_gross_str.red().to_string()
            } else {
                avg_gross_str
            };
            print_row("Avg Daily Gross:", avg_display);
        }

        // Best and worst days of the month (gross)
        if let Some(best_day) = month_summaries.iter().max_by_key(|s| s.gross_pnl) {
            let best_str = Self::format_currency(best_day.gross_pnl);
            let best_display = if best_day.gross_pnl > Decimal::ZERO {
                best_str.green().to_string()
            } else {
                best_str.red().to_string()
            };
            print_row("Best Day (Gross):", format!("{} ({})",
                best_day.date.format("%m/%d"),
                best_display));
        }

        if let Some(worst_day) = month_summaries.iter().min_by_key(|s| s.gross_pnl) {
            let worst_str = Self::format_currency(worst_day.gross_pnl);
            let worst_display = if worst_day.gross_pnl > Decimal::ZERO {
                worst_str.green().to_string()
            } else {
                worst_str.red().to_string()
            };
            print_row("Worst Day (Gross):", format!("{} ({})",
                worst_day.date.format("%m/%d"),
                worst_display));
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