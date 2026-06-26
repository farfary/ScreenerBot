//! Configuration API Routes
//!
//! Provides REST API endpoints for viewing and managing bot configuration.
//! All responses follow the standard ScreenerBot API format.

use axum::{
    response::Response,
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod getters;
mod import_export;
mod operations;
pub mod types;

// Re-export handler functions for use by the router
use getters::{
    get_ai_config, get_config_metadata, get_events_config, get_filtering_config, get_full_config,
    get_gui_config, get_gui_defaults, get_holder_watch_config, get_maintenance_config,
    get_monitoring_config, get_network_config, get_ohlcv_config, get_performance_config,
    get_pools_config, get_positions_config, get_rpc_config, get_services_config, get_sol_price_config,
    get_strategies_config, get_summary_config, get_swaps_config, get_telegram_config,
    get_tokens_config, get_trader_config, get_wallet_config, patch_any_config,
};
use import_export::{export_config, import_config, import_config_preview};
use operations::{get_config_diff, reload_config_from_disk, reset_config_to_defaults};

// Needed for generic type parameter in routes
use crate::config;

// ============================================================================
// ROUTES
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // GET endpoints - View configuration
        .route("/config", get(get_full_config))
        .route("/config/rpc", get(get_rpc_config))
        .route("/config/trader", get(get_trader_config))
        .route("/config/positions", get(get_positions_config))
        .route("/config/filtering", get(get_filtering_config))
        .route("/config/swaps", get(get_swaps_config))
        .route("/config/tokens", get(get_tokens_config))
        .route("/config/pools", get(get_pools_config))
        .route("/config/maintenance", get(get_maintenance_config))
        .route("/config/sol_price", get(get_sol_price_config))
        .route("/config/summary", get(get_summary_config))
        .route("/config/events", get(get_events_config))
        .route("/config/services", get(get_services_config))
        .route("/config/monitoring", get(get_monitoring_config))
        .route("/config/ohlcv", get(get_ohlcv_config))
        .route("/config/gui", get(get_gui_config))
        .route("/config/gui/defaults", get(get_gui_defaults))
        .route("/config/telegram", get(get_telegram_config))
        .route("/config/ai", get(get_ai_config))
        .route("/config/strategies", get(get_strategies_config))
        .route("/config/holder_watch", get(get_holder_watch_config))
        .route("/config/wallet", get(get_wallet_config))
        .route("/config/performance", get(get_performance_config))
        .route("/config/network", get(get_network_config))
        .route("/config/metadata", get(get_config_metadata))
        // PATCH endpoints - Partial updates (use JSON with only fields to update)
        .route(
            "/config/trader",
            patch(patch_any_config::<config::TraderConfig>),
        )
        .route(
            "/config/positions",
            patch(patch_any_config::<config::PositionsConfig>),
        )
        .route(
            "/config/filtering",
            patch(patch_any_config::<config::FilteringConfig>),
        )
        .route(
            "/config/swaps",
            patch(patch_any_config::<config::SwapsConfig>),
        )
        .route(
            "/config/tokens",
            patch(patch_any_config::<config::TokensConfig>),
        )
        .route(
            "/config/pools",
            patch(patch_any_config::<config::PoolsConfig>),
        )
        .route(
            "/config/maintenance",
            patch(patch_any_config::<config::MaintenanceConfig>),
        )
        .route("/config/rpc", patch(patch_any_config::<config::RpcConfig>))
        .route(
            "/config/sol_price",
            patch(patch_any_config::<config::SolPriceConfig>),
        )
        .route(
            "/config/events",
            patch(patch_any_config::<config::EventsConfig>),
        )
        .route(
            "/config/services",
            patch(patch_any_config::<config::ServicesConfig>),
        )
        .route(
            "/config/monitoring",
            patch(patch_any_config::<config::MonitoringConfig>),
        )
        .route(
            "/config/ohlcv",
            patch(patch_any_config::<config::OhlcvConfig>),
        )
        .route("/config/gui", patch(patch_any_config::<config::GuiConfig>))
        .route(
            "/config/telegram",
            patch(patch_any_config::<config::TelegramConfig>),
        )
        .route("/config/ai", patch(patch_any_config::<config::AiConfig>))
        .route(
            "/config/strategies",
            patch(patch_any_config::<config::StrategiesConfig>),
        )
        .route(
            "/config/holder_watch",
            patch(patch_any_config::<config::HolderWatchConfig>),
        )
        .route(
            "/config/wallet",
            patch(patch_any_config::<config::WalletConfig>),
        )
        .route(
            "/config/performance",
            patch(patch_any_config::<config::PerformanceConfig>),
        )
        .route(
            "/config/network",
            patch(patch_any_config::<config::NetworkConfig>),
        )
        // Import/Export endpoints
        .route("/config/export", post(export_config))
        .route("/config/import/preview", post(import_config_preview))
        .route("/config/import", post(import_config))
        // Utility endpoints
        .route("/config/reload", post(reload_config_from_disk))
        .route("/config/reset", post(reset_config_to_defaults))
        .route("/config/diff", get(get_config_diff))
}
