use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::positions;
use crate::utils::get_sol_balance;

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
    entry_price_usd: f64,
    current_price_usd: Option<f64>,
    amount: f64,
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
            name: "get_positions".to_string(),
            description: "Get all current open trading positions with entry prices, current values, and unrealized P&L.".to_string(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let positions = positions::get_open_positions().await;

        let mut summaries = Vec::new();
        for pos in positions {
            let pnl = positions::calculate_position_pnl_safe(&pos, pos.current_price).await;

            summaries.push(PositionSummary {
                position_id: pos.id.unwrap_or(0),
                mint: pos.mint.clone(),
                symbol: Some(pos.symbol.clone()),
                entry_price_usd: pos.average_entry_price,
                current_price_usd: pos.current_price,
                amount: pos
                    .remaining_token_amount
                    .unwrap_or(pos.token_amount.unwrap_or(0)) as f64,
                cost_sol: pos.total_size_sol,
                current_value_sol: pos.current_price.map(|cp| {
                    let remaining = pos
                        .remaining_token_amount
                        .unwrap_or(pos.token_amount.unwrap_or(0))
                        as f64;
                    remaining * cp
                }),
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
    entry_price_usd: f64,
    current_price_usd: Option<f64>,
    amount: f64,
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
            name: "get_position".to_string(),
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
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetPositionParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let position = match positions::get_position_by_id(params.position_id).await {
            Some(p) => p,
            None => return ToolResult::error(format!("Position {} not found", params.position_id)),
        };

        let pnl = positions::calculate_position_pnl_safe(&position, position.current_price).await;
        let total_fees = (position.entry_fee_lamports.unwrap_or(0)
            + position.exit_fee_lamports.unwrap_or(0)) as f64
            / 1_000_000_000.0;

        let details = PositionDetails {
            position_id: position.id.unwrap_or(0),
            mint: position.mint.clone(),
            symbol: Some(position.symbol.clone()),
            entry_price_usd: position.average_entry_price,
            current_price_usd: position.current_price,
            amount: position
                .remaining_token_amount
                .unwrap_or(position.token_amount.unwrap_or(0)) as f64,
            cost_sol: position.total_size_sol,
            current_value_sol: position.current_price.map(|cp| {
                let remaining = position
                    .remaining_token_amount
                    .unwrap_or(position.token_amount.unwrap_or(0))
                    as f64;
                remaining * cp
            }),
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

        ToolResult::success(serde_json::to_value(details).unwrap())
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
            name: "get_balance".to_string(),
            description: "Get the current SOL balance of the trading wallet.".to_string(),
            category: ToolCategory::Portfolio,
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        let wallet_address = match crate::utils::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(format!("Failed to get wallet address: {}", e)),
        };

        let balance = match get_sol_balance(&wallet_address).await {
            Ok(b) => b,
            Err(e) => return ToolResult::error(format!("Failed to get balance: {}", e)),
        };

        let info = BalanceInfo {
            sol_balance: balance,
            wallet_address,
        };

        ToolResult::success(serde_json::to_value(info).unwrap())
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
    win_rate_percent: f64,
    total_trades: usize,
    open_positions: usize,
}

#[async_trait]
impl Tool for GetPnLTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_pnl".to_string(),
            description: "Get profit and loss statistics including total realized P&L, unrealized P&L, win rate, and trade counts.".to_string(),
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
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetPnLParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let period = params.period.unwrap_or_else(|| "all".to_string());

        // Calculate timeframe
        let since = match period.as_str() {
            "today" => Some(chrono::Utc::now() - chrono::Duration::days(1)),
            "week" => Some(chrono::Utc::now() - chrono::Duration::weeks(1)),
            "month" => Some(chrono::Utc::now() - chrono::Duration::days(30)),
            "all" => None,
            _ => return ToolResult::error(format!("Invalid period: {}", period)),
        };

        // Get trading stats from database
        let start_time =
            since.unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap());
        let stats = match positions::get_period_trading_stats(start_time, None).await {
            Ok(s) => s,
            Err(e) => return ToolResult::error(format!("Failed to get trading stats: {}", e)),
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

        let win_rate = if stats.sells > 0 { stats.win_rate } else { 0.0 };

        let pnl_stats = PnLStats {
            period: period.clone(),
            total_realized_pnl_sol: stats.net_pnl_sol,
            total_unrealized_pnl_sol: total_unrealized,
            total_pnl_sol: stats.net_pnl_sol + total_unrealized,
            total_wins: (stats.win_rate * stats.sells as f64 / 100.0) as usize,
            total_losses: stats.sells as usize
                - (stats.win_rate * stats.sells as f64 / 100.0) as usize,
            win_rate_percent: win_rate,
            total_trades: stats.sells as usize,
            open_positions: open_positions.len(),
        };

        ToolResult::success(serde_json::to_value(pnl_stats).unwrap())
    }
}
