//! OHLCV route — serves candlestick chart data for token price history.

mod handlers;
mod types;

use crate::webserver::state::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Token list and stats endpoints
        .route("/ohlcv/tokens", get(handlers::get_all_tokens_handler))
        .route("/ohlcv/stats", get(handlers::get_stats_handler))
        .route("/ohlcv/cleanup", post(handlers::cleanup_inactive_handler))
        .route("/ohlcv/cache/clear", post(handlers::clear_all_handler))
        // Data endpoints
        .route("/ohlcv/:mint", get(handlers::get_ohlcv_data_handler))
        .route("/ohlcv/:mint/pools", get(handlers::get_pools_handler))
        .route("/ohlcv/:mint/gaps", get(handlers::get_gaps_handler))
        .route("/ohlcv/:mint/status", get(handlers::get_status_handler))
        .route(
            "/ohlcv/:mint/delete",
            delete(handlers::delete_token_handler),
        )
        // Control endpoints
        .route("/ohlcv/:mint/refresh", post(handlers::refresh_handler))
        .route(
            "/ohlcv/:mint/monitor",
            post(handlers::add_monitoring_handler),
        )
        .route(
            "/ohlcv/:mint/monitor",
            delete(handlers::remove_monitoring_handler),
        )
        .route("/ohlcv/:mint/view", post(handlers::record_view_handler))
        // System endpoints
        .route("/ohlcv/metrics", get(handlers::get_metrics_handler))
}
