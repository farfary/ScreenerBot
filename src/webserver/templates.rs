/// HTML templates for the webserver dashboard
///
/// Templates are stored in `src/webserver/templates/` and embedded at compile time
/// using `include_str!`. This keeps the Rust module focused on wiring helpers
/// while HTML, CSS, and JavaScript live in dedicated files.

const BASE_TEMPLATE: &str = include_str!("templates/base.html");
const FOUNDATION_STYLES: &str = include_str!("templates/styles/foundation.css");
const LAYOUT_STYLES: &str = include_str!("templates/styles/layout.css");
const COMPONENT_STYLES: &str = include_str!("templates/styles/components.css");
const TOKEN_MODAL_STYLES: &str = include_str!("templates/styles/token-modal.css");
const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
const COMMON_SCRIPTS: &str = include_str!("templates/scripts/common.js");
const THEME_SCRIPTS: &str = include_str!("templates/scripts/theme.js");

const HOME_PAGE: &str = include_str!("templates/pages/home.html");
const STATUS_PAGE: &str = include_str!("templates/pages/status.html");
const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");

/// Render the base layout with shared chrome and inject the requested content.
pub fn base_template(title: &str, active_tab: &str, content: &str) -> String {
    let mut html = BASE_TEMPLATE.replace("{{TITLE}}", title);
    html = html.replace("{{NAV_TABS}}", &nav_tabs(active_tab));
    html = html.replace("{{CONTENT}}", content);
    let combined_styles = [
        FOUNDATION_STYLES,
        LAYOUT_STYLES,
        COMPONENT_STYLES,
        TOKEN_MODAL_STYLES,
        COMMON_STYLES,
    ]
    .join("\n");
    html = html.replace("/*__INJECTED_STYLES__*/", &combined_styles);
    html = html.replace("/*__COMMON_SCRIPTS__*/", COMMON_SCRIPTS);
    html = html.replace("/*__THEME_SCRIPTS__*/", THEME_SCRIPTS);
    html
}

fn nav_tabs(active: &str) -> String {
    let tabs = vec![
        ("home", "🏠 Home"),
        ("status", "📊 Status"),
        ("services", "🔧 Services"),
        ("positions", "💰 Positions"),
        ("tokens", "🪙 Tokens"),
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

pub fn tokens_content() -> String {
    render_page(TOKENS_PAGE)
}

pub fn events_content() -> String {
    render_page(EVENTS_PAGE)
}

pub fn services_content() -> String {
    render_page(SERVICES_PAGE)
}

pub fn config_content() -> String {
    render_page(CONFIG_PAGE)
}
