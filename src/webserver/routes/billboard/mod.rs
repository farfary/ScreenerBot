//! Billboard API routes
//!
//! Fetches featured tokens from the website's billboard API, plus external sources
//! (Jupiter top tokens, DexScreener trending). Every category is normalized into
//! one `BillboardCard` and enriched from our LOCAL database only — never a live
//! provider call for stats.

mod cache;
mod cards;
mod handlers;
mod logos;
pub mod types;

use crate::webserver::state::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub use types::{BillboardCard, BillboardToken, ExternalToken};

/// Warm the billboard cache in the background so the first dashboard load is
/// served instantly instead of blocking on the remote website fetch.
pub fn prewarm() {
    cache::spawn_prewarm();
}

/// Billboard routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/billboard", get(handlers::get_billboard_handler))
        .route("/billboard/all", get(handlers::get_billboard_all_handler))
        .route(
            "/billboard/jupiter/organic",
            get(handlers::get_jupiter_organic_handler),
        )
        .route(
            "/billboard/jupiter/traded",
            get(handlers::get_jupiter_traded_handler),
        )
        .route(
            "/billboard/dexscreener/trending",
            get(handlers::get_dexscreener_trending_handler),
        )
}
