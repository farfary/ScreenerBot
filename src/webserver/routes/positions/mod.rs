//! Position management routes (list, detail, debug views, force-close).
use axum::{routing::get, routing::post, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod debug;
mod detail;
mod force_close;
mod list;
pub mod types;

// Re-export handler functions
use debug::get_position_debug_info;
use detail::get_position_details;
use force_close::force_close_position;
use list::{get_positions, get_positions_stats};

// Re-export load_positions_with_filters as it's used by other modules (e.g. snapshot.rs)
pub use list::load_positions_with_filters;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/positions", get(get_positions))
        .route("/positions/stats", get(get_positions_stats))
        .route("/positions/:key/details", get(get_position_details))
        .route("/positions/:mint/debug", get(get_position_debug_info))
        .route(
            "/positions/:position_id/force-close",
            post(force_close_position),
        )
}
