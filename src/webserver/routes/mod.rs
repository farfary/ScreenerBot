/// Route aggregation module
///
/// Combines all route modules into the main API router

use axum::{ response::{ Html, Json }, Router };
use serde_json::json;
use std::sync::Arc;
use crate::webserver::{ state::AppState, templates };

pub mod status;
pub mod tokens;
pub mod events;

/// Create the main API router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(home_page))
        .route("/home", axum::routing::get(home_page))
        .route("/status", axum::routing::get(status_page))
        .route("/positions", axum::routing::get(positions_page))
        .route("/tokens", axum::routing::get(tokens_page))
        .route("/events", axum::routing::get(events_page))
        .route("/api", axum::routing::get(api_info))
        .nest("/api/v1", api_v1_routes())
        .with_state(state)
}

/// Home page handler
async fn home_page() -> Html<String> {
    let content = templates::home_content();
    Html(templates::base_template("Home", "home", &content))
}

/// Status page handler
async fn status_page() -> Html<String> {
    let content = templates::status_content();
    Html(templates::base_template("Status", "status", &content))
}

/// Positions page handler
async fn positions_page() -> Html<String> {
    let content = templates::positions_content();
    Html(templates::base_template("Positions", "positions", &content))
}

/// Tokens page handler
async fn tokens_page() -> Html<String> {
    let content = templates::tokens_content();
    Html(templates::base_template("Tokens", "tokens", &content))
}

/// Events page handler
async fn events_page() -> Html<String> {
    let content = templates::events_content();
    Html(templates::base_template("Events", "events", &content))
}

/// API info page - JSON format for programmatic access
async fn api_info() -> Json<serde_json::Value> {
    Json(
        json!({
        "name": "ScreenerBot API",
        "version": "0.1.0",
        "description": "Automated Solana DeFi trading bot dashboard API",
        "phase": "Phase 1 - System Status",
        "endpoints": {
            "health": "GET /api/v1/health",
            "status": "GET /api/v1/status",
            "services": "GET /api/v1/status/services",
            "metrics": "GET /api/v1/status/metrics",
            "tokens": "GET /api/v1/tokens",
            "events": "GET /api/v1/events",
            "events_categories": "GET /api/v1/events/categories"
        },
        "documentation": "See docs/webserver-dashboard-api.md for full API documentation",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })
    )
}

/// API v1 routes
fn api_v1_routes() -> Router<Arc<AppState>> {
    Router::new().merge(status::routes()).merge(tokens::routes()).merge(events::routes())
}
