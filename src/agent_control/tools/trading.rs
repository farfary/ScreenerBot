//! Agent-facing manual trading tools: buy, add to (DCA), partial sell and close.
//! Every tool passes the same `trader::manual::guard` preflight as the dashboard
//! trade dialog, then calls the canonical `trader::manual` API.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::config::with_config;
use crate::positions::{self, PositionManagement};
use crate::trader::manual::{self, guard};
use crate::trader::{TradeResult, MAX_MANUAL_SLIPPAGE_PCT};

#[derive(Serialize)]
struct TradeResponse {
    mint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executed_size_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executed_price_sol: Option<f64>,
    message: String,
}

fn slippage_schema() -> serde_json::Value {
    json!({
        "type": "number",
        "description": format!(
            "Per-trade slippage override in percent, (0, {MAX_MANUAL_SLIPPAGE_PCT}]. Omit to use the configured slippage."
        )
    })
}

fn finish(
    result: Result<TradeResult, crate::trader::Error>,
    mint: String,
    message: String,
) -> ToolResult {
    match result {
        Ok(trade) if trade.success => ToolResult::success(json!(TradeResponse {
            mint,
            signature: trade.tx_signature,
            position_id: trade.position_id,
            executed_size_sol: trade.executed_size_sol,
            executed_price_sol: trade.executed_price_sol,
            message,
        })),
        Ok(trade) => ToolResult::error(format!(
            "Trade failed: {}",
            trade.error.unwrap_or_else(|| "unknown error".to_owned())
        )),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

/// Agent trades are capped at the configured trade size: an agent can size down,
/// never past what the owner set the auto trader to risk per trade.
fn checked_size(size_sol: Option<f64>, default_sol: f64) -> Result<f64, String> {
    let cap = with_config(|cfg| cfg.trader.trade_size_sol);
    let size = size_sol.unwrap_or(default_sol);
    if !size.is_finite() || size <= 0.0 {
        return Err("Amount must be greater than 0".to_owned());
    }
    if size > cap {
        return Err(format!(
            "Amount {size} SOL exceeds the configured trade size of {cap} SOL (trader.trade_size_sol)"
        ));
    }
    Ok(size)
}

// ============================================================================
// BuyTokenTool
// ============================================================================

pub struct BuyTokenTool;

#[derive(Deserialize)]
struct BuyTokenParams {
    mint_address: String,
    #[serde(default)]
    amount_sol: Option<f64>,
    #[serde(default)]
    management: Option<PositionManagement>,
    #[serde(default)]
    slippage_pct: Option<f64>,
}

#[async_trait]
impl Tool for BuyTokenTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "buy_token".to_owned(),
            description: "Open a new position with a real on-chain buy. Refused while the \
                          emergency stop is active, for blacklisted tokens, or when a position \
                          is already open (use add_to_position). The size is capped at \
                          trader.trade_size_sol."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": { "type": "string", "description": "Token mint address to buy" },
                    "amount_sol": {
                        "type": "number",
                        "description": "SOL to spend. Defaults to trader.trade_size_sol, which is also the maximum."
                    },
                    "management": {
                        "type": "string",
                        "enum": ["auto_trader", "user_only"],
                        "description": "auto_trader (default): the auto trader's exit policy (stop loss, trailing, ROI, time) manages the position. user_only: only manual sells close it."
                    },
                    "slippage_pct": slippage_schema()
                },
                "required": ["mint_address"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: BuyTokenParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };
        let size = match checked_size(
            params.amount_sol,
            with_config(|cfg| cfg.trader.trade_size_sol),
        ) {
            Ok(size) => size,
            Err(message) => return ToolResult::error(message),
        };
        let slippage = match guard::validate_slippage(params.slippage_pct) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        if let Err(e) = guard::preflight(
            guard::ManualTradeKind::Buy,
            &params.mint_address,
            guard::BlacklistPolicy::Enforce,
        )
        .await
        {
            return ToolResult::error(e.to_string());
        }
        let management = params.management.unwrap_or(PositionManagement::AutoTrader);
        let result = manual::manual_buy(&params.mint_address, size, management, slippage).await;
        finish(
            result,
            params.mint_address,
            format!("Bought with {size} SOL"),
        )
    }
}

// ============================================================================
// AddToPositionTool
// ============================================================================

pub struct AddToPositionTool;

#[derive(Deserialize)]
struct AddToPositionParams {
    mint_address: String,
    #[serde(default)]
    amount_sol: Option<f64>,
    #[serde(default)]
    slippage_pct: Option<f64>,
}

#[async_trait]
impl Tool for AddToPositionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "add_to_position".to_owned(),
            description: "Add to (DCA into) an open position with a real on-chain buy. The size \
                          defaults to the configured DCA size and is capped at \
                          trader.trade_size_sol."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": { "type": "string", "description": "Mint of the open position" },
                    "amount_sol": {
                        "type": "number",
                        "description": "SOL to add. Defaults to trader.trade_size_sol * trader.dca_size_percentage / 100."
                    },
                    "slippage_pct": slippage_schema()
                },
                "required": ["mint_address"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: AddToPositionParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };
        let dca_default =
            with_config(|cfg| cfg.trader.trade_size_sol * (cfg.trader.dca_size_percentage / 100.0));
        let size = match checked_size(params.amount_sol, dca_default) {
            Ok(size) => size,
            Err(message) => return ToolResult::error(message),
        };
        let slippage = match guard::validate_slippage(params.slippage_pct) {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };
        if let Err(e) = guard::preflight(
            guard::ManualTradeKind::Add,
            &params.mint_address,
            guard::BlacklistPolicy::Enforce,
        )
        .await
        {
            return ToolResult::error(e.to_string());
        }
        let result = manual::manual_add(&params.mint_address, size, slippage).await;
        finish(
            result,
            params.mint_address,
            format!("Added {size} SOL to the position"),
        )
    }
}

// ============================================================================
// SellTokenTool
// ============================================================================

pub struct SellTokenTool;

#[derive(Deserialize)]
struct SellTokenParams {
    mint_address: String,
    #[serde(default)]
    percentage: Option<f64>,
    #[serde(default)]
    slippage_pct: Option<f64>,
}

async fn sell(mint: String, percentage: Option<f64>, slippage_pct: Option<f64>) -> ToolResult {
    let percentage = match percentage.map(guard::validate_percentage).transpose() {
        Ok(pct) => pct,
        Err(e) => return ToolResult::error(e.to_string()),
    };
    let slippage = match guard::validate_slippage(slippage_pct) {
        Ok(v) => v,
        Err(e) => return ToolResult::error(e.to_string()),
    };
    if let Err(e) = guard::preflight(
        guard::ManualTradeKind::Sell,
        &mint,
        guard::BlacklistPolicy::Ignore,
    )
    .await
    {
        return ToolResult::error(e.to_string());
    }
    let result = manual::manual_sell(&mint, percentage, slippage).await;
    let message = match percentage {
        Some(pct) if pct < 100.0 => format!("Sold {pct}% of the position"),
        _ => "Closed the full position".to_owned(),
    };
    finish(result, mint, message)
}

#[async_trait]
impl Tool for SellTokenTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sell_token".to_owned(),
            description: "Sell part or all of an open position with a real on-chain sell."
                .to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": { "type": "string", "description": "Mint of the open position" },
                    "percentage": {
                        "type": "number",
                        "description": "Percent of the position to sell, (0, 100]. Defaults to positions.partial_exit_default_pct.",
                        "exclusiveMinimum": 0,
                        "maximum": 100
                    },
                    "slippage_pct": slippage_schema()
                },
                "required": ["mint_address"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: SellTokenParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };
        let percentage = params
            .percentage
            .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct));
        sell(params.mint_address, Some(percentage), params.slippage_pct).await
    }
}

// ============================================================================
// ClosePositionTool
// ============================================================================

pub struct ClosePositionTool;

#[derive(Deserialize)]
struct ClosePositionParams {
    position_id: i64,
    #[serde(default)]
    slippage_pct: Option<f64>,
}

#[async_trait]
impl Tool for ClosePositionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "close_position".to_owned(),
            description: "Close an entire open position (sell 100%) by position id.".to_owned(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "position_id": { "type": "integer", "description": "Position id to close" },
                    "slippage_pct": slippage_schema()
                },
                "required": ["position_id"]
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: ClosePositionParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };
        let Some(position) = positions::get_position_by_id(params.position_id).await else {
            return ToolResult::error(format!("Position {} not found", params.position_id));
        };
        if position.exit_time.is_some() {
            return ToolResult::error(format!("Position {} is already closed", params.position_id));
        }
        sell(position.mint, None, params.slippage_pct).await
    }
}
