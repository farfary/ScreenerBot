//! Tokens API routes for token listing, details, favorites, blacklist, and OHLCV data

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod blacklist;
mod detail;
mod favorites;
mod identity;
mod list;
mod ohlcv;
pub mod types;

// Re-export handler functions for use by the router
use blacklist::{add_to_blacklist, get_blacklist_status, remove_from_blacklist};
use detail::{get_token_analysis, get_token_detail, refresh_token_data};
use favorites::{add_favorite, get_favorites, remove_favorite, update_favorite};
use identity::get_token_identities;
use list::{filter_tokens, get_tokens_stats, search_tokens};
use ohlcv::{
    deprioritize_token_ohlcv, focus_token, get_token_dexscreener, get_token_ohlcv,
    get_token_ohlcv_status, get_token_transactions, refresh_token_ohlcv, unfocus_token,
};

// Re-export get_tokens_list as pub(crate) since it's used by dashboard routes
pub(crate) use list::get_tokens_list;

// Constants
const MAX_PAGE_SIZE: usize = 200;

// =============================================================================
// ROUTE REGISTRATION
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tokens/list", get(get_tokens_list))
        .route("/tokens/stats", get(get_tokens_stats))
        .route("/tokens/filter", post(filter_tokens))
        .route("/tokens/search", get(search_tokens))
        .route("/tokens/identities", get(get_token_identities))
        .route("/tokens/favorites", get(get_favorites))
        .route("/tokens/favorites", post(add_favorite))
        .route("/tokens/favorites/:mint", delete(remove_favorite))
        .route("/tokens/favorites/:mint", patch(update_favorite))
        .route("/tokens/:mint", get(get_token_detail))
        .route("/tokens/:mint/analysis", get(get_token_analysis))
        .route("/tokens/:mint/refresh", post(refresh_token_data))
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
        .route("/tokens/:mint/ohlcv/status", get(get_token_ohlcv_status))
        .route("/tokens/:mint/ohlcv/refresh", post(refresh_token_ohlcv))
        .route(
            "/tokens/:mint/ohlcv/deprioritize",
            post(deprioritize_token_ohlcv),
        )
        .route("/tokens/:mint/focus", post(focus_token))
        .route("/tokens/:mint/unfocus", post(unfocus_token))
        .route("/tokens/:mint/dexscreener", get(get_token_dexscreener))
        .route("/tokens/:mint/blacklist", post(add_to_blacklist))
        .route("/tokens/:mint/blacklist", delete(remove_from_blacklist))
        .route("/tokens/:mint/blacklist", get(get_blacklist_status))
        // Transactions endpoint
        .route("/tokens/:mint/transactions", get(get_token_transactions))
}
