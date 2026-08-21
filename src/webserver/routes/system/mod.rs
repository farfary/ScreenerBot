//! System route — serves system information (version, uptime, resource usage).

mod handlers;
pub mod types;

use crate::webserver::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub(crate) use handlers::schedule_graceful_restart;
pub use types::*;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/reboot", post(handlers::reboot_system))
        .route("/bootstrap", get(handlers::boot_status))
        .route("/paths", get(handlers::get_paths))
        .route("/paths/open-data", post(handlers::open_data_directory))
        .route("/open-url", post(handlers::open_url))
        .route("/data-stats", get(handlers::get_data_stats))
        .route("/client-ready", post(handlers::client_ready))
}
