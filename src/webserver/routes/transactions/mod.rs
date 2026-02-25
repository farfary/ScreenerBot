//! Transactions route — serves recent transaction history and details.
//!
//! Provides endpoints for listing, filtering, and viewing transaction details

use axum::{routing::{post, get}, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod handlers;
pub mod types;

pub use types::*;

/// Create transaction history routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/list", post(handlers::list_transactions))
        .route("/summary", post(handlers::get_summary))
        .route("/:signature", get(handlers::get_transaction_detail))
}
