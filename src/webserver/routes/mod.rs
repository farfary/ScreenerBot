use crate::webserver::{state::AppState, templates};
use axum::{
    http::{header as http_header, StatusCode},
    response::{Html, IntoResponse, Response},
    Router,
};
use std::sync::Arc;

pub mod actions;
pub mod ai;
pub mod asset_serving;
pub mod auth;
pub mod billboard;
pub mod blacklist;
pub mod config;
pub mod connectivity;
pub mod dashboard;
pub mod events;
pub mod features;
pub mod filtering;
pub mod header;
pub mod initialization;
pub mod lockscreen;
pub mod ohlcv;
pub mod positions;
pub mod services;
pub mod status;
pub mod strategies;
pub mod system;
pub mod telegram;
pub mod tokens;
pub mod tools;
pub mod trader;
pub mod trading;
pub mod transactions;
pub mod ui_state;
pub mod updates;
pub mod wallet;
pub mod wallets;

use asset_serving::*;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(home_page))
        .route("/home", axum::routing::get(home_page))
        .route("/login", axum::routing::get(login_page))
        .route("/services", axum::routing::get(services_page))
        .route("/tokens", axum::routing::get(tokens_page))
        .route("/positions", axum::routing::get(positions_page))
        .route("/events", axum::routing::get(events_page))
        .route("/transactions", axum::routing::get(transactions_page))
        .route("/filtering", axum::routing::get(filtering_page))
        .route("/wallets", axum::routing::get(wallets_page))
        .route("/tools", axum::routing::get(tools_page))
        .route("/ai", axum::routing::get(ai_page))
        .route("/config", axum::routing::get(config_page))
        .route("/strategies", axum::routing::get(strategies_page))
        .route("/trader", axum::routing::get(trader_page))
        .route("/initialization", axum::routing::get(initialization_page))
        .route("/updates", axum::routing::get(updates_page))
        .route("/about", axum::routing::get(about_page))
        .route("/scripts/core/:file", axum::routing::get(get_core_script))
        .route("/scripts/pages/*file", axum::routing::get(get_page_script))
        .route("/scripts/ui/*file", axum::routing::get(get_ui_script))
        .route("/assets/:file", axum::routing::get(get_asset))
        .route("/assets/fonts/:file", axum::routing::get(get_font))
        .route(
            "/assets/providers/:file",
            axum::routing::get(get_provider_logo),
        )
        .nest("/api", api_routes())
        .with_state(state)
}

/// Home page handler
async fn home_page() -> Html<String> {
    let content = templates::home_content();
    Html(templates::base_template("Home", "home", &content))
}

/// Tokens page handler
async fn tokens_page() -> Html<String> {
    let content = templates::tokens_content();
    Html(templates::base_template("Tokens", "tokens", &content))
}

/// Positions page handler
async fn positions_page() -> Html<String> {
    let content = templates::positions_content();
    Html(templates::base_template("Positions", "positions", &content))
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

/// Transactions page handler
async fn transactions_page() -> Html<String> {
    let content = templates::transactions_content();
    Html(templates::base_template(
        "Transactions",
        "transactions",
        &content,
    ))
}

/// Filtering page handler
async fn filtering_page() -> Html<String> {
    let content = templates::filtering_content();
    Html(templates::base_template("Filtering", "filtering", &content))
}

/// Config page handler
async fn config_page() -> Html<String> {
    let content = templates::config_content();
    Html(templates::base_template("Config", "config", &content))
}

/// Strategies page handler
async fn strategies_page() -> Html<String> {
    let content = templates::strategies_content();
    Html(templates::base_template(
        "Strategies",
        "strategies",
        &content,
    ))
}

/// Auto Trader page handler
async fn trader_page() -> Html<String> {
    let content = templates::trader_content();
    Html(templates::base_template("Auto Trader", "trader", &content))
}

/// Wallets page handler
async fn wallets_page() -> Html<String> {
    let content = templates::wallets_content();
    Html(templates::base_template("Wallets", "wallets", &content))
}

/// Tools page handler
async fn tools_page() -> Html<String> {
    let content = templates::tools_content();
    Html(templates::base_template("Tools", "tools", &content))
}

/// Assistant page handler
async fn ai_page() -> Html<String> {
    let content = templates::ai_content();
    Html(templates::base_template("Assistant", "ai", &content))
}

/// Initialization page handler
async fn initialization_page() -> Html<String> {
    let content = templates::initialization_content();
    Html(templates::base_template(
        "Initialization",
        "initialization",
        &content,
    ))
}

/// Updates page handler
async fn updates_page() -> Html<String> {
    let content = templates::updates_content();
    Html(templates::base_template("Updates", "updates", &content))
}

/// About page handler
async fn about_page() -> Html<String> {
    let content = templates::about_content();
    Html(templates::base_template("About", "about", &content))
}

/// Login page handler
async fn login_page() -> Html<String> {
    let content = templates::login_content();
    Html(templates::login_template("Login", &content))
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(status::routes())
        .merge(tokens::routes())
        .merge(events::routes())
        .merge(filtering::routes())
        .merge(positions::routes())
        .merge(dashboard::routes())
        .merge(wallet::routes())
        .merge(blacklist::routes())
        .merge(config::routes())
        .merge(services::routes())
        .merge(ohlcv::ohlcv_routes())
        .merge(actions::routes())
        .merge(header::routes())
        .merge(ui_state::routes())
        .merge(billboard::routes())
        .nest("/connectivity", connectivity::routes())
        .nest("/features", features::routes())
        .nest("/initialization", initialization::routes())
        .nest("/trading", trading::routes())
        .nest("/trader", trader::routes())
        .nest("/system", system::routes())
        .nest("/transactions", transactions::routes())
        .nest("/strategies", strategies::routes())
        .nest("/tools", tools::routes())
        .nest("/wallets", wallets::routes())
        .nest("/lockscreen", lockscreen::routes())
        .nest("/auth", auth::routes())
        .nest("/telegram", telegram::routes())
        .nest("/ai", ai::routes())
        .merge(updates::routes())
        .route("/pages/:page", axum::routing::get(get_page_content))
}

/// SPA page content handler - returns just the content HTML (not full template)
async fn get_page_content(axum::extract::Path(page): axum::extract::Path<String>) -> Html<String> {
    let content = match page.as_str() {
        "home" => templates::home_content(),
        "tokens" => templates::tokens_content(),
        "positions" => templates::positions_content(),
        "events" => templates::events_content(),
        "services" => templates::services_content(),
        "transactions" => templates::transactions_content(),
        "filtering" => templates::filtering_content(),
        "wallets" => templates::wallets_content(),
        "tools" => templates::tools_content(),
        "config" => templates::config_content(),
        "strategies" => templates::strategies_content(),
        "trader" => templates::trader_content(),
        "initialization" => templates::initialization_content(),
        "updates" => templates::updates_content(),
        "about" => templates::about_content(),
        "ai" => templates::ai_content(),
        _ => {
            // Escape page name to prevent XSS
            let escaped_page = page
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#x27;");
            format!(
                "<div style=\"padding:2rem;text-align:center;\">
                    <h1 style=\"color:#ef4444;\">Page Not Found</h1>
                    <p style=\"color:#9ca3af;margin-top:1rem;\">Page '{}' does not exist.</p>
                </div>",
                escaped_page
            )
        }
    };

    Html(content)
}
