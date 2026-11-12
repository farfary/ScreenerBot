// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token blacklist database — persists permanently blocked token addresses and the removals a
//! user made. This module owns the write rule: no automatic source files a token the user
//! removed, and no automatic source replaces a stronger entry.

use crate::errors::DatabaseError;
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::database::WriteTransaction;
use crate::tokens::types::TokenResult;

use super::{TokenBlacklistRecord, TokenDatabase};

/// Seconds an `auto_swap_route` entry stays before the hourly cleanup lifts it so the next
/// quote retests the route.
pub const ROUTE_ENTRY_LIFETIME_SECS: i64 = 24 * 60 * 60;

/// Who wrote a blacklist entry. The stored `source` text is [`BlacklistSource::code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlacklistSource {
    /// The user, from the dashboard or Telegram.
    Manual,
    /// The hourly authority scan: a mint or freeze authority is present.
    AutoCleanup,
    /// The loss rule after a position closed at a significant loss.
    AutoLoss,
    /// The background LLM analysis.
    AutoLlmAnalysis,
    /// The swap path found no route for the token.
    AutoSwapRoute,
}

impl BlacklistSource {
    /// Every source, in the order the dashboard lists them.
    pub const ALL: [Self; 5] = [
        Self::Manual,
        Self::AutoCleanup,
        Self::AutoLoss,
        Self::AutoLlmAnalysis,
        Self::AutoSwapRoute,
    ];

    /// The text stored in `blacklist.source` and sent to the dashboard.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::AutoCleanup => "auto_cleanup",
            Self::AutoLoss => "auto_loss",
            Self::AutoLlmAnalysis => "auto_llm_analysis",
            Self::AutoSwapRoute => "auto_swap_route",
        }
    }
}

/// Whether `source` names an automatic rule. Every other source is a decision a person made.
pub fn is_automatic_source(source: &str) -> bool {
    source.starts_with("auto_")
}

/// How firmly an entry holds. A write never replaces an entry that holds more firmly: a
/// person's entry outranks an automatic entry that stays until a person removes it (loss,
/// LLM), which outranks one the hourly cleanup lifts when its cause ends (route, authority).
fn entry_strength(source: &str) -> u8 {
    if !is_automatic_source(source) {
        2
    } else if source == BlacklistSource::AutoSwapRoute.code()
        || source == BlacklistSource::AutoCleanup.code()
    {
        0
    } else {
        1
    }
}

/// What [`TokenDatabase::add_to_blacklist`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlacklistWrite {
    /// The entry was stored.
    Written,
    /// An automatic source was refused because the user removed this token's entry.
    RefusedUserRemoved,
    /// An automatic source was refused because the token already holds a firmer entry.
    RefusedFirmerEntry,
}

impl BlacklistWrite {
    /// Whether the entry was stored.
    pub const fn is_written(self) -> bool {
        matches!(self, Self::Written)
    }
}

fn query_failed(operation: &'static str) -> impl Fn(rusqlite::Error) -> crate::tokens::Error {
    move |error| DatabaseError::classify_sqlite_failure(operation, error).into()
}

fn read_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<TokenBlacklistRecord> {
    Ok(TokenBlacklistRecord {
        mint: row.get(0)?,
        reason: row.get(1)?,
        source: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        added_at: row.get(3)?,
    })
}

impl TokenDatabase {
    /// Add a token to the blacklist with a reason and source.
    ///
    /// The one write path. A manual source (any source that is not `auto_*`) replaces the
    /// entry and forgets an earlier user removal. An automatic source is refused while the
    /// user's removal of this token is remembered, and never replaces a firmer entry.
    pub fn add_to_blacklist(
        &self,
        mint: &str,
        reason: &str,
        source: &str,
    ) -> TokenResult<BlacklistWrite> {
        let mut conn = self.conn()?;
        let chain_id = self.chain_id();
        let tx = conn
            .write_tx()
            .map_err(query_failed("begin blacklist write"))?;

        if is_automatic_source(source) {
            let cleared = tx
                .query_row(
                    "SELECT 1 FROM blacklist_cleared WHERE chain_id = ?1 AND mint = ?2",
                    params![chain_id, mint],
                    |_| Ok(()),
                )
                .optional()
                .map_err(query_failed("read blacklist removal"))?
                .is_some();
            if cleared {
                return Ok(BlacklistWrite::RefusedUserRemoved);
            }
            let existing: Option<Option<String>> = tx
                .query_row(
                    "SELECT source FROM blacklist WHERE chain_id = ?1 AND mint = ?2",
                    params![chain_id, mint],
                    |row| row.get(0),
                )
                .optional()
                .map_err(query_failed("read blacklist entry"))?;
            if let Some(existing) = existing {
                if entry_strength(existing.as_deref().unwrap_or_default()) > entry_strength(source)
                {
                    return Ok(BlacklistWrite::RefusedFirmerEntry);
                }
            }
        } else {
            tx.execute(
                "DELETE FROM blacklist_cleared WHERE chain_id = ?1 AND mint = ?2",
                params![chain_id, mint],
            )
            .map_err(query_failed("forget blacklist removal"))?;
        }

        tx.execute(
            "INSERT OR REPLACE INTO blacklist (chain_id, mint, reason, source, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![chain_id, mint, reason, source, Utc::now().timestamp()],
        )
        .map_err(query_failed("add to blacklist"))?;
        tx.commit()
            .map_err(query_failed("commit blacklist write"))?;
        Ok(BlacklistWrite::Written)
    }

    /// List all blacklist entries with metadata for diagnostics/analytics
    pub fn list_blacklisted_tokens(&self) -> TokenResult<Vec<TokenBlacklistRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT mint, reason, source, added_at \
                 FROM blacklist WHERE chain_id = ?1 \
                 ORDER BY added_at DESC",
            )
            .map_err(query_failed("prepare blacklist query"))?;
        let rows = stmt
            .query_map(params![self.chain_id()], read_entry)
            .map_err(query_failed("query blacklist"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(query_failed("read blacklist row"))
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> TokenResult<bool> {
        let conn = self.conn()?;
        let blacklisted = conn
            .prepare("SELECT 1 FROM blacklist WHERE chain_id = ?1 AND mint = ?2")
            .map_err(query_failed("prepare blacklist check"))?
            .exists(params![self.chain_id(), mint])
            .map_err(query_failed("check blacklist"))?;
        Ok(blacklisted)
    }

    /// The token's blacklist entry, if it has one.
    pub fn get_blacklist_entry(&self, mint: &str) -> TokenResult<Option<TokenBlacklistRecord>> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT mint, reason, source, added_at FROM blacklist WHERE chain_id = ?1 AND mint = ?2",
            params![self.chain_id(), mint],
            read_entry,
        )
        .optional()
        .map_err(query_failed("read blacklist entry"))
    }

    /// Whether the user removed this token's blacklist entry and no manual blacklist has
    /// followed.
    pub fn is_blacklist_cleared(&self, mint: &str) -> TokenResult<bool> {
        let conn = self.conn()?;
        let cleared = conn
            .prepare("SELECT 1 FROM blacklist_cleared WHERE chain_id = ?1 AND mint = ?2")
            .map_err(query_failed("prepare blacklist removal check"))?
            .exists(params![self.chain_id(), mint])
            .map_err(query_failed("check blacklist removal"))?;
        Ok(cleared)
    }

    /// The user's removal of a token from the blacklist: delete the entry and remember the
    /// removal, so no automatic source files the token again. Returns the removed entry.
    /// A token with no entry has nothing to remember and changes nothing.
    pub fn clear_blacklist_entry(&self, mint: &str) -> TokenResult<Option<TokenBlacklistRecord>> {
        let mut conn = self.conn()?;
        let chain_id = self.chain_id();
        let tx = conn
            .write_tx()
            .map_err(query_failed("begin blacklist removal"))?;
        let removed = tx
            .query_row(
                "SELECT mint, reason, source, added_at FROM blacklist
                 WHERE chain_id = ?1 AND mint = ?2",
                params![chain_id, mint],
                read_entry,
            )
            .optional()
            .map_err(query_failed("read blacklist entry"))?;
        if let Some(entry) = &removed {
            tx.execute(
                "DELETE FROM blacklist WHERE chain_id = ?1 AND mint = ?2",
                params![chain_id, mint],
            )
            .map_err(query_failed("remove from blacklist"))?;
            tx.execute(
                "INSERT OR REPLACE INTO blacklist_cleared
                     (chain_id, mint, cleared_at, previous_reason, previous_source)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    chain_id,
                    mint,
                    Utc::now().timestamp(),
                    entry.reason,
                    entry.source
                ],
            )
            .map_err(query_failed("remember blacklist removal"))?;
        }
        tx.commit()
            .map_err(query_failed("commit blacklist removal"))?;
        Ok(removed)
    }

    /// Lift an automatic entry without remembering a removal, so the rule that filed it may
    /// file it again if its cause returns. The entry goes only while it is still the one
    /// read (same source and time): a manual blacklist written since stays. Returns whether
    /// an entry was removed.
    pub fn lift_blacklist_entry(&self, entry: &TokenBlacklistRecord) -> TokenResult<bool> {
        let conn = self.conn()?;
        let removed = conn
            .execute(
                "DELETE FROM blacklist
                 WHERE chain_id = ?1 AND mint = ?2 AND source = ?3 AND added_at = ?4",
                params![self.chain_id(), entry.mint, entry.source, entry.added_at],
            )
            .map_err(query_failed("lift blacklist entry"))?;
        Ok(removed > 0)
    }
}
