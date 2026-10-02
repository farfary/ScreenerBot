// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Settings keyboards: quick settings, notification toggles and trading controls.

use super::Labels;
use crate::i18n::{ids, MessageId};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Quick settings menu
pub fn settings_menu() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![
        // Row 1: Notification settings
        vec![
            l.btn(
                "🔔",
                ids::TELEGRAM_BUTTON_NOTIFICATIONS,
                "settings:notifications",
            ),
            l.btn("⚡", ids::TELEGRAM_BUTTON_TRADING, "settings:trading"),
        ],
        // Row 2: Monitor controls
        vec![
            l.btn(
                "📥",
                ids::TELEGRAM_BUTTON_ENTRY_MONITOR,
                "toggle:entry_monitor",
            ),
            l.btn(
                "📤",
                ids::TELEGRAM_BUTTON_EXIT_MONITOR,
                "toggle:exit_monitor",
            ),
        ],
        // Row 3: Back
        vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK_TO_MENU, "menu:main")],
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
    let l = Labels::new();
    let toggle = |enabled: bool, id: MessageId, key: &str| -> InlineKeyboardButton {
        let emoji = if enabled { "🟢" } else { "⚪" };
        l.btn(emoji, id, &format!("toggle:{key}"))
    };

    InlineKeyboardMarkup::new(vec![
        vec![
            toggle(
                pos_opened,
                ids::TELEGRAM_BUTTON_NOTIFY_OPENED,
                "notify_opened",
            ),
            toggle(
                pos_closed,
                ids::TELEGRAM_BUTTON_NOTIFY_CLOSED,
                "notify_closed",
            ),
        ],
        vec![
            toggle(
                partial_exit,
                ids::TELEGRAM_BUTTON_NOTIFY_PARTIAL,
                "notify_partial",
            ),
            toggle(dca, ids::TELEGRAM_BUTTON_NOTIFY_DCA, "notify_dca"),
        ],
        vec![toggle(
            errors,
            ids::TELEGRAM_BUTTON_NOTIFY_ERRORS,
            "notify_errors",
        )],
        vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK, "menu:settings")],
    ])
}

/// Trading controls with current state
pub fn trading_controls(
    entry_enabled: bool,
    exit_enabled: bool,
    auto_trading: bool,
) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let toggle = |enabled: bool, id: MessageId, key: &str| -> InlineKeyboardButton {
        let emoji = if enabled { "🟢" } else { "🔴" };
        l.btn(emoji, id, &format!("toggle:{key}"))
    };

    InlineKeyboardMarkup::new(vec![
        vec![
            toggle(
                entry_enabled,
                ids::TELEGRAM_BUTTON_ENTRY_MONITOR,
                "entry_monitor",
            ),
            toggle(
                exit_enabled,
                ids::TELEGRAM_BUTTON_EXIT_MONITOR,
                "exit_monitor",
            ),
        ],
        vec![toggle(
            auto_trading,
            ids::TELEGRAM_BUTTON_AUTO_TRADING,
            "auto_trading",
        )],
        vec![l.btn("🚨", ids::TELEGRAM_BUTTON_FORCE_STOP, "confirm:force_stop")],
        vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK, "menu:settings")],
    ])
}
