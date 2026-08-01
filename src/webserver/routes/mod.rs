//! Web API route registration, static asset serving, and top-level request handlers.
use crate::webserver::{state::AppState, templates};
use axum::{
    extract::Path as AxumPath,
    http::{header as http_header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;

pub mod account;
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
        .route("/", get(home_page))
        .route("/home", get(home_page))
        .route("/login", get(login_page))
        .route("/services", get(services_page))
        .route("/tokens", get(tokens_page))
        .route("/positions", get(positions_page))
        .route("/events", get(events_page))
        .route("/transactions", get(transactions_page))
        .route("/filtering", get(filtering_page))
        .route("/wallets", get(wallets_page))
        .route("/tools", get(tools_page))
        .route("/ai", get(ai_page))
        .route("/config", get(config_page))
        .route("/trader", get(trader_page))
        .route("/initialization", get(initialization_page))
        .route("/oauth/callback", get(account::handlers::oauth_callback))
        .route("/updates", get(updates_page))
        .route("/about", get(about_page))
        .route("/scripts/core/:file", get(get_core_script))
        .route("/scripts/pages/*file", get(get_page_script))
        .route("/scripts/ui/*file", get(get_ui_script))
        .route("/assets/:file", get(get_asset))
        .route("/assets/fonts/:file", get(get_font))
        .route("/assets/providers/:file", get(get_provider_logo))
        .route("/assets/solana/:file", get(get_solana_asset))
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

/// Register all API route groups under `/api`.
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
        .merge(ohlcv::routes())
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
        .nest("/account", account::routes())
        .nest("/telegram", telegram::routes())
        .nest("/ai", ai::routes())
        .merge(updates::routes())
        .route("/pages/:page", get(get_page_content))
}

/// SPA page content handler - returns just the content HTML (not full template)
async fn get_page_content(AxumPath(page): AxumPath<String>) -> Html<String> {
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
