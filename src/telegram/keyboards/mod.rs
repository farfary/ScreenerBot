// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Telegram keyboard builders for ScreenerBot
//!
//! Provides pre-built keyboard layouts for:
//! - Reply keyboard (persistent bottom keyboard)
//! - Main menu navigation (inline)
//! - Position management actions
//! - Confirmation dialogs
//! - Settings quick toggles
//! - Token explorer (see `tokens`)
//!
//! Button labels are plain text rendered for the Telegram language; the icon
//! is a glyph prefix kept in Rust. Callback data is a protocol id and is never
//! localized.

mod settings;
mod tokens;

pub use settings::{notification_settings, settings_menu, trading_controls};
pub use tokens::{
    filter_stats_keyboard, pagination_keyboard, token_detail_keyboard, tokens_list_keyboard,
    tokens_menu,
};

use crate::i18n::{ids, LanguageIdentifier, MessageId, UiArg, UiText};
use crate::telegram::reply::ReplyCommand;
use crate::telegram::text::{locale, tg_plain_with, with_icon};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup};

/// Button label renderer bound to one language, so a keyboard resolves the
/// language once.
struct Labels(LanguageIdentifier);

impl Labels {
    fn new() -> Self {
        Self(locale())
    }

    fn word(&self, id: MessageId) -> String {
        tg_plain_with(&self.0, &UiText::new(id))
    }

    fn with(&self, id: MessageId, args: &[(&'static str, String)]) -> String {
        let text = args.iter().fold(UiText::new(id), |text, (name, value)| {
            text.arg(*name, UiArg::Text(value.clone()))
        });
        tg_plain_with(&self.0, &text)
    }

    /// Callback button: `icon` glyph, then the localized word.
    fn btn(&self, icon: &str, id: MessageId, callback_data: &str) -> InlineKeyboardButton {
        btn(&with_icon(icon, &self.word(id)), callback_data)
    }

    /// Callback button without an icon.
    fn text_btn(&self, id: MessageId, callback_data: &str) -> InlineKeyboardButton {
        btn(&self.word(id), callback_data)
    }
}

// === REPLY KEYBOARD (Bottom persistent keyboard) ===

/// Create the main reply keyboard that appears at the bottom of Telegram
/// This replaces the default keyboard and persists until removed
pub fn main_reply_keyboard() -> KeyboardMarkup {
    let locale = locale();
    KeyboardMarkup::new(ReplyCommand::ROWS.iter().map(|row| {
        row.iter()
            .map(|cmd| KeyboardButton::new(cmd.label(&locale)))
            .collect::<Vec<_>>()
    }))
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
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![
        // Row 1: Primary info
        vec![
            l.btn("📊", ids::TELEGRAM_BUTTON_POSITIONS, "menu:positions"),
            l.btn("💰", ids::TELEGRAM_BUTTON_BALANCE, "cmd:balance"),
            l.btn("📈", ids::TELEGRAM_BUTTON_STATS, "cmd:stats"),
        ],
        // Row 2: Token Explorer & Controls
        vec![
            l.btn("🔍", ids::TELEGRAM_BUTTON_TOKENS, "menu:tokens"),
            l.btn("⏸️", ids::TELEGRAM_BUTTON_PAUSE, "cmd:pause_entries"),
            l.btn("⏹️", ids::TELEGRAM_BUTTON_STOP, "cmd:stop_trader"),
        ],
        // Row 3: Settings & Refresh
        vec![
            l.btn("⚙️", ids::TELEGRAM_BUTTON_SETTINGS, "menu:settings"),
            l.btn("🔄", ids::TELEGRAM_BUTTON_REFRESH, "menu:refresh"),
        ],
    ])
}

/// Compact main menu (for use after other messages)
pub fn main_menu_compact() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn("📊", ids::TELEGRAM_BUTTON_POSITIONS, "menu:positions"),
        l.btn("💰", ids::TELEGRAM_BUTTON_BALANCE, "cmd:balance"),
        l.btn("◀️", ids::TELEGRAM_BUTTON_MENU, "menu:main"),
    ]])
}

// === POSITIONS ===

/// Positions list with individual position buttons
/// `positions` is a list of (symbol, mint, pnl_pct)
pub fn positions_list(positions: &[(String, String, f64)]) -> InlineKeyboardMarkup {
    let l = Labels::new();
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
        rows.push(vec![l.btn(
            "❌",
            ids::TELEGRAM_BUTTON_CLOSE_ALL_POSITIONS,
            "confirm:closeall",
        )]);
    }

    // Back button
    rows.push(vec![l.btn(
        "◀️",
        ids::TELEGRAM_BUTTON_BACK_TO_MENU,
        "menu:main",
    )]);

    InlineKeyboardMarkup::new(rows)
}

/// Single position detail view with action buttons
pub fn position_actions(mint: &str, _symbol: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);
    let sell = |percent: u32| {
        btn(
            &l.with(
                ids::TELEGRAM_BUTTON_SELL_PERCENT,
                &[("percent", percent.to_string())],
            ),
            &format!("sell:{m}:{percent}"),
        )
    };
    let dca = |amount: &str| {
        btn(
            &with_icon(
                "➕",
                &l.with(
                    ids::TELEGRAM_BUTTON_DCA_AMOUNT,
                    &[("amount", amount.to_owned())],
                ),
            ),
            &format!("dca:{m}:{amount}"),
        )
    };

    InlineKeyboardMarkup::new(vec![
        // Row 1: Sell percentages
        vec![sell(25), sell(50), sell(75), sell(100)],
        // Row 2: DCA options
        vec![dca("0.1"), dca("0.25"), dca("0.5")],
        // Row 3: Actions
        vec![
            l.btn("🚫", ids::TELEGRAM_BUTTON_BLACKLIST, &format!("bl:{m}")),
            l.btn(
                "❌",
                ids::TELEGRAM_BUTTON_CLOSE_POSITION,
                &format!("confirm:close:{m}"),
            ),
        ],
        // Row 4: Navigation
        vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK, "menu:positions")],
    ])
}

/// Compact position actions (for notifications)
pub fn position_actions_compact(mint: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        l.btn("📊", ids::TELEGRAM_BUTTON_DETAILS, &format!("pos:{m}")),
        l.btn("🚫", ids::TELEGRAM_BUTTON_BLACKLIST, &format!("bl:{m}")),
    ]])
}

// === CONFIRMATION DIALOGS ===

/// Confirmation dialog for closing a position
pub fn confirm_close(mint: &str, _symbol: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        l.btn(
            "✅",
            ids::TELEGRAM_BUTTON_CONFIRM_CLOSE,
            &format!("exec:close:{m}"),
        ),
        l.btn(
            "❌",
            ids::TELEGRAM_BUTTON_CANCEL,
            &format!("cancel:close:{m}"),
        ),
    ]])
}

/// Confirmation dialog for closing all positions
pub fn confirm_close_all() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn(
            "✅",
            ids::TELEGRAM_BUTTON_CONFIRM_CLOSE_ALL,
            "exec:closeall",
        ),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, "menu:positions"),
    ]])
}

/// Confirmation dialog for selling a percentage
pub fn confirm_sell(mint: &str, percent: u32) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &with_icon(
                "✅",
                &l.with(
                    ids::TELEGRAM_BUTTON_CONFIRM_SELL,
                    &[("percent", percent.to_string())],
                ),
            ),
            &format!("exec:sell:{m}:{percent}"),
        ),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, &format!("pos:{m}")),
    ]])
}

/// Confirmation dialog for DCA
pub fn confirm_dca(mint: &str, amount: f64) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &with_icon(
                "✅",
                &l.with(
                    ids::TELEGRAM_BUTTON_CONFIRM_DCA,
                    &[("amount", amount.to_string())],
                ),
            ),
            &format!("exec:dca:{m}:{amount}"),
        ),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, &format!("pos:{m}")),
    ]])
}

/// Confirmation for force stop
pub fn confirm_force_stop() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn(
            "🚨",
            ids::TELEGRAM_BUTTON_CONFIRM_FORCE_STOP,
            "exec:force_stop",
        ),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, "menu:main"),
    ]])
}

/// Blacklist button labelled with the token symbol.
fn blacklist_symbol_label(l: &Labels, symbol: &str) -> String {
    with_icon(
        "🚫",
        &l.with(
            ids::TELEGRAM_BUTTON_BLACKLIST_SYMBOL,
            &[("symbol", symbol.to_owned())],
        ),
    )
}

/// Confirmation for blacklisting a token (from position context)
pub fn confirm_blacklist(mint: &str, symbol: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(&blacklist_symbol_label(&l, symbol), &format!("exec:bl:{m}")),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, &format!("pos:{m}")),
    ]])
}

/// Confirmation for blacklisting a token (from token explorer - no position)
pub fn confirm_token_blacklist(mint: &str, symbol: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &blacklist_symbol_label(&l, symbol),
            &format!("exec:tokenbl:{m}"),
        ),
        l.btn("❌", ids::TELEGRAM_BUTTON_CANCEL, "tokens:menu"),
    ]])
}

/// Confirmation for buying a token
pub fn confirm_token_buy(mint: &str, _symbol: &str, amount: f64) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![vec![
        btn(
            &with_icon(
                "✅",
                &l.with(
                    ids::TELEGRAM_BUTTON_CONFIRM_BUY,
                    &[("amount", amount.to_string())],
                ),
            ),
            &format!("exec:tokenbuy:{m}:{amount}"),
        ),
        l.btn(
            "❌",
            ids::TELEGRAM_BUTTON_CANCEL,
            &format!("token:view:{m}"),
        ),
    ]])
}

// === NOTIFICATION BUTTONS ===

/// Solscan transaction link button.
fn solscan_row(l: &Labels, signature: &str) -> Vec<InlineKeyboardButton> {
    let url = format!("https://solscan.io/tx/{signature}");
    vec![url_btn(
        &with_icon("🔗", &l.word(ids::TELEGRAM_BUTTON_SOLSCAN)),
        &url,
    )]
}

/// Buttons for position opened notification
pub fn on_position_opened(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![
        vec![
            l.btn("📊", ids::TELEGRAM_BUTTON_DETAILS, &format!("pos:{m}")),
            l.btn("🚫", ids::TELEGRAM_BUTTON_BLACKLIST, &format!("bl:{m}")),
        ],
        solscan_row(&l, signature),
    ])
}

/// Buttons for position closed notification
pub fn on_position_closed(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![
        vec![
            l.btn("📋", ids::TELEGRAM_BUTTON_HISTORY, "cmd:history"),
            l.btn(
                "🚫",
                ids::TELEGRAM_BUTTON_BLACKLIST,
                &format!("exec:bl:{m}"),
            ),
        ],
        solscan_row(&l, signature),
    ])
}

/// Buttons for partial exit notification
pub fn on_partial_exit(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![
        vec![
            l.btn("📊", ids::TELEGRAM_BUTTON_POSITION, &format!("pos:{m}")),
            l.text_btn(ids::TELEGRAM_BUTTON_SELL_MORE, &format!("pos:{m}")),
        ],
        solscan_row(&l, signature),
    ])
}

/// Buttons for DCA notification
pub fn on_dca_executed(mint: &str, signature: &str) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);

    InlineKeyboardMarkup::new(vec![
        vec![
            l.btn("📊", ids::TELEGRAM_BUTTON_POSITION, &format!("pos:{m}")),
            l.btn("➕", ids::TELEGRAM_BUTTON_MORE_DCA, &format!("pos:{m}")),
        ],
        solscan_row(&l, signature),
    ])
}

/// Buttons for error notification
pub fn on_error() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn("📊", ids::TELEGRAM_BUTTON_STATUS, "cmd:status"),
        l.btn("🔄", ids::TELEGRAM_BUTTON_REFRESH, "menu:refresh"),
    ]])
}

/// Buttons for startup/shutdown notification
pub fn on_system_event() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn("📊", ids::TELEGRAM_BUTTON_STATUS, "cmd:status"),
        l.btn("📊", ids::TELEGRAM_BUTTON_POSITIONS, "menu:positions"),
    ]])
}

// === AUTHENTICATION ===

/// Authentication prompt (no buttons, user types password/code)
pub fn auth_prompt() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![l.btn(
        "❌",
        ids::TELEGRAM_BUTTON_CANCEL,
        "auth:cancel",
    )]])
}

/// Session expired message
pub fn session_expired() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![l.btn(
        "🔑",
        ids::TELEGRAM_BUTTON_REAUTHENTICATE,
        "auth:start",
    )]])
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

    #[test]
    fn reply_keyboard_keeps_the_three_by_three_layout() {
        let keyboard = main_reply_keyboard();
        assert_eq!(keyboard.keyboard.len(), 3);
        assert!(keyboard.keyboard.iter().all(|row| row.len() == 3));
    }
}
