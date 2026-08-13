//! Published token-profile content owned by screenerbot.io.
//!
//! The website owns SOL payment, moderation, revisions and publication. The local
//! app consumes only the published projection and keeps the last good feed while
//! offline. A published profile is presentation content, never a risk or identity
//! verdict.

mod cache;
pub mod types;

use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;
use axum::{response::Response, routing::get as route_get, Router};
use std::sync::Arc;

pub use types::PublishedTokenProfile;

pub async fn get(mint: &str) -> Option<PublishedTokenProfile> {
    cache::get(mint).await
}

pub fn prewarm() {
    cache::spawn_prewarm();
}

async fn handler() -> Response {
    let profiles = cache::all().await;
    success_response(serde_json::json!({ "profiles": profiles }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/token-profiles", route_get(handler))
}
