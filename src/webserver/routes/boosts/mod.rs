//! Token boosts — the paid promotion standing published by screenerbot.io.
//!
//! A boost is bought on the website (`screenerbot.io/boost`): a confirmed payment
//! promotes a mint for a fixed window, and enough active boosts unlock the Golden
//! tier. The website owns the money, the ledger and the thresholds; the desktop app
//! only READS the resulting standing.
//!
//! This module is the single source of that truth in the app. Everything that
//! surfaces a boost reads [`cache::active_boosts`]:
//!
//! - `GET /api/boosts` — the flat feed the dashboard polls to gold-mark table rows
//! - `routes::featured` — the boosted band at the top of the featured row and the
//!   Boosted category of the featured dialog
//!
//! Nothing else may fetch the website feed: two callers would mean two orders and
//! two definitions of "boosted", which is exactly how the same token ends up gold
//! in one surface and plain in another.

mod cache;
pub mod types;

use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;
use axum::{response::Response, routing::get, Router};
use std::sync::Arc;

pub use cache::active_boosts;
pub use types::BoostStanding;

/// Warm the boost cache in the background so the first dashboard paint already
/// knows which tokens are boosted.
pub fn prewarm() {
    cache::spawn_prewarm();
}

/// GET /api/boosts — every mint with an active boost, ranked.
///
/// Deliberately identity-free: it is a hot, small poll whose only job is to tell
/// the dashboard WHICH mints are boosted and how strongly. Names, logos and market
/// stats belong to the surface already rendering the token.
async fn get_boosts_handler() -> Response {
    let standings = active_boosts().await;
    success_response(serde_json::json!({
        "success": true,
        "count": standings.len(),
        "tokens": standings,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/boosts", get(get_boosts_handler))
}
