use axum::{extract::State, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::tokens::cleanup::get_blacklist_summary;
use crate::tokens::database::get_global_database;
use crate::webserver::state::AppState;

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
    let db = match get_global_database() {
        Some(db) => db,
        None => {
            return Json(BlacklistStatsResponse {
                total_count: 0,
                by_reason: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    match get_blacklist_summary(&db) {
        Ok(summary) => {
            let mut by_reason = std::collections::HashMap::new();
            by_reason.insert(
                "MintAuthority".to_string(),
                summary.authority_mint_count,
            );
            by_reason.insert(
                "FreezeAuthority".to_string(),
                summary.authority_freeze_count,
            );
            by_reason.insert("Manual".to_string(), summary.manual_count);
            if summary.non_authority_auto_count > 0 {
                by_reason.insert(
                    "NonAuthorityAuto".to_string(),
                    summary.non_authority_auto_count,
                );
                for (reason, count) in summary.non_authority_breakdown.iter() {
                    by_reason.insert(format!("NonAuthority::{reason}"), *count);
                }
            }

            Json(BlacklistStatsResponse {
                total_count: summary.total_count,
                by_reason,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(_e) => {
            // Return empty stats on error
            Json(BlacklistStatsResponse {
                total_count: 0,
                by_reason: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}
