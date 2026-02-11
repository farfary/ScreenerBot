use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::config::with_config;
use crate::positions;
use crate::trader::manual;

// ============================================================================
// Helper: Mint address validation
// ============================================================================

/// Validate Solana address format (base58, 32-44 characters)
fn is_valid_solana_address(addr: &str) -> bool {
    addr.len() >= 32 && addr.len() <= 44 && addr.chars().all(|c| c.is_ascii_alphanumeric())
}

// ============================================================================
// BuyTokenTool - Execute buy order
// ============================================================================

pub struct BuyTokenTool;

#[derive(Deserialize)]
struct BuyTokenParams {
    mint_address: String,
    amount_sol: f64,
    #[serde(default)]
    slippage_bps: Option<u16>,
}

#[derive(Serialize)]
struct TradeResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price_usd: Option<f64>,
    message: String,
}

#[async_trait]
impl Tool for BuyTokenTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "buy_token".to_string(),
            description: "Execute a buy order for a token. This will create a new position. REQUIRES USER CONFIRMATION.".to_string(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "The Solana token mint address to buy"
                    },
                    "amount_sol": {
                        "type": "number",
                        "description": "Amount of SOL to spend on this purchase"
                    },
                    "slippage_bps": {
                        "type": "integer",
                        "description": "Slippage tolerance in basis points (default: from config)"
                    }
                },
                "required": ["mint_address", "amount_sol"]
            }),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: BuyTokenParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate mint address format
        if !is_valid_solana_address(&params.mint_address) {
            return ToolResult::error("Invalid mint address format".to_string());
        }

        // Validate amount
        if params.amount_sol <= 0.0 {
            return ToolResult::error("Amount must be greater than 0".to_string());
        }

        let max_trade_size = with_config(|cfg| cfg.trader.trade_size_sol);
        if params.amount_sol > max_trade_size {
            return ToolResult::error(format!(
                "Amount exceeds max trade size of {} SOL",
                max_trade_size
            ));
        }

        // Check if position already exists
        if positions::is_open_position(&params.mint_address).await {
            return ToolResult::error(format!(
                "Position already exists for {}. Use add_to_position instead.",
                params.mint_address
            ));
        }

        // Execute buy
        let result = match manual::manual_buy(&params.mint_address, params.amount_sol).await {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::error(format!("Buy failed: {}", e));
            }
        };

        // Build response
        let response = TradeResponse {
            success: result.success,
            signature: result.tx_signature.clone(),
            position_id: result
                .position_id
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok()),
            amount: result.executed_size_sol,
            price_usd: None, // TradeResult doesn't have USD price
            message: if result.success {
                format!("Successfully bought {} tokens", params.mint_address)
            } else {
                format!(
                    "Buy failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                )
            },
        };

        if response.success {
            match serde_json::to_value(response) {
                Ok(v) => ToolResult::success(v),
                Err(e) => ToolResult::error(format!("Serialization error: {}", e)),
            }
        } else {
            ToolResult::error(response.message)
        }
    }
}

// ============================================================================
// SellTokenTool - Execute sell order
// ============================================================================

pub struct SellTokenTool;

#[derive(Deserialize)]
struct SellTokenParams {
    mint_address: String,
    percentage: f64,
    #[serde(default)]
    slippage_bps: Option<u16>,
}

#[async_trait]
impl Tool for SellTokenTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sell_token".to_string(),
            description: "Execute a sell order for a token position. REQUIRES USER CONFIRMATION."
                .to_string(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "The Solana token mint address to sell"
                    },
                    "percentage": {
                        "type": "number",
                        "description": "Percentage of position to sell (1-100)",
                        "minimum": 1,
                        "maximum": 100
                    },
                    "slippage_bps": {
                        "type": "integer",
                        "description": "Slippage tolerance in basis points (default: from config)"
                    }
                },
                "required": ["mint_address", "percentage"]
            }),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: SellTokenParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate percentage
        if params.percentage <= 0.0 || params.percentage > 100.0 {
            return ToolResult::error("Percentage must be between 1 and 100".to_string());
        }

        // Check if position exists
        if !positions::is_open_position(&params.mint_address).await {
            return ToolResult::error(format!(
                "No open position found for {}",
                params.mint_address
            ));
        }

        // Execute sell
        let result = match manual::manual_sell(&params.mint_address, Some(params.percentage)).await
        {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::error(format!("Sell failed: {}", e));
            }
        };

        // Build response
        let response = TradeResponse {
            success: result.success,
            signature: result.tx_signature.clone(),
            position_id: result
                .position_id
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok()),
            amount: result.executed_size_sol,
            price_usd: None,
            message: if result.success {
                format!(
                    "Successfully sold {}% of {}",
                    params.percentage, params.mint_address
                )
            } else {
                format!(
                    "Sell failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                )
            },
        };

        if response.success {
            match serde_json::to_value(response) {
                Ok(v) => ToolResult::success(v),
                Err(e) => ToolResult::error(format!("Serialization error: {}", e)),
            }
        } else {
            ToolResult::error(response.message)
        }
    }
}

// ============================================================================
// ClosePositionTool - Close entire position
// ============================================================================

pub struct ClosePositionTool;

#[derive(Deserialize)]
struct ClosePositionParams {
    position_id: i64,
    #[serde(default)]
    slippage_bps: Option<u16>,
}

#[async_trait]
impl Tool for ClosePositionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "close_position".to_string(),
            description: "Close an entire position (sell 100%). REQUIRES USER CONFIRMATION."
                .to_string(),
            category: ToolCategory::Trading,
            parameters: json!({
                "type": "object",
                "properties": {
                    "position_id": {
                        "type": "integer",
                        "description": "The position ID to close"
                    },
                    "slippage_bps": {
                        "type": "integer",
                        "description": "Slippage tolerance in basis points (default: from config)"
                    }
                },
                "required": ["position_id"]
            }),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: ClosePositionParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Get position
        let position = match positions::get_position_by_id(params.position_id).await {
            Some(p) => p,
            None => {
                return ToolResult::error(format!("Position {} not found", params.position_id));
            }
        };

        // Execute sell (100%)
        let result = match manual::manual_sell(&position.mint, Some(100.0)).await {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::error(format!("Close position failed: {}", e));
            }
        };

        // Build response
        let response = TradeResponse {
            success: result.success,
            signature: result.tx_signature.clone(),
            position_id: Some(params.position_id),
            amount: result.executed_size_sol,
            price_usd: None,
            message: if result.success {
                format!("Successfully closed position {}", params.position_id)
            } else {
                format!(
                    "Close failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                )
            },
        };

        if response.success {
            match serde_json::to_value(response) {
                Ok(v) => ToolResult::success(v),
                Err(e) => ToolResult::error(format!("Serialization error: {}", e)),
            }
        } else {
            ToolResult::error(response.message)
        }
    }
}
