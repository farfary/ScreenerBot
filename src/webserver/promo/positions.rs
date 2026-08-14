//! Promo generators for the positions list and positions stats.

use chrono::{Duration, Utc};

use crate::webserver::routes::positions::types::{PositionResponse, PositionsStatsResponse};

use super::aggregates::{self, closed_exit_offset_hours};
use super::data::*;

/// Generate promo positions list (open, closed, archived, or the working set).
///
/// Archived positions are served ONLY for `status=archived`, mirroring the real
/// route: `all` deliberately excludes them so they appear in one tab only.
pub fn get_promo_positions(status: Option<&str>) -> Vec<PositionResponse> {
    let now = Utc::now();
    let mut positions = Vec::new();
    let mut id_counter: i64 = 1;

    let include_open = status.is_none() || status == Some("open") || status == Some("all");
    let include_closed = status.is_none() || status == Some("closed") || status == Some("all");
    let include_archived = status == Some("archived");

    if include_open {
        for (symbol, name, mint, logo, entry, current, size, hold_min) in PROMO_OPEN_TOKENS.iter() {
            let entry_time = now - Duration::minutes(*hold_min);
            let pnl_pct = (current - entry) / entry * 100.0;
            let unrealized_pnl = (current - entry) / entry * size;

            positions.push(PositionResponse {
                id: Some(id_counter),
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                name: name.to_string(),
                logo_url: Some(logo.to_string()),
                entry_price: *entry,
                entry_time: entry_time.timestamp(),
                exit_price: None,
                exit_time: None,
                position_type: "long".to_owned(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: current * 1.05,
                price_lowest: entry * 0.95,
                entry_transaction_signature: Some(format!("promo_entry_sig_{id_counter}")),
                exit_transaction_signature: None,
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: None,
                sol_received: None,
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_owned()),
                transaction_entry_verified: true,
                transaction_exit_verified: false,
                entry_fee_lamports: Some(5000),
                exit_fee_lamports: None,
                current_price: Some(*current),
                current_price_updated: Some(now.timestamp()),
                phantom_confirmations: 0,
                synthetic_exit: false,
                closed_reason: None,
                pnl: None,
                pnl_percent: None,
                unrealized_pnl: Some(unrealized_pnl),
                unrealized_pnl_percent: Some(pnl_pct),
                dca_count: 0,
                average_entry_price: *entry,
                partial_exit_count: 0,
                average_exit_price: None,
                remaining_token_amount: Some((size / entry * 1e9) as u64),
                total_exited_amount: 0,
                token_decimals: Some(9),
                archived: false,
                archived_at: None,
                origin: crate::positions::PositionOrigin::Auto { strategy_id: None },
                management: crate::positions::PositionManagement::AutoTrader,
            });
            id_counter += 1;
        }
    }

    if include_closed {
        for (i, (symbol, name, mint, logo, entry, exit, size, reason)) in
            PROMO_CLOSED_TOKENS.iter().enumerate()
        {
            // Exit schedule + hold time MUST match aggregates so the list, the period
            // stats and the calendar all reconcile.
            let exit_time = now - Duration::hours(closed_exit_offset_hours(i));
            let hold_minutes = 90 + (i as i64 % 6) * 45;
            let entry_time = exit_time - Duration::minutes(hold_minutes);
            let pnl = (exit - entry) / entry * size;
            let pnl_pct = (exit - entry) / entry * 100.0;

            positions.push(PositionResponse {
                id: Some(id_counter),
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                name: name.to_string(),
                logo_url: Some(logo.to_string()),
                entry_price: *entry,
                entry_time: entry_time.timestamp(),
                exit_price: Some(*exit),
                exit_time: Some(exit_time.timestamp()),
                position_type: "long".to_owned(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: exit.max(*entry) * 1.02,
                price_lowest: exit.min(*entry) * 0.97,
                entry_transaction_signature: Some(format!("promo_entry_sig_{id_counter}")),
                exit_transaction_signature: Some(format!("promo_exit_sig_{id_counter}")),
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: Some(*exit),
                sol_received: Some(size + pnl),
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_owned()),
                transaction_entry_verified: true,
                transaction_exit_verified: true,
                entry_fee_lamports: Some(5000),
                exit_fee_lamports: Some(5000),
                current_price: None,
                current_price_updated: None,
                phantom_confirmations: 0,
                synthetic_exit: false,
                closed_reason: Some(reason.to_string()),
                pnl: Some(pnl),
                pnl_percent: Some(pnl_pct),
                unrealized_pnl: None,
                unrealized_pnl_percent: None,
                dca_count: 0,
                average_entry_price: *entry,
                partial_exit_count: 0,
                average_exit_price: Some(*exit),
                remaining_token_amount: None,
                total_exited_amount: (size / entry * 1e9) as u64,
                token_decimals: Some(9),
                archived: false,
                archived_at: None,
                origin: crate::positions::PositionOrigin::Auto { strategy_id: None },
                management: crate::positions::PositionManagement::AutoTrader,
            });
            id_counter += 1;
        }
    }

    if include_archived {
        // Archived trades are older than every closed one, so their retirement
        // reads as history rather than as something the trader just did.
        for (i, (symbol, name, mint, entry, exit, size, reason)) in
            PROMO_ARCHIVED_TOKENS.iter().enumerate()
        {
            let exit_time = now - Duration::days(9 + i as i64 * 3);
            let entry_time = exit_time - Duration::minutes(120 + (i as i64 % 4) * 60);
            let archived_at = exit_time + Duration::hours(6);
            let pnl = (exit - entry) / entry * size;
            let pnl_pct = (exit - entry) / entry * 100.0;
            let token_amount = (size / entry * 1e9) as u64;

            positions.push(PositionResponse {
                id: Some(id_counter),
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                name: name.to_string(),
                logo_url: None,
                entry_price: *entry,
                entry_time: entry_time.timestamp(),
                exit_price: Some(*exit),
                exit_time: Some(exit_time.timestamp()),
                position_type: "long".to_owned(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: exit.max(*entry) * 1.02,
                price_lowest: exit.min(*entry) * 0.97,
                entry_transaction_signature: Some(format!("promo_entry_sig_{id_counter}")),
                exit_transaction_signature: Some(format!("promo_exit_sig_{id_counter}")),
                token_amount: Some(token_amount),
                effective_entry_price: Some(*entry),
                effective_exit_price: Some(*exit),
                sol_received: Some(size + pnl),
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_owned()),
                transaction_entry_verified: true,
                transaction_exit_verified: true,
                entry_fee_lamports: Some(5000),
                exit_fee_lamports: Some(5000),
                current_price: None,
                current_price_updated: None,
                phantom_confirmations: 0,
                synthetic_exit: false,
                closed_reason: Some(reason.to_string()),
                pnl: Some(pnl),
                pnl_percent: Some(pnl_pct),
                unrealized_pnl: None,
                unrealized_pnl_percent: None,
                dca_count: 0,
                average_entry_price: *entry,
                partial_exit_count: 0,
                average_exit_price: Some(*exit),
                remaining_token_amount: None,
                total_exited_amount: token_amount,
                token_decimals: Some(9),
                archived: true,
                archived_at: Some(archived_at.timestamp()),
                origin: crate::positions::PositionOrigin::Auto { strategy_id: None },
                management: crate::positions::PositionManagement::AutoTrader,
            });
            id_counter += 1;
        }
    }

    positions
}

/// Generate promo positions stats.
pub fn get_promo_positions_stats() -> PositionsStatsResponse {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let realized = aggregates::period_over(trades.iter());

    PositionsStatsResponse {
        total: open.count + trades.len(),
        open: open.count,
        closed: trades.len(),
        total_invested_sol: open.invested_sol,
        // Realized (closed) P&L plus current unrealized on open positions.
        total_pnl: realized.net_pnl_sol + open.unrealized_pnl_sol,
    }
}
