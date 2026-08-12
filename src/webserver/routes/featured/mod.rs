//! Featured token routes
//!
//! The discovery surfaces of the dashboard — the featured row above the status bar
//! and the full featured dialog — read from here. Two kinds of source feed them:
//!
//! - **Boosted** — tokens their owners PAID to promote on screenerbot.io. The
//!   standing comes from `routes::boosts`; these cards carry `boosts`/`golden` and
//!   every surface pins them above the organic rows.
//! - **Discovery** — Jupiter top-organic/top-traded and DexScreener trending. Third
//!   party rankings, never gold, never `boosts > 0`.
//!
//! Every category is normalized into one `FeaturedCard` and enriched from our LOCAL
//! database — never a live provider call for stats.

mod cache;
mod cards;
mod handlers;
mod identity;
pub mod types;

use crate::webserver::state::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub use types::{ExternalToken, FeaturedCard};

/// Featured routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/featured", get(handlers::get_featured_handler))
        .route("/featured/all", get(handlers::get_featured_all_handler))
        .route(
            "/featured/jupiter/organic",
            get(handlers::get_jupiter_organic_handler),
        )
        .route(
            "/featured/jupiter/traded",
            get(handlers::get_jupiter_traded_handler),
        )
        .route(
            "/featured/dexscreener/trending",
            get(handlers::get_dexscreener_trending_handler),
        )
}
