/// Blacklist API routes
///
/// Provides blacklist management and statistics endpoints

use axum::{ extract::State, response::Json, routing::get, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::webserver::state::AppState;
use crate::tokens::blacklist::{ get_blacklist_summary, BlacklistSummary };

#[derive(Debug, Serialize, Deserialize)]
pub struct BlacklistStatsResponse {
    pub total_count: usize,
    pub by_reason: std::collections::HashMap<String, usize>,
    pub timestamp: String,
}

/// Create blacklist routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/blacklist/stats", get(get_blacklist_stats))
}

/// Get blacklist statistics
async fn get_blacklist_stats() -> Json<BlacklistStatsResponse> {
    match get_blacklist_summary() {
        Ok(summary) => {
            let mut by_reason = std::collections::HashMap::new();
            by_reason.insert("LowLiquidity".to_string(), summary.low_liquidity_count);
            by_reason.insert("NoRoute".to_string(), summary.no_route_count);
            by_reason.insert("ApiError".to_string(), summary.api_error_count);
            by_reason.insert("SystemToken".to_string(), summary.system_token_count);
            by_reason.insert("StableToken".to_string(), summary.stable_token_count);
            by_reason.insert("Manual".to_string(), summary.manual_count);
            by_reason.insert("PoorPerformance".to_string(), summary.poor_performance_count);

            Json(BlacklistStatsResponse {
                total_count: summary.total_count,
                by_reason,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(e) => {
            // Return empty stats on error
            Json(BlacklistStatsResponse {
                total_count: 0,
                by_reason: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}
