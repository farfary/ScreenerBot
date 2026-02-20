use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod debug;
mod detail;
mod list;
pub mod types;

// Re-export handler functions
use debug::get_position_debug_info;
use detail::get_position_details;
use list::{get_positions, get_positions_stats};

// Re-export load_positions_with_filters as it's used by other modules (e.g. snapshot.rs)
pub use list::load_positions_with_filters;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/positions", get(get_positions))
        .route("/positions/stats", get(get_positions_stats))
        .route("/positions/:key/details", get(get_position_details))
        .route("/positions/:mint/debug", get(get_position_debug_info))
}
