use crate::webserver::{state::AppState, templates};
use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Router,
};
use std::sync::Arc;

pub mod blacklist;
pub mod config;
pub mod connectivity;
pub mod dashboard;
pub mod events;
pub mod filtering_api;
pub mod initialization;
pub mod ohlcv;
pub mod positions;
pub mod services;
pub mod status;
pub mod strategies;
pub mod system;
pub mod tokens;
pub mod trader;
pub mod trading;
pub mod transactions;
pub mod wallet;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(services_page))
        .route("/services", axum::routing::get(services_page))
        .route("/tokens", axum::routing::get(tokens_page))
        .route("/positions", axum::routing::get(positions_page))
        .route("/events", axum::routing::get(events_page))
        .route("/transactions", axum::routing::get(transactions_page))
        .route("/filtering", axum::routing::get(filtering_page))
        .route("/wallet", axum::routing::get(wallet_page))
        .route("/config", axum::routing::get(config_page))
        .route("/strategies", axum::routing::get(strategies_page))
        .route("/trader", axum::routing::get(trader_page))
        .route("/initialization", axum::routing::get(initialization_page))
        .route("/scripts/core/:file", axum::routing::get(get_core_script))
        .route("/scripts/pages/:file", axum::routing::get(get_page_script))
        .route("/scripts/ui/:file", axum::routing::get(get_ui_script))
        .nest("/api", api_routes())
        .with_state(state)
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

/// Trader page handler
async fn trader_page() -> Html<String> {
    let content = templates::trader_content();
    Html(templates::base_template("Trader", "trader", &content))
}

/// Wallet page handler
async fn wallet_page() -> Html<String> {
    let content = templates::wallet_content();
    Html(templates::base_template("Wallet", "wallet", &content))
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

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(status::routes())
        .merge(tokens::routes())
        .merge(events::routes())
        .merge(filtering_api::routes())
        .merge(positions::routes())
        .merge(dashboard::routes())
        .merge(wallet::routes())
        .merge(blacklist::routes())
        .merge(config::routes())
        .merge(services::routes())
        .merge(ohlcv::ohlcv_routes())
        .nest("/connectivity", connectivity::routes())
        .nest("/initialization", initialization::routes())
        .nest("/trading", trading::routes())
        .nest("/trader", trader::routes())
        .nest("/system", system::routes())
        .nest("/transactions", transactions::routes())
        .nest("/strategies", strategies::routes())
        .route("/pages/:page", axum::routing::get(get_page_content))
}

/// SPA page content handler - returns just the content HTML (not full template)
async fn get_page_content(axum::extract::Path(page): axum::extract::Path<String>) -> Html<String> {
    let content = match page.as_str() {
        "tokens" => templates::tokens_content(),
        "positions" => templates::positions_content(),
        "events" => templates::events_content(),
        "services" => templates::services_content(),
        "transactions" => templates::transactions_content(),
        "filtering" => templates::filtering_content(),
        "wallet" => templates::wallet_content(),
        "config" => templates::config_content(),
        "strategies" => templates::strategies_content(),
        "trader" => templates::trader_content(),
        "initialization" => templates::initialization_content(),
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

/// Serve core JavaScript modules
async fn get_core_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "lifecycle.js" => Some(templates::CORE_LIFECYCLE),
        "app_state.js" => Some(templates::CORE_APP_STATE),
        "poller.js" => Some(templates::CORE_POLLER),
        "dom.js" => Some(templates::CORE_DOM),
        "utils.js" => Some(templates::CORE_UTILS),
        "router.js" => Some(templates::CORE_ROUTER),
        "header.js" => Some(templates::CORE_HEADER),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve page JavaScript modules
async fn get_page_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "services.js" => Some(templates::SERVICES_PAGE_SCRIPT),
        "transactions.js" => Some(templates::TRANSACTIONS_PAGE_SCRIPT),
        "events.js" => Some(templates::EVENTS_PAGE_SCRIPT),
        "tokens.js" => Some(templates::TOKENS_PAGE_SCRIPT),
        "positions.js" => Some(templates::POSITIONS_PAGE_SCRIPT),
        "filtering.js" => Some(templates::FILTERING_PAGE_SCRIPT),
        "wallet.js" => Some(templates::WALLET_PAGE_SCRIPT),
        "config.js" => Some(templates::CONFIG_PAGE_SCRIPT),
        "strategies.js" => Some(templates::STRATEGIES_PAGE_SCRIPT),
        "trader.js" => Some(templates::TRADER_PAGE_SCRIPT),
        "initialization.js" => Some(templates::INITIALIZATION_PAGE_SCRIPT),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve UI component JavaScript modules
async fn get_ui_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "data_table.js" => Some(templates::DATA_TABLE_UI),
        "dropdown.js" => Some(templates::DROPDOWN_UI),
        "table_toolbar.js" => Some(templates::TABLE_TOOLBAR_UI),
        "events_dialog.js" => Some(templates::EVENTS_DIALOG_UI),
        "trade_action_dialog.js" => Some(templates::TRADE_ACTION_DIALOG_UI),
        "tab_bar.js" => Some(templates::TAB_BAR_UI),
        "table_settings_dialog.js" => Some(templates::TABLE_SETTINGS_DIALOG_UI),
        "token_details_dialog.js" => Some(templates::TOKEN_DETAILS_DIALOG_UI),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}
