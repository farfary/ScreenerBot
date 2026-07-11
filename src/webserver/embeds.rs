//! Embedded static assets for the webserver
//!
//! All HTML templates, CSS, JavaScript, fonts, and images are embedded at compile time
//! using `include_str!` and `include_bytes!` macros. This module is separated from
//! `templates.rs` to keep rendering logic distinct from asset declarations.
//!
//! Constants use `pub(super)` visibility so they're accessible within the webserver
//! module but not publicly exposed. Public constants (marked `pub`) are exposed
//! for use by routes like `asset_serving.rs`.

// HTML base template
pub(super) const BASE_TEMPLATE: &str = include_str!("templates/base.html");

// CSS Styles - Foundation and Layout
pub(super) const FOUNDATION_STYLES: &str = include_str!("templates/styles/foundation.css");
pub(super) const SCROLLBAR_STYLES: &str = include_str!("templates/styles/base/scrollbar.css");
pub(super) const FLOATING_STYLES: &str = include_str!("templates/styles/base/floating.css");
pub(super) const LAYOUT_STYLES: &str = include_str!("templates/styles/layout.css");
pub(super) const COMPONENT_STYLES: &str = include_str!("templates/styles/components.css");
pub(super) const HEADER_STYLES: &str = include_str!("templates/styles/header.css");
pub(super) const DROPDOWN_STYLES: &str = include_str!("templates/styles/ui/dropdown.css");
pub(super) const COMMON_STYLES: &str = include_str!("templates/styles/common.css");
pub(super) const FORM_CONTROLS_STYLES: &str =
    include_str!("templates/styles/components/form_controls.css");
pub(super) const NOTIFICATION_STYLES: &str =
    include_str!("templates/styles/components/notifications.css");
pub(super) const TOAST_STYLES: &str = include_str!("templates/styles/components/toast.css");

// CSS Styles - Page-specific
pub(super) const SERVICES_PAGE_STYLES: &str = include_str!("templates/styles/pages/services.css");
pub(super) const TRANSACTIONS_PAGE_STYLES: &str =
    include_str!("templates/styles/pages/transactions.css");
pub(super) const EVENTS_PAGE_STYLES: &str = include_str!("templates/styles/pages/events.css");
pub(super) const TOKENS_PAGE_STYLES: &str = include_str!("templates/styles/pages/tokens.css");
pub(super) const POSITIONS_PAGE_STYLES: &str = include_str!("templates/styles/pages/positions.css");
pub(super) const FILTERING_BASE_STYLES: &str =
    include_str!("templates/styles/pages/filtering/base.css");
pub(super) const FILTERING_RESULTS_STYLES: &str =
    include_str!("templates/styles/pages/filtering/results.css");
pub(super) const FILTERING_RESULTS_EXPLORER_STYLES: &str =
    include_str!("templates/styles/pages/filtering/results_explorer.css");
pub(super) const CONFIG_PAGE_STYLES: &str = include_str!("templates/styles/pages/config.css");
pub(super) const CONFIG_SIDEBAR_STYLES: &str =
    include_str!("templates/styles/pages/config/sidebar.css");
pub(super) const CONFIG_FIELDS_STYLES: &str =
    include_str!("templates/styles/pages/config/fields.css");
pub(super) const CONFIG_RESPONSIVE_STYLES: &str =
    include_str!("templates/styles/pages/config/responsive.css");
pub(super) const STRATEGIES_PAGE_STYLES: &str =
    include_str!("templates/styles/pages/strategies.css");
pub(super) const STRATEGIES_CONDITION_CARDS_STYLES: &str =
    include_str!("templates/styles/pages/strategies/condition_cards.css");
pub(super) const STRATEGIES_MODALS_STYLES: &str =
    include_str!("templates/styles/pages/strategies/modals.css");
pub(super) const STRATEGIES_EDITOR_STYLES: &str =
    include_str!("templates/styles/pages/strategies/editor.css");
pub(super) const STRATEGIES_UI_COMPONENTS_STYLES: &str =
    include_str!("templates/styles/pages/strategies/ui_components.css");
pub(super) const TRADER_PAGE_STYLES: &str = include_str!("templates/styles/pages/trader.css");
pub(super) const TRADER_CONFIG_COMPONENTS_STYLES: &str =
    include_str!("templates/styles/pages/trader/config_components.css");
pub(super) const TRADER_CONTROLS_STYLES: &str =
    include_str!("templates/styles/pages/trader/controls.css");
pub(super) const TRADER_METRICS_STYLES: &str =
    include_str!("templates/styles/pages/trader/metrics.css");
pub(super) const WALLETS_PAGE_STYLES: &str = include_str!("templates/styles/pages/wallets.css");
pub(super) const WALLETS_MAIN_WALLET_STYLES: &str =
    include_str!("templates/styles/pages/wallets/main_wallet.css");
pub(super) const WALLETS_MODALS_STYLES: &str =
    include_str!("templates/styles/pages/wallets/modals.css");
pub(super) const WALLETS_IMPORT_EXPORT_STYLES: &str =
    include_str!("templates/styles/pages/wallets/import_export.css");
pub(super) const TOOLS_BASE_STYLES: &str = include_str!("templates/styles/pages/tools/base.css");
pub(super) const TOOLS_CONTENT_STYLES: &str =
    include_str!("templates/styles/pages/tools/content.css");
pub(super) const TOOLS_COMPONENTS_STYLES: &str =
    include_str!("templates/styles/pages/tools/components.css");
pub(super) const TOOLS_WALLET_STYLES: &str =
    include_str!("templates/styles/pages/tools/wallet_tools.css");
pub(super) const TOOLS_TOKEN_STYLES: &str =
    include_str!("templates/styles/pages/tools/token_tools.css");
pub(super) const TOOLS_TRADING_STYLES: &str =
    include_str!("templates/styles/pages/tools/trading_tools.css");
pub(super) const AI_BASE_STYLES: &str = include_str!("templates/styles/pages/ai/base.css");
pub(super) const AI_SETTINGS_TESTING_STYLES: &str =
    include_str!("templates/styles/pages/ai/settings_testing.css");
pub(super) const AI_PROVIDERS_STYLES: &str =
    include_str!("templates/styles/pages/ai/providers.css");
pub(super) const AI_INSTRUCTIONS_STYLES: &str =
    include_str!("templates/styles/pages/ai/instructions.css");
pub(super) const AI_CHAT_STYLES: &str = include_str!("templates/styles/pages/ai/chat.css");
pub(super) const AI_CHAT_MESSAGES_STYLES: &str =
    include_str!("templates/styles/pages/ai/chat_messages.css");
pub(super) const AI_CHAT_INPUT_STYLES: &str =
    include_str!("templates/styles/pages/ai/chat_input.css");
pub(super) const AI_AUTOMATION_STYLES: &str =
    include_str!("templates/styles/pages/ai/automation.css");
pub(super) const HOME_PAGE_STYLES: &str = include_str!("templates/styles/pages/home.css");
pub(super) const HOME_CALENDAR_STYLES: &str =
    include_str!("templates/styles/pages/home/calendar.css");
pub(super) const UPDATES_PAGE_STYLES: &str = include_str!("templates/styles/pages/updates.css");
pub(super) const ABOUT_PAGE_STYLES: &str = include_str!("templates/styles/pages/about.css");
pub(super) const SPLASH_PAGE_STYLES: &str = include_str!("templates/styles/pages/splash.css");
pub(super) const ONBOARDING_PAGE_STYLES: &str =
    include_str!("templates/styles/pages/onboarding.css");
pub(super) const SETUP_PAGE_STYLES: &str = include_str!("templates/styles/pages/setup.css");
pub(super) const SETUP_STEPS_STYLES: &str = include_str!("templates/styles/pages/setup/steps.css");
pub(super) const LOCKSCREEN_PAGE_STYLES: &str =
    include_str!("templates/styles/pages/lockscreen.css");
pub(super) const LOGIN_PAGE_STYLES: &str = include_str!("templates/styles/pages/login.css");

// CSS Styles - UI Components
pub(super) const DATA_TABLE_STYLES: &str = include_str!("templates/styles/ui/data_table.css");
pub(super) const DATA_TABLE_CORE_STYLES: &str =
    include_str!("templates/styles/ui/data_table/core.css");
pub(super) const DATA_TABLE_COLUMN_TYPES_STYLES: &str =
    include_str!("templates/styles/ui/data_table/column_types.css");
pub(super) const DATA_TABLE_PAGINATION_STYLES: &str =
    include_str!("templates/styles/ui/data_table/pagination.css");
pub(super) const TABLE_TOOLBAR_STYLES: &str = include_str!("templates/styles/ui/table_toolbar.css");
pub(super) const EVENTS_DIALOG_STYLES: &str = include_str!("templates/styles/ui/events_dialog.css");
pub(super) const TRADE_ACTION_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/trade_action_dialog.css");
pub(super) const TAB_BAR_STYLES: &str = include_str!("templates/styles/ui/tab_bar.css");
pub(super) const ACTION_BAR_STYLES: &str = include_str!("templates/styles/ui/action_bar.css");
pub(super) const TABLE_SETTINGS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/table_settings_dialog.css");
pub(super) const CONFIRMATION_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/confirmation_dialog.css");
pub(super) const POSITION_REMOVE_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/position_remove_dialog.css");
pub(super) const SETUP_DIALOG_STYLES: &str = include_str!("templates/styles/ui/setup_dialog.css");
pub(super) const CONTEXT_MENU_STYLES: &str = include_str!("templates/styles/ui/context_menu.css");
pub(super) const ADVANCED_CHART_STYLES: &str =
    include_str!("templates/styles/ui/advanced_chart.css");

// CSS Styles - Token Details
pub(super) const TOKEN_DETAILS_BASE_STYLES: &str =
    include_str!("templates/styles/token_details/base.css");
pub(super) const TOKEN_DETAILS_OVERVIEW_STYLES: &str =
    include_str!("templates/styles/token_details/overview_chart.css");
pub(super) const TOKEN_DETAILS_SECURITY_STYLES: &str =
    include_str!("templates/styles/token_details/security.css");
pub(super) const TOKEN_DETAILS_SECURITY_HOLDERS_STYLES: &str =
    include_str!("templates/styles/token_details/security_holders.css");
pub(super) const TOKEN_DETAILS_SECURITY_RISKS_STYLES: &str =
    include_str!("templates/styles/token_details/security_risks.css");
pub(super) const TOKEN_DETAILS_POSITIONS_STYLES: &str =
    include_str!("templates/styles/token_details/positions.css");
pub(super) const TOKEN_DETAILS_POOLS_STYLES: &str =
    include_str!("templates/styles/token_details/pools.css");
pub(super) const TOKEN_DETAILS_LINKS_STYLES: &str =
    include_str!("templates/styles/token_details/links.css");
pub(super) const TOKEN_DETAILS_TRANSACTIONS_SHARED_STYLES: &str =
    include_str!("templates/styles/token_details/transactions_shared.css");

// CSS Styles - Transaction and Position Details
pub(super) const TRANSACTION_DETAILS_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/transaction_details_dialog.css");
pub(super) const TRANSACTION_DETAILS_TAB_CONTENT_STYLES: &str =
    include_str!("templates/styles/ui/transaction_details/tab_content.css");
pub(super) const POSITION_DETAILS_BASE_STYLES: &str =
    include_str!("templates/styles/ui/position_details/base.css");
pub(super) const POSITION_DETAILS_OVERVIEW_STYLES: &str =
    include_str!("templates/styles/ui/position_details/overview.css");
pub(super) const POSITION_DETAILS_HISTORY_STYLES: &str =
    include_str!("templates/styles/ui/position_details/history.css");
pub(super) const POSITION_DETAILS_TRANSACTIONS_STYLES: &str =
    include_str!("templates/styles/ui/position_details/transactions.css");
pub(super) const POSITION_DETAILS_TRANSACTIONS_DETAIL_STYLES: &str =
    include_str!("templates/styles/ui/position_details/transactions_detail.css");

// CSS Styles - Settings and Misc
pub(super) const SETTINGS_BASE_STYLES: &str = include_str!("templates/styles/settings/base.css");
pub(super) const SETTINGS_TABS_STYLES: &str = include_str!("templates/styles/settings/tabs.css");
pub(super) const SETTINGS_SECURITY_STYLES: &str =
    include_str!("templates/styles/settings/security.css");
pub(super) const SETTINGS_UPDATES_STYLES: &str =
    include_str!("templates/styles/settings/updates.css");
pub(super) const SETTINGS_DATA_STYLES: &str = include_str!("templates/styles/settings/data.css");
pub(super) const STATUS_BAR_STYLES: &str = include_str!("templates/styles/status_bar.css");
pub(super) const HINT_POPOVER_STYLES: &str = include_str!("templates/styles/ui/hint_popover.css");
pub(super) const SEARCH_DIALOG_STYLES: &str = include_str!("templates/styles/ui/search_dialog.css");
pub(super) const CUSTOM_SELECT_STYLES: &str = include_str!("templates/styles/ui/custom_select.css");
pub(super) const BILLBOARD_DIALOG_STYLES: &str =
    include_str!("templates/styles/ui/billboard_dialog.css");
pub(super) const BILLBOARD_ROW_STYLES: &str = include_str!("templates/styles/ui/billboard_row.css");
pub(super) const POOL_SELECTOR_STYLES: &str = include_str!("templates/styles/ui/pool_selector.css");
pub(super) const EXIT_DIALOG_STYLES: &str = include_str!("templates/styles/ui/exit_dialog.css");
pub(super) const GLOBAL_CHAT_STYLES: &str =
    include_str!("templates/styles/components/global_chat.css");
pub(super) const CONFIG_IMPORT_EXPORT_DIALOG_STYLES: &str =
    include_str!("templates/styles/config_import_export_dialog.css");

// Assets (logos, icons) - Public for asset_serving.rs
pub const LOGO_SVG: &str = include_str!("assets/logo.svg");
pub const LOGO_PNG: &[u8] = include_bytes!("assets/logo.png");
pub const LIGHTWEIGHT_CHARTS_JS: &[u8] = include_bytes!("assets/lightweight-charts.js");

// AI Provider logos - Public for asset_serving.rs
pub const PROVIDER_OPENAI: &[u8] = include_bytes!("assets/providers/openai.png");
pub const PROVIDER_ANTHROPIC: &[u8] = include_bytes!("assets/providers/anthropic.png");
pub const PROVIDER_GROQ: &[u8] = include_bytes!("assets/providers/groq.png");
pub const PROVIDER_DEEPSEEK: &[u8] = include_bytes!("assets/providers/deepseek.png");
pub const PROVIDER_GEMINI: &[u8] = include_bytes!("assets/providers/gemini.png");
pub const PROVIDER_OLLAMA: &[u8] = include_bytes!("assets/providers/ollama.png");
pub const PROVIDER_TOGETHER: &[u8] = include_bytes!("assets/providers/together.png");
pub const PROVIDER_OPENROUTER: &[u8] = include_bytes!("assets/providers/openrouter.png");
pub const PROVIDER_MISTRAL: &[u8] = include_bytes!("assets/providers/mistral.png");

// Lucide icon font - Public for asset_serving.rs
pub(super) const LUCIDE_ICON_CSS: &str = include_str!("assets/lucide-font/lucide.css");
pub const LUCIDE_FONT_WOFF2: &[u8] = include_bytes!("assets/lucide-font/lucide.woff2");
pub const LUCIDE_FONT_WOFF: &[u8] = include_bytes!("assets/lucide-font/lucide.woff");
pub const LUCIDE_FONT_TTF: &[u8] = include_bytes!("assets/lucide-font/lucide.ttf");
pub const LUCIDE_FONT_EOT: &[u8] = include_bytes!("assets/lucide-font/lucide.eot");
pub const LUCIDE_FONT_SVG: &str = include_str!("assets/lucide-font/lucide.svg");

// Trading terminal fonts - Public for asset_serving.rs
pub const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("assets/fonts/JetBrainsMono-Regular.woff2");
pub const JETBRAINS_MONO_MEDIUM: &[u8] = include_bytes!("assets/fonts/JetBrainsMono-Medium.woff2");
pub const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("assets/fonts/JetBrainsMono-Bold.woff2");
pub const ORBITRON_VARIABLE: &[u8] = include_bytes!("assets/fonts/Orbitron-Variable.woff2");

// Core JavaScript modules - Public for asset_serving.rs
pub const CORE_LIFECYCLE: &str = include_str!("templates/scripts/core/lifecycle.js");
pub const CORE_MENU_MANAGER: &str = include_str!("templates/scripts/core/menu_manager.js");
pub const CORE_APP_STATE: &str = include_str!("templates/scripts/core/app_state.js");
pub const CORE_POLLER: &str = include_str!("templates/scripts/core/poller.js");
pub const CORE_ESCAPE_STACK: &str = include_str!("templates/scripts/core/escape_stack.js");
pub const CORE_TOKEN_ACCENT: &str = include_str!("templates/scripts/core/token_accent.js");
pub const CORE_DOM: &str = include_str!("templates/scripts/core/dom.js");
pub const CORE_UTILS: &str = include_str!("templates/scripts/core/utils.js");
pub const CORE_BOOTSTRAP: &str = include_str!("templates/scripts/core/bootstrap.js");
pub const CORE_ROUTER: &str = include_str!("templates/scripts/core/router.js");
pub const CORE_CONNECTIVITY_WATCHER: &str =
    include_str!("templates/scripts/core/connectivity_watcher.js");
pub const CORE_TOOLTIP: &str = include_str!("templates/scripts/core/tooltip.js");
pub const CORE_HEADER: &str = include_str!("templates/scripts/core/header.js");
pub const CORE_NOTIFICATIONS: &str = include_str!("templates/scripts/core/notifications.js");
pub const CORE_TOAST: &str = include_str!("templates/scripts/core/toast.js");
pub const CORE_REQUEST_MANAGER: &str = include_str!("templates/scripts/core/request_manager.js");
pub const CORE_CLIENT_READY: &str = include_str!("templates/scripts/core/client_ready.js");
pub const CORE_SPLASH: &str = include_str!("templates/scripts/core/splash.js");
pub const CORE_ONBOARDING: &str = include_str!("templates/scripts/core/onboarding.js");
pub const CORE_SETUP: &str = include_str!("templates/scripts/core/setup.js");
pub const CORE_STATUS_BAR: &str = include_str!("templates/scripts/core/status_bar.js");
pub const CORE_HINTS: &str = include_str!("templates/scripts/core/hints.js");
pub const CORE_LOCKSCREEN: &str = include_str!("templates/scripts/core/lockscreen.js");
pub const CORE_SOUNDS: &str = include_str!("templates/scripts/core/sounds.js");
pub const CORE_GLOBAL_CHAT: &str = include_str!("templates/scripts/core/global_chat.js");
pub const CORE_CHAT_WIDGET: &str = include_str!("templates/scripts/core/chat_widget.js");

// Theme scripts
pub(super) const THEME_SCRIPTS: &str = include_str!("templates/scripts/theme.js");

// UI Component JavaScript - Public for asset_serving.rs
pub const DATA_TABLE_UI: &str = include_str!("templates/scripts/ui/data_table.js");
pub const DATA_TABLE_COLUMN_MANAGEMENT: &str =
    include_str!("templates/scripts/ui/data_table/column_management.js");
pub const DATA_TABLE_CLIENT_PAGINATION: &str =
    include_str!("templates/scripts/ui/data_table/client_pagination.js");
pub const DATA_TABLE_SERVER_PAGINATION: &str =
    include_str!("templates/scripts/ui/data_table/server_pagination.js");
pub const DATA_TABLE_EVENT_HANDLERS: &str =
    include_str!("templates/scripts/ui/data_table/event_handlers.js");
pub const DROPDOWN_UI: &str = include_str!("templates/scripts/ui/dropdown.js");
pub const TABLE_TOOLBAR_UI: &str = include_str!("templates/scripts/ui/table_toolbar.js");
pub const TOAST_UI: &str = include_str!("templates/scripts/ui/toast.js");
pub const EVENTS_DIALOG_UI: &str = include_str!("templates/scripts/ui/events_dialog.js");
pub const CONFIRMATION_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/confirmation_dialog.js");
pub const POSITION_REMOVE_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/position_remove_dialog.js");
pub const TRADE_ACTION_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/trade_action_dialog.js");
pub const MANUAL_TRADE_UI: &str = include_str!("templates/scripts/ui/manual_trade.js");
pub const TRADE_ACTION_QUICK_TRADE_JS: &str =
    include_str!("templates/scripts/ui/trade_action/quick_trade.js");
pub const TRADE_ACTION_QUOTE_MANAGER_JS: &str =
    include_str!("templates/scripts/ui/trade_action/quote_manager.js");
pub const TAB_BAR_UI: &str = include_str!("templates/scripts/ui/tab_bar.js");
pub const ACTION_BAR_UI: &str = include_str!("templates/scripts/ui/action_bar.js");
pub const TABLE_SETTINGS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/table_settings_dialog.js");
pub const TOKEN_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/token_details_dialog.js");
pub const IMAGE_LIGHTBOX_UI: &str = include_str!("templates/scripts/ui/image_lightbox.js");
pub const TOKEN_DETAILS_OVERVIEW_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/overview_tab.js");
pub const TOKEN_DETAILS_SECURITY_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/security_tab.js");
pub const TOKEN_DETAILS_POOLS_LINKS_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/pools_links_tab.js");
pub const TOKEN_DETAILS_TRADE_ACTIONS_UI: &str =
    include_str!("templates/scripts/ui/token_details/trade_actions.js");
pub const TOKEN_DETAILS_TRANSACTIONS_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/transactions_tab.js");
pub const TOKEN_DETAILS_CHART_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/chart_tab.js");
pub const TOKEN_DETAILS_UTILITIES_UI: &str =
    include_str!("templates/scripts/ui/token_details/utilities.js");
pub const TOKEN_DETAILS_STATE_HANDLING_UI: &str =
    include_str!("templates/scripts/ui/token_details/state_handling.js");
pub const TOKEN_DETAILS_POSITIONS_TAB_UI: &str =
    include_str!("templates/scripts/ui/token_details/positions_tab.js");
pub const TRANSACTION_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/transaction_details_dialog.js");
pub const POSITION_DETAILS_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/position_details_dialog.js");
pub const POSITION_DETAILS_ANALYTICS_TAB_JS: &str =
    include_str!("templates/scripts/ui/position_details/analytics_tab.js");
pub const POSITION_DETAILS_SECONDARY_TABS_JS: &str =
    include_str!("templates/scripts/ui/position_details/secondary_tabs.js");
pub const POSITION_DETAILS_OVERVIEW_TAB_JS: &str =
    include_str!("templates/scripts/ui/position_details/overview_tab.js");
pub const POSITION_DETAILS_CHART_TAB_JS: &str =
    include_str!("templates/scripts/ui/position_details/chart_tab.js");
pub const POSITION_DETAILS_UTILITIES_JS: &str =
    include_str!("templates/scripts/ui/position_details/utilities.js");
pub const TOOL_FAVORITES_UI: &str = include_str!("templates/scripts/ui/tool_favorites.js");
pub const CONTEXT_MENU_UI: &str = include_str!("templates/scripts/ui/context_menu.js");
pub const CONTEXT_MENU_ACTIONS_JS: &str =
    include_str!("templates/scripts/ui/context_menu/actions.js");
pub const CONTEXT_MENU_BUILDERS_JS: &str =
    include_str!("templates/scripts/ui/context_menu/builders.js");
pub const ADVANCED_CHART_UI: &str = include_str!("templates/scripts/ui/advanced_chart.js");
pub const ADVANCED_CHART_INDICATORS_JS: &str =
    include_str!("templates/scripts/ui/advanced_chart/indicators.js");
pub const ADVANCED_CHART_FORMATTERS_JS: &str =
    include_str!("templates/scripts/ui/advanced_chart/formatters.js");
pub const SETTINGS_DIALOG_UI: &str = include_str!("templates/scripts/ui/settings_dialog.js");
pub const SETTINGS_SECURITY_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/security_tab.js");
pub const SETTINGS_DATA_TAB_UI: &str = include_str!("templates/scripts/ui/settings/data_tab.js");
pub const SETTINGS_UPDATES_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/updates_tab.js");
pub const SETTINGS_INTERFACE_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/interface_tab.js");
pub const SETTINGS_HINTS_TAB_UI: &str = include_str!("templates/scripts/ui/settings/hints_tab.js");
pub const SETTINGS_NAVIGATION_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/navigation_tab.js");
pub const SETTINGS_LICENSES_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/licenses_tab.js");
pub const SETTINGS_TELEGRAM_TAB_UI: &str =
    include_str!("templates/scripts/ui/settings/telegram_tab.js");
pub const NOTIFICATION_PANEL_UI: &str = include_str!("templates/scripts/ui/notification_panel.js");
pub const HINT_POPOVER_UI: &str = include_str!("templates/scripts/ui/hint_popover.js");
pub const SEARCH_DIALOG_UI: &str = include_str!("templates/scripts/ui/search_dialog.js");
pub const CUSTOM_SELECT_UI: &str = include_str!("templates/scripts/ui/custom_select.js");
pub const BILLBOARD_DIALOG_UI: &str = include_str!("templates/scripts/ui/billboard_dialog.js");
pub const BILLBOARD_ROW_UI: &str = include_str!("templates/scripts/ui/billboard_row.js");
pub const POOL_SELECTOR_UI: &str = include_str!("templates/scripts/ui/pool_selector.js");
pub const EXIT_DIALOG_UI: &str = include_str!("templates/scripts/ui/exit_dialog.js");
pub const CONFIG_IMPORT_EXPORT_DIALOG_UI: &str =
    include_str!("templates/scripts/ui/config_import_export_dialog.js");
pub const INPUT_DIALOG_UI: &str = include_str!("templates/scripts/ui/input_dialog.js");
pub const SETUP_DIALOG_UI: &str = include_str!("templates/scripts/ui/setup_dialog.js");

// Page-specific JavaScript - Public for asset_serving.rs
pub const SERVICES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/services.js");
pub const TRANSACTIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/transactions.js");
pub const EVENTS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/events.js");
pub const TOKENS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tokens.js");
pub const TOKENS_CONSTANTS_JS: &str = include_str!("templates/scripts/pages/tokens/constants.js");
pub const TOKENS_FORMATTERS_JS: &str = include_str!("templates/scripts/pages/tokens/formatters.js");
pub const TOKENS_OHLCV_JS: &str = include_str!("templates/scripts/pages/tokens/ohlcv.js");
pub const TOKENS_FAVORITES_JS: &str = include_str!("templates/scripts/pages/tokens/favorites.js");
pub const POSITIONS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/positions.js");
pub const FILTERING_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/filtering.js");
pub const FILTERING_CONFIG_METADATA_JS: &str =
    include_str!("templates/scripts/pages/filtering/config_metadata.js");
pub const FILTERING_RENDERERS_JS: &str =
    include_str!("templates/scripts/pages/filtering/renderers.js");
pub const CONFIG_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/config.js");
pub const CONFIG_UTILS_JS: &str = include_str!("templates/scripts/pages/config/utils.js");
pub const CONFIG_FIELD_RENDERERS_JS: &str =
    include_str!("templates/scripts/pages/config/field_renderers.js");
pub const STRATEGIES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/strategies.js");
pub const STRATEGIES_CONDITION_EDITOR_JS: &str =
    include_str!("templates/scripts/pages/strategies/condition_editor.js");
pub const STRATEGIES_CONDITION_CATALOG_JS: &str =
    include_str!("templates/scripts/pages/strategies/condition_catalog.js");
pub const TRADER_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/trader.js");
pub const TRADER_EXAMPLES_JS: &str = include_str!("templates/scripts/pages/trader/examples.js");
pub const TRADER_CONTROLS_JS: &str = include_str!("templates/scripts/pages/trader/controls.js");
pub const TRADER_CONFIG_CARDS_JS: &str =
    include_str!("templates/scripts/pages/trader/config_cards.js");
pub const TRADER_FEATURES_JS: &str = include_str!("templates/scripts/pages/trader/features.js");
pub const WALLETS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/wallets.js");
pub const WALLETS_BULK_OPERATIONS_JS: &str =
    include_str!("templates/scripts/pages/wallets/bulk_operations.js");
pub const WALLETS_RENDERERS_JS: &str = include_str!("templates/scripts/pages/wallets/renderers.js");
pub const TOOLS_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/tools.js");
pub const TOOLS_WALLET_TOOLS: &str = include_str!("templates/scripts/pages/tools/wallet_tools.js");
pub const TOOLS_TOKEN_TOOLS: &str = include_str!("templates/scripts/pages/tools/token_tools.js");
pub const TOOLS_TRADING_TOOLS: &str =
    include_str!("templates/scripts/pages/tools/trading_tools.js");
pub const TOOLS_MULTI_WALLET_TOOLS: &str =
    include_str!("templates/scripts/pages/tools/multi_wallet_tools.js");
pub const AI_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/ai.js");
pub const AI_PROVIDERS_TAB: &str = include_str!("templates/scripts/pages/ai/providers_tab.js");
pub const AI_INSTRUCTIONS_TAB: &str =
    include_str!("templates/scripts/pages/ai/instructions_tab.js");
pub const AI_AUTOMATION_TAB: &str = include_str!("templates/scripts/pages/ai/automation_tab.js");
pub const HOME_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/home.js");
pub const HOME_CALENDAR_JS: &str =
    include_str!("templates/scripts/pages/home/portfolio_calendar.js");
pub const HOME_CUSTOMIZE_JS: &str = include_str!("templates/scripts/pages/home/customize.js");
pub const UPDATES_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/updates.js");
pub const ABOUT_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/about.js");
pub const LOGIN_PAGE_SCRIPT: &str = include_str!("templates/scripts/pages/login.js");

// HTML Page Templates
pub(super) const TOKENS_PAGE: &str = include_str!("templates/pages/tokens.html");
pub(super) const EVENTS_PAGE: &str = include_str!("templates/pages/events.html");
pub(super) const SERVICES_PAGE: &str = include_str!("templates/pages/services.html");
pub(super) const TRANSACTIONS_PAGE: &str = include_str!("templates/pages/transactions.html");
pub(super) const POSITIONS_PAGE: &str = include_str!("templates/pages/positions.html");
pub(super) const FILTERING_PAGE: &str = include_str!("templates/pages/filtering.html");
pub(super) const CONFIG_PAGE: &str = include_str!("templates/pages/config.html");
pub(super) const STRATEGIES_PAGE: &str = include_str!("templates/pages/strategies.html");
pub(super) const TRADER_PAGE: &str = include_str!("templates/pages/trader.html");
pub(super) const WALLETS_PAGE: &str = include_str!("templates/pages/wallets.html");
pub(super) const TOOLS_PAGE: &str = include_str!("templates/pages/tools.html");
pub(super) const AI_PAGE: &str = include_str!("templates/pages/ai.html");
pub(super) const HOME_PAGE: &str = include_str!("templates/pages/home.html");
pub(super) const UPDATES_PAGE: &str = include_str!("templates/pages/updates.html");
pub(super) const ABOUT_PAGE: &str = include_str!("templates/pages/about.html");
pub(super) const SPLASH_PAGE: &str = include_str!("templates/pages/splash.html");
pub(super) const ONBOARDING_PAGE: &str = include_str!("templates/pages/onboarding.html");
pub(super) const SETUP_PAGE: &str = include_str!("templates/pages/setup.html");
pub(super) const LOCKSCREEN_PAGE: &str = include_str!("templates/pages/lockscreen.html");
pub(super) const LOGIN_PAGE: &str = include_str!("templates/pages/login.html");
