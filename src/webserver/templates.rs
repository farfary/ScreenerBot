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
const DROPDOWN_STYLES: &str = include_str!("templates/styles/dropdown.css");
const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
const SERVICES_PAGE_STYLES: &str = include_str!("templates/styles/pages/services.css");
const TRANSACTIONS_PAGE_STYLES: &str = include_str!("templates/styles/pages/transactions.css");
const EVENTS_PAGE_STYLES: &str = include_str!("templates/styles/pages/events.css");
const TOKENS_PAGE_STYLES: &str = include_str!("templates/styles/pages/tokens.css");
const POSITIONS_PAGE_STYLES: &str = include_str!("templates/styles/pages/positions.css");
const FILTERING_PAGE_STYLES: &str = include_str!("templates/styles/pages/filtering.css");
const CONFIG_PAGE_STYLES: &str = include_str!("templates/styles/pages/config.css");
const STRATEGIES_PAGE_STYLES: &str = include_str!("templates/styles/pages/strategies.css");
const DATA_TABLE_STYLES: &str = include_str!("templates/styles/ui/data_table.css");
const TABLE_TOOLBAR_STYLES: &str = include_str!("templates/styles/ui/table_toolbar.css");
const EVENTS_DIALOG_STYLES: &str = include_str!("templates/styles/ui/events_dialog.css");
const TRADE_ACTION_DIALOG_STYLES: &str = include_str!("templates/styles/ui/trade_action_dialog.css");
const TAB_BAR_STYLES: &str = include_str!("templates/styles/ui/tab_bar.css");
const TABLE_SETTINGS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/table_settings_dialog.css");
const TOKEN_DETAILS_DIALOG_STYLES: &str = include_str!("templates/styles/token_details_dialog.css");

pub const CORE_LIFECYCLE: &str = include_str!("templates/scripts/core/lifecycle.js");
pub const CORE_APP_STATE: &str = include_str!("templates/scripts/core/app_state.js");
pub const CORE_POLLER: &str = include_str!("templates/scripts/core/poller.js");
pub const CORE_DOM: &str = include_str!("templates/scripts/core/dom.js");
pub const CORE_UTILS: &str = include_str!("templates/scripts/core/utils.js");
pub const CORE_ROUTER: &str = include_str!("templates/scripts/core/router.js");
pub const CORE_HEADER: &str = include_str!("templates/scripts/core/header.js");

const THEME_SCRIPTS: &str = include_str!("templates/scripts/theme.js");

pub const DATA_TABLE_UI: &str = include_str!("templates/scripts/ui/data_table.js");
pub const DROPDOWN_UI: &str = include_str!("templates/scripts/ui/dropdown.js");
pub const TABLE_TOOLBAR_UI: &str = include_str!("templates/scripts/ui/table_toolbar.js");
pub const EVENTS_DIALOG_UI: &str = include_str!("templates/scripts/ui/events_dialog.js");
pub const TRADE_ACTION_DIALOG_UI: &str = include_str!("templates/scripts/ui/trade_action_dialog.js");
pub const TAB_BAR_UI: &str = include_str!("templates/scripts/ui/tab_bar.js");
pub const TABLE_SETTINGS_DIALOG_UI: &str = include_str!("templates/scripts/ui/table_settings_dialog.js");
pub const TOKEN_DETAILS_DIALOG_UI: &str = include_str!("templates/scripts/ui/token_details_dialog.js");

pub const SERVICES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/services.js");
pub const TRANSACTIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/transactions.js");
pub const EVENTS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/events.js");
pub const TOKENS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tokens.js");
pub const POSITIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/positions.js");
pub const FILTERING_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/filtering.js");
pub const CONFIG_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/config.js");
pub const STRATEGIES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/strategies.js");

const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
const TRANSACTIONS_PAGE: &str = include_str!("templates/pages/transactions.html");
const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
const FILTERING_PAGE: &str = include_str!("templates/pages/filtering.html");
const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");
const STRATEGIES_PAGE: &str = include_str!("templates/pages/strategies.html");

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);
    let mut combined_styles = vec![
        FOUNDATION_STYLES,
        LAYOUT_STYLES,
        COMPONENT_STYLES,
        DROPDOWN_STYLES,
        COMMON_STYLES,
        DATA_TABLE_STYLES,
        TABLE_TOOLBAR_STYLES,
        EVENTS_DIALOG_STYLES,
        TRADE_ACTION_DIALOG_STYLES,
        TAB_BAR_STYLES,
        TABLE_SETTINGS_DIALOG_STYLES,
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
        ("services", "🔧 Services"),
        ("tokens", "🪙 Tokens"),
        ("transactions", "💱 Transactions"),
        ("positions", "📊 Positions"),
        ("strategies", "🎯 Strategies"),
        ("events", "📡 Events"),
        ("filtering", "🔍 Filtering"),
        ("config", "⚙️ Config"),
    ];

    tabs.iter()
        .map(|(name, label)| {
            let active_class = if *name == active { " active" } else { "" };
            // Use data-page attribute for client-side routing (SPA)
            format!(
                "<a href=\"#\" data-page=\"{}\" class=\"tab{}\">{}</a>",
                name, active_class, label
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
