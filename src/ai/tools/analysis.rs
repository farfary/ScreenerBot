use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::tokens;

// ============================================================================
// Helper: Mint address validation
// ============================================================================

/// Validate Solana address format (base58, 32-44 characters)
fn is_valid_solana_address(addr: &str) -> bool {
    addr.len() >= 32 && addr.len() <= 44 && addr.chars().all(|c| c.is_ascii_alphanumeric())
}

// ============================================================================
// AnalyzeTokenTool - Comprehensive token analysis
// ============================================================================

pub struct AnalyzeTokenTool;

#[derive(Deserialize)]
struct AnalyzeTokenParams {
    mint_address: String,
}

#[derive(Serialize)]
struct TokenAnalysis {
    mint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    market_cap_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price_change_24h_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume_24h_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    liquidity_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_risks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    holder_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_holder_percent: Option<f64>,
}

#[async_trait]
impl Tool for AnalyzeTokenTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "analyze_token".to_string(),
            description: "Analyze a token's security, holder distribution, liquidity, and other metrics. Provides comprehensive token analysis.".to_string(),
            category: ToolCategory::Analysis,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "The Solana token mint address to analyze"
                    }
                },
                "required": ["mint_address"]
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: AnalyzeTokenParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate mint address format
        if !is_valid_solana_address(&params.mint_address) {
            return ToolResult::error("Invalid mint address format".to_string());
        }

        // Get token data from database
        let token = match tokens::get_full_token_async(&params.mint_address).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ToolResult::error(format!(
                    "Token {} not found in database. Try fetching market data first.",
                    params.mint_address
                ));
            }
            Err(e) => return ToolResult::error(format!("Database error: {}", e)),
        };

        // Build analysis response
        let analysis = TokenAnalysis {
            mint: token.mint.clone(),
            symbol: Some(token.symbol.clone()),
            name: Some(token.name.clone()),
            market_cap_usd: token.market_cap,
            price_usd: Some(token.price_usd),
            price_change_24h_percent: token.price_change_h24,
            volume_24h_usd: token.volume_h24,
            liquidity_usd: token.liquidity_usd,
            security_score: token
                .security_score_normalised
                .map(|s| format!("{}/100", s)),
            security_risks: if token.security_risks.is_empty() {
                None
            } else {
                Some(
                    token
                        .security_risks
                        .iter()
                        .map(|r| format!("{}: {}", r.name, r.value))
                        .collect(),
                )
            },
            holder_count: token.total_holders.map(|h| h as u64),
            top_holder_percent: if !token.top_holders.is_empty() {
                Some(token.top_holders[0].pct)
            } else {
                None
            },
        };

        ToolResult::success(serde_json::to_value(analysis).unwrap())
    }
}

// ============================================================================
// GetMarketDataTool - Real-time market data
// ============================================================================

pub struct GetMarketDataTool;

#[derive(Deserialize)]
struct GetMarketDataParams {
    mint_address: String,
}

#[derive(Serialize)]
struct MarketData {
    mint: String,
    price_usd: Option<f64>,
    price_change_24h_percent: Option<f64>,
    price_change_1h_percent: Option<f64>,
    volume_24h_usd: Option<f64>,
    liquidity_usd: Option<f64>,
    market_cap_usd: Option<f64>,
    price_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dex: Option<String>,
}

#[async_trait]
impl Tool for GetMarketDataTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_market_data".to_string(),
            description: "Get current market data for a token including price, volume, market cap, and price changes.".to_string(),
            category: ToolCategory::Analysis,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "The Solana token mint address"
                    }
                },
                "required": ["mint_address"]
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetMarketDataParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Request immediate update to get fresh market data
        let update_result = match tokens::request_immediate_update(&params.mint_address).await {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to fetch market data: {}", e)),
        };

        // Get the updated token data
        let token = match tokens::get_full_token_async(&params.mint_address).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ToolResult::error(format!(
                    "Token {} not found after update",
                    params.mint_address
                ));
            }
            Err(e) => return ToolResult::error(format!("Database error: {}", e)),
        };

        let market_data = MarketData {
            mint: token.mint.clone(),
            price_usd: Some(token.price_usd),
            price_change_24h_percent: token.price_change_h24,
            price_change_1h_percent: token.price_change_h1,
            volume_24h_usd: token.volume_h24,
            liquidity_usd: token.liquidity_usd,
            market_cap_usd: token.market_cap,
            price_sol: Some(token.price_sol),
            pool_address: token.pool_price_last_used_pool.clone(),
            dex: Some(format!("{:?}", token.data_source)),
        };

        ToolResult::success(serde_json::to_value(market_data).unwrap())
    }
}

// ============================================================================
// CheckSecurityTool - Security analysis
// ============================================================================

pub struct CheckSecurityTool;

#[derive(Deserialize)]
struct CheckSecurityParams {
    mint_address: String,
}

#[derive(Serialize)]
struct SecurityData {
    mint: String,
    score: Option<String>,
    level: Option<String>,
    risks: Vec<SecurityRisk>,
    freeze_authority_enabled: Option<bool>,
    mint_authority_enabled: Option<bool>,
    top_10_holders_percent: Option<f64>,
    total_supply: Option<f64>,
}

#[derive(Serialize)]
struct SecurityRisk {
    name: String,
    level: String,
    description: String,
}

#[async_trait]
impl Tool for CheckSecurityTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "check_security".to_string(),
            description: "Run security checks on a token including freeze authority, mint authority, and known scam checks.".to_string(),
            category: ToolCategory::Analysis,
            parameters: json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "The Solana token mint address to check"
                    }
                },
                "required": ["mint_address"]
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: CheckSecurityParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Request immediate update to get fresh security data
        let _ = tokens::request_immediate_update(&params.mint_address).await;

        // Get token security data
        let token = match tokens::get_full_token_async(&params.mint_address).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return ToolResult::error(format!("Token {} not found", params.mint_address));
            }
            Err(e) => return ToolResult::error(format!("Database error: {}", e)),
        };

        let security_data = SecurityData {
            mint: token.mint.clone(),
            score: token.security_score.map(|s| s.to_string()),
            level: token.security_score_normalised.map(|s| {
                if s < 30 {
                    "Low Risk".to_string()
                } else if s < 70 {
                    "Medium Risk".to_string()
                } else {
                    "High Risk".to_string()
                }
            }),
            risks: token
                .security_risks
                .iter()
                .map(|r| SecurityRisk {
                    name: r.name.clone(),
                    level: r.level.clone(),
                    description: r.description.clone(),
                })
                .collect(),
            freeze_authority_enabled: Some(token.freeze_authority.is_some()),
            mint_authority_enabled: Some(token.mint_authority.is_some()),
            top_10_holders_percent: token.top_10_holders_pct,
            total_supply: token.supply.as_ref().and_then(|s| s.parse::<f64>().ok()),
        };

        ToolResult::success(serde_json::to_value(security_data).unwrap())
    }
}
