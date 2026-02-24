//! Dashboard page routes (home, overview, and related utilities).
use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod home;
mod overview;
mod types;
mod utils;

pub use home::get_home_dashboard;
pub use overview::get_dashboard_overview;
pub use types::*;

/// Create dashboard routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dashboard/overview", get(get_dashboard_overview))
        .route("/dashboard/home", get(get_home_dashboard))
}
