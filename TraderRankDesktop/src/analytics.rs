use crate::models::{Trade, Side, MatchedTrade, DailySummary, WeeklySummary, MonthlySummary, TradingSummary, TimeSlotPerformance};
use chrono::{DateTime, NaiveDate, Utc, Datelike, Weekday};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

pub struct TradingAnalytics;

impl TradingAnalytics {
    /// Primary entry point: takes raw trades AND the already-matched round trips.
    /// DailySummary is built by grouping matched trades by `exit_time.date_naive()`,
    /// so overnight/multi-day round trips correctly land on their close day.
    /// Raw trades are still needed for `total_volume` and `time_slot_performance`.
    pub fn analyze_trades_with_matched(
        trades: &[Trade],
        matched: &[MatchedTrade],
    ) -> TradingSummary {
        // 1. Group matched trades by exit-day (the day P&L is realized)
        let mut by_exit_day: HashMap<NaiveDate, Vec<&MatchedTrade>> = HashMap::new();
        for mt in matched {
            by_exit_day
                .entry(mt.exit_time.date_naive())
                .or_default()
                .push(mt);
        }

        // 2. Group raw trades by day for total_volume + hourly performance
        let mut raw_by_day: HashMap<NaiveDate, Vec<Trade>> = HashMap::new();
        for t in trades {
            raw_by_day
                .entry(t.time.date_naive())
                .or_default()
                .push(t.clone());
        }

        // 3. Union of all days that have either a close or any fill
        let all_days: HashSet<NaiveDate> = by_exit_day
            .keys()
            .copied()
            .chain(raw_by_day.keys().copied())
            .collect();

        let mut daily_summaries: Vec<DailySummary> = all_days
            .into_iter()
            .map(|day| {
                Self::build_daily_summary(
                    day,
                    by_exit_day.get(&day).map(|v| v.as_slice()).unwrap_or(&[]),
                    raw_by_day.get(&day).map(|v| v.as_slice()).unwrap_or(&[]),
                )
            })
            .filter(|s| s.total_trades > 0) // drop days with no closes
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

        // Calculate monthly summaries
        let monthly_summaries = Self::calculate_monthly_summaries(&daily_summaries);

        // Find best and worst months
        let best_month = monthly_summaries
            .iter()
            .max_by_key(|m| m.realized_pnl)
            .map(|m| ((m.year, m.month), m.realized_pnl));

        let worst_month = monthly_summaries
            .iter()
            .min_by_key(|m| m.realized_pnl)
            .map(|m| ((m.year, m.month), m.realized_pnl));

        TradingSummary {
            start_date: daily_summaries.first().map(|s| s.date).unwrap_or_else(|| Utc::now()),
            end_date: daily_summaries.last().map(|s| s.date).unwrap_or_else(|| Utc::now()),
            daily_summaries,
            weekly_summaries,
            monthly_summaries,
            total_pnl,
            total_volume,
            total_trades,
            overall_win_rate,
            best_day,
            worst_day,
            best_week,
            worst_week,
            best_month,
            worst_month,
            most_profitable_hour,
            least_profitable_hour,
        }
    }

    /// Build a single day's summary by aggregating the matched trades that
    /// closed on that day (`closes`) and the raw fills that occurred on that
    /// day (`raw`, used only for total_volume + hourly bars).
    fn build_daily_summary(
        day: NaiveDate,
        closes: &[&MatchedTrade],
        raw: &[Trade],
    ) -> DailySummary {
        let date_utc = DateTime::<Utc>::from_naive_utc_and_offset(
            day.and_hms_opt(0, 0, 0).unwrap(),
            Utc,
        );
        let mut summary = DailySummary::new(date_utc);

        // Volume = notional of every fill that touched this day. (Entry side of
        // an overnight trade may live on a different day; for "how active was
        // this day" we count all fills.)
        summary.total_volume = raw
            .iter()
            .map(|t| t.quantity * t.fill_price)
            .sum();

        // Realized P&L, wins, losses, commission — straight aggregation of closes.
        let mut symbols_set: HashSet<String> = HashSet::new();
        let mut winning_pnls: Vec<Decimal> = Vec::new();
        let mut losing_pnls: Vec<Decimal> = Vec::new();

        for mt in closes {
            symbols_set.insert(mt.symbol.clone());
            summary.total_commission += mt.commission;

            // Classify on GROSS P&L (matches the prior convention in
            // calculate_daily_summary). Hourly performance still classifies on
            // NET — that distinction is preserved.
            if mt.gross_pnl > Decimal::ZERO {
                summary.winning_trades += 1;
                winning_pnls.push(mt.net_pnl);
                if mt.net_pnl > summary.largest_win {
                    summary.largest_win = mt.net_pnl;
                }
            } else if mt.gross_pnl < Decimal::ZERO {
                summary.losing_trades += 1;
                losing_pnls.push(mt.net_pnl);
                if mt.net_pnl < summary.largest_loss {
                    summary.largest_loss = mt.net_pnl;
                }
            }
            summary.realized_pnl += mt.net_pnl;
        }

        summary.gross_pnl = summary.realized_pnl + summary.total_commission;
        summary.total_trades = summary.winning_trades + summary.losing_trades;
        summary.symbols_traded = symbols_set.into_iter().collect();

        if !winning_pnls.is_empty() {
            let sum: Decimal = winning_pnls.iter().sum();
            summary.avg_win = sum / Decimal::from(winning_pnls.len() as u32);
        }
        if !losing_pnls.is_empty() {
            let sum: Decimal = losing_pnls.iter().sum();
            summary.avg_loss = sum / Decimal::from(losing_pnls.len() as u32);
        }
        summary.win_rate = if summary.total_trades > 0 {
            (summary.winning_trades as f64) / (summary.total_trades as f64) * 100.0
        } else {
            0.0
        };

        // Hourly performance from raw fills — unchanged. Hourly still runs its
        // own per-day FIFO; overnight trades are not represented in the hour
        // bars (acceptable v1 limitation; documented in plan/risks).
        summary.time_slot_performance = Self::calculate_hourly_performance(raw);

        summary
    }

    fn calculate_hourly_performance(trades: &[Trade]) -> Vec<TimeSlotPerformance> {
        // Run position tracker ONCE per symbol across the full day.
        // When a closing fill occurs, attribute the P&L to the HOUR of that closing fill.
        // Supports both long (Buy->Sell) and short (Sell->Buy) round trips.

        // Group trades by symbol
        let mut trades_by_symbol: HashMap<String, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            trades_by_symbol.entry(trade.symbol.clone())
                .or_insert_with(Vec::new)
                .push(trade);
        }

        // hourly_data: hour -> (trade_count, pnl, wins, losses)
        let mut hourly_data: HashMap<u32, (u32, Decimal, u32, u32)> = HashMap::new();

        // Count all trades per hour for the trade_count field
        for trade in trades {
            let hour = trade.hour_of_day();
            hourly_data.entry(hour).or_insert((0, Decimal::ZERO, 0, 0)).0 += 1;
        }

        for (_symbol, mut symbol_trades) in trades_by_symbol {
            if symbol_trades.len() < 2 {
                continue;
            }

            symbol_trades.sort_by_key(|t| t.time);

            // position > 0 = long, position < 0 = short
            let mut position = Decimal::ZERO;
            let mut cost_basis = Decimal::ZERO;
            let mut opening_commission = Decimal::ZERO;

            for trade in &symbol_trades {
                match trade.side {
                    Side::Buy => {
                        if position < Decimal::ZERO {
                            // Closing short position
                            let abs_pos = position.abs();
                            let qty_to_close = trade.quantity.min(abs_pos);
                            if qty_to_close > Decimal::ZERO {
                                let trade_pnl = (cost_basis - trade.fill_price) * qty_to_close;
                                let trade_commission = opening_commission * qty_to_close / abs_pos
                                    + trade.commission * qty_to_close / trade.quantity;
                                let net_pnl = trade_pnl - trade_commission;

                                // Attribute to the hour of the closing fill
                                let hour = trade.hour_of_day();
                                let entry = hourly_data.entry(hour).or_insert((0, Decimal::ZERO, 0, 0));
                                entry.1 += net_pnl;
                                if net_pnl > Decimal::ZERO {
                                    entry.2 += 1;
                                } else if net_pnl < Decimal::ZERO {
                                    entry.3 += 1;
                                }
                            }

                            let remaining_ratio = (abs_pos - qty_to_close) / abs_pos;
                            opening_commission = opening_commission * remaining_ratio;
                            position += qty_to_close;

                            let qty_remaining = trade.quantity - qty_to_close;
                            if qty_remaining > Decimal::ZERO {
                                position = qty_remaining;
                                cost_basis = trade.fill_price;
                                opening_commission = trade.commission * qty_remaining / trade.quantity;
                            } else if position == Decimal::ZERO {
                                cost_basis = Decimal::ZERO;
                                opening_commission = Decimal::ZERO;
                            }
                        } else if position > Decimal::ZERO {
                            // Adding to existing long
                            let total_cost = cost_basis * position + trade.fill_price * trade.quantity;
                            position += trade.quantity;
                            cost_basis = total_cost / position;
                            opening_commission += trade.commission;
                        } else {
                            // position == 0: Opening new long
                            position = trade.quantity;
                            cost_basis = trade.fill_price;
                            opening_commission = trade.commission;
                        }
                    }
                    Side::Sell => {
                        if position > Decimal::ZERO {
                            // Closing long position
                            let qty_to_close = trade.quantity.min(position);
                            if qty_to_close > Decimal::ZERO {
                                let trade_pnl = (trade.fill_price - cost_basis) * qty_to_close;
                                let trade_commission = opening_commission * qty_to_close / position
                                    + trade.commission * qty_to_close / trade.quantity;
                                let net_pnl = trade_pnl - trade_commission;

                                // Attribute to the hour of the closing fill
                                let hour = trade.hour_of_day();
                                let entry = hourly_data.entry(hour).or_insert((0, Decimal::ZERO, 0, 0));
                                entry.1 += net_pnl;
                                if net_pnl > Decimal::ZERO {
                                    entry.2 += 1;
                                } else if net_pnl < Decimal::ZERO {
                                    entry.3 += 1;
                                }
                            }

                            let remaining_ratio = (position - qty_to_close) / position;
                            opening_commission = opening_commission * remaining_ratio;
                            position -= qty_to_close;

                            let qty_remaining = trade.quantity - qty_to_close;
                            if qty_remaining > Decimal::ZERO {
                                position = -qty_remaining;
                                cost_basis = trade.fill_price;
                                opening_commission = trade.commission * qty_remaining / trade.quantity;
                            } else if position == Decimal::ZERO {
                                cost_basis = Decimal::ZERO;
                                opening_commission = Decimal::ZERO;
                            }
                        } else if position < Decimal::ZERO {
                            // Adding to existing short
                            let abs_pos = position.abs();
                            let total_cost = cost_basis * abs_pos + trade.fill_price * trade.quantity;
                            position -= trade.quantity;
                            cost_basis = total_cost / position.abs();
                            opening_commission += trade.commission;
                        } else {
                            // position == 0: Opening new short
                            position = -trade.quantity;
                            cost_basis = trade.fill_price;
                            opening_commission = trade.commission;
                        }
                    }
                }
            }
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

    /// Public: recompute weekly summaries from a (possibly filtered) set of daily summaries
    pub fn calculate_weekly_from_daily(daily_summaries: &[DailySummary]) -> Vec<WeeklySummary> {
        Self::calculate_weekly_summaries(daily_summaries)
    }

    /// Public: recompute monthly summaries from a (possibly filtered) set of daily summaries
    pub fn calculate_monthly_from_daily(daily_summaries: &[DailySummary]) -> Vec<MonthlySummary> {
        Self::calculate_monthly_summaries(daily_summaries)
    }

    /// Regenerate monthly summaries from daily summaries (for backward compatibility with cached data)
    #[allow(dead_code)]
    pub fn regenerate_monthly_summaries(daily_summaries: &[DailySummary]) -> Vec<MonthlySummary> {
        Self::calculate_monthly_summaries(daily_summaries)
    }

    fn calculate_monthly_summaries(daily_summaries: &[DailySummary]) -> Vec<MonthlySummary> {
        if daily_summaries.is_empty() {
            return Vec::new();
        }

        let mut month_groups: HashMap<(i32, u32), Vec<&DailySummary>> = HashMap::new();

        // Group daily summaries by year and month
        for daily in daily_summaries {
            let year = daily.date.year();
            let month = daily.date.month();
            month_groups.entry((year, month))
                .or_insert_with(Vec::new)
                .push(daily);
        }

        let mut monthly_summaries: Vec<MonthlySummary> = month_groups
            .into_iter()
            .map(|((year, month), dailies)| {
                let mut monthly = MonthlySummary::new(year, month);
                let owned_dailies: Vec<DailySummary> = dailies.into_iter().cloned().collect();
                monthly.update_from_daily_summaries(&owned_dailies);
                monthly
            })
            .collect();

        // Sort by year and month
        monthly_summaries.sort_by_key(|m| (m.year, m.month));
        monthly_summaries
    }
}
