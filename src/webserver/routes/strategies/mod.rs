// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy CRUD and configuration routes for the web UI.
use axum::{
    middleware::from_fn_with_state,
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::database::catalog;
use crate::webserver::middleware::store_gate;
use crate::webserver::state::AppState;

// Submodules
mod crud;
mod mode;
mod overview;
mod schemas;
mod testing;
mod types;
mod utils;
mod validation;

// Re-export types for external use
pub use types::*;

// =============================================================================
// ROUTER SETUP
// =============================================================================

/// Create the strategies router with all routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Strategy CRUD
        .route("/", get(crud::list_strategies).post(crud::create_strategy))
        .route("/overview", get(overview::get_strategies_overview))
        // Inline validation (must be before /:id routes to avoid path conflict)
        .route(
            "/validate",
            post(validation::validate_strategy_inline_handler),
        )
        // Condition schemas
        .route("/conditions/schemas", get(schemas::get_condition_schemas))
        // Routes with path parameters (must come after static routes)
        .route("/{id}/enabled", patch(crud::set_strategy_enabled_handler))
        .route("/{id}/mode", patch(mode::set_strategy_mode_handler))
        .route("/{id}/readiness", get(mode::get_strategy_readiness))
        .route(
            "/{id}",
            get(crud::get_strategy_detail)
                .put(crud::update_strategy_handler)
                .delete(crud::delete_strategy_handler),
        )
        // Testing
        .route("/{id}/test", post(testing::test_strategy))
        .route_layer(from_fn_with_state(&catalog::STRATEGIES, store_gate))
}
