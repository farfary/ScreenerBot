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
const TOOLBAR_STYLES: &str = include_str!("templates/styles/toolbar.css");
const TOKEN_MODAL_STYLES: &str = include_str!("templates/styles/token-modal.css");
const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
const SERVICES_PAGE_STYLES: &str = include_str!("templates/styles/pages/services.css");
const DATA_TABLE_STYLES: &str = include_str!("templates/styles/ui/data_table.css");

pub const CORE_LIFECYCLE: &str = include_str!("templates/scripts/core/lifecycle.js");
pub const CORE_APP_STATE: &str = include_str!("templates/scripts/core/app_state.js");
pub const CORE_POLLER: &str = include_str!("templates/scripts/core/poller.js");
pub const CORE_DOM: &str = include_str!("templates/scripts/core/dom.js");
pub const CORE_UTILS: &str = include_str!("templates/scripts/core/utils.js");
pub const CORE_ROUTER: &str = include_str!("templates/scripts/core/router.js");

const THEME_SCRIPTS: &str = include_str!("templates/scripts/theme.js");

pub const DATA_TABLE_UI: &str = include_str!("templates/scripts/ui/data_table.js");

pub const SERVICES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/services.js");

const HOME_PAGE: &str = include_str!("templates/pages/home.html");
const STATUS_PAGE: &str = include_str!("templates/pages/status.html");
const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
const FILTERING_PAGE: &str = include_str!("templates/pages/filtering.html");
const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");
const TRANSACTIONS_PAGE: &str = include_str!("templates/pages/transactions.html");
const WALLET_PAGE: &str = include_str!("templates/pages/wallet.html");

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);
    let mut combined_styles = vec![
        FOUNDATION_STYLES,
        LAYOUT_STYLES,
        COMPONENT_STYLES,
        TOOLBAR_STYLES,
        TOKEN_MODAL_STYLES,
        COMMON_STYLES,
        DATA_TABLE_STYLES,
    ];
    if active_tab == "services" {
        combined_styles.push(SERVICES_PAGE_STYLES);
    }
    html = html.replace("/*__INJECTED_STYLES__*/", &combined_styles.join("\n"));
    html = html.replace("/*__THEME_SCRIPTS__*/", THEME_SCRIPTS);
    html
}

fn nav_tabs(active: &str) -> String {
    let tabs = vec![
        ("home", "🏠 Home"),
        ("status", "📊 Status"),
        ("services", "🔧 Services"),
        ("positions", "💰 Positions"),
        ("filtering", "🔍 Filtering"),
        ("tokens", "🪙 Tokens"),
        ("wallet", "👛 Wallet"),
        ("transactions", "💱 Transactions"),
        ("events", "📡 Events"),
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

pub fn home_content() -> String {
    render_page(HOME_PAGE)
}

pub fn status_content() -> String {
    render_page(STATUS_PAGE)
}

pub fn positions_content() -> String {
    render_page(POSITIONS_PAGE)
}

pub fn filtering_content() -> String {
    render_page(FILTERING_PAGE)
}

pub fn tokens_content() -> String {
    use crate::config::with_config;

    let (default_page_size, max_page_size) = with_config(|cfg| {
        (
            cfg.webserver.tokens_tab.default_page_size,
            cfg.webserver.tokens_tab.max_page_size,
        )
    });

    TOKENS_PAGE
        .replace(
            "__TOKENS_DEFAULT_PAGE_SIZE__",
            &default_page_size.to_string(),
        )
        .replace("__TOKENS_MAX_PAGE_SIZE__", &max_page_size.to_string())
}

pub fn events_content() -> String {
    render_page(EVENTS_PAGE)
}

pub fn services_content() -> String {
    render_page(SERVICES_PAGE)
}

pub fn wallet_content() -> String {
    render_page(WALLET_PAGE)
}

pub fn config_content() -> String {
    render_page(CONFIG_PAGE)
}

pub fn transactions_content() -> String {
    render_page(TRANSACTIONS_PAGE)
}
