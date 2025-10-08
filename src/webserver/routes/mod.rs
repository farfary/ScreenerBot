use crate::webserver::{state::AppState, templates};
use axum::{response::Html, Router};
use std::sync::Arc;

pub mod blacklist;
pub mod config;
pub mod dashboard;
pub mod events;
pub mod ohlcv;
pub mod positions;
pub mod services;
pub mod status;
pub mod system;
pub mod tokens;
pub mod trader;
pub mod trading;
pub mod wallet;
pub mod websocket;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(home_page))
        .route("/home", axum::routing::get(home_page))
        .route("/status", axum::routing::get(status_page))
        .route("/positions", axum::routing::get(positions_page))
        .route("/tokens", axum::routing::get(tokens_page))
        .route("/events", axum::routing::get(events_page))
        .route("/services", axum::routing::get(services_page))
        .route("/config", axum::routing::get(config_page))
        .nest("/api", api_routes())
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

/// Services page handler
async fn services_page() -> Html<String> {
    let content = templates::services_content();
    Html(templates::base_template("Services", "services", &content))
}

/// Config page handler
async fn config_page() -> Html<String> {
    let content = templates::config_content();
    Html(templates::base_template(
        "Configuration",
        "config",
        &content,
    ))
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(status::routes())
        .merge(tokens::routes())
        .merge(events::routes())
        .merge(positions::routes())
        .merge(dashboard::routes())
        .merge(wallet::routes())
        .merge(blacklist::routes())
        .merge(config::routes())
        .merge(websocket::routes())
        .merge(services::routes())
        .merge(ohlcv::ohlcv_routes())
        .nest("/trading", trading::routes())
        .nest("/trader", trader::routes())
        .nest("/system", system::routes())
        .route("/pages/:page", axum::routing::get(get_page_content))
}

/// SPA page content handler - returns just the content HTML (not full template)
async fn get_page_content(axum::extract::Path(page): axum::extract::Path<String>) -> Html<String> {
    let content = match page.as_str() {
        "home" => templates::home_content(),
        "status" => templates::status_content(),
        "positions" => templates::positions_content(),
        "tokens" => templates::tokens_content(),
        "events" => templates::events_content(),
        "services" => templates::services_content(),
        "config" => templates::config_content(),
        _ => format!("<h1>Page Not Found: {}</h1>", page),
    };

    Html(content)
}
