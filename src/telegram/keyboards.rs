//! Telegram keyboard builders for ScreenerBot
//!
//! Provides pre-built keyboard layouts for:
//! - Reply keyboard (persistent bottom keyboard)
//! - Main menu navigation (inline)
//! - Position management actions
//! - Confirmation dialogs
//! - Settings quick toggles

use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup};

// === REPLY KEYBOARD (Bottom persistent keyboard) ===

/// Create the main reply keyboard that appears at the bottom of Telegram
/// This replaces the default keyboard and persists until removed
pub fn main_reply_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new(vec![
        // Row 1: Primary actions
        vec![
            KeyboardButton::new("📊 Status"),
            KeyboardButton::new("💰 Balance"),
            KeyboardButton::new("📈 Positions"),
        ],
        // Row 2: Trading controls
        vec![
            KeyboardButton::new("⏸️ Pause"),
            KeyboardButton::new("▶️ Resume"),
            KeyboardButton::new("🛑 Stop"),
        ],
        // Row 3: Info
        vec![
            KeyboardButton::new("📉 Stats"),
            KeyboardButton::new("⚙️ Menu"),
            KeyboardButton::new("❓ Help"),
        ],
    ])
    .resize_keyboard() // Make keyboard smaller/fit content
    .persistent() // Keep keyboard visible
}

// === HELPER FUNCTIONS ===

/// Create a callback button
fn btn(text: &str, callback_data: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text.to_string(), callback_data.to_string())
}

/// Create a URL button (returns callback button if URL is invalid)
fn url_btn(text: &str, url: &str) -> InlineKeyboardButton {
    match url.parse() {
        Ok(parsed_url) => InlineKeyboardButton::url(text.to_string(), parsed_url),
        Err(_) => InlineKeyboardButton::callback(text.to_string(), "error:invalid_url".to_owned()),
    }
}

/// Truncate mint to first 8 characters for callback data
pub fn mint_short(mint: &str) -> String {
    mint.chars().take(8).collect()
}

// === MAIN MENU ===

/// Main menu keyboard with primary navigation options
pub fn main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        // Row 1: Primary info
        vec![
            btn("📊 Positions", "menu:positions"),
            btn("💰 Balance", "cmd:balance"),
            btn("📈 Stats", "cmd:stats"),
        ],
        // Row 2: Token Explorer & Controls
        vec![
            btn("🔍 Tokens", "menu:tokens"),
            btn("⏸️ Pause", "cmd:pause_entries"),
            btn("⏹️ Stop", "cmd:stop_trader"),
        ],
        // Row 3: Settings & Refresh
        vec![
            btn("⚙️ Settings", "menu:settings"),
            btn("🔄 Refresh", "menu:refresh"),
        ],
    ])
}

/// Compact main menu (for use after other messages)
pub fn main_menu_compact() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("📊 Positions", "menu:positions"),
        btn("💰 Balance", "cmd:balance"),
        btn("◀️ Menu", "menu:main"),
    ]])
}

// === POSITIONS ===

/// Positions list with individual position buttons
/// `positions` is a list of (symbol, mint, pnl_pct)
pub fn positions_list(positions: &[(String, String, f64)]) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Add up to 5 position buttons (2 per row)
    for chunk in positions.chunks(2) {
        let row: Vec<InlineKeyboardButton> = chunk
            .iter()
            .map(|(symbol, mint, pnl)| {
                let emoji = if *pnl >= 0.0 { "📈" } else { "📉" };
                let text = format!("{} {} {:.1}%", emoji, symbol, pnl);
                btn(&text, &format!("pos:{}", mint_short(mint)))
            })
            .collect();
        rows.push(row);
    }

    // Close All button (only if positions exist)
    if !positions.is_empty() {
        rows.push(vec![btn("❌ Close All Positions", "confirm:closeall")]);
    }

    // Back button
    rows.push(vec![btn("◀️ Back to Menu", "menu:main")]);

    InlineKeyboardMarkup::new(rows)
}

/// Single position detail view with action buttons
pub fn position_actions(mint: &str, _symbol: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![
        // Row 1: Sell percentages
        vec![
            btn("Sell 25%", &format!("sell:{m}:25")),
            btn("Sell 50%", &format!("sell:{m}:50")),
            btn("Sell 75%", &format!("sell:{m}:75")),
            btn("Sell 100%", &format!("sell:{m}:100")),
        ],
        // Row 2: DCA options
        vec![
            btn("➕ DCA 0.1", &format!("dca:{m}:0.1")),
            btn("➕ DCA 0.25", &format!("dca:{m}:0.25")),
            btn("➕ DCA 0.5", &format!("dca:{m}:0.5")),
        ],
        // Row 3: Actions
        vec![
            btn("🚫 Blacklist", &format!("bl:{m}")),
            btn("❌ Close Position", &format!("confirm:close:{m}")),
        ],
        // Row 4: Navigation
        vec![btn("◀️ Back", "menu:positions")],
    ])
}

/// Compact position actions (for notifications)
pub fn position_actions_compact(mint: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn("📊 Details", &format!("pos:{m}")),
        btn("🚫 Blacklist", &format!("bl:{m}")),
    ]])
}

// === CONFIRMATION DIALOGS ===

/// Confirmation dialog for closing a position
pub fn confirm_close(mint: &str, _symbol: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn("✅ Confirm Close", &format!("exec:close:{m}")),
        btn("❌ Cancel", &format!("cancel:close:{m}")),
    ]])
}

/// Confirmation dialog for closing all positions
pub fn confirm_close_all() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("✅ Close ALL Positions", "exec:closeall"),
        btn("❌ Cancel", "menu:positions"),
    ]])
}

/// Confirmation dialog for selling a percentage
pub fn confirm_sell(mint: &str, percent: u32) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &format!("✅ Confirm Sell {percent}%"),
            &format!("exec:sell:{m}:{percent}"),
        ),
        btn("❌ Cancel", &format!("pos:{m}")),
    ]])
}

/// Confirmation dialog for DCA
pub fn confirm_dca(mint: &str, amount: f64) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &format!("✅ DCA {amount} SOL"),
            &format!("exec:dca:{m}:{amount}"),
        ),
        btn("❌ Cancel", &format!("pos:{m}")),
    ]])
}

/// Confirmation for force stop
pub fn confirm_force_stop() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("🚨 CONFIRM FORCE STOP", "exec:force_stop"),
        btn("❌ Cancel", "menu:main"),
    ]])
}

/// Confirmation for blacklisting a token (from position context)
pub fn confirm_blacklist(mint: &str, symbol: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(&format!("🚫 Blacklist {symbol}"), &format!("exec:bl:{m}")),
        btn("❌ Cancel", &format!("pos:{m}")),
    ]])
}

/// Confirmation for blacklisting a token (from token explorer - no position)
pub fn confirm_token_blacklist(mint: &str, symbol: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &format!("🚫 Blacklist {symbol}"),
            &format!("exec:tokenbl:{m}"),
        ),
        btn("❌ Cancel", "tokens:menu"),
    ]])
}

/// Confirmation for buying a token
pub fn confirm_token_buy(mint: &str, symbol: &str, amount: f64) -> InlineKeyboardMarkup {
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &format!("✅ Buy {amount} SOL"),
            &format!("exec:tokenbuy:{m}:{amount}"),
        ),
        btn("❌ Cancel", &format!("token:view:{m}")),
    ]])
}

// === SETTINGS ===

/// Quick settings menu
pub fn settings_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        // Row 1: Notification settings
        vec![
            btn("🔔 Notifications", "settings:notifications"),
            btn("⚡ Trading", "settings:trading"),
        ],
        // Row 2: Monitor controls
        vec![
            btn("📥 Entry Monitor", "toggle:entry_monitor"),
            btn("📤 Exit Monitor", "toggle:exit_monitor"),
        ],
        // Row 3: Back
        vec![btn("◀️ Back to Menu", "menu:main")],
    ])
}

/// Notification toggles with current state
pub fn notification_settings(
    pos_opened: bool,
    pos_closed: bool,
    partial_exit: bool,
    dca: bool,
    errors: bool,
) -> InlineKeyboardMarkup {
    let toggle = |enabled: bool, name: &str, key: &str| -> InlineKeyboardButton {
        let emoji = if enabled { "🟢" } else { "⚪" };
        btn(&format!("{emoji} {name}"), &format!("toggle:{key}"))
    };

    InlineKeyboardMarkup::new(vec![
        vec![
            toggle(pos_opened, "Opened", "notify_opened"),
            toggle(pos_closed, "Closed", "notify_closed"),
        ],
        vec![
            toggle(partial_exit, "Partial", "notify_partial"),
            toggle(dca, "DCA", "notify_dca"),
        ],
        vec![toggle(errors, "Errors", "notify_errors")],
        vec![btn("◀️ Back", "menu:settings")],
    ])
}

/// Trading controls with current state
pub fn trading_controls(
    entry_enabled: bool,
    exit_enabled: bool,
    auto_trading: bool,
) -> InlineKeyboardMarkup {
    let toggle = |enabled: bool, name: &str, key: &str| -> InlineKeyboardButton {
        let emoji = if enabled { "🟢" } else { "🔴" };
        btn(&format!("{emoji} {name}"), &format!("toggle:{key}"))
    };

    InlineKeyboardMarkup::new(vec![
        vec![
            toggle(entry_enabled, "Entry Monitor", "entry_monitor"),
            toggle(exit_enabled, "Exit Monitor", "exit_monitor"),
        ],
        vec![toggle(auto_trading, "Auto Trading", "auto_trading")],
        vec![btn("🚨 Force Stop", "confirm:force_stop")],
        vec![btn("◀️ Back", "menu:settings")],
    ])
}

// === NOTIFICATION BUTTONS ===

/// Buttons for position opened notification
pub fn on_position_opened(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);
    let solscan_url = format!("https://solscan.io/tx/{signature}");

    InlineKeyboardMarkup::new(vec![
        vec![
            btn("📊 Details", &format!("pos:{m}")),
            btn("🚫 Blacklist", &format!("bl:{m}")),
        ],
        vec![url_btn("🔗 Solscan", &solscan_url)],
    ])
}

/// Buttons for position closed notification
pub fn on_position_closed(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);
    let solscan_url = format!("https://solscan.io/tx/{signature}");

    InlineKeyboardMarkup::new(vec![
        vec![
            btn("📋 History", "cmd:history"),
            btn("🚫 Blacklist", &format!("exec:bl:{m}")),
        ],
        vec![url_btn("🔗 Solscan", &solscan_url)],
    ])
}

/// Buttons for partial exit notification
pub fn on_partial_exit(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);
    let solscan_url = format!("https://solscan.io/tx/{signature}");

    InlineKeyboardMarkup::new(vec![
        vec![
            btn("📊 Position", &format!("pos:{m}")),
            btn("Sell More", &format!("pos:{m}")),
        ],
        vec![url_btn("🔗 Solscan", &solscan_url)],
    ])
}

/// Buttons for DCA notification
pub fn on_dca_executed(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let m = mint_short(mint);
    let solscan_url = format!("https://solscan.io/tx/{signature}");

    InlineKeyboardMarkup::new(vec![
        vec![
            btn("📊 Position", &format!("pos:{m}")),
            btn("➕ More DCA", &format!("pos:{m}")),
        ],
        vec![url_btn("🔗 Solscan", &solscan_url)],
    ])
}

/// Buttons for error notification
pub fn on_error() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("📊 Status", "cmd:status"),
        btn("🔄 Refresh", "menu:refresh"),
    ]])
}

/// Buttons for startup/shutdown notification
pub fn on_system_event() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("📊 Status", "cmd:status"),
        btn("📊 Positions", "menu:positions"),
    ]])
}

// === AUTHENTICATION ===

/// Authentication prompt (no buttons, user types password/code)
pub fn auth_prompt() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![btn("❌ Cancel", "auth:cancel")]])
}

/// Session expired message
pub fn session_expired() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![btn("🔑 Re-authenticate", "auth:start")]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_short() {
        assert_eq!(mint_short("DezN1234567890abcdef"), "DezN1234");
        assert_eq!(mint_short("ABC"), "ABC");
    }

    #[test]
    fn test_main_menu_structure() {
        let keyboard = main_menu();
        assert_eq!(keyboard.inline_keyboard.len(), 3); // 3 rows
    }

    #[test]
    fn test_callback_data_length() {
        // Ensure callback data doesn't exceed 64 bytes
        let m = mint_short("DezN1234567890abcdef");
        let callback = format!("exec:sell:{m}:100");
        assert!(callback.len() <= 64);
    }
}

// === PAGINATION KEYBOARD ===

/// Create pagination controls
pub fn pagination_keyboard(
    session_id: &str,
    current_page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut row = Vec::new();

    // Previous Button
    if current_page > 0 {
        row.push(btn(
            "⬅️ Prev",
            &format!("page:{}:{}:{}", session_id, current_page - 1, total_pages),
        ));
    } else {
        // Spacer if no prev button to keep alignment
        row.push(btn("⏺️", "noop"));
    }

    // Page Indicator (middle)
    row.push(btn(
        &format!("{}/{}", current_page + 1, total_pages),
        "noop", // No action on click
    ));

    // Next Button
    if current_page < total_pages.saturating_sub(1) {
        row.push(btn(
            "Next ➡️",
            &format!("page:{}:{}:{}", session_id, current_page + 1, total_pages),
        ));
    } else {
        row.push(btn("⏺️", "noop"));
    }

    InlineKeyboardMarkup::new(vec![row])
}

// === TOKEN EXPLORER KEYBOARDS ===

/// Main token explorer menu with navigation options
pub fn tokens_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        // Row 1: Primary views
        vec![
            btn("✅ Passed", "tokens:passed"),
            btn("❌ Rejected", "tokens:rejected"),
        ],
        // Row 2: Additional views
        vec![
            btn("🆕 New (24h)", "tokens:recent"),
            btn("📋 All Tokens", "tokens:all"),
        ],
        // Row 3: Tools
        vec![
            btn("🔍 Search Token", "tokens:search"),
            btn("📊 Filter Stats", "tokens:stats"),
        ],
        // Row 4: Navigation
        vec![btn("◀️ Back to Menu", "menu:main")],
    ])
}

/// Paginated token list with navigation controls
/// `view` is one of: passed, rejected, recent, all, blacklisted
pub fn tokens_list_keyboard(
    view: &str,
    current_page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Row 1: Pagination (only if multiple pages)
    if total_pages > 1 {
        let mut nav_row = Vec::new();

        // Previous Button (pages are 1-indexed, so check > 1)
        if current_page > 1 {
            nav_row.push(btn(
                "⬅️ Prev",
                &format!("tokens:page:{}:{}", view, current_page - 1),
            ));
        }

        // Page Indicator
        nav_row.push(btn(
            &format!("{}/{}", current_page + 1, total_pages),
            "noop",
        ));

        // Next Button
        if current_page < total_pages.saturating_sub(1) {
            nav_row.push(btn(
                "➡️ Next",
                &format!("tokens:page:{}:{}", view, current_page + 1),
            ));
        }

        rows.push(nav_row);
    }

    // Row 2: Actions
    rows.push(vec![
        btn("🔄 Refresh", &format!("tokens:refresh:{view}")),
        btn("◀️ Back", "tokens:menu"),
    ]);

    InlineKeyboardMarkup::new(rows)
}

/// Token detail actions with buy options or position link
/// If has_position is true, shows "View Position" instead of buy buttons
pub fn token_detail_keyboard(mint: &str, has_position: bool) -> InlineKeyboardMarkup {
    let m = mint_short(mint);
    let dex_url = format!("https://dexscreener.com/solana/{mint}");

    if has_position {
        // Token already in position - show position link
        InlineKeyboardMarkup::new(vec![
            // Row 1: Position link
            vec![btn("📊 View Position", &format!("pos:{m}"))],
            // Row 2: Actions
            vec![
                btn("🚫 Blacklist", &format!("token:blacklist:{m}")),
                url_btn("🔗 DexScreener", &dex_url),
            ],
            // Row 3: Navigation
            vec![btn("◀️ Back to Tokens", "tokens:menu")],
        ])
    } else {
        // No position - show buy buttons
        InlineKeyboardMarkup::new(vec![
            // Row 1: Buy options
            vec![
                btn("💰 0.1 SOL", &format!("token:buy:{m}:0.1")),
                btn("💰 0.25 SOL", &format!("token:buy:{m}:0.25")),
                btn("💰 0.5 SOL", &format!("token:buy:{m}:0.5")),
            ],
            // Row 2: Actions
            vec![
                btn("🚫 Blacklist", &format!("token:blacklist:{m}")),
                url_btn("🔗 DexScreener", &dex_url),
            ],
            // Row 3: Navigation
            vec![btn("◀️ Back to Tokens", "tokens:menu")],
        ])
    }
}

/// Filter stats view with refresh and back buttons
pub fn filter_stats_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        btn("🔄 Refresh Stats", "tokens:stats:refresh"),
        btn("◀️ Back", "tokens:menu"),
    ]])
}
