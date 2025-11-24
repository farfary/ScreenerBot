/// HTML templates for the webserver dashboard
///
/// Templates are stored in `src/webserver/templates/` and embedded at compile time
/// using `include_str!`. This keeps the Rust module focused on wiring helpers
/// while HTML, CSS, and JavaScript live in dedicated files.
use crate::arguments;

const BASE_TEMPLATE: &str = include_str!("templates/base.html");
const FOUNDATION_STYLES: &str = include_str!("templates/styles/foundation.css");
const LAYOUT_STYLES: &str = include_str!("templates/styles/layout.css");
const COMPONENT_STYLES: &str = include_str!("templates/styles/components.css");
const HEADER_STYLES: &str = include_str!("templates/styles/header.css");
const DROPDOWN_STYLES: &str = include_str!("templates/styles/dropdown.css");
const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
const NOTIFICATION_STYLES: &str = include_str!("templates/styles/components/notifications.css");
const TOAST_STYLES: &str = include_str!("templates/styles/components/toast.css");
const SERVICES_PAGE_STYLES: &str = include_str!("templates/styles/pages/services.css");
const TRANSACTIONS_PAGE_STYLES: &str = include_str!("templates/styles/pages/transactions.css");
const EVENTS_PAGE_STYLES: &str = include_str!("templates/styles/pages/events.css");
const TOKENS_PAGE_STYLES: &str = include_str!("templates/styles/pages/tokens.css");
const POSITIONS_PAGE_STYLES: &str = include_str!("templates/styles/pages/positions.css");
const FILTERING_PAGE_STYLES: &str = include_str!("templates/styles/pages/filtering.css");
const CONFIG_PAGE_STYLES: &str = include_str!("templates/styles/pages/config.css");
const STRATEGIES_PAGE_STYLES: &str = include_str!("templates/styles/pages/strategies.css");
const TRADER_PAGE_STYLES: &str = include_str!("templates/styles/pages/trader.css");
const WALLET_PAGE_STYLES: &str = include_str!("templates/styles/pages/wallet.css");
const INITIALIZATION_PAGE_STYLES: &str = include_str!("templates/styles/pages/initialization.css");
const HOME_PAGE_STYLES: &str = include_str!("templates/styles/pages/home.css");
const DATA_TABLE_STYLES: &str = include_str!("templates/styles/ui/data_table.css");
const TABLE_TOOLBAR_STYLES: &str = include_str!("templates/styles/ui/table_toolbar.css");
const EVENTS_DIALOG_STYLES: &str = include_str!("templates/styles/ui/events_dialog.css");
const TRADE_ACTION_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/trade_action_dialog.css");
const TAB_BAR_STYLES: &str = include_str!("templates/styles/ui/tab_bar.css");
const ACTION_BAR_STYLES: &str = include_str!("templates/styles/ui/action_bar.css");
const TABLE_SETTINGS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/table_settings_dialog.css");
const CONFIRMATION_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/confirmation_dialog.css");
const TOKEN_DETAILS_DIALOG_STYLES: &str = include_str!("templates/styles/token_details_dialog.css");

// Assets (logos, icons)
pub const LOGO_SVG: &str = include_str!("assets/logo.svg");
pub const LOGO_PNG: &[u8] = include_bytes!("assets/logo.png");

// Lucide icon font
const LUCIDE_ICON_CSS: &str = include_str!("assets/lucide-font/lucide.css");
pub const LUCIDE_FONT_WOFF2: &[u8] = include_bytes!("assets/lucide-font/lucide.woff2");
pub const LUCIDE_FONT_WOFF: &[u8] = include_bytes!("assets/lucide-font/lucide.woff");
pub const LUCIDE_FONT_TTF: &[u8] = include_bytes!("assets/lucide-font/lucide.ttf");
pub const LUCIDE_FONT_EOT: &[u8] = include_bytes!("assets/lucide-font/lucide.eot");
pub const LUCIDE_FONT_SVG: &str = include_str!("assets/lucide-font/lucide.svg");

pub const CORE_LIFECYCLE: &str = include_str!("templates/scripts/core/lifecycle.js");
pub const CORE_APP_STATE: &str = include_str!("templates/scripts/core/app_state.js");
pub const CORE_POLLER: &str = include_str!("templates/scripts/core/poller.js");
pub const CORE_DOM: &str = include_str!("templates/scripts/core/dom.js");
pub const CORE_UTILS: &str = include_str!("templates/scripts/core/utils.js");
pub const CORE_ROUTER: &str = include_str!("templates/scripts/core/router.js");
pub const CORE_HEADER: &str = include_str!("templates/scripts/core/header.js");
pub const CORE_NOTIFICATIONS: &str = include_str!("templates/scripts/core/notifications.js");
pub const CORE_TOAST: &str = include_str!("templates/scripts/core/toast.js");
pub const CORE_REQUEST_MANAGER: &str = include_str!("templates/scripts/core/request_manager.js");

const THEME_SCRIPTS: &str = include_str!("templates/scripts/theme.js");

pub const DATA_TABLE_UI: &str = include_str!("templates/scripts/ui/data_table.js");
pub const DROPDOWN_UI: &str = include_str!("templates/scripts/ui/dropdown.js");
pub const TABLE_TOOLBAR_UI: &str = include_str!("templates/scripts/ui/table_toolbar.js");
pub const TOAST_UI: &str = include_str!("templates/scripts/ui/toast.js");
pub const EVENTS_DIALOG_UI: &str = include_str!("templates/scripts/ui/events_dialog.js");
pub const CONFIRMATION_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/confirmation_dialog.js");
pub const TRADE_ACTION_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/trade_action_dialog.js");
pub const TAB_BAR_UI: &str = include_str!("templates/scripts/ui/tab_bar.js");
pub const ACTION_BAR_UI: &str = include_str!("templates/scripts/ui/action_bar.js");
pub const TABLE_SETTINGS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/table_settings_dialog.js");
pub const TOKEN_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/token_details_dialog.js");
pub const NOTIFICATION_PANEL_UI: &str = include_str!("templates/scripts/ui/notification_panel.js");

pub const SERVICES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/services.js");
pub const TRANSACTIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/transactions.js");
pub const EVENTS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/events.js");
pub const TOKENS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tokens.js");
pub const POSITIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/positions.js");
pub const FILTERING_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/filtering.js");
pub const CONFIG_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/config.js");
pub const STRATEGIES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/strategies.js");
pub const TRADER_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/trader.js");
pub const WALLET_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/wallet.js");
pub const INITIALIZATION_PAGE_SCRIPT: &str =
    include_str!("templates/scripts/pages/initialization.js");
pub const HOME_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/home.js");

const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
const TRANSACTIONS_PAGE: &str = include_str!("templates/pages/transactions.html");
const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
const FILTERING_PAGE: &str = include_str!("templates/pages/filtering.html");
const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");
const STRATEGIES_PAGE: &str = include_str!("templates/pages/strategies.html");
const TRADER_PAGE: &str = include_str!("templates/pages/trader.html");
const WALLET_PAGE: &str = include_str!("templates/pages/wallet.html");
const INITIALIZATION_PAGE: &str = include_str!("templates/pages/initialization.html");
const HOME_PAGE: &str = include_str!("templates/pages/home.html");

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);

    // Prepare Lucide icon font CSS with corrected paths
    let lucide_css = LUCIDE_ICON_CSS
        .replace("url('lucide.eot", "url('/assets/fonts/lucide.eot")
        .replace("url('lucide.woff2", "url('/assets/fonts/lucide.woff2")
        .replace("url('lucide.woff", "url('/assets/fonts/lucide.woff")
        .replace("url('lucide.ttf", "url('/assets/fonts/lucide.ttf")
        .replace("url('lucide.svg", "url('/assets/fonts/lucide.svg");

    let mut combined_styles = vec![
        FOUNDATION_STYLES,
        &lucide_css,
        LAYOUT_STYLES,
        HEADER_STYLES,
        COMPONENT_STYLES,
        DROPDOWN_STYLES,
        COMMON_STYLES,
        NOTIFICATION_STYLES,
        TOAST_STYLES,
        DATA_TABLE_STYLES,
        TABLE_TOOLBAR_STYLES,
        EVENTS_DIALOG_STYLES,
        TRADE_ACTION_DIALOG_STYLES,
        TAB_BAR_STYLES,
        ACTION_BAR_STYLES,
        TABLE_SETTINGS_DIALOG_STYLES,
        CONFIRMATION_DIALOG_STYLES,
        TOKEN_DETAILS_DIALOG_STYLES,
    ];
    if active_tab == "services" {
        combined_styles.push(SERVICES_PAGE_STYLES);
    }
    if active_tab == "transactions" {
        combined_styles.push(TRANSACTIONS_PAGE_STYLES);
    }
    if active_tab == "events" {
        combined_styles.push(EVENTS_PAGE_STYLES);
    }
    if active_tab == "tokens" {
        combined_styles.push(TOKENS_PAGE_STYLES);
    }
    if active_tab == "positions" {
        combined_styles.push(POSITIONS_PAGE_STYLES);
    }
    if active_tab == "filtering" {
        combined_styles.push(FILTERING_PAGE_STYLES);
    }
    if active_tab == "config" {
        combined_styles.push(CONFIG_PAGE_STYLES);
    }
    if active_tab == "strategies" {
        combined_styles.push(STRATEGIES_PAGE_STYLES);
    }
    if active_tab == "trader" {
        combined_styles.push(TRADER_PAGE_STYLES);
    }
    if active_tab == "wallet" {
        combined_styles.push(WALLET_PAGE_STYLES);
    }
    if active_tab == "initialization" {
        combined_styles.push(INITIALIZATION_PAGE_STYLES);
    }
    if active_tab == "home" {
        combined_styles.push(HOME_PAGE_STYLES);
    }
    html = html.replace("/*__INJECTED_STYLES__*/", &combined_styles.join("\n"));
    let mut page_style_injections = String::new();
    for (page, styles) in [
        ("services", SERVICES_PAGE_STYLES),
        ("transactions", TRANSACTIONS_PAGE_STYLES),
        ("events", EVENTS_PAGE_STYLES),
        ("tokens", TOKENS_PAGE_STYLES),
        ("positions", POSITIONS_PAGE_STYLES),
        ("filtering", FILTERING_PAGE_STYLES),
        ("config", CONFIG_PAGE_STYLES),
        ("strategies", STRATEGIES_PAGE_STYLES),
        ("trader", TRADER_PAGE_STYLES),
        ("wallet", WALLET_PAGE_STYLES),
        ("initialization", INITIALIZATION_PAGE_STYLES),
        ("home", HOME_PAGE_STYLES),
    ] {
        if styles.trim().is_empty() {
            continue;
        }
        let page_json =
            serde_json::to_string(page).expect("failed to serialize page name for style map");
        let styles_json =
            serde_json::to_string(styles).expect("failed to serialize page styles for style map");
        page_style_injections.push_str(&format!(
            "window.__PAGE_STYLES__[{}] = {};\n",
            page_json, styles_json
        ));
    }
    html = html.replace("/*__PAGE_STYLES__*/", &page_style_injections);
    html = html.replace("/*__THEME_SCRIPTS__*/", THEME_SCRIPTS);
    html
}

fn nav_tabs(active: &str) -> String {
    let tabs = vec![
        ("home", "icon-home", "Home"),
        ("positions", "icon-chart-candlestick", "Positions"),
        ("tokens", "icon-coins", "Tokens"),
        ("filtering", "icon-list-filter", "Filtering"),
        ("wallet", "icon-wallet", "Wallet"),
        ("trader", "icon-bot", "Trader"),
        ("strategies", "icon-target", "Strategies"),
        ("transactions", "icon-activity", "Transactions"),
        ("services", "icon-server", "Services"),
        ("config", "icon-settings", "Config"),
        ("events", "icon-radio-tower", "Events"),
    ];

    tabs.iter()
        .map(|(name, icon_class, label)| {
            let active_class = if *name == active { " active" } else { "" };
            // Use data-page attribute for client-side routing (SPA)
            format!(
                "<a href=\"#\" data-page=\"{}\" class=\"tab{}\"><i class=\"{}\"></i> {}</a>",
                name, active_class, icon_class, label
            )
        })
        .collect::<Vec<_>>()
        .join("\n        ")
}

fn render_page(template: &'static str) -> String {
    template.to_string()
}

pub fn tokens_content() -> String {
    render_page(TOKENS_PAGE)
}

pub fn events_content() -> String {
    render_page(EVENTS_PAGE)
}

pub fn services_content() -> String {
    render_page(SERVICES_PAGE)
}

pub fn transactions_content() -> String {
    render_page(TRANSACTIONS_PAGE)
}

pub fn positions_content() -> String {
    render_page(POSITIONS_PAGE)
}

pub fn filtering_content() -> String {
    render_page(FILTERING_PAGE)
}

pub fn config_content() -> String {
    render_page(CONFIG_PAGE)
}

pub fn strategies_content() -> String {
    render_page(STRATEGIES_PAGE)
}

pub fn trader_content() -> String {
    render_page(TRADER_PAGE)
}

pub fn wallet_content() -> String {
    render_page(WALLET_PAGE)
}

pub fn initialization_content() -> String {
    render_page(INITIALIZATION_PAGE)
}

pub fn home_content() -> String {
    render_page(HOME_PAGE)
}
