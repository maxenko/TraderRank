use crate::models::{Trade, Side, DailySummary, WeeklySummary, TradingSummary, TimeSlotPerformance};
use chrono::{DateTime, Utc, Datelike, Weekday};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

pub struct TradingAnalytics;

impl TradingAnalytics {
    pub fn analyze_trades(trades: &[Trade]) -> TradingSummary {
        let mut daily_trades: HashMap<String, Vec<Trade>> = HashMap::new();

        // Group trades by date
        for trade in trades {
            let date_key = format!("{}", trade.time.date_naive());
            daily_trades.entry(date_key).or_insert_with(Vec::new).push(trade.clone());
        }

        let mut daily_summaries: Vec<DailySummary> = daily_trades
            .into_iter()
            .map(|(_, day_trades)| Self::calculate_daily_summary(day_trades))
            .collect();

        daily_summaries.sort_by_key(|s| s.date);

        let total_pnl = daily_summaries.iter().map(|s| s.realized_pnl).sum();
        let total_volume = daily_summaries.iter().map(|s| s.total_volume).sum();
        let total_trades = daily_summaries.iter().map(|s| s.total_trades).sum();

        let total_wins: u32 = daily_summaries.iter().map(|s| s.winning_trades).sum();
        let overall_win_rate = if total_trades > 0 {
            (total_wins as f64) / (total_trades as f64) * 100.0
        } else {
            0.0
        };

        let best_day = daily_summaries
            .iter()
            .max_by_key(|s| s.realized_pnl)
            .map(|s| (s.date, s.realized_pnl));

        let worst_day = daily_summaries
            .iter()
            .min_by_key(|s| s.realized_pnl)
            .map(|s| (s.date, s.realized_pnl));

        let (most_profitable_hour, least_profitable_hour) = Self::analyze_hourly_performance(&daily_summaries);

        // Calculate weekly summaries
        let weekly_summaries = Self::calculate_weekly_summaries(&daily_summaries);

        // Find best and worst weeks
        let best_week = weekly_summaries
            .iter()
            .max_by_key(|w| w.realized_pnl)
            .map(|w| ((w.year, w.week_number), w.realized_pnl));

        let worst_week = weekly_summaries
            .iter()
            .min_by_key(|w| w.realized_pnl)
            .map(|w| ((w.year, w.week_number), w.realized_pnl));

        TradingSummary {
            start_date: daily_summaries.first().map(|s| s.date).unwrap_or_else(|| Utc::now()),
            end_date: daily_summaries.last().map(|s| s.date).unwrap_or_else(|| Utc::now()),
            daily_summaries,
            weekly_summaries,
            total_pnl,
            total_volume,
            total_trades,
            overall_win_rate,
            best_day,
            worst_day,
            best_week,
            worst_week,
            most_profitable_hour,
            least_profitable_hour,
        }
    }

    fn calculate_daily_summary(mut trades: Vec<Trade>) -> DailySummary {
        trades.sort_by_key(|t| t.time);

        let date = trades.first().unwrap().time.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let date_utc = DateTime::<Utc>::from_naive_utc_and_offset(date, Utc);
        let mut summary = DailySummary::new(date_utc);

        // Keep a copy for hourly performance calculation
        let all_trades = trades.clone();

        // Group trades by symbol first
        let mut trades_by_symbol: HashMap<String, Vec<Trade>> = HashMap::new();
        for trade in trades {
            trades_by_symbol.entry(trade.symbol.clone())
                .or_insert_with(Vec::new)
                .push(trade);
        }

        let mut realized_trades = Vec::new();
        let mut symbols_set = HashSet::new();
        let mut total_commission = Decimal::ZERO;
        let mut total_volume = Decimal::ZERO;

        // Process each symbol's trades separately
        for (symbol, mut symbol_trades) in trades_by_symbol {
            // Skip symbols with only one trade (no matching pair)
            if symbol_trades.len() < 2 {
                eprintln!("Warning: Unmatched trade for {}: {} {} shares at ${}",
                    symbol,
                    match symbol_trades[0].side {
                        Side::Buy => "Buy",
                        Side::Sell => "Sell",
                    },
                    symbol_trades[0].quantity,
                    symbol_trades[0].fill_price
                );
                total_commission += symbol_trades[0].commission;
                // Still count the volume for unmatched trades
                total_volume += symbol_trades[0].quantity * symbol_trades[0].fill_price;
                continue;
            }

            symbols_set.insert(symbol.clone());
            symbol_trades.sort_by_key(|t| t.time);

            let mut position = Decimal::ZERO;
            let mut avg_price = Decimal::ZERO;

            for trade in &symbol_trades {
                total_commission += trade.commission;
                // Add to total volume (quantity * fill_price for each trade)
                total_volume += trade.quantity * trade.fill_price;

                match trade.side {
                    Side::Buy => {
                        if position < Decimal::ZERO {
                            // Closing short position (buying back)
                            let qty_to_close = trade.quantity.min(-position);
                            if qty_to_close > Decimal::ZERO {
                                let trade_pnl = (avg_price - trade.fill_price) * qty_to_close;
                                realized_trades.push(trade_pnl);
                                position += qty_to_close;
                            }

                            // If buying more than needed to close, start long position
                            let qty_remaining = trade.quantity - qty_to_close;
                            if qty_remaining > Decimal::ZERO {
                                position = qty_remaining;
                                avg_price = trade.fill_price;
                            } else if position == Decimal::ZERO {
                                avg_price = Decimal::ZERO;
                            }
                        } else if position == Decimal::ZERO {
                            // Opening long position
                            position = trade.quantity;
                            avg_price = trade.fill_price;
                        } else {
                            // Adding to long position
                            let total_value = avg_price * position + trade.fill_price * trade.quantity;
                            position += trade.quantity;
                            avg_price = total_value / position;
                        }
                    }
                    Side::Sell => {
                        if position > Decimal::ZERO {
                            // Closing long position (selling)
                            let qty_to_close = trade.quantity.min(position);
                            if qty_to_close > Decimal::ZERO {
                                let trade_pnl = (trade.fill_price - avg_price) * qty_to_close;
                                realized_trades.push(trade_pnl);
                                position -= qty_to_close;
                            }

                            // Check if trying to sell more than owned
                            let qty_remaining = trade.quantity - qty_to_close;
                            if qty_remaining > Decimal::ZERO {
                                eprintln!("Warning: {} - Selling {} more shares than owned (had {} shares)",
                                    symbol, qty_remaining, qty_to_close);
                                // Don't open a short position from overselling
                            }

                            if position == Decimal::ZERO {
                                avg_price = Decimal::ZERO;
                            }
                        } else if position == Decimal::ZERO {
                            // Opening short position
                            position = -trade.quantity;
                            avg_price = trade.fill_price;
                        } else {
                            // Adding to short position
                            let total_value = avg_price * (-position) + trade.fill_price * trade.quantity;
                            position -= trade.quantity;
                            avg_price = total_value / (-position);
                        }
                    }
                }
            }

            // Warn about unclosed positions
            if position != Decimal::ZERO {
                let position_type = if position > Decimal::ZERO { "long" } else { "short" };
                eprintln!("Warning: {} - Unclosed {} position of {} shares",
                    symbol, position_type, position.abs());
            }
        }

        // Process realized trades
        let mut winning_pnls = Vec::new();
        let mut losing_pnls = Vec::new();

        for pnl in realized_trades {
            // Subtract commission from P&L (proportionally from total commission)
            if pnl > Decimal::ZERO {
                summary.winning_trades += 1;
                winning_pnls.push(pnl);
                if pnl > summary.largest_win {
                    summary.largest_win = pnl;
                }
            } else if pnl < Decimal::ZERO {
                summary.losing_trades += 1;
                losing_pnls.push(pnl);
                if pnl < summary.largest_loss {
                    summary.largest_loss = pnl;
                }
            }
            summary.realized_pnl += pnl;
        }

        // Subtract total commission from realized P&L
        summary.realized_pnl -= total_commission;
        summary.total_commission = total_commission;
        summary.total_volume = total_volume;

        summary.total_trades = (summary.winning_trades + summary.losing_trades) as u32;
        summary.symbols_traded = symbols_set.into_iter().collect();

        if !winning_pnls.is_empty() {
            let sum: Decimal = winning_pnls.iter().sum();
            summary.avg_win = sum / Decimal::from(winning_pnls.len());
        }

        if !losing_pnls.is_empty() {
            let sum: Decimal = losing_pnls.iter().sum();
            summary.avg_loss = sum / Decimal::from(losing_pnls.len());
        }

        summary.win_rate = if summary.total_trades > 0 {
            (summary.winning_trades as f64) / (summary.total_trades as f64) * 100.0
        } else {
            0.0
        };

        summary.gross_pnl = summary.realized_pnl + summary.total_commission;

        summary.time_slot_performance = Self::calculate_hourly_performance(&all_trades);

        summary
    }

    fn calculate_hourly_performance(trades: &[Trade]) -> Vec<TimeSlotPerformance> {
        // Group trades by hour, then by symbol for proper pairing
        let mut hourly_symbol_trades: HashMap<u32, HashMap<String, Vec<Trade>>> = HashMap::new();

        for trade in trades {
            let hour = trade.hour_of_day();
            hourly_symbol_trades
                .entry(hour)
                .or_insert_with(HashMap::new)
                .entry(trade.symbol.clone())
                .or_insert_with(Vec::new)
                .push(trade.clone());
        }

        let mut hourly_data: HashMap<u32, (u32, Decimal, u32, u32)> = HashMap::new();

        for (hour, symbol_trades) in hourly_symbol_trades {
            let mut hour_pnl = Decimal::ZERO;
            let mut hour_wins = 0;
            let mut hour_losses = 0;
            let mut hour_trade_count = 0;

            for (_symbol, mut trades) in symbol_trades {
                hour_trade_count += trades.len() as u32;

                // Skip symbols with only one trade in this hour
                if trades.len() < 2 {
                    continue;
                }

                trades.sort_by_key(|t| t.time);

                // Calculate P&L for matched trades only
                let mut position = Decimal::ZERO;
                let mut avg_price = Decimal::ZERO;

                for trade in &trades {
                    match trade.side {
                        Side::Buy => {
                            if position < Decimal::ZERO {
                                let qty_to_close = trade.quantity.min(-position);
                                if qty_to_close > Decimal::ZERO {
                                    let trade_pnl = (avg_price - trade.fill_price) * qty_to_close - trade.commission;
                                    hour_pnl += trade_pnl;
                                    if trade_pnl > Decimal::ZERO {
                                        hour_wins += 1;
                                    } else if trade_pnl < Decimal::ZERO {
                                        hour_losses += 1;
                                    }
                                }
                                position += trade.quantity;
                                if position >= Decimal::ZERO {
                                    avg_price = trade.fill_price;
                                }
                            } else {
                                if position == Decimal::ZERO {
                                    avg_price = trade.fill_price;
                                } else {
                                    let total_value = avg_price * position + trade.fill_price * trade.quantity;
                                    position += trade.quantity;
                                    avg_price = total_value / position;
                                }
                            }
                        }
                        Side::Sell => {
                            if position > Decimal::ZERO {
                                let qty_to_close = trade.quantity.min(position);
                                if qty_to_close > Decimal::ZERO {
                                    let trade_pnl = (trade.fill_price - avg_price) * qty_to_close - trade.commission;
                                    hour_pnl += trade_pnl;
                                    if trade_pnl > Decimal::ZERO {
                                        hour_wins += 1;
                                    } else if trade_pnl < Decimal::ZERO {
                                        hour_losses += 1;
                                    }
                                }
                                position -= trade.quantity;
                                if position <= Decimal::ZERO {
                                    position = position.max(Decimal::ZERO);
                                    if position == Decimal::ZERO {
                                        avg_price = Decimal::ZERO;
                                    }
                                }
                            } else if position == Decimal::ZERO {
                                position = -trade.quantity;
                                avg_price = trade.fill_price;
                            } else {
                                let total_value = avg_price * (-position) + trade.fill_price * trade.quantity;
                                position -= trade.quantity;
                                avg_price = total_value / (-position);
                            }
                        }
                    }
                }
            }

            hourly_data.insert(hour, (hour_trade_count, hour_pnl, hour_wins, hour_losses));
        }

        let mut slots: Vec<TimeSlotPerformance> = hourly_data
            .into_iter()
            .map(|(hour, (trades, pnl, wins, losses))| {
                let win_rate = if wins + losses > 0 {
                    (wins as f64) / ((wins + losses) as f64) * 100.0
                } else {
                    0.0
                };

                TimeSlotPerformance {
                    hour,
                    trades,
                    pnl,
                    win_rate,
                }
            })
            .collect();

        slots.sort_by_key(|s| s.hour);
        slots
    }

    fn analyze_hourly_performance(summaries: &[DailySummary]) -> (Option<(u32, Decimal)>, Option<(u32, Decimal)>) {
        let mut hourly_totals: HashMap<u32, Decimal> = HashMap::new();

        for summary in summaries {
            for slot in &summary.time_slot_performance {
                *hourly_totals.entry(slot.hour).or_insert(Decimal::ZERO) += slot.pnl;
            }
        }

        let most_profitable = hourly_totals
            .iter()
            .max_by_key(|(_, pnl)| **pnl)
            .map(|(hour, pnl)| (*hour, *pnl));

        let least_profitable = hourly_totals
            .iter()
            .min_by_key(|(_, pnl)| **pnl)
            .map(|(hour, pnl)| (*hour, *pnl));

        (most_profitable, least_profitable)
    }

    fn calculate_weekly_summaries(daily_summaries: &[DailySummary]) -> Vec<WeeklySummary> {
        if daily_summaries.is_empty() {
            return Vec::new();
        }

        let mut weekly_summaries = Vec::new();
        let mut week_groups: HashMap<(i32, u32), Vec<DailySummary>> = HashMap::new();

        // Group daily summaries by ISO week
        for daily in daily_summaries {
            let iso_week = daily.date.iso_week();
            let week_key = (iso_week.year(), iso_week.week());
            week_groups.entry(week_key)
                .or_insert_with(Vec::new)
                .push(daily.clone());
        }

        // Create weekly summaries
        for ((year, week_num), mut daily_in_week) in week_groups {
            // Sort by date
            daily_in_week.sort_by_key(|d| d.date);

            let start_date = daily_in_week.first().unwrap().date;
            let _end_date = daily_in_week.last().unwrap().date;

            // Calculate week start and end dates (Monday to Sunday)
            let naive_start = start_date.date_naive();
            let days_from_monday = match naive_start.weekday() {
                Weekday::Mon => 0,
                Weekday::Tue => 1,
                Weekday::Wed => 2,
                Weekday::Thu => 3,
                Weekday::Fri => 4,
                Weekday::Sat => 5,
                Weekday::Sun => 6,
            };

            let week_start = naive_start - chrono::Duration::days(days_from_monday as i64);
            let week_end = week_start + chrono::Duration::days(6);

            let week_start_utc = DateTime::<Utc>::from_naive_utc_and_offset(
                week_start.and_hms_opt(0, 0, 0).unwrap(),
                Utc
            );
            let week_end_utc = DateTime::<Utc>::from_naive_utc_and_offset(
                week_end.and_hms_opt(23, 59, 59).unwrap(),
                Utc
            );

            let mut weekly = WeeklySummary::new(week_num, year, week_start_utc, week_end_utc);
            weekly.update_from_daily_summaries(daily_in_week);
            weekly_summaries.push(weekly);
        }

        // Sort by week start date
        weekly_summaries.sort_by_key(|w| w.start_date);
        weekly_summaries
    }
}