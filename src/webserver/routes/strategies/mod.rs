//! Strategy CRUD and configuration routes for the web UI.
use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Submodules
mod crud;
mod deployment;
mod performance;
mod schemas;
mod templates;
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
        // Inline validation (must be before /:id routes to avoid path conflict)
        .route(
            "/validate",
            post(validation::validate_strategy_inline_handler),
        )
        // Condition schemas
        .route("/conditions/schemas", get(schemas::get_condition_schemas))
        // Templates
        .route("/templates", get(templates::list_templates))
        // Routes with path parameters (must come after static routes)
        .route("/:id/enabled", patch(crud::set_strategy_enabled_handler))
        .route(
            "/:id",
            get(crud::get_strategy_detail)
                .put(crud::update_strategy_handler)
                .delete(crud::delete_strategy_handler),
        )
        // Performance and testing
        .route(
            "/:id/performance",
            get(performance::get_strategy_performance_stats),
        )
        .route("/:id/test", post(testing::test_strategy))
        // Validate / Deploy (by ID)
        .route("/:id/validate", post(validation::validate_strategy_handler))
        .route("/:id/deploy", post(deployment::deploy_strategy_handler))
}
