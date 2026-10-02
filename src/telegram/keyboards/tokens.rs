// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Keyboards for message pagination and the token explorer.

use super::{btn, mint_short, url_btn, Labels};
use crate::i18n::ids;
use crate::telegram::text::with_icon;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

// === PAGINATION KEYBOARD ===

/// Create pagination controls
pub fn pagination_keyboard(
    session_id: &str,
    current_page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let mut row = Vec::new();

    // Previous Button
    if current_page > 0 {
        row.push(l.btn(
            "⬅️",
            ids::TELEGRAM_BUTTON_PREVIOUS,
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
            &format!("{} ➡️", l.word(ids::TELEGRAM_BUTTON_NEXT)),
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
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![
        // Row 1: Primary views
        vec![
            l.btn("✅", ids::TELEGRAM_BUTTON_PASSED, "tokens:passed"),
            l.btn("❌", ids::TELEGRAM_BUTTON_REJECTED, "tokens:rejected"),
        ],
        // Row 2: Additional views
        vec![
            l.btn("🆕", ids::TELEGRAM_BUTTON_NEW_24H, "tokens:recent"),
            l.btn("📋", ids::TELEGRAM_BUTTON_ALL_TOKENS, "tokens:all"),
        ],
        // Row 3: Tools
        vec![
            l.btn("🔍", ids::TELEGRAM_BUTTON_SEARCH_TOKEN, "tokens:search"),
            l.btn("📊", ids::TELEGRAM_BUTTON_FILTER_STATS, "tokens:stats"),
        ],
        // Row 4: Navigation
        vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK_TO_MENU, "menu:main")],
    ])
}

/// Paginated token list with navigation controls
/// `view` is one of: passed, rejected, recent, all, blacklisted
pub fn tokens_list_keyboard(
    view: &str,
    current_page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Row 1: Pagination (only if multiple pages)
    if total_pages > 1 {
        let mut nav_row = Vec::new();

        // Previous Button (pages are 1-indexed, so check > 1)
        if current_page > 1 {
            nav_row.push(l.btn(
                "⬅️",
                ids::TELEGRAM_BUTTON_PREVIOUS,
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
            nav_row.push(l.btn(
                "➡️",
                ids::TELEGRAM_BUTTON_NEXT,
                &format!("tokens:page:{}:{}", view, current_page + 1),
            ));
        }

        rows.push(nav_row);
    }

    // Row 2: Actions
    rows.push(vec![
        l.btn(
            "🔄",
            ids::TELEGRAM_BUTTON_REFRESH,
            &format!("tokens:refresh:{view}"),
        ),
        l.btn("◀️", ids::TELEGRAM_BUTTON_BACK, "tokens:menu"),
    ]);

    InlineKeyboardMarkup::new(rows)
}

/// Token detail actions with buy options or position link
/// If has_position is true, shows "View Position" instead of buy buttons
pub fn token_detail_keyboard(mint: &str, has_position: bool) -> InlineKeyboardMarkup {
    let l = Labels::new();
    let m = mint_short(mint);
    let dex_url = format!("https://dexscreener.com/solana/{mint}");
    let dex = url_btn(
        &with_icon("🔗", &l.word(ids::TELEGRAM_BUTTON_DEXSCREENER)),
        &dex_url,
    );
    let blacklist = l.btn(
        "🚫",
        ids::TELEGRAM_BUTTON_BLACKLIST,
        &format!("token:blacklist:{m}"),
    );
    let back = vec![l.btn("◀️", ids::TELEGRAM_BUTTON_BACK_TO_TOKENS, "tokens:menu")];

    if has_position {
        // Token already in position - show position link
        InlineKeyboardMarkup::new(vec![
            // Row 1: Position link
            vec![l.btn(
                "📊",
                ids::TELEGRAM_BUTTON_VIEW_POSITION,
                &format!("pos:{m}"),
            )],
            // Row 2: Actions
            vec![blacklist, dex],
            // Row 3: Navigation
            back,
        ])
    } else {
        // No position - show buy buttons
        let buy = |amount: &str| {
            let label = l.with(
                ids::TELEGRAM_BUTTON_BUY_AMOUNT,
                &[("amount", amount.to_owned())],
            );
            btn(&with_icon("💰", &label), &format!("token:buy:{m}:{amount}"))
        };
        InlineKeyboardMarkup::new(vec![
            // Row 1: Buy options
            vec![buy("0.1"), buy("0.25"), buy("0.5")],
            // Row 2: Actions
            vec![blacklist, dex],
            // Row 3: Navigation
            back,
        ])
    }
}

/// Filter stats view with refresh and back buttons
pub fn filter_stats_keyboard() -> InlineKeyboardMarkup {
    let l = Labels::new();
    InlineKeyboardMarkup::new(vec![vec![
        l.btn(
            "🔄",
            ids::TELEGRAM_BUTTON_REFRESH_STATS,
            "tokens:stats:refresh",
        ),
        l.btn("◀️", ids::TELEGRAM_BUTTON_BACK, "tokens:menu"),
    ]])
}
