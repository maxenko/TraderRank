use crate::models::{MatchedTrade, Side, Trade};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Match raw trade executions into round-trip trades with P&L.
///
/// Position tracking algorithm — global per symbol, FIFO across the entire dataset
/// (no per-day reset, so overnight/multi-day round trips match correctly):
/// - Per symbol, process trades chronologically across all days
/// - Buy fills build a long position with weighted-average cost basis
/// - Sell fills close the long position and produce a MatchedTrade (side="Long")
/// - Sell fills at position<=0 open/add to a short position
/// - Buy fills close a short position and produce a MatchedTrade (side="Short")
/// - Positions left open at end of dataset are skipped (unrealized — not matched)
///
/// Returns matched trades sorted by exit_time descending (most recent first).
pub fn match_trades(trades: &[Trade]) -> Vec<MatchedTrade> {
    // Group by symbol across the ENTIRE dataset (no day boundary).
    // Same FIFO algorithm — just don't reset position at midnight.
    let mut by_symbol: HashMap<&str, Vec<&Trade>> = HashMap::new();
    for trade in trades {
        by_symbol.entry(trade.symbol.as_str()).or_default().push(trade);
    }

    let mut all_matched: Vec<MatchedTrade> = Vec::new();

    for (symbol, mut symbol_trades) in by_symbol {
        // Need at least 2 trades to form a round trip
        if symbol_trades.len() < 2 {
            continue;
        }

        // Sort by time for chronological processing (stable sort handles ties gracefully)
        symbol_trades.sort_by_key(|t| t.time);

        // Track position state -- mirrors analytics.rs exactly
        // position > 0 = long, position < 0 = short
        let mut position = Decimal::ZERO;
        let mut cost_basis = Decimal::ZERO;
        let mut opening_commission = Decimal::ZERO;

        // Track entry metadata for the current position
        let mut entry_fills: u32 = 0;
        let mut first_entry_time = symbol_trades[0].time;

        for trade in &symbol_trades {
            match trade.side {
                Side::Buy => {
                    if position < Decimal::ZERO {
                        // Closing short position (buying to cover)
                        let abs_pos = position.abs();
                        let qty_to_close = trade.quantity.min(abs_pos);
                        if qty_to_close > Decimal::ZERO {
                            // Short P&L = (entry_price - exit_price) * qty
                            let gross_pnl = (cost_basis - trade.fill_price) * qty_to_close;

                            let entry_comm =
                                opening_commission * qty_to_close / abs_pos;
                            let exit_comm =
                                trade.commission * qty_to_close / trade.quantity;
                            let total_comm = entry_comm + exit_comm;

                            let net_pnl = gross_pnl - total_comm;

                            all_matched.push(MatchedTrade {
                                symbol: symbol.to_string(),
                                entry_time: first_entry_time,
                                exit_time: trade.time,
                                side: "Short".to_string(),
                                quantity: qty_to_close,
                                entry_price: cost_basis,
                                exit_price: trade.fill_price,
                                gross_pnl,
                                commission: total_comm,
                                net_pnl,
                                entry_fills,
                                exit_fills: 1,
                            });
                        }

                        // Update opening commission proportionally
                        let remaining_ratio = (abs_pos - qty_to_close) / abs_pos;
                        opening_commission *= remaining_ratio;
                        position += qty_to_close; // moves toward zero

                        let qty_remaining = trade.quantity - qty_to_close;
                        if qty_remaining > Decimal::ZERO {
                            // Bought more than needed to close short -- open a new long
                            position = qty_remaining;
                            cost_basis = trade.fill_price;
                            opening_commission = trade.commission * qty_remaining / trade.quantity;
                            entry_fills = 1;
                            first_entry_time = trade.time;
                        } else if position == Decimal::ZERO {
                            cost_basis = Decimal::ZERO;
                            opening_commission = Decimal::ZERO;
                            entry_fills = 0;
                        }
                    } else if position > Decimal::ZERO {
                        // Adding to existing long position -- weighted average cost basis
                        let total_cost =
                            cost_basis * position + trade.fill_price * trade.quantity;
                        position += trade.quantity;
                        cost_basis = total_cost / position;
                        opening_commission += trade.commission;
                        entry_fills += 1;
                    } else {
                        // position == 0: Opening a new long position
                        position = trade.quantity;
                        cost_basis = trade.fill_price;
                        opening_commission = trade.commission;
                        entry_fills = 1;
                        first_entry_time = trade.time;
                    }
                }
                Side::Sell => {
                    if position > Decimal::ZERO {
                        // Closing long position
                        let qty_to_close = trade.quantity.min(position);
                        if qty_to_close > Decimal::ZERO {
                            // P&L = (sell_price - buy_price) * qty
                            let gross_pnl = (trade.fill_price - cost_basis) * qty_to_close;

                            let entry_comm =
                                opening_commission * qty_to_close / position;
                            let exit_comm =
                                trade.commission * qty_to_close / trade.quantity;
                            let total_comm = entry_comm + exit_comm;

                            let net_pnl = gross_pnl - total_comm;

                            all_matched.push(MatchedTrade {
                                symbol: symbol.to_string(),
                                entry_time: first_entry_time,
                                exit_time: trade.time,
                                side: "Long".to_string(),
                                quantity: qty_to_close,
                                entry_price: cost_basis,
                                exit_price: trade.fill_price,
                                gross_pnl,
                                commission: total_comm,
                                net_pnl,
                                entry_fills,
                                exit_fills: 1,
                            });
                        }

                        // Update opening commission proportionally for remaining position
                        let remaining_ratio = (position - qty_to_close) / position;
                        opening_commission *= remaining_ratio;
                        position -= qty_to_close;

                        let qty_remaining = trade.quantity - qty_to_close;
                        if qty_remaining > Decimal::ZERO {
                            // Sold more than owned -- open a new short position
                            position = -qty_remaining;
                            cost_basis = trade.fill_price;
                            opening_commission = trade.commission * qty_remaining / trade.quantity;
                            entry_fills = 1;
                            first_entry_time = trade.time;
                        } else if position == Decimal::ZERO {
                            cost_basis = Decimal::ZERO;
                            opening_commission = Decimal::ZERO;
                            entry_fills = 0;
                        }
                    } else if position < Decimal::ZERO {
                        // Adding to existing short position
                        let abs_pos = position.abs();
                        let total_cost =
                            cost_basis * abs_pos + trade.fill_price * trade.quantity;
                        position -= trade.quantity; // more negative
                        cost_basis = total_cost / position.abs();
                        opening_commission += trade.commission;
                        entry_fills += 1;
                    } else {
                        // position == 0: Opening a new short position
                        position = -trade.quantity;
                        cost_basis = trade.fill_price;
                        opening_commission = trade.commission;
                        entry_fills = 1;
                        first_entry_time = trade.time;
                    }
                }
            }
        }

        // Position left open at end of dataset: skip (unrealized — not matched)
        if position != Decimal::ZERO {
            eprintln!(
                "Warning: {} ended with open position of {} (unrealized — not matched)",
                symbol, position
            );
        }
    }

    // Sort by exit_time descending (most recent first)
    all_matched.sort_by(|a, b| b.exit_time.cmp(&a.exit_time));

    all_matched
}
