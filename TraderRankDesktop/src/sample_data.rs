use crate::models::*;
use crate::state::{AppState, WeeklyRConfig, SymbolStats, HourlyStats};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

fn make_date(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap(),
    )
}

fn make_datetime(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> chrono::DateTime<Utc> {
    Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap(),
    )
}

pub fn generate_sample_data() -> AppState {
    // Generate 8 weeks of daily summaries (about 40 trading days)
    let mut daily_summaries = Vec::new();
    let mut all_trades = Vec::new();

    // Structured daily data: (month, day, trades, wins, pnl, gross_pnl, commission, volume, symbols)
    let daily_data: Vec<(u32, u32, u32, u32, i64, i64, i64, i64, Vec<&str>)> = vec![
        // Week 1 - Jan 27-31
        (1, 27, 24, 15, 31200, 34500, 3300, 485000, vec!["AAPL", "NVDA", "SPY"]),
        (1, 28, 18, 10, -12800, -9200, 3600, 362000, vec!["TSLA", "AMD", "SPY"]),
        (1, 29, 22, 14, 42500, 45800, 3300, 440000, vec!["NVDA", "AAPL", "META"]),
        (1, 30, 20, 13, 28700, 31700, 3000, 398000, vec!["SPY", "AMD", "TSLA"]),
        (1, 31, 26, 16, -8500, -5200, 3300, 520000, vec!["AAPL", "NVDA", "AMD", "SPY"]),
        // Week 2 - Feb 3-7
        (2, 3, 28, 19, 56800, 60200, 3400, 558000, vec!["NVDA", "TSLA", "META"]),
        (2, 4, 22, 13, 18900, 22200, 3300, 445000, vec!["AAPL", "SPY", "AMD"]),
        (2, 5, 30, 18, -24600, -20800, 3800, 602000, vec!["TSLA", "NVDA", "SPY", "META"]),
        (2, 6, 20, 14, 38200, 41200, 3000, 395000, vec!["AMD", "AAPL", "NVDA"]),
        (2, 7, 24, 15, 22400, 25600, 3200, 480000, vec!["SPY", "META", "TSLA"]),
        // Week 3 - Feb 10-14
        (2, 10, 26, 17, 44100, 47500, 3400, 525000, vec!["NVDA", "AAPL", "SPY"]),
        (2, 11, 18, 9, -32500, -28800, 3700, 365000, vec!["TSLA", "AMD"]),
        (2, 12, 22, 14, 28300, 31600, 3300, 438000, vec!["META", "SPY", "NVDA"]),
        (2, 13, 20, 12, 15600, 18600, 3000, 400000, vec!["AAPL", "AMD", "TSLA"]),
        (2, 14, 24, 16, 36800, 40000, 3200, 478000, vec!["NVDA", "SPY", "META"]),
        // Week 4 - Feb 17-21 (drawdown week)
        (2, 18, 32, 14, -45200, -41000, 4200, 640000, vec!["TSLA", "NVDA", "AMD", "SPY"]),
        (2, 19, 28, 12, -28900, -24600, 4300, 560000, vec!["AAPL", "TSLA", "META"]),
        (2, 20, 22, 11, -18600, -15200, 3400, 438000, vec!["SPY", "AMD", "NVDA"]),
        (2, 21, 24, 15, 22400, 25800, 3400, 482000, vec!["NVDA", "AAPL", "META"]),
        // Week 5 - Feb 24-28 (recovery)
        (2, 24, 20, 13, 34500, 37500, 3000, 400000, vec!["NVDA", "SPY", "AAPL"]),
        (2, 25, 24, 16, 41200, 44600, 3400, 485000, vec!["TSLA", "META", "AMD"]),
        (2, 26, 18, 11, 12800, 15600, 2800, 358000, vec!["AAPL", "SPY"]),
        (2, 27, 26, 17, 48300, 52000, 3700, 522000, vec!["NVDA", "AMD", "META", "SPY"]),
        (2, 28, 22, 14, 28600, 31800, 3200, 440000, vec!["TSLA", "AAPL", "NVDA"]),
        // Week 6 - Mar 3-7
        (3, 3, 24, 15, 32100, 35400, 3300, 480000, vec!["SPY", "NVDA", "META"]),
        (3, 4, 20, 13, 25800, 28800, 3000, 398000, vec!["AAPL", "AMD", "TSLA"]),
        (3, 5, 28, 17, -15600, -11800, 3800, 558000, vec!["TSLA", "NVDA", "SPY", "META"]),
        (3, 6, 22, 15, 38900, 42200, 3300, 442000, vec!["NVDA", "AAPL", "AMD"]),
        (3, 7, 26, 17, 42800, 46200, 3400, 520000, vec!["SPY", "META", "TSLA", "NVDA"]),
        // Week 7 - Mar 10-14
        (3, 10, 22, 14, 29400, 32600, 3200, 438000, vec!["AAPL", "NVDA", "SPY"]),
        (3, 11, 24, 16, 36200, 39600, 3400, 482000, vec!["AMD", "TSLA", "META"]),
        (3, 12, 20, 11, -8200, -4800, 3400, 402000, vec!["SPY", "NVDA"]),
        (3, 13, 26, 18, 52400, 56000, 3600, 528000, vec!["NVDA", "AAPL", "META", "AMD"]),
        (3, 14, 22, 14, 28500, 31800, 3300, 440000, vec!["TSLA", "SPY", "NVDA"]),
        // Week 8 - Mar 17-21
        (3, 17, 24, 16, 38200, 41600, 3400, 485000, vec!["NVDA", "META", "SPY"]),
        (3, 18, 20, 12, 18600, 21600, 3000, 400000, vec!["AAPL", "AMD", "TSLA"]),
        (3, 19, 28, 18, 45800, 49400, 3600, 562000, vec!["SPY", "NVDA", "META", "AMD"]),
        (3, 20, 22, 13, 21400, 24600, 3200, 442000, vec!["TSLA", "AAPL", "NVDA"]),
        (3, 21, 26, 17, 35600, 39200, 3600, 520000, vec!["NVDA", "SPY", "META", "AAPL"]),
    ];

    // Hourly distribution template (market hours 9-16)
    let hourly_weights: &[(u32, f64)] = &[
        (9, 0.25), (10, 0.20), (11, 0.12), (12, 0.08),
        (13, 0.08), (14, 0.10), (15, 0.17),
    ];

    for (month, day, trades, wins, pnl_cents, gross_cents, comm_cents, vol_cents, syms) in &daily_data {
        let date = make_date(2026, *month, *day);
        let losses = trades - wins;
        let pnl = Decimal::new(*pnl_cents, 2);
        let gross = Decimal::new(*gross_cents, 2);
        let comm = Decimal::new(*comm_cents, 2);
        let volume = Decimal::new(*vol_cents, 2);

        let win_rate = if *trades > 0 {
            (*wins as f64) / (*trades as f64) * 100.0
        } else {
            0.0
        };

        // Compute avg win/loss from totals
        let (avg_win, avg_loss) = if pnl > Decimal::ZERO {
            let total_win_amount = pnl + comm;
            let avg_w = if *wins > 0 {
                total_win_amount * dec!(0.7) / Decimal::from(*wins)
            } else {
                Decimal::ZERO
            };
            let avg_l = if losses > 0 {
                -(total_win_amount * dec!(0.3)) / Decimal::from(losses)
            } else {
                Decimal::ZERO
            };
            (avg_w, avg_l)
        } else {
            let total_loss_amount = pnl.abs() + comm;
            let avg_l = if losses > 0 {
                -(total_loss_amount * dec!(0.6)) / Decimal::from(losses)
            } else {
                Decimal::ZERO
            };
            let avg_w = if *wins > 0 {
                (total_loss_amount * dec!(0.4)) / Decimal::from(*wins)
            } else {
                Decimal::ZERO
            };
            (avg_w, avg_l)
        };

        let largest_win = avg_win * dec!(2.5);
        let largest_loss = avg_loss * dec!(2.2);

        // Generate hourly performance
        let time_slots: Vec<TimeSlotPerformance> = hourly_weights
            .iter()
            .map(|(hour, weight)| {
                let h_trades = ((*trades as f64) * weight).round() as u32;
                let h_pnl = pnl * Decimal::try_from(*weight).unwrap_or(Decimal::ZERO);
                TimeSlotPerformance {
                    hour: *hour,
                    trades: h_trades.max(1),
                    pnl: h_pnl,
                    win_rate: win_rate + (if *hour == 9 { 5.0 } else if *hour == 15 { 3.0 } else { -2.0 }),
                }
            })
            .collect();

        // Generate sample trades for this day
        for (i, sym) in syms.iter().enumerate() {
            let trades_per_sym = (*trades as usize / syms.len()).max(1);
            for j in 0..trades_per_sym {
                let hour = 9 + (j % 7) as u32;
                let is_win = (i + j) % 3 != 0; // roughly 66% wins
                let trade_pnl = if is_win { avg_win } else { avg_loss };
                let price = match *sym {
                    "AAPL" => dec!(185.50),
                    "NVDA" => dec!(142.30),
                    "TSLA" => dec!(248.75),
                    "AMD" => dec!(168.20),
                    "SPY" => dec!(512.40),
                    "META" => dec!(525.80),
                    _ => dec!(100.00),
                };
                let qty = dec!(100);
                let trade_comm = Decimal::new(150, 2);
                let time = make_datetime(2026, *month, *day, hour, (j as u32 * 7) % 60, 0);

                all_trades.push(Trade {
                    symbol: sym.to_string(),
                    side: if j % 2 == 0 { Side::Buy } else { Side::Sell },
                    quantity: qty,
                    fill_price: price,
                    time,
                    net_amount: trade_pnl,
                    commission: trade_comm,
                });
            }
        }

        daily_summaries.push(DailySummary {
            date,
            total_trades: *trades,
            winning_trades: *wins,
            losing_trades: losses,
            realized_pnl: pnl,
            gross_pnl: gross,
            total_commission: comm,
            total_volume: volume,
            win_rate,
            avg_win,
            avg_loss,
            largest_win,
            largest_loss,
            symbols_traded: syms.iter().map(|s| s.to_string()).collect(),
            time_slot_performance: time_slots,
        });
    }

    // Build weekly summaries
    let weekly_summaries = build_weekly_summaries(&daily_summaries);

    // Build monthly summaries
    let monthly_summaries = build_monthly_summaries(&daily_summaries);

    // Compute overall stats
    let total_pnl: Decimal = daily_summaries.iter().map(|d| d.realized_pnl).sum();
    let total_trades: u32 = daily_summaries.iter().map(|d| d.total_trades).sum();
    let total_wins: u32 = daily_summaries.iter().map(|d| d.winning_trades).sum();
    let total_losses: u32 = daily_summaries.iter().map(|d| d.losing_trades).sum();
    let total_commission: Decimal = daily_summaries.iter().map(|d| d.total_commission).sum();
    let total_gross: Decimal = daily_summaries.iter().map(|d| d.gross_pnl).sum();

    let overall_win_rate = if total_trades > 0 {
        (total_wins as f64) / (total_trades as f64) * 100.0
    } else {
        0.0
    };

    // Compute avg win/loss overall
    let avg_win = if total_wins > 0 {
        let sum_wins: Decimal = daily_summaries
            .iter()
            .map(|d| d.avg_win * Decimal::from(d.winning_trades))
            .sum();
        sum_wins / Decimal::from(total_wins)
    } else {
        Decimal::ZERO
    };

    let avg_loss = if total_losses > 0 {
        let sum_losses: Decimal = daily_summaries
            .iter()
            .map(|d| d.avg_loss * Decimal::from(d.losing_trades))
            .sum();
        sum_losses / Decimal::from(total_losses)
    } else {
        Decimal::ZERO
    };

    // Expectancy = (win% * avg_win) + (loss% * avg_loss)
    // Derive win% from Decimal directly — never round-trip through f64
    let win_pct = if total_trades > 0 {
        Decimal::from(total_wins) / Decimal::from(total_trades)
    } else {
        Decimal::ZERO
    };
    let loss_pct = Decimal::ONE - win_pct;
    let expectancy = (win_pct * avg_win) + (loss_pct * avg_loss);

    // Profit factor — trade-level: sum of winning trade P&L / sum of losing trade P&L (abs)
    let gross_wins: Decimal = all_trades
        .iter()
        .filter(|t| t.net_amount > Decimal::ZERO)
        .map(|t| t.net_amount)
        .sum();
    let gross_losses: Decimal = all_trades
        .iter()
        .filter(|t| t.net_amount < Decimal::ZERO)
        .map(|t| t.net_amount.abs())
        .sum();
    let profit_factor = if gross_losses > Decimal::ZERO {
        Some(gross_wins / gross_losses)
    } else {
        None
    };

    // Max drawdown
    let mut peak = Decimal::ZERO;
    let mut cumulative = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for d in &daily_summaries {
        cumulative += d.realized_pnl;
        if cumulative > peak {
            peak = cumulative;
        }
        let dd = peak - cumulative;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    // Sharpe ratio (annualized from daily, sample variance N-1)
    let daily_returns: Vec<f64> = daily_summaries
        .iter()
        .map(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d.realized_pnl).unwrap_or(0.0))
        .collect();
    let n = daily_returns.len() as f64;
    let mean_return = daily_returns.iter().sum::<f64>() / n;
    let sharpe = if n > 1.0 {
        let variance = daily_returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        if std_dev > 0.0 {
            (mean_return / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Payoff ratio
    let payoff_ratio = if avg_loss != Decimal::ZERO {
        Some(avg_win / avg_loss.abs())
    } else {
        None
    };

    // Streaks
    let mut current_streak: i32 = 0;
    let mut max_win_streak: u32 = 0;
    let mut max_loss_streak: u32 = 0;
    let mut cur_win: u32 = 0;
    let mut cur_loss: u32 = 0;
    for d in &daily_summaries {
        if d.realized_pnl > Decimal::ZERO {
            cur_win += 1;
            cur_loss = 0;
            current_streak = cur_win as i32;
            if cur_win > max_win_streak {
                max_win_streak = cur_win;
            }
        } else {
            cur_loss += 1;
            cur_win = 0;
            current_streak = -(cur_loss as i32);
            if cur_loss > max_loss_streak {
                max_loss_streak = cur_loss;
            }
        }
    }

    // Symbol stats
    let mut sym_map: HashMap<String, (Decimal, u32, u32, u32)> = HashMap::new(); // pnl, trades, wins, losses
    for d in &daily_summaries {
        let per_sym_pnl = d.realized_pnl / Decimal::from(d.symbols_traded.len().max(1) as u32);
        let per_sym_trades = d.total_trades / d.symbols_traded.len().max(1) as u32;
        let per_sym_wins = d.winning_trades / d.symbols_traded.len().max(1) as u32;
        for sym in &d.symbols_traded {
            let entry = sym_map.entry(sym.clone()).or_insert((Decimal::ZERO, 0, 0, 0));
            entry.0 += per_sym_pnl;
            entry.1 += per_sym_trades;
            entry.2 += per_sym_wins;
            entry.3 += per_sym_trades.saturating_sub(per_sym_wins);
        }
    }
    let mut symbol_stats: Vec<SymbolStats> = sym_map
        .into_iter()
        .map(|(sym, (pnl, trades, wins, _losses))| {
            let wr = if trades > 0 {
                (wins as f64 / trades as f64) * 100.0
            } else {
                0.0
            };
            SymbolStats {
                symbol: sym,
                total_pnl: pnl,
                trade_count: trades,
                win_rate: wr,
            }
        })
        .collect();
    symbol_stats.sort_by(|a, b| b.total_pnl.cmp(&a.total_pnl));

    // Hourly stats
    let mut hourly_map: HashMap<u32, (Decimal, u32, f64)> = HashMap::new();
    for d in &daily_summaries {
        for ts in &d.time_slot_performance {
            let entry = hourly_map.entry(ts.hour).or_insert((Decimal::ZERO, 0, 0.0));
            entry.0 += ts.pnl;
            entry.1 += ts.trades;
            entry.2 += ts.win_rate;
        }
    }
    let day_count = daily_summaries.len() as f64;
    let mut hourly_stats: Vec<HourlyStats> = hourly_map
        .into_iter()
        .map(|(hour, (pnl, trades, wr_sum))| HourlyStats {
            hour,
            total_pnl: pnl,
            trade_count: trades,
            avg_win_rate: wr_sum / day_count,
        })
        .collect();
    hourly_stats.sort_by_key(|h| h.hour);

    // Weekly R configs (default R=$100) — keyed by ISO Monday
    let r_configs: Vec<WeeklyRConfig> = weekly_summaries
        .iter()
        .map(|w| {
            let date = w.start_date.date_naive();
            let days_from_mon = date.weekday().num_days_from_monday();
            let monday = date - chrono::Duration::days(days_from_mon as i64);
            WeeklyRConfig {
                week_start: monday,
                r_value: dec!(100),
            }
        })
        .collect();

    // Daily P&L for equity chart
    let daily_pnls: Vec<(String, Decimal)> = daily_summaries
        .iter()
        .map(|d| {
            (d.date.format("%m/%d").to_string(), d.realized_pnl)
        })
        .collect();

    AppState {
        daily_summaries,
        weekly_summaries,
        monthly_summaries,
        trades: all_trades,
        matched_trades: Vec::new(),
        total_pnl,
        total_trades,
        total_wins,
        total_losses,
        total_commission,
        total_gross,
        overall_win_rate,
        avg_win,
        avg_loss,
        expectancy,
        profit_factor,
        sharpe_ratio: sharpe,
        max_drawdown: max_dd,
        payoff_ratio,
        current_streak,
        max_win_streak,
        max_loss_streak,
        symbol_stats,
        hourly_stats,
        daily_pnls,
        r_configs,
        exclusions: std::collections::HashMap::new(),
    }
}

fn build_weekly_summaries(daily: &[DailySummary]) -> Vec<WeeklySummary> {
    let mut weeks: Vec<Vec<&DailySummary>> = Vec::new();
    let mut current_week: Vec<&DailySummary> = Vec::new();
    let mut current_week_num: Option<u32> = None;

    for d in daily {
        let wk = d.date.iso_week().week();
        if current_week_num != Some(wk) {
            if !current_week.is_empty() {
                weeks.push(current_week);
                current_week = Vec::new();
            }
            current_week_num = Some(wk);
        }
        current_week.push(d);
    }
    if !current_week.is_empty() {
        weeks.push(current_week);
    }

    weeks
        .into_iter()
        .map(|days| {
            let first = &days[0];
            let last = &days[days.len() - 1];
            let wk = first.date.iso_week().week();
            let yr = first.date.year();

            let total_trades: u32 = days.iter().map(|d| d.total_trades).sum();
            let winning_trades: u32 = days.iter().map(|d| d.winning_trades).sum();
            let losing_trades: u32 = days.iter().map(|d| d.losing_trades).sum();
            let realized_pnl: Decimal = days.iter().map(|d| d.realized_pnl).sum();
            let gross_pnl: Decimal = days.iter().map(|d| d.gross_pnl).sum();
            let total_commission: Decimal = days.iter().map(|d| d.total_commission).sum();
            let total_volume: Decimal = days.iter().map(|d| d.total_volume).sum();
            let win_rate = if total_trades > 0 {
                (winning_trades as f64) / (total_trades as f64) * 100.0
            } else {
                0.0
            };

            let total_win_amt: Decimal = days
                .iter()
                .map(|d| d.avg_win * Decimal::from(d.winning_trades))
                .sum();
            let total_loss_amt: Decimal = days
                .iter()
                .map(|d| d.avg_loss * Decimal::from(d.losing_trades))
                .sum();
            let avg_win = if winning_trades > 0 {
                total_win_amt / Decimal::from(winning_trades)
            } else {
                Decimal::ZERO
            };
            let avg_loss = if losing_trades > 0 {
                total_loss_amt / Decimal::from(losing_trades)
            } else {
                Decimal::ZERO
            };

            let mut all_syms = std::collections::HashSet::new();
            for d in &days {
                for s in &d.symbols_traded {
                    all_syms.insert(s.clone());
                }
            }

            let best_day = days.iter().max_by_key(|d| d.realized_pnl).map(|d| (d.date, d.realized_pnl));
            let worst_day = days.iter().min_by_key(|d| d.realized_pnl).map(|d| (d.date, d.realized_pnl));
            let trading_days = days.len() as u32;
            let profitable_days = days.iter().filter(|d| d.realized_pnl > Decimal::ZERO).count() as u32;
            let avg_daily_pnl = if trading_days > 0 {
                realized_pnl / Decimal::from(trading_days)
            } else {
                Decimal::ZERO
            };

            let largest_win = days.iter().map(|d| d.largest_win).max().unwrap_or(Decimal::ZERO);
            let largest_loss = days.iter().map(|d| d.largest_loss).min().unwrap_or(Decimal::ZERO);

            WeeklySummary {
                week_number: wk,
                year: yr,
                start_date: first.date,
                end_date: last.date,
                total_trades,
                winning_trades,
                losing_trades,
                realized_pnl,
                gross_pnl,
                total_commission,
                total_volume,
                win_rate,
                avg_win,
                avg_loss,
                largest_win,
                largest_loss,
                best_day,
                worst_day,
                trading_days,
                profitable_days,
                avg_daily_pnl,
                symbols_traded: all_syms.into_iter().collect(),
                daily_summaries: days.into_iter().cloned().collect(),
            }
        })
        .collect()
}

fn build_monthly_summaries(daily: &[DailySummary]) -> Vec<MonthlySummary> {
    let mut month_map: std::collections::BTreeMap<(i32, u32), Vec<&DailySummary>> =
        std::collections::BTreeMap::new();

    for d in daily {
        let key = (d.date.year(), d.date.month());
        month_map.entry(key).or_default().push(d);
    }

    month_map
        .into_iter()
        .map(|((year, month), days)| {
            let month_name = match month {
                1 => "January", 2 => "February", 3 => "March", 4 => "April",
                5 => "May", 6 => "June", 7 => "July", 8 => "August",
                9 => "September", 10 => "October", 11 => "November", 12 => "December",
                _ => "Unknown",
            }
            .to_string();

            let total_trades: u32 = days.iter().map(|d| d.total_trades).sum();
            let winning_trades: u32 = days.iter().map(|d| d.winning_trades).sum();
            let losing_trades: u32 = days.iter().map(|d| d.losing_trades).sum();
            let realized_pnl: Decimal = days.iter().map(|d| d.realized_pnl).sum();
            let gross_pnl: Decimal = days.iter().map(|d| d.gross_pnl).sum();
            let total_commission: Decimal = days.iter().map(|d| d.total_commission).sum();
            let total_volume: Decimal = days.iter().map(|d| d.total_volume).sum();
            let win_rate = if total_trades > 0 {
                (winning_trades as f64) / (total_trades as f64) * 100.0
            } else {
                0.0
            };
            let trading_days = days.len() as u32;
            let profitable_days = days.iter().filter(|d| d.realized_pnl > Decimal::ZERO).count() as u32;
            let avg_daily_pnl = if trading_days > 0 {
                realized_pnl / Decimal::from(trading_days)
            } else {
                Decimal::ZERO
            };
            let best_day = days.iter().max_by_key(|d| d.realized_pnl).map(|d| (d.date, d.realized_pnl));
            let worst_day = days.iter().min_by_key(|d| d.realized_pnl).map(|d| (d.date, d.realized_pnl));

            MonthlySummary {
                year,
                month,
                month_name,
                total_trades,
                winning_trades,
                losing_trades,
                realized_pnl,
                gross_pnl,
                total_commission,
                total_volume,
                win_rate,
                trading_days,
                profitable_days,
                avg_daily_pnl,
                best_day,
                worst_day,
            }
        })
        .collect()
}
