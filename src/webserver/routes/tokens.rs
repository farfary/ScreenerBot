/// Tokens API routes
///
/// Provides endpoints for accessing tokens with available prices from the pool service

use axum::{ routing::get, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{ pools, tokens::cache::TokenDatabase, webserver::state::AppState };

/// Token with price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenWithPrice {
    pub mint: String,
    pub symbol: String,
    pub price_sol: f64,
    pub pool_address: String,
    pub updated_at: i64,
}

/// Tokens list response
#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub tokens: Vec<TokenWithPrice>,
    pub count: usize,
    pub timestamp: String,
}

/// Create tokens routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/tokens", get(get_tokens_with_prices))
}

/// Get all tokens with available prices
async fn get_tokens_with_prices() -> Json<TokensResponse> {
    // Get all tokens that have available prices from pool service
    let available_mints = pools::get_available_tokens();

    let mut tokens_with_prices = Vec::new();

    // Get token details from database
    let db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return Json(TokensResponse {
                tokens: vec![],
                count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    for mint in &available_mints {
        // Get price from pool service
        if let Some(price_result) = pools::get_pool_price(mint) {
            // Try to get symbol from database, fallback to short mint
            let symbol = match db.get_token_by_mint(mint) {
                Ok(Some(token)) => token.symbol,
                _ => format!("{}...", &mint[..8]),
            };

            // Calculate Unix timestamp from Instant
            let age_seconds = price_result.timestamp.elapsed().as_secs();
            let now_unix = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let price_unix_time = now_unix - (age_seconds as i64);

            tokens_with_prices.push(TokenWithPrice {
                mint: mint.clone(),
                symbol,
                price_sol: price_result.price_sol,
                pool_address: price_result.pool_address,
                updated_at: price_unix_time,
            });
        }
    }

    // Sort by symbol
    tokens_with_prices.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let count = tokens_with_prices.len();

    Json(TokensResponse {
        tokens: tokens_with_prices,
        count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
