/// HTML templates for the webserver dashboard
///
/// Templates are stored in `src/webserver/templates/` and embedded at compile time
/// using `include_str!`. This keeps the Rust module focused on wiring helpers
/// while HTML, CSS, and JavaScript live in dedicated files.
use crate::{arguments, version};

const BASE_TEMPLATE: &str = include_str!("templates/base.html");
const FOUNDATION_STYLES: &str = include_str!("templates/styles/foundation.css");
const LAYOUT_STYLES: &str = include_str!("templates/styles/layout.css");
const COMPONENT_STYLES: &str = include_str!("templates/styles/components.css");
const HEADER_STYLES: &str = include_str!("templates/styles/header.css");
const DROPDOWN_STYLES: &str = include_str!("templates/styles/ui/dropdown.css");
const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
const FORM_CONTROLS_STYLES: &str = include_str!("templates/styles/components/form_controls.css");
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
const WALLETS_PAGE_STYLES: &str = include_str!("templates/styles/pages/wallets.css");
const TOOLS_PAGE_STYLES: &str = include_str!("templates/styles/pages/tools.css");
const HOME_PAGE_STYLES: &str = include_str!("templates/styles/pages/home.css");
const UPDATES_PAGE_STYLES: &str = include_str!("templates/styles/pages/updates.css");
const SPLASH_PAGE_STYLES: &str = include_str!("templates/styles/pages/splash.css");
const ONBOARDING_PAGE_STYLES: &str = include_str!("templates/styles/pages/onboarding.css");
const SETUP_PAGE_STYLES: &str = include_str!("templates/styles/pages/setup.css");
const LOCKSCREEN_PAGE_STYLES: &str = include_str!("templates/styles/pages/lockscreen.css");
const LOGIN_PAGE_STYLES: &str = include_str!("templates/styles/pages/login.css");
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
const CONTEXT_MENU_STYLES: &str = include_str!("templates/styles/ui/context_menu.css");
const ADVANCED_CHART_STYLES: &str = include_str!("templates/styles/ui/advanced_chart.css");
const TOKEN_DETAILS_DIALOG_STYLES: &str = include_str!("templates/styles/token_details_dialog.css");
const TRANSACTION_DETAILS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/transaction_details_dialog.css");
const POSITION_DETAILS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/position_details_dialog.css");
const SETTINGS_DIALOG_STYLES: &str = include_str!("templates/styles/settings_dialog.css");
const STATUS_BAR_STYLES: &str = include_str!("templates/styles/status_bar.css");
const HINT_POPOVER_STYLES: &str = include_str!("templates/styles/ui/hint_popover.css");
const SEARCH_DIALOG_STYLES: &str = include_str!("templates/styles/ui/search_dialog.css");
const CUSTOM_SELECT_STYLES: &str = include_str!("templates/styles/ui/custom_select.css");
const BILLBOARD_DIALOG_STYLES: &str = include_str!("templates/styles/ui/billboard_dialog.css");
const BILLBOARD_ROW_STYLES: &str = include_str!("templates/styles/ui/billboard_row.css");
const POOL_SELECTOR_STYLES: &str = include_str!("templates/styles/ui/pool_selector.css");
const EXIT_DIALOG_STYLES: &str = include_str!("templates/styles/ui/exit_dialog.css");

// Assets (logos, icons)
pub const LOGO_SVG: &str = include_str!("assets/logo.svg");
pub const LOGO_PNG: &[u8] = include_bytes!("assets/logo.png");
pub const LIGHTWEIGHT_CHARTS_JS: &[u8] = include_bytes!("assets/lightweight-charts.js");

// Lucide icon font
const LUCIDE_ICON_CSS: &str = include_str!("assets/lucide-font/lucide.css");
pub const LUCIDE_FONT_WOFF2: &[u8] = include_bytes!("assets/lucide-font/lucide.woff2");
pub const LUCIDE_FONT_WOFF: &[u8] = include_bytes!("assets/lucide-font/lucide.woff");
pub const LUCIDE_FONT_TTF: &[u8] = include_bytes!("assets/lucide-font/lucide.ttf");
pub const LUCIDE_FONT_EOT: &[u8] = include_bytes!("assets/lucide-font/lucide.eot");
pub const LUCIDE_FONT_SVG: &str = include_str!("assets/lucide-font/lucide.svg");

// Trading terminal fonts - JetBrains Mono (tabular numbers) and Orbitron (branding)
pub const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("assets/fonts/JetBrainsMono-Regular.woff2");
pub const JETBRAINS_MONO_MEDIUM: &[u8] = include_bytes!("assets/fonts/JetBrainsMono-Medium.woff2");
pub const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("assets/fonts/JetBrainsMono-Bold.woff2");
pub const ORBITRON_VARIABLE: &[u8] = include_bytes!("assets/fonts/Orbitron-Variable.woff2");

pub const CORE_LIFECYCLE: &str = include_str!("templates/scripts/core/lifecycle.js");
pub const CORE_APP_STATE: &str = include_str!("templates/scripts/core/app_state.js");
pub const CORE_POLLER: &str = include_str!("templates/scripts/core/poller.js");
pub const CORE_DOM: &str = include_str!("templates/scripts/core/dom.js");
pub const CORE_UTILS: &str = include_str!("templates/scripts/core/utils.js");
pub const CORE_BOOTSTRAP: &str = include_str!("templates/scripts/core/bootstrap.js");
pub const CORE_ROUTER: &str = include_str!("templates/scripts/core/router.js");
pub const CORE_HEADER: &str = include_str!("templates/scripts/core/header.js");
pub const CORE_NOTIFICATIONS: &str = include_str!("templates/scripts/core/notifications.js");
pub const CORE_TOAST: &str = include_str!("templates/scripts/core/toast.js");
pub const CORE_REQUEST_MANAGER: &str = include_str!("templates/scripts/core/request_manager.js");
pub const CORE_SPLASH: &str = include_str!("templates/scripts/core/splash.js");
pub const CORE_ONBOARDING: &str = include_str!("templates/scripts/core/onboarding.js");
pub const CORE_SETUP: &str = include_str!("templates/scripts/core/setup.js");
pub const CORE_STATUS_BAR: &str = include_str!("templates/scripts/core/status_bar.js");
pub const CORE_HINTS: &str = include_str!("templates/scripts/core/hints.js");
pub const CORE_LOCKSCREEN: &str = include_str!("templates/scripts/core/lockscreen.js");
pub const CORE_SOUNDS: &str = include_str!("templates/scripts/core/sounds.js");

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
pub const TRANSACTION_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/transaction_details_dialog.js");
pub const POSITION_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/position_details_dialog.js");
pub const TOOL_FAVORITES_UI: &str = include_str!("templates/scripts/ui/tool_favorites.js");
pub const CONTEXT_MENU_UI: &str = include_str!("templates/scripts/ui/context_menu.js");
pub const ADVANCED_CHART_UI: &str = include_str!("templates/scripts/ui/advanced_chart.js");
pub const SETTINGS_DIALOG_UI: &str = include_str!("templates/scripts/ui/settings_dialog.js");
pub const NOTIFICATION_PANEL_UI: &str = include_str!("templates/scripts/ui/notification_panel.js");
pub const HINT_POPOVER_UI: &str = include_str!("templates/scripts/ui/hint_popover.js");
pub const SEARCH_DIALOG_UI: &str = include_str!("templates/scripts/ui/search_dialog.js");
pub const CUSTOM_SELECT_UI: &str = include_str!("templates/scripts/ui/custom_select.js");
pub const BILLBOARD_DIALOG_UI: &str = include_str!("templates/scripts/ui/billboard_dialog.js");
pub const BILLBOARD_ROW_UI: &str = include_str!("templates/scripts/ui/billboard_row.js");
pub const POOL_SELECTOR_UI: &str = include_str!("templates/scripts/ui/pool_selector.js");
pub const EXIT_DIALOG_UI: &str = include_str!("templates/scripts/ui/exit_dialog.js");

pub const SERVICES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/services.js");
pub const TRANSACTIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/transactions.js");
pub const EVENTS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/events.js");
pub const TOKENS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tokens.js");
pub const POSITIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/positions.js");
pub const FILTERING_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/filtering.js");
pub const CONFIG_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/config.js");
pub const STRATEGIES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/strategies.js");
pub const TRADER_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/trader.js");
pub const WALLETS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/wallets.js");
pub const TOOLS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tools.js");
pub const HOME_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/home.js");
pub const UPDATES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/updates.js");
pub const ABOUT_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/about.js");
pub const LOGIN_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/login.js");

const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
const TRANSACTIONS_PAGE: &str = include_str!("templates/pages/transactions.html");
const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
const FILTERING_PAGE: &str = include_str!("templates/pages/filtering.html");
const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");
const STRATEGIES_PAGE: &str = include_str!("templates/pages/strategies.html");
const TRADER_PAGE: &str = include_str!("templates/pages/trader.html");
const WALLETS_PAGE: &str = include_str!("templates/pages/wallets.html");
const TOOLS_PAGE: &str = include_str!("templates/pages/tools.html");
const HOME_PAGE: &str = include_str!("templates/pages/home.html");
const UPDATES_PAGE: &str = include_str!("templates/pages/updates.html");
const ABOUT_PAGE: &str = include_str!("templates/pages/about.html");
const SPLASH_PAGE: &str = include_str!("templates/pages/splash.html");
const ONBOARDING_PAGE: &str = include_str!("templates/pages/onboarding.html");
const SETUP_PAGE: &str = include_str!("templates/pages/setup.html");
const LOCKSCREEN_PAGE: &str = include_str!("templates/pages/lockscreen.html");
const LOGIN_PAGE: &str = include_str!("templates/pages/login.html");

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    use crate::global;

    let asset_version = option_env!("ASSET_VERSION_TS")
        .map(|ts| format!("{}-{}", version::get_version(), ts))
        .unwrap_or_else(|| version::get_version().to_string());

    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);

    // Inject security credentials for GUI mode
    let is_gui = global::is_gui_mode();
    let security_token = if is_gui {
        global::get_security_token().unwrap_or_default()
    } else {
        String::new()
    };
    let port = global::get_webserver_port();

    html = html.replace("{{SECURITY_TOKEN}}", &security_token);
    html = html.replace("{{WEBSERVER_PORT}}", &port.to_string());
    html = html.replace("{{IS_GUI_MODE}}", if is_gui { "true" } else { "false" });
    html = html.replace("{{ASSET_VERSION}}", asset_version.as_str());

    // Inject splash, onboarding, setup, and lockscreen screens
    html = html.replace("{{SPLASH_SCREEN}}", SPLASH_PAGE);
    html = html.replace("{{ONBOARDING_SCREEN}}", ONBOARDING_PAGE);
    html = html.replace("{{SETUP_SCREEN}}", SETUP_PAGE);
    html = html.replace("{{LOCKSCREEN}}", LOCKSCREEN_PAGE);

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
        FORM_CONTROLS_STYLES,
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
        CONTEXT_MENU_STYLES,
        ADVANCED_CHART_STYLES,
        TOKEN_DETAILS_DIALOG_STYLES,
        TRANSACTION_DETAILS_DIALOG_STYLES,
        POSITION_DETAILS_DIALOG_STYLES,
        SETTINGS_DIALOG_STYLES,
        HINT_POPOVER_STYLES,
        SEARCH_DIALOG_STYLES,
        CUSTOM_SELECT_STYLES,
        BILLBOARD_DIALOG_STYLES,
        BILLBOARD_ROW_STYLES,
        POOL_SELECTOR_STYLES,
        EXIT_DIALOG_STYLES,
        // Splash, onboarding, and setup screens (always included for proper transitions)
        SPLASH_PAGE_STYLES,
        ONBOARDING_PAGE_STYLES,
        SETUP_PAGE_STYLES,
        // Lockscreen overlay (security)
        LOCKSCREEN_PAGE_STYLES,
        // Status bar (always visible at bottom)
        STATUS_BAR_STYLES,
        // All page styles included upfront to prevent FOUC (Flash of Unstyled Content)
        // when navigating via SPA router. The CSS is small enough that bundling all
        // is better than risking style injection race conditions in WebView.
        SERVICES_PAGE_STYLES,
        TRANSACTIONS_PAGE_STYLES,
        EVENTS_PAGE_STYLES,
        TOKENS_PAGE_STYLES,
        POSITIONS_PAGE_STYLES,
        FILTERING_PAGE_STYLES,
        CONFIG_PAGE_STYLES,
        STRATEGIES_PAGE_STYLES,
        TRADER_PAGE_STYLES,
        WALLETS_PAGE_STYLES,
        TOOLS_PAGE_STYLES,
        HOME_PAGE_STYLES,
        UPDATES_PAGE_STYLES,
    ];
    // Suppress unused variable warning - active_tab was used for conditional style loading
    // but we now include all styles upfront to prevent FOUC in WebView
    let _ = active_tab;
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
        ("wallets", WALLETS_PAGE_STYLES),
        ("tools", TOOLS_PAGE_STYLES),
        ("home", HOME_PAGE_STYLES),
        ("updates", UPDATES_PAGE_STYLES),
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
    use crate::config;
    use crate::global;

    // In initialization mode (before config is loaded), return minimal nav
    if !global::is_initialization_complete() {
        // Only show initialization tab during setup
        let active_class = if active == "initialization" {
            " active"
        } else {
            ""
        };
        return format!(
            "<a href=\"#\" data-page=\"initialization\" class=\"tab{}\"><i class=\"icon-settings\"></i> Setup</a>",
            active_class
        );
    }

    // Get tabs from config, filter enabled ones, and sort by order
    let mut tabs = config::with_config(|cfg| cfg.gui.dashboard.navigation.tabs.clone());
    tabs.retain(|t| t.enabled);
    tabs.sort_by_key(|t| t.order);

    tabs.iter()
        .map(|tab| {
            let active_class = if tab.id == active { " active" } else { "" };
            // Use data-page attribute for client-side routing (SPA)
            format!(
                "<a href=\"#\" data-page=\"{}\" class=\"tab{}\"><i class=\"{}\"></i> {}</a>",
                tab.id, active_class, tab.icon, tab.label
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

pub fn wallets_content() -> String {
    render_page(WALLETS_PAGE)
}

pub fn tools_content() -> String {
    render_page(TOOLS_PAGE)
}

pub fn initialization_content() -> String {
    // Legacy redirect - initialization now uses the setup screen
    render_page(SETUP_PAGE)
}

pub fn updates_content() -> String {
    render_page(UPDATES_PAGE)
}

pub fn about_content() -> String {
    render_page(ABOUT_PAGE)
}

pub fn home_content() -> String {
    render_page(HOME_PAGE)
}

pub fn splash_content() -> String {
    render_page(SPLASH_PAGE)
}

pub fn onboarding_content() -> String {
    render_page(ONBOARDING_PAGE)
}

pub fn setup_content() -> String {
    render_page(SETUP_PAGE)
}

pub fn login_content() -> String {
    render_page(LOGIN_PAGE)
}

/// Render the login page template (minimal template without navigation)
pub fn login_template(title: &str, content: &str) -> String {
    use crate::version;

    let asset_version = option_env!("ASSET_VERSION_TS")
        .map(|ts| format!("{}-{}", version::get_version(), ts))
        .unwrap_or_else(|| version::get_version().to_string());

    // Prepare Lucide icon font CSS with corrected paths
    let lucide_css = LUCIDE_ICON_CSS
        .replace("url('lucide.eot", "url('/assets/fonts/lucide.eot")
        .replace("url('lucide.woff2", "url('/assets/fonts/lucide.woff2")
        .replace("url('lucide.woff", "url('/assets/fonts/lucide.woff")
        .replace("url('lucide.ttf", "url('/assets/fonts/lucide.ttf")
        .replace("url('lucide.svg", "url('/assets/fonts/lucide.svg");

    // Minimal styles for login page
    let combined_styles = [FOUNDATION_STYLES, &lucide_css, LOGIN_PAGE_STYLES].join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ScreenerBot</title>
    <style>{}</style>
</head>
<body>
    {}
    <script type="module" src="/scripts/pages/login.js?v={}"></script>
</body>
</html>"#,
        title, combined_styles, content, asset_version
    )
}
