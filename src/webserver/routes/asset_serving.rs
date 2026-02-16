use crate::webserver::templates;
use axum::{
    http::{header as http_header, StatusCode},
    response::{IntoResponse, Response},
};

/// Serve core JavaScript modules
pub async fn get_core_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "lifecycle.js" => Some(templates::CORE_LIFECYCLE),
        "app_state.js" => Some(templates::CORE_APP_STATE),
        "poller.js" => Some(templates::CORE_POLLER),
        "dom.js" => Some(templates::CORE_DOM),
        "utils.js" => Some(templates::CORE_UTILS),
        "bootstrap.js" => Some(templates::CORE_BOOTSTRAP),
        "router.js" => Some(templates::CORE_ROUTER),
        "header.js" => Some(templates::CORE_HEADER),
        "notifications.js" => Some(templates::CORE_NOTIFICATIONS),
        "toast.js" => Some(templates::CORE_TOAST),
        "request_manager.js" => Some(templates::CORE_REQUEST_MANAGER),
        "splash.js" => Some(templates::CORE_SPLASH),
        "onboarding.js" => Some(templates::CORE_ONBOARDING),
        "setup.js" => Some(templates::CORE_SETUP),
        "status_bar.js" => Some(templates::CORE_STATUS_BAR),
        "hints.js" => Some(templates::CORE_HINTS),
        "lockscreen.js" => Some(templates::CORE_LOCKSCREEN),
        "sounds.js" => Some(templates::CORE_SOUNDS),
        "global_chat.js" => Some(templates::CORE_GLOBAL_CHAT),
        "chat_widget.js" => Some(templates::CORE_CHAT_WIDGET),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                http_header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve page JavaScript modules
pub async fn get_page_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "home.js" => Some(templates::HOME_PAGE_SCRIPT),
        "services.js" => Some(templates::SERVICES_PAGE_SCRIPT),
        "transactions.js" => Some(templates::TRANSACTIONS_PAGE_SCRIPT),
        "events.js" => Some(templates::EVENTS_PAGE_SCRIPT),
        "tokens.js" => Some(templates::TOKENS_PAGE_SCRIPT),
        "tokens/constants.js" => Some(templates::TOKENS_CONSTANTS_JS),
        "tokens/formatters.js" => Some(templates::TOKENS_FORMATTERS_JS),
        "tokens/ohlcv.js" => Some(templates::TOKENS_OHLCV_JS),
        "tokens/favorites.js" => Some(templates::TOKENS_FAVORITES_JS),
        "positions.js" => Some(templates::POSITIONS_PAGE_SCRIPT),
        "filtering.js" => Some(templates::FILTERING_PAGE_SCRIPT),
        "filtering/config_metadata.js" => Some(templates::FILTERING_CONFIG_METADATA_JS),
        "filtering/renderers.js" => Some(templates::FILTERING_RENDERERS_JS),
        "config.js" => Some(templates::CONFIG_PAGE_SCRIPT),
        "config/utils.js" => Some(templates::CONFIG_UTILS_JS),
        "config/field_renderers.js" => Some(templates::CONFIG_FIELD_RENDERERS_JS),
        "strategies.js" => Some(templates::STRATEGIES_PAGE_SCRIPT),
        "strategies/condition_editor.js" => Some(templates::STRATEGIES_CONDITION_EDITOR_JS),
        "strategies/condition_catalog.js" => Some(templates::STRATEGIES_CONDITION_CATALOG_JS),
        "trader.js" => Some(templates::TRADER_PAGE_SCRIPT),
        "trader/examples.js" => Some(templates::TRADER_EXAMPLES_JS),
        "trader/controls.js" => Some(templates::TRADER_CONTROLS_JS),
        "trader/features.js" => Some(templates::TRADER_FEATURES_JS),
        "wallets.js" => Some(templates::WALLETS_PAGE_SCRIPT),
        "tools.js" => Some(templates::TOOLS_PAGE_SCRIPT),
        "tools/wallet_tools.js" => Some(templates::TOOLS_WALLET_TOOLS),
        "tools/token_tools.js" => Some(templates::TOOLS_TOKEN_TOOLS),
        "tools/trading_tools.js" => Some(templates::TOOLS_TRADING_TOOLS),
        "tools/multi_wallet_tools.js" => Some(templates::TOOLS_MULTI_WALLET_TOOLS),
        "ai.js" => Some(templates::AI_PAGE_SCRIPT),
        "ai/providers_tab.js" => Some(templates::AI_PROVIDERS_TAB),
        "ai/instructions_tab.js" => Some(templates::AI_INSTRUCTIONS_TAB),
        "ai/automation_tab.js" => Some(templates::AI_AUTOMATION_TAB),
        "updates.js" => Some(templates::UPDATES_PAGE_SCRIPT),
        "about.js" => Some(templates::ABOUT_PAGE_SCRIPT),
        "login.js" => Some(templates::LOGIN_PAGE_SCRIPT),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                http_header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve UI component JavaScript modules
pub async fn get_ui_script(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content = match file.as_str() {
        "data_table.js" => Some(templates::DATA_TABLE_UI),
        "data_table/column_management.js" => Some(templates::DATA_TABLE_COLUMN_MANAGEMENT),
        "data_table/client_pagination.js" => Some(templates::DATA_TABLE_CLIENT_PAGINATION),
        "data_table/server_pagination.js" => Some(templates::DATA_TABLE_SERVER_PAGINATION),
        "dropdown.js" => Some(templates::DROPDOWN_UI),
        "table_toolbar.js" => Some(templates::TABLE_TOOLBAR_UI),
        "toast.js" => Some(templates::TOAST_UI),
        "events_dialog.js" => Some(templates::EVENTS_DIALOG_UI),
        "confirmation_dialog.js" => Some(templates::CONFIRMATION_DIALOG_UI),
        "trade_action_dialog.js" => Some(templates::TRADE_ACTION_DIALOG_UI),
        "trade_action/quick_trade.js" => Some(templates::TRADE_ACTION_QUICK_TRADE_JS),
        "trade_action/quote_manager.js" => Some(templates::TRADE_ACTION_QUOTE_MANAGER_JS),
        "tab_bar.js" => Some(templates::TAB_BAR_UI),
        "action_bar.js" => Some(templates::ACTION_BAR_UI),
        "table_settings_dialog.js" => Some(templates::TABLE_SETTINGS_DIALOG_UI),
        "token_details_dialog.js" => Some(templates::TOKEN_DETAILS_DIALOG_UI),
        "token_details/overview_tab.js" => Some(templates::TOKEN_DETAILS_OVERVIEW_TAB_UI),
        "token_details/security_tab.js" => Some(templates::TOKEN_DETAILS_SECURITY_TAB_UI),
        "token_details/pools_links_tab.js" => Some(templates::TOKEN_DETAILS_POOLS_LINKS_TAB_UI),
        "transaction_details_dialog.js" => Some(templates::TRANSACTION_DETAILS_DIALOG_UI),
        "position_details_dialog.js" => Some(templates::POSITION_DETAILS_DIALOG_UI),
        "position_details/analytics_tab.js" => Some(templates::POSITION_DETAILS_ANALYTICS_TAB_JS),
        "position_details/secondary_tabs.js" => {
            Some(templates::POSITION_DETAILS_SECONDARY_TABS_JS)
        }
        "tool_favorites.js" => Some(templates::TOOL_FAVORITES_UI),
        "context_menu.js" => Some(templates::CONTEXT_MENU_UI),
        "advanced_chart.js" => Some(templates::ADVANCED_CHART_UI),
        "settings_dialog.js" => Some(templates::SETTINGS_DIALOG_UI),
        "settings/security_tab.js" => Some(templates::SETTINGS_SECURITY_TAB_UI),
        "settings/data_tab.js" => Some(templates::SETTINGS_DATA_TAB_UI),
        "settings/updates_tab.js" => Some(templates::SETTINGS_UPDATES_TAB_UI),
        "notification_panel.js" => Some(templates::NOTIFICATION_PANEL_UI),
        "hint_popover.js" => Some(templates::HINT_POPOVER_UI),
        "search_dialog.js" => Some(templates::SEARCH_DIALOG_UI),
        "custom_select.js" => Some(templates::CUSTOM_SELECT_UI),
        "billboard_dialog.js" => Some(templates::BILLBOARD_DIALOG_UI),
        "billboard_row.js" => Some(templates::BILLBOARD_ROW_UI),
        "pool_selector.js" => Some(templates::POOL_SELECTOR_UI),
        "exit_dialog.js" => Some(templates::EXIT_DIALOG_UI),
        "config_import_export_dialog.js" => Some(templates::CONFIG_IMPORT_EXPORT_DIALOG_UI),
        "input_dialog.js" => Some(templates::INPUT_DIALOG_UI),
        _ => None,
    };

    match content {
        Some(js) => (
            StatusCode::OK,
            [(
                http_header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            js,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve static assets (logos, icons)
pub async fn get_asset(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    match file.as_str() {
        "logo.svg" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            templates::LOGO_SVG,
        )
            .into_response(),
        "logo.png" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/png")],
            templates::LOGO_PNG,
        )
            .into_response(),
        "lightweight-charts.js" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "application/javascript")],
            templates::LIGHTWEIGHT_CHARTS_JS,
        )
            .into_response(),
        _ => (StatusCode::NOT_FOUND, "Asset not found").into_response(),
    }
}

/// Serve AI provider logos
pub async fn get_provider_logo(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    let content_type = [(http_header::CONTENT_TYPE, "image/png")];
    match file.as_str() {
        "openai.png" => (StatusCode::OK, content_type, templates::PROVIDER_OPENAI).into_response(),
        "anthropic.png" => {
            (StatusCode::OK, content_type, templates::PROVIDER_ANTHROPIC).into_response()
        }
        "groq.png" => (StatusCode::OK, content_type, templates::PROVIDER_GROQ).into_response(),
        "deepseek.png" => {
            (StatusCode::OK, content_type, templates::PROVIDER_DEEPSEEK).into_response()
        }
        "gemini.png" => (StatusCode::OK, content_type, templates::PROVIDER_GEMINI).into_response(),
        "ollama.png" => (StatusCode::OK, content_type, templates::PROVIDER_OLLAMA).into_response(),
        "together.png" => {
            (StatusCode::OK, content_type, templates::PROVIDER_TOGETHER).into_response()
        }
        "openrouter.png" => {
            (StatusCode::OK, content_type, templates::PROVIDER_OPENROUTER).into_response()
        }
        "mistral.png" => {
            (StatusCode::OK, content_type, templates::PROVIDER_MISTRAL).into_response()
        }
        _ => (StatusCode::NOT_FOUND, "Provider logo not found").into_response(),
    }
}

/// Serve fonts (Lucide icons, JetBrains Mono, Orbitron)
pub async fn get_font(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    match file.as_str() {
        // Lucide icon font
        "lucide.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            templates::LUCIDE_FONT_WOFF2,
        )
            .into_response(),
        "lucide.woff" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff")],
            templates::LUCIDE_FONT_WOFF,
        )
            .into_response(),
        "lucide.ttf" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/ttf")],
            templates::LUCIDE_FONT_TTF,
        )
            .into_response(),
        "lucide.eot" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "application/vnd.ms-fontobject")],
            templates::LUCIDE_FONT_EOT,
        )
            .into_response(),
        "lucide.svg" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            templates::LUCIDE_FONT_SVG,
        )
            .into_response(),
        // JetBrains Mono - tabular numbers for trading data
        "JetBrainsMono-Regular.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            templates::JETBRAINS_MONO_REGULAR,
        )
            .into_response(),
        "JetBrainsMono-Medium.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            templates::JETBRAINS_MONO_MEDIUM,
        )
            .into_response(),
        "JetBrainsMono-Bold.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            templates::JETBRAINS_MONO_BOLD,
        )
            .into_response(),
        // Orbitron - futuristic branding font
        "Orbitron-Variable.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            templates::ORBITRON_VARIABLE,
        )
            .into_response(),
        _ => (StatusCode::NOT_FOUND, "Font not found").into_response(),
    }
}
