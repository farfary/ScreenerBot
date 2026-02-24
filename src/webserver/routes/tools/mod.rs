//! Tools API routes for wallet utilities, token operations, and trading tools

use axum::{
    response::Response,
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod ata_cleanup;
mod burn_tokens;
mod favorites;
mod multi_wallet;
mod trade_watcher;
pub mod types;

// Re-export handler functions for use by the router
use ata_cleanup::{
    cleanup_atas, clear_ata_cache, generate_keypair, generate_keypairs, get_ata_stats, scan_atas,
};
use burn_tokens::{burn_selected_tokens, scan_burnable_tokens};
use favorites::{
    add_favorite, delete_favorite, get_favorites_list, mark_favorite_used, update_favorite,
};
use multi_wallet::{
    abort_multi_buy, abort_multi_sell, cleanup_subwallet_atas, consolidate_wallets,
    get_multi_buy_status, get_multi_sell_status, get_multi_wallet_sessions, get_wallets_summary,
    preview_multi_buy, preview_multi_sell, start_multi_buy, start_multi_sell,
};
use trade_watcher::{
    add_watched_token_handler, delete_watched_token_handler, get_trade_watcher_status_handler,
    get_watched_tokens_handler, search_pools_handler, start_trade_watcher_handler,
    stop_trade_watcher_handler,
};

// =============================================================================
// Routes
// =============================================================================

/// Create multi-wallet routes
fn multi_wallet_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Multi-Buy
        .route("/multi-buy/preview", post(preview_multi_buy))
        .route("/multi-buy/start", post(start_multi_buy))
        .route("/multi-buy/:id", get(get_multi_buy_status))
        .route("/multi-buy/:id/abort", post(abort_multi_buy))
        // Multi-Sell
        .route("/multi-sell/preview", post(preview_multi_sell))
        .route("/multi-sell/start", post(start_multi_sell))
        .route("/multi-sell/:id", get(get_multi_sell_status))
        .route("/multi-sell/:id/abort", post(abort_multi_sell))
        // Wallet Management
        .route("/wallets/summary", get(get_wallets_summary))
        .route("/wallets/consolidate", post(consolidate_wallets))
        .route("/wallets/cleanup-atas", post(cleanup_subwallet_atas))
        // Sessions
        .route("/multi-wallet/sessions", get(get_multi_wallet_sessions))
}

/// Create tools routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Wallet Cleanup (ATA management)
        .route("/ata-scan", get(scan_atas))
        .route("/ata-stats", get(get_ata_stats))
        .route("/ata-cleanup", post(cleanup_atas))
        .route("/ata-clear-cache", post(clear_ata_cache))
        // Burn Tokens
        .route("/burn-tokens/scan", get(scan_burnable_tokens))
        .route("/burn-tokens/burn", post(burn_selected_tokens))
        // Wallet Generator
        .route("/generate-keypair", post(generate_keypair))
        .route("/generate-keypairs", post(generate_keypairs))
        // Tool Favorites
        .route("/favorites", get(get_favorites_list))
        .route("/favorites", post(add_favorite))
        .route("/favorites/:id", patch(update_favorite))
        .route("/favorites/:id", delete(delete_favorite))
        .route("/favorites/:id/use", post(mark_favorite_used))
        // Trade Watcher
        .route("/search-pools/:mint", get(search_pools_handler))
        .route("/watched-tokens", get(get_watched_tokens_handler))
        .route("/watched-tokens", post(add_watched_token_handler))
        .route("/watched-tokens/:id", delete(delete_watched_token_handler))
        .route("/trade-watcher/start", post(start_trade_watcher_handler))
        .route("/trade-watcher/stop", post(stop_trade_watcher_handler))
        .route(
            "/trade-watcher/status",
            get(get_trade_watcher_status_handler),
        )
        // Merge multi-wallet routes
        .merge(multi_wallet_routes())
}
