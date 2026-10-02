// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Agent-facing portfolio tools — position and balance queries.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::chains::adapter;
use crate::chains::solana::assets::ata::get_sol_balance;
use crate::positions;
use crate::positions::Position;

/// Remaining holding of an open position in token units and its value at the
/// current pool price. Position amounts are stored in raw base units and pool
/// prices are SOL per whole token, so both values stay `None` when the mint's
/// decimals are unknown rather than reporting an unscaled amount.
struct Holding {
    token_amount: Option<f64>,
    current_value_sol: Option<f64>,
}

async fn position_holding(position: &Position) -> Holding {
    let raw_amount = position
        .remaining_token_amount
        .or(position.token_amount)
        .unwrap_or_default();
    let token_amount = crate::tokens::get_decimals(crate::chains::active_chain(), &position.mint)
        .await
        .map(|decimals| raw_amount as f64 / 10_f64.powi(i32::from(decimals)));
    let current_value_sol = token_amount
        .zip(position.current_price)
        .map(|(amount, price)| amount * price);
    Holding {
        token_amount,
        current_value_sol,
    }
}

// ============================================================================
// GetPositionsTool - List all open positions
// ============================================================================

pub struct GetPositionsTool;

#[derive(Serialize)]
struct PositionSummary {
    position_id: i64,
    mint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    entry_price_sol: f64,
    current_price_sol: Option<f64>,
    token_amount: Option<f64>,
    cost_sol: f64,
    current_value_sol: Option<f64>,
    unrealized_pnl_sol: Option<f64>,
    unrealized_pnl_percent: Option<f64>,
    opened_at: String,
}

#[async_trait]
impl Tool for GetPositionsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_positions".to_owned(),
            description: "Get all current open trading positions with entry and current prices (SOL per token), remaining token amount, cost and current value in SOL, and unrealized P&L.".to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let positions = positions::get_open_positions().await;

        let mut summaries = Vec::new();
        for pos in positions {
            let pnl = positions::calculate_position_pnl_safe(&pos, pos.current_price).await;
            let holding = position_holding(&pos).await;

            summaries.push(PositionSummary {
                position_id: pos.id.unwrap_or_default(),
                mint: pos.mint.clone(),
                symbol: Some(pos.symbol.clone()),
                entry_price_sol: pos.average_entry_price,
                current_price_sol: pos.current_price,
                token_amount: holding.token_amount,
                cost_sol: pos.total_size_sol,
                current_value_sol: holding.current_value_sol,
                unrealized_pnl_sol: pnl.as_ref().map(|p| p.0),
                unrealized_pnl_percent: pnl.as_ref().map(|p| p.1),
                opened_at: pos.entry_time.to_rfc3339(),
            });
        }

        ToolResult::success(json!({
            "positions": summaries,
            "count": summaries.len()
        }))
    }
}

// ============================================================================
// GetPositionTool - Get specific position details
// ============================================================================

pub struct GetPositionTool;

#[derive(Deserialize)]
struct GetPositionParams {
    position_id: i64,
}

#[derive(Serialize)]
struct PositionDetails {
    position_id: i64,
    mint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    entry_price_sol: f64,
    current_price_sol: Option<f64>,
    token_amount: Option<f64>,
    cost_sol: f64,
    current_value_sol: Option<f64>,
    unrealized_pnl_sol: Option<f64>,
    unrealized_pnl_percent: Option<f64>,
    opened_at: String,
    entry_signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    partial_close_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_fees_sol: Option<f64>,
}

#[async_trait]
impl Tool for GetPositionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_position".to_owned(),
            description: "Get detailed information about a specific position by its ID."
                .to_string(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "position_id": {
                        "type": "integer",
                        "description": "The position ID to retrieve"
                    }
                },
                "required": ["position_id"]
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetPositionParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };

        let position = match positions::get_position_by_id(params.position_id).await {
            Some(p) => p,
            None => return ToolResult::error("Position not found".to_owned()),
        };

        let pnl = positions::calculate_position_pnl_safe(&position, position.current_price).await;
        let holding = position_holding(&position).await;
        let total_fees = adapter().raw_to_native(
            position.entry_fee_lamports.unwrap_or_default()
                + position.exit_fee_lamports.unwrap_or_default(),
        );

        let details = PositionDetails {
            position_id: position.id.unwrap_or_default(),
            mint: position.mint.clone(),
            symbol: Some(position.symbol.clone()),
            entry_price_sol: position.average_entry_price,
            current_price_sol: position.current_price,
            token_amount: holding.token_amount,
            cost_sol: position.total_size_sol,
            current_value_sol: holding.current_value_sol,
            unrealized_pnl_sol: pnl.as_ref().map(|p| p.0),
            unrealized_pnl_percent: pnl.as_ref().map(|p| p.1),
            opened_at: position.entry_time.to_rfc3339(),
            entry_signature: position
                .entry_transaction_signature
                .clone()
                .unwrap_or_default(),
            partial_close_count: Some(position.partial_exit_count as usize),
            total_fees_sol: Some(total_fees),
        };

        match serde_json::to_value(details) {
            Ok(v) => ToolResult::success(v),
            Err(e) => ToolResult::error(format!("Serialization error: {e}")),
        }
    }
}

// ============================================================================
// GetBalanceTool - Get wallet SOL balance
// ============================================================================

pub struct GetBalanceTool;

#[derive(Serialize)]
struct BalanceInfo {
    sol_balance: f64,
    wallet_address: String,
}

#[async_trait]
impl Tool for GetBalanceTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_balance".to_owned(),
            description: "Get the current SOL balance of the trading wallet.".to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let wallet_address = match crate::utils::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(format!("Failed to get wallet address: {e}")),
        };

        let balance = match get_sol_balance(&wallet_address).await {
            Ok(b) => b,
            Err(e) => return ToolResult::error(format!("Failed to get balance: {e}")),
        };

        let info = BalanceInfo {
            sol_balance: balance,
            wallet_address,
        };

        match serde_json::to_value(info) {
            Ok(v) => ToolResult::success(v),
            Err(e) => ToolResult::error(format!("Serialization error: {e}")),
        }
    }
}

// ============================================================================
// GetPnLTool - Get profit and loss statistics
// ============================================================================

pub struct GetPnLTool;

#[derive(Deserialize)]
struct GetPnLParams {
    #[serde(default)]
    period: Option<String>,
}

#[derive(Serialize)]
struct PnLStats {
    period: String,
    total_realized_pnl_sol: f64,
    total_unrealized_pnl_sol: f64,
    total_pnl_sol: f64,
    total_wins: usize,
    total_losses: usize,
    /// Null when no position closed in the period.
    win_rate_percent: Option<f64>,
    total_trades: usize,
    open_positions: usize,
}

#[async_trait]
impl Tool for GetPnLTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_pnl".to_owned(),
            description: "Get profit and loss statistics including total realized P&L, unrealized P&L, win rate, and trade counts.".to_owned(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {
                    "period": {
                        "type": "string",
                        "description": "Time period for P&L calculation",
                        "enum": ["today", "week", "month", "all"]
                    }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetPnLParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };

        let period = params.period.unwrap_or_else(|| "all".to_owned());

        // Calculate timeframe
        let since = match period.as_str() {
            "today" => Some(chrono::Utc::now() - chrono::Duration::days(1)),
            "week" => Some(chrono::Utc::now() - chrono::Duration::weeks(1)),
            "month" => Some(chrono::Utc::now() - chrono::Duration::days(30)),
            "all" => None,
            _ => return ToolResult::error(format!("Invalid period: {period}")),
        };

        // Get trading stats from database
        let start_time =
            since.unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap());
        let stats = match positions::get_period_trading_stats(start_time, None).await {
            Ok(s) => s,
            Err(e) => return ToolResult::error(format!("Failed to get trading stats: {e}")),
        };

        // Calculate unrealized P&L from open positions
        let open_positions = positions::get_open_positions().await;
        let mut total_unrealized = 0.0;
        for pos in open_positions.iter() {
            if let Some((pnl_sol, _pnl_pct)) =
                positions::calculate_position_pnl_safe(pos, pos.current_price).await
            {
                total_unrealized += pnl_sol;
            }
        }

        let pnl_stats = PnLStats {
            period: period.clone(),
            total_realized_pnl_sol: stats.net_pnl_sol,
            total_unrealized_pnl_sol: total_unrealized,
            total_pnl_sol: stats.net_pnl_sol + total_unrealized,
            total_wins: stats.wins as usize,
            total_losses: (stats.closed_positions - stats.wins) as usize,
            win_rate_percent: (stats.closed_positions > 0).then_some(stats.win_rate),
            total_trades: stats.closed_positions as usize,
            open_positions: open_positions.len(),
        };

        match serde_json::to_value(pnl_stats) {
            Ok(v) => ToolResult::success(v),
            Err(e) => ToolResult::error(format!("Serialization error: {e}")),
        }
    }
}
