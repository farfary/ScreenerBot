/// API request type definitions
///
/// Standard request structures for REST API endpoints

use serde::{Deserialize, Serialize};

// ================================================================================================
// Phase 2: Query Parameters (Future)
// ================================================================================================

// /// Query parameters for position listing
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PositionQueryParams {
//     pub status: Option<String>,      // "open", "closed", "all"
//     pub token_mint: Option<String>,  // Filter by token
//     pub page: Option<usize>,         // Pagination
//     pub page_size: Option<usize>,
//     pub sort_by: Option<String>,     // "pnl", "entry_time", "exit_time"
//     pub sort_order: Option<String>,  // "asc", "desc"
// }

// /// Query parameters for token search
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TokenQueryParams {
//     pub query: Option<String>,          // Search query
//     pub min_liquidity: Option<f64>,     // Minimum liquidity
//     pub verified_only: Option<bool>,    // Only verified tokens
//     pub page: Option<usize>,
//     pub page_size: Option<usize>,
// }

// /// Query parameters for transaction history
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TransactionQueryParams {
//     pub transaction_type: Option<String>,  // "swap", "transfer", etc.
//     pub status: Option<String>,            // "confirmed", "failed", etc.
//     pub start_time: Option<String>,        // ISO 8601 timestamp
//     pub end_time: Option<String>,
//     pub page: Option<usize>,
//     pub page_size: Option<usize>,
// }

// ================================================================================================
// Phase 3: Trading Operations (Future)
// ================================================================================================

// /// Buy order request
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BuyRequest {
//     pub token_mint: String,
//     pub amount_sol: f64,
//     pub slippage_percent: Option<f64>,
//     pub max_price_sol: Option<f64>,
// }

// /// Sell order request
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SellRequest {
//     pub position_id: Option<i64>,
//     pub token_mint: String,
//     pub amount: Option<f64>,  // None = sell all
//     pub slippage_percent: Option<f64>,
//     pub min_price_sol: Option<f64>,
// }

// /// Close position request
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ClosePositionRequest {
//     pub position_id: i64,
//     pub slippage_percent: Option<f64>,
// }

// ================================================================================================
// Phase 3: Configuration Updates (Future)
// ================================================================================================

// /// Configuration update request
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ConfigUpdateRequest {
//     pub max_positions: Option<usize>,
//     pub position_size_sol: Option<f64>,
//     pub stop_loss_percent: Option<f64>,
//     pub take_profit_percent: Option<f64>,
//     pub trading_enabled: Option<bool>,
// }

// /// Blacklist operation request
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BlacklistRequest {
//     pub token_mint: String,
//     pub reason: Option<String>,
// }

// ================================================================================================
// WebSocket Requests
// ================================================================================================

/// WebSocket subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub r#type: String, // "subscribe"
    pub channels: Vec<String>,
}

/// WebSocket unsubscribe request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub r#type: String, // "unsubscribe"
    pub channels: Vec<String>,
}

// ================================================================================================
// Common Request Types
// ================================================================================================

/// Standard pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,

    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

impl PaginationParams {
    /// Calculate offset for SQL queries
    pub fn offset(&self) -> usize {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// Validate parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("Page must be >= 1".to_string());
        }

        if self.page_size == 0 || self.page_size > 100 {
            return Err("Page size must be between 1 and 100".to_string());
        }

        Ok(())
    }
}
