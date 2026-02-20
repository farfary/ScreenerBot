//! Wallet management API routes
//!
//! CRUD endpoints for multi-wallet management.
//! Includes bulk import/export with CSV and Excel support.

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

pub mod crud;
pub mod export;
pub mod import;
pub mod types;
pub mod utils;

// Re-export types for external consumers
pub use types::*;

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(crud::list_wallets))
        .route("/", post(crud::create_wallet))
        .route("/import", post(crud::import_wallet))
        .route("/import/preview", post(import::import_preview))
        .route("/import/execute", post(import::import_execute))
        .route("/export", get(export::export_wallets_csv))
        .route("/export/full", post(export::export_wallets_full))
        .route("/summary", get(crud::get_summary))
        .route("/main", get(crud::get_main_wallet))
        .route("/:id", get(crud::get_wallet))
        .route("/:id", put(crud::update_wallet))
        .route("/:id", delete(crud::delete_wallet))
        .route("/:id/export", post(crud::export_wallet))
        .route("/:id/set-main", post(crud::set_main_wallet))
        .route("/:id/archive", post(crud::archive_wallet))
        .route("/:id/restore", post(crud::restore_wallet))
}
