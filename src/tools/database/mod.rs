// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tools Database Module
//!
//! SQLite database for persistent storage of tool operations:
//! - Tool favorites
//! - Multi-wallet sessions
//! - Watched tokens

// Sub-modules
mod favorites;
mod multi_wallet;
mod types;
mod watched_tokens;

// Re-export types
pub use types::{
    MwSessionConfig, MwSessionRow, MwWalletOpRow, ToolFavoriteRow, WatchedToken,
    WatchedTokenConfig,
};

// Re-export favorites operations
pub use favorites::{
    get_tool_favorites, increment_tool_favorite_use, remove_tool_favorite, update_tool_favorite,
    upsert_tool_favorite,
};

// Re-export multi-wallet operations
pub use multi_wallet::{
    add_wallet_op, create_mw_session, get_mw_session, get_recent_mw_sessions, get_session_ops,
    update_mw_session_metrics, update_mw_session_status, update_wallet_op_status,
};

// Re-export watched tokens operations
pub use watched_tokens::{
    add_watched_token, delete_watched_token, get_active_watched_tokens, get_watched_tokens,
    update_watched_token_status, update_watched_token_tracking,
};
