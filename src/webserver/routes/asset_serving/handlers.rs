use crate::webserver::{embeds, templates};
use axum::{
    extract::Path,
    http::{header as http_header, StatusCode},
    response::{IntoResponse, Response},
};
use regex::Regex;
use std::sync::LazyLock;

/// Rewrite ES module import paths to include the asset version query parameter.
/// This ensures browser module cache is busted on version changes.
fn version_js_imports(js: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(from\s+["'])([^"']+\.js)(["'])"#).unwrap());

    let version = crate::version::get_version();
    let ts = option_env!("ASSET_VERSION_TS").unwrap_or("0");
    let ver = format!("{version}-{ts}");

    RE.replace_all(js, |caps: &regex::Captures| {
        format!("{}{}?v={}{}", &caps[1], &caps[2], ver, &caps[3])
    })
    .into_owned()
}

/// Serve a JS module with versioned imports.
///
/// Dashboard assets are embedded in the binary and served over localhost, so there is
/// no benefit to browser caching them — and stale caching is actively harmful: the
/// cache-buster is the (intentionally frozen) app version, so a rebuilt binary would
/// otherwise keep serving the previously cached JS (CSS is inlined in the page HTML, so
/// it is always fresh — this is why CSS changes appeared but JS changes did not). Send
/// `no-store` so every load fetches the current embedded script.
fn serve_js(content: &str) -> Response {
    let versioned = version_js_imports(content);
    (
        StatusCode::OK,
        [
            (
                http_header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (
                http_header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate",
            ),
        ],
        versioned,
    )
        .into_response()
}

fn serve_css(content: String) -> Response {
    (
        StatusCode::OK,
        [
            (http_header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (
                http_header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate",
            ),
        ],
        content,
    )
        .into_response()
}

/// Serve the isolated stylesheet bundle for one routable dashboard page.
pub async fn get_page_styles(Path(page): Path<String>) -> Response {
    let page = page.strip_suffix(".css").unwrap_or(&page);
    match templates::page_styles(page) {
        Some(css) => serve_css(css),
        None => (StatusCode::NOT_FOUND, "Page stylesheet not found").into_response(),
    }
}

/// Serve core JavaScript modules
pub async fn get_core_script(Path(file): Path<String>) -> Response {
    let content = match file.as_str() {
        "lifecycle.js" => Some(embeds::CORE_LIFECYCLE),
        "menu_manager.js" => Some(embeds::CORE_MENU_MANAGER),
        "app_state.js" => Some(embeds::CORE_APP_STATE),
        "poller.js" => Some(embeds::CORE_POLLER),
        "escape_stack.js" => Some(embeds::CORE_ESCAPE_STACK),
        "token_accent.js" => Some(embeds::CORE_TOKEN_ACCENT),
        "boosts.js" => Some(embeds::CORE_BOOSTS),
        "dom.js" => Some(embeds::CORE_DOM),
        "utils.js" => Some(embeds::CORE_UTILS),
        "bootstrap.js" => Some(embeds::CORE_BOOTSTRAP),
        "router.js" => Some(embeds::CORE_ROUTER),
        "connectivity_watcher.js" => Some(embeds::CORE_CONNECTIVITY_WATCHER),
        "tooltip.js" => Some(embeds::CORE_TOOLTIP),
        "header.js" => Some(embeds::CORE_HEADER),
        "header_metrics.js" => Some(embeds::CORE_HEADER_METRICS),
        "notifications.js" => Some(embeds::CORE_NOTIFICATIONS),
        "toast.js" => Some(embeds::CORE_TOAST),
        "action_toasts.js" => Some(embeds::CORE_ACTION_TOASTS),
        "agent_approvals.js" => Some(embeds::CORE_AGENT_APPROVALS),
        "request_manager.js" => Some(embeds::CORE_REQUEST_MANAGER),
        "client_ready.js" => Some(embeds::CORE_CLIENT_READY),
        "brand_text.js" => Some(embeds::CORE_BRAND_TEXT),
        "splash.js" => Some(embeds::CORE_SPLASH),
        "onboarding.js" => Some(embeds::CORE_ONBOARDING),
        "setup_runtime.js" => Some(embeds::CORE_SETUP_RUNTIME),
        "setup.js" => Some(embeds::CORE_SETUP),
        "status_bar.js" => Some(embeds::CORE_STATUS_BAR),
        "hints.js" => Some(embeds::CORE_HINTS),
        "lockscreen.js" => Some(embeds::CORE_LOCKSCREEN),
        "sounds.js" => Some(embeds::CORE_SOUNDS),
        "global_chat.js" => Some(embeds::CORE_GLOBAL_CHAT),
        "chat_widget.js" => Some(embeds::CORE_CHAT_WIDGET),
        _ => None,
    };

    match content {
        Some(js) => serve_js(js),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve the Promo Studio runtime modules.
///
/// Refuses to serve anything unless the binary was started with `--promo-capture`,
/// so a normal install never exposes the automation surface even though the code
/// ships in the same binary.
pub async fn get_promo_script(Path(file): Path<String>) -> Response {
    if !crate::webserver::promo::is_promo_capture_enabled() {
        return (StatusCode::NOT_FOUND, "Script not found").into_response();
    }

    let content = match file.as_str() {
        "runtime.js" => Some(embeds::PROMO_RUNTIME),
        "overlay.js" => Some(embeds::PROMO_OVERLAY),
        "audio.js" => Some(embeds::PROMO_AUDIO),
        _ => None,
    };

    match content {
        Some(js) => serve_js(js),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve page JavaScript modules
pub async fn get_page_script(Path(file): Path<String>) -> Response {
    let file = file.strip_prefix('/').unwrap_or(&file);
    let content = match file {
        "home.js" => Some(embeds::HOME_PAGE_SCRIPT),
        "home/portfolio_calendar.js" => Some(embeds::HOME_CALENDAR_JS),
        "services.js" => Some(embeds::SERVICES_PAGE_SCRIPT),
        "transactions.js" => Some(embeds::TRANSACTIONS_PAGE_SCRIPT),
        "events.js" => Some(embeds::EVENTS_PAGE_SCRIPT),
        "tokens.js" => Some(embeds::TOKENS_PAGE_SCRIPT),
        "tokens/constants.js" => Some(embeds::TOKENS_CONSTANTS_JS),
        "tokens/formatters.js" => Some(embeds::TOKENS_FORMATTERS_JS),
        "tokens/ohlcv.js" => Some(embeds::TOKENS_OHLCV_JS),
        "tokens/favorites.js" => Some(embeds::TOKENS_FAVORITES_JS),
        "positions.js" => Some(embeds::POSITIONS_PAGE_SCRIPT),
        "filtering.js" => Some(embeds::FILTERING_PAGE_SCRIPT),
        "filtering/config_metadata.js" => Some(embeds::FILTERING_CONFIG_METADATA_JS),
        "filtering/renderers.js" => Some(embeds::FILTERING_RENDERERS_JS),
        "config.js" => Some(embeds::CONFIG_PAGE_SCRIPT),
        "config/utils.js" => Some(embeds::CONFIG_UTILS_JS),
        "config/field_renderers.js" => Some(embeds::CONFIG_FIELD_RENDERERS_JS),
        "strategies.js" => Some(embeds::STRATEGIES_PAGE_SCRIPT),
        "strategies/condition_editor.js" => Some(embeds::STRATEGIES_CONDITION_EDITOR_JS),
        "strategies/condition_catalog.js" => Some(embeds::STRATEGIES_CONDITION_CATALOG_JS),
        "trader.js" => Some(embeds::TRADER_PAGE_SCRIPT),
        "trader/examples.js" => Some(embeds::TRADER_EXAMPLES_JS),
        "trader/controls.js" => Some(embeds::TRADER_CONTROLS_JS),
        "trader/config_cards.js" => Some(embeds::TRADER_CONFIG_CARDS_JS),
        "trader/features.js" => Some(embeds::TRADER_FEATURES_JS),
        "trader/wallet_copy.js" => Some(embeds::TRADER_WALLET_COPY_JS),
        "wallets.js" => Some(embeds::WALLETS_PAGE_SCRIPT),
        "wallets/bulk_operations.js" => Some(embeds::WALLETS_BULK_OPERATIONS_JS),
        "wallets/renderers.js" => Some(embeds::WALLETS_RENDERERS_JS),
        "wallets/watched.js" => Some(embeds::WALLETS_WATCHED_JS),
        "tools.js" => Some(embeds::TOOLS_PAGE_SCRIPT),
        "tools/wallet_tools.js" => Some(embeds::TOOLS_WALLET_TOOLS),
        "tools/token_tools.js" => Some(embeds::TOOLS_TOKEN_TOOLS),
        "tools/trading_tools.js" => Some(embeds::TOOLS_TRADING_TOOLS),
        "tools/multi_wallet_tools.js" => Some(embeds::TOOLS_MULTI_WALLET_TOOLS),
        "assistant.js" => Some(embeds::ASSISTANT_PAGE_SCRIPT),
        "assistant/config_contract.js" => Some(embeds::ASSISTANT_CONFIG_CONTRACT),
        "assistant/providers_tab.js" => Some(embeds::ASSISTANT_PROVIDERS_TAB),
        "assistant/instructions_tab.js" => Some(embeds::ASSISTANT_INSTRUCTIONS_TAB),
        "assistant/automation_tab.js" => Some(embeds::ASSISTANT_AUTOMATION_TAB),
        "login.js" => Some(embeds::LOGIN_PAGE_SCRIPT),
        _ => None,
    };

    match content {
        Some(js) => serve_js(js),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve UI component JavaScript modules
pub async fn get_ui_script(Path(file): Path<String>) -> Response {
    let file = file.strip_prefix('/').unwrap_or(&file);
    let content = match file {
        "data_table.js" => Some(embeds::DATA_TABLE_UI),
        "data_table/column_management.js" => Some(embeds::DATA_TABLE_COLUMN_MANAGEMENT),
        "data_table/client_pagination.js" => Some(embeds::DATA_TABLE_CLIENT_PAGINATION),
        "data_table/server_pagination.js" => Some(embeds::DATA_TABLE_SERVER_PAGINATION),
        "data_table/event_handlers.js" => Some(embeds::DATA_TABLE_EVENT_HANDLERS),
        "table_toolbar.js" => Some(embeds::TABLE_TOOLBAR_UI),
        "toast.js" => Some(embeds::TOAST_UI),
        "quick_trade_shortcuts.js" => Some(embeds::QUICK_TRADE_SHORTCUTS),
        "events_dialog.js" => Some(embeds::EVENTS_DIALOG_UI),
        "confirmation_dialog.js" => Some(embeds::CONFIRMATION_DIALOG_UI),
        "position_remove_dialog.js" => Some(embeds::POSITION_REMOVE_DIALOG_UI),
        "setup_dialog.js" => Some(embeds::SETUP_DIALOG_UI),
        "trade_action_dialog.js" => Some(embeds::TRADE_ACTION_DIALOG_UI),
        "manual_trade.js" => Some(embeds::MANUAL_TRADE_UI),
        "trade_action/quick_trade.js" => Some(embeds::TRADE_ACTION_QUICK_TRADE_JS),
        "trade_action/quote_manager.js" => Some(embeds::TRADE_ACTION_QUOTE_MANAGER_JS),
        "tab_bar.js" => Some(embeds::TAB_BAR_UI),
        "dialog_tab_bar.js" => Some(embeds::DIALOG_TAB_BAR_UI),
        "action_bar.js" => Some(embeds::ACTION_BAR_UI),
        "expand_toggle.js" => Some(embeds::EXPAND_TOGGLE_UI),
        "table_settings_dialog.js" => Some(embeds::TABLE_SETTINGS_DIALOG_UI),
        "token_details_dialog.js" => Some(embeds::TOKEN_DETAILS_DIALOG_UI),
        "image_lightbox.js" => Some(embeds::IMAGE_LIGHTBOX_UI),
        "token_identity.js" => Some(embeds::TOKEN_IDENTITY_UI),
        "token_details/overview_tab.js" => Some(embeds::TOKEN_DETAILS_OVERVIEW_TAB_UI),
        "token_details/security_tab.js" => Some(embeds::TOKEN_DETAILS_SECURITY_TAB_UI),
        "token_details/pools_links_tab.js" => Some(embeds::TOKEN_DETAILS_POOLS_LINKS_TAB_UI),
        "token_details/trade_actions.js" => Some(embeds::TOKEN_DETAILS_TRADE_ACTIONS_UI),
        "token_details/transactions_tab.js" => Some(embeds::TOKEN_DETAILS_TRANSACTIONS_TAB_UI),
        "token_details/chart_tab.js" => Some(embeds::TOKEN_DETAILS_CHART_TAB_UI),
        "token_details/utilities.js" => Some(embeds::TOKEN_DETAILS_UTILITIES_UI),
        "token_details/state_handling.js" => Some(embeds::TOKEN_DETAILS_STATE_HANDLING_UI),
        "token_details/positions_tab.js" => Some(embeds::TOKEN_DETAILS_POSITIONS_TAB_UI),
        "transaction_details_dialog.js" => Some(embeds::TRANSACTION_DETAILS_DIALOG_UI),
        "position_details_dialog.js" => Some(embeds::POSITION_DETAILS_DIALOG_UI),
        "position_details/activity_tab.js" => Some(embeds::POSITION_DETAILS_ACTIVITY_TAB_JS),
        "position_details/activity_event.js" => Some(embeds::POSITION_DETAILS_ACTIVITY_EVENT_JS),
        "position_details/overview_tab.js" => Some(embeds::POSITION_DETAILS_OVERVIEW_TAB_JS),
        "position_details/chart_tab.js" => Some(embeds::POSITION_DETAILS_CHART_TAB_JS),
        "position_details/utilities.js" => Some(embeds::POSITION_DETAILS_UTILITIES_JS),
        "tool_favorites.js" => Some(embeds::TOOL_FAVORITES_UI),
        "context_menu.js" => Some(embeds::CONTEXT_MENU_UI),
        "context_menu/actions.js" => Some(embeds::CONTEXT_MENU_ACTIONS_JS),
        "context_menu/builders.js" => Some(embeds::CONTEXT_MENU_BUILDERS_JS),
        "advanced_chart.js" => Some(embeds::ADVANCED_CHART_UI),
        "advanced_chart/indicators.js" => Some(embeds::ADVANCED_CHART_INDICATORS_JS),
        "advanced_chart/themes.js" => Some(embeds::ADVANCED_CHART_THEMES_JS),
        "chart_data.js" => Some(embeds::CHART_DATA_JS),
        "settings_dialog.js" => Some(embeds::SETTINGS_DIALOG_UI),
        "account/panel.js" => Some(embeds::ACCOUNT_PANEL_UI),
        "settings/security_tab.js" => Some(embeds::SETTINGS_SECURITY_TAB_UI),
        "settings/account_tab.js" => Some(embeds::SETTINGS_ACCOUNT_TAB_UI),
        "settings/data_tab.js" => Some(embeds::SETTINGS_DATA_TAB_UI),
        "settings/updates_tab.js" => Some(embeds::SETTINGS_UPDATES_TAB_UI),
        "settings/interface_tab.js" => Some(embeds::SETTINGS_INTERFACE_TAB_UI),
        "settings/hints_tab.js" => Some(embeds::SETTINGS_HINTS_TAB_UI),
        "settings/agent_connections_tab.js" => Some(embeds::SETTINGS_AGENT_CONNECTIONS_TAB_UI),
        "settings/navigation_tab.js" => Some(embeds::SETTINGS_NAVIGATION_TAB_UI),
        "settings/licenses_tab.js" => Some(embeds::SETTINGS_LICENSES_TAB_UI),
        "settings/telegram_tab.js" => Some(embeds::SETTINGS_TELEGRAM_TAB_UI),
        "notification_panel.js" => Some(embeds::NOTIFICATION_PANEL_UI),
        "hint_popover.js" => Some(embeds::HINT_POPOVER_UI),
        "search_dialog.js" => Some(embeds::SEARCH_DIALOG_UI),
        "custom_select.js" => Some(embeds::CUSTOM_SELECT_UI),
        "number_field.js" => Some(embeds::NUMBER_FIELD_UI),
        "featured_dialog.js" => Some(embeds::FEATURED_DIALOG_UI),
        "featured_row.js" => Some(embeds::FEATURED_ROW_UI),
        "pool_selector.js" => Some(embeds::POOL_SELECTOR_UI),
        "exit_dialog.js" => Some(embeds::EXIT_DIALOG_UI),
        "config_import_export_dialog.js" => Some(embeds::CONFIG_IMPORT_EXPORT_DIALOG_UI),
        "input_dialog.js" => Some(embeds::INPUT_DIALOG_UI),
        _ => None,
    };

    match content {
        Some(js) => serve_js(js),
        None => (StatusCode::NOT_FOUND, "Script not found").into_response(),
    }
}

/// Serve static assets (logos, icons)
pub async fn get_asset(Path(file): Path<String>) -> Response {
    match file.as_str() {
        "logo.svg" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            embeds::LOGO_SVG,
        )
            .into_response(),
        "logo.png" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/png")],
            embeds::LOGO_PNG,
        )
            .into_response(),
        "google-g.png" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/png")],
            embeds::GOOGLE_G_PNG,
        )
            .into_response(),
        "lightweight-charts.js" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "application/javascript")],
            embeds::LIGHTWEIGHT_CHARTS_JS,
        )
            .into_response(),
        _ => (StatusCode::NOT_FOUND, "Asset not found").into_response(),
    }
}

/// Serve the official Solana brand assets (solana.com/branding), embedded in the binary.
pub async fn get_solana_asset(Path(file): Path<String>) -> Response {
    let svg = [(http_header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")];
    let png = [(http_header::CONTENT_TYPE, "image/png")];
    match file.as_str() {
        "solanaLogoMark.svg" => (StatusCode::OK, svg, embeds::SOLANA_LOGO_MARK_SVG).into_response(),
        "solanaLogoMark.png" => (StatusCode::OK, png, embeds::SOLANA_LOGO_MARK_PNG).into_response(),
        "solanaWordMark.svg" => (StatusCode::OK, svg, embeds::SOLANA_WORD_MARK_SVG).into_response(),
        "solanaWordMark.png" => (StatusCode::OK, png, embeds::SOLANA_WORD_MARK_PNG).into_response(),
        "solanaLogo.svg" => (StatusCode::OK, svg, embeds::SOLANA_LOGO_SVG).into_response(),
        "solanaLogo.png" => (StatusCode::OK, png, embeds::SOLANA_LOGO_PNG).into_response(),
        "solanaVerticalLogo.svg" => {
            (StatusCode::OK, svg, embeds::SOLANA_VERTICAL_LOGO_SVG).into_response()
        }
        "solanaVerticalLogo.png" => {
            (StatusCode::OK, png, embeds::SOLANA_VERTICAL_LOGO_PNG).into_response()
        }
        "solanaFoundationLogo.svg" => {
            (StatusCode::OK, svg, embeds::SOLANA_FOUNDATION_LOGO_SVG).into_response()
        }
        "solanaFoundationLogo.png" => {
            (StatusCode::OK, png, embeds::SOLANA_FOUNDATION_LOGO_PNG).into_response()
        }
        _ => (StatusCode::NOT_FOUND, "Solana asset not found").into_response(),
    }
}

/// Serve LLM provider logos
pub async fn get_provider_logo(Path(file): Path<String>) -> Response {
    let content_type = [(http_header::CONTENT_TYPE, "image/png")];
    match file.as_str() {
        "openai.png" => (StatusCode::OK, content_type, embeds::PROVIDER_OPENAI).into_response(),
        "anthropic.png" => {
            (StatusCode::OK, content_type, embeds::PROVIDER_ANTHROPIC).into_response()
        }
        "groq.png" => (StatusCode::OK, content_type, embeds::PROVIDER_GROQ).into_response(),
        "deepseek.png" => (StatusCode::OK, content_type, embeds::PROVIDER_DEEPSEEK).into_response(),
        "gemini.png" => (StatusCode::OK, content_type, embeds::PROVIDER_GEMINI).into_response(),
        "ollama.png" => (StatusCode::OK, content_type, embeds::PROVIDER_OLLAMA).into_response(),
        "together.png" => (StatusCode::OK, content_type, embeds::PROVIDER_TOGETHER).into_response(),
        "openrouter.png" => {
            (StatusCode::OK, content_type, embeds::PROVIDER_OPENROUTER).into_response()
        }
        "mistral.png" => (StatusCode::OK, content_type, embeds::PROVIDER_MISTRAL).into_response(),
        _ => (StatusCode::NOT_FOUND, "Provider logo not found").into_response(),
    }
}

/// Serve fonts (Lucide icons, Inter, JetBrains Mono, Orbitron)
pub async fn get_font(Path(file): Path<String>) -> Response {
    match file.as_str() {
        // Lucide icon font
        "lucide.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::LUCIDE_FONT_WOFF2,
        )
            .into_response(),
        "lucide.woff" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff")],
            embeds::LUCIDE_FONT_WOFF,
        )
            .into_response(),
        "lucide.ttf" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/ttf")],
            embeds::LUCIDE_FONT_TTF,
        )
            .into_response(),
        "lucide.eot" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "application/vnd.ms-fontobject")],
            embeds::LUCIDE_FONT_EOT,
        )
            .into_response(),
        "lucide.svg" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            embeds::LUCIDE_FONT_SVG,
        )
            .into_response(),
        // Inter - human-readable interface text
        "Inter-Variable.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::INTER_VARIABLE,
        )
            .into_response(),
        "Inter-VariableItalic.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::INTER_VARIABLE_ITALIC,
        )
            .into_response(),
        // JetBrains Mono - tabular numbers for trading data
        "JetBrainsMono-Regular.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::JETBRAINS_MONO_REGULAR,
        )
            .into_response(),
        "JetBrainsMono-Medium.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::JETBRAINS_MONO_MEDIUM,
        )
            .into_response(),
        "JetBrainsMono-Bold.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::JETBRAINS_MONO_BOLD,
        )
            .into_response(),
        // Orbitron - futuristic branding font
        "Orbitron-Variable.woff2" => (
            StatusCode::OK,
            [(http_header::CONTENT_TYPE, "font/woff2")],
            embeds::ORBITRON_VARIABLE,
        )
            .into_response(),
        _ => (StatusCode::NOT_FOUND, "Font not found").into_response(),
    }
}
