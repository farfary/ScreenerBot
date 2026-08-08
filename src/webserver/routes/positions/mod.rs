//! Position management routes (list, detail, debug views, force-close, archive/delete).
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod activity;
mod debug;
mod detail;
mod force_close;
mod list;
mod manage;
pub mod types;

// Re-export handler functions
use activity::get_token_activity;
use debug::get_position_debug_info;
use detail::get_position_details;
use force_close::force_close_position;
use list::{get_positions, get_positions_stats};
use manage::{
    archive_position, delete_all_archived, delete_position, set_management, unarchive_position,
};

// Re-export load_positions_with_filters as it's used by other modules (e.g. snapshot.rs)
pub use list::load_positions_with_filters;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/positions", get(get_positions))
        .route("/positions/stats", get(get_positions_stats))
        // Bulk delete of all archived positions — static segment, registered before
        // the `:position_id` param route so it is matched first.
        .route("/positions/archived", delete(delete_all_archived))
        .route("/positions/:key/details", get(get_position_details))
        // The token's ALL-TIME activity (every position ever opened on the mint + every
        // wallet transaction that touched it). Separate from `/details` on purpose — that
        // route is on the manual-trade path and must stay cheap.
        .route("/positions/:key/activity", get(get_token_activity))
        .route("/positions/:mint/debug", get(get_position_debug_info))
        .route(
            "/positions/:position_id/force-close",
            post(force_close_position),
        )
        .route("/positions/:position_id/archive", post(archive_position))
        .route(
            "/positions/:position_id/unarchive",
            post(unarchive_position),
        )
        .route("/positions/:position_id/management", post(set_management))
        .route("/positions/:position_id", delete(delete_position))
}
