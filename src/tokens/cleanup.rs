// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Cleanup logic - Blacklist management and database maintenance
//!
//! Automatically blacklists tokens based on permanent token characteristics:
//! - mint_authority present (can mint unlimited tokens)
//! - freeze_authority present (can freeze user accounts)
//!
//! NOTE: Liquidity and security scores are FILTERING criteria, not blacklist criteria.
//! Blacklist is for tokens that should NEVER be traded due to fundamental risks.
//!
//! The chain's native and stablecoin quote assets are never blacklisted automatically: a
//! stablecoin issuer holds mint and freeze authority by design, and blacklisting the quote side
//! drops its pools from discovery and refuses trades into it.
//!
//! An automatic entry lasts only while its cause does. The hourly loop lifts a no-route entry
//! after [`ROUTE_ENTRY_LIFETIME_SECS`] and an authority entry once the stored Rugcheck report
//! shows neither authority; loss and LLM entries stay until a person removes them. A token the
//! user removes from the blacklist is not filed again by any automatic source
//! (`TokenDatabase::add_to_blacklist`).

use crate::chains::{ChainId, ChainScope};
use crate::errors::{DatabaseError, InternalError};
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::tokens::database::{
    is_automatic_source, BlacklistSource, BlacklistWrite, TokenBlacklistRecord, TokenDatabase,
    ROUTE_ENTRY_LIFETIME_SECS,
};
use crate::tokens::types::TokenResult;
use crate::tokens::Error;
use crate::utils::{check_shutdown_or_delay, run_or_shutdown};
use chrono::Utc;
use rusqlite::params;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

// ============================================================================
// BLACKLIST CONDITIONS
// ============================================================================

/// Check if a token should be blacklisted based on authorities
///
/// Blacklist ONLY if:
/// - mint_authority IS NOT NULL (can mint unlimited tokens)
/// - freeze_authority IS NOT NULL (can freeze user accounts)
///
/// These are permanent token characteristics that make tokens fundamentally unsafe.
/// Liquidity and security scores belong in filtering, not blacklisting.
pub async fn should_blacklist_for_authorities(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<String>> {
    if let Some(rugcheck_data) = db.get_rugcheck_data(mint)? {
        // Check mint authority
        if let Some(mint_auth) = &rugcheck_data.mint_authority {
            return Ok(Some(format!("Mint authority present: {mint_auth}")));
        }

        // Check freeze authority
        if let Some(freeze_auth) = &rugcheck_data.freeze_authority {
            return Ok(Some(format!("Freeze authority present: {freeze_auth}")));
        }
    }

    Ok(None)
}

/// True for the chain's native asset and its canonical stablecoin quote assets, which no
/// automatic rule may blacklist.
pub fn is_exempt_from_auto_blacklist(chain: ChainId, mint: &str) -> bool {
    crate::chains::adapter_for(chain).is_quote_asset(mint)
}

/// Evaluate all blacklist conditions for a token
///
/// Only checks for permanent token characteristics (authorities).
/// Filtering criteria (liquidity, security scores) are handled by the filtering system.
pub async fn evaluate_blacklist(mint: &str, db: &TokenDatabase) -> TokenResult<Option<String>> {
    if is_exempt_from_auto_blacklist(db.chain(), mint) || db.is_blacklisted(mint)? {
        return Ok(None);
    }

    // ONLY check authorities - this is the correct blacklist criteria
    should_blacklist_for_authorities(mint, db).await
}

// ============================================================================
// CLEANUP TASKS
// ============================================================================

/// Run cleanup scan on all tokens
///
/// Checks all non-blacklisted tokens and blacklists any that meet conditions
pub async fn run_cleanup_scan(db: &TokenDatabase) -> TokenResult<CleanupResult> {
    let mut checked = 0;
    let mut blacklisted = 0;
    let mut errors = 0;

    // Get all non-blacklisted tokens (limit to 10000 for performance)
    let tokens = db.list_tokens(10000)?;

    for token in tokens {
        checked += 1;

        // Skip if already blacklisted
        if db.is_blacklisted(&token.mint)? {
            continue;
        }

        // Evaluate blacklist conditions
        match evaluate_blacklist(&token.mint, db).await {
            Ok(Some(reason)) => {
                match blacklist_token(
                    &token.mint,
                    &reason,
                    BlacklistSource::AutoCleanup.code(),
                    db,
                ) {
                    Ok(BlacklistWrite::Written) => {
                        blacklisted += 1;
                        logger::info(
                            LogTag::Tokens,
                            &format!("[CLEANUP] Blacklisted {}: {}", token.mint, reason),
                        );

                        // Record blacklist event
                        tokio::spawn({
                            let mint = token.mint.clone();
                            let reason_msg = reason.clone();
                            async move {
                                record_token_event(
                                    &mint,
                                    "token_blacklisted",
                                    Severity::Info,
                                    serde_json::json!({
                                        "reason": reason_msg,
                                        "source": BlacklistSource::AutoCleanup.code(),
                                    }),
                                )
                                .await;
                            }
                        });
                    }
                    Ok(BlacklistWrite::RefusedUserRemoved | BlacklistWrite::RefusedFirmerEntry) => {
                    }
                    Err(e) => {
                        errors += 1;
                        logger::error(
                            LogTag::Tokens,
                            &format!("[CLEANUP] Failed to blacklist {}: {}", token.mint, e),
                        );
                    }
                }
            }
            Ok(None) => {
                // No blacklist needed
            }
            Err(e) => {
                errors += 1;
                logger::error(
                    LogTag::Tokens,
                    &format!("[CLEANUP] Error evaluating {}: {}", token.mint, e),
                );
            }
        }
    }

    Ok(CleanupResult {
        checked,
        blacklisted,
        errors,
    })
}

/// Remove an automatic entry an earlier build filed against an exempt quote asset. An entry the
/// owner made by hand stays. Runs once when the cleanup loop starts.
pub fn lift_automatic_quote_asset_entries(db: &TokenDatabase) -> TokenResult<()> {
    let adapter = crate::chains::adapter_for(db.chain());
    let native = adapter.native_asset_address();
    for mint in std::iter::once(native).chain(adapter.stable_assets().iter().copied()) {
        let Some(entry) = db.get_blacklist_entry(mint)? else {
            continue;
        };
        if !is_automatic_source(&entry.source) {
            continue;
        }
        if db.lift_blacklist_entry(&entry)? {
            logger::info(
                LogTag::Tokens,
                &format!(
                    "[CLEANUP] Lifted the {} blacklist entry on quote asset {mint}: {}",
                    entry.source, entry.reason
                ),
            );
        }
    }
    Ok(())
}

/// Lift every entry `ended` selects, without remembering a removal: the rule that filed it
/// may file it again if its cause returns. Returns the entries lifted.
fn lift_entries(
    db: &TokenDatabase,
    ended: impl Fn(&TokenBlacklistRecord) -> TokenResult<bool>,
) -> TokenResult<Vec<TokenBlacklistRecord>> {
    let mut lifted = Vec::new();
    for entry in db.list_blacklisted_tokens()? {
        if ended(&entry)? && db.lift_blacklist_entry(&entry)? {
            crate::tokens::store::invalidate_token_snapshot(db.chain(), &entry.mint);
            lifted.push(entry);
        }
    }
    Ok(lifted)
}

/// Lift the `auto_swap_route` entries at least [`ROUTE_ENTRY_LIFETIME_SECS`] old at `now`, so
/// the next quote retests the route and files the token again if it still has none.
pub fn lift_stale_route_entries(
    db: &TokenDatabase,
    now: i64,
) -> TokenResult<Vec<TokenBlacklistRecord>> {
    lift_entries(db, |entry| {
        Ok(entry.source == BlacklistSource::AutoSwapRoute.code()
            && now.saturating_sub(entry.added_at) >= ROUTE_ENTRY_LIFETIME_SECS)
    })
}

/// Lift the `auto_cleanup` entries whose token has a stored Rugcheck report showing neither a
/// mint nor a freeze authority. A token with no report keeps its entry: a missing report is
/// not a renounced authority.
pub fn lift_renounced_authority_entries(
    db: &TokenDatabase,
) -> TokenResult<Vec<TokenBlacklistRecord>> {
    lift_entries(db, |entry| {
        if entry.source != BlacklistSource::AutoCleanup.code() {
            return Ok(false);
        }
        Ok(db.get_rugcheck_data(&entry.mint)?.is_some_and(|report| {
            report.mint_authority.is_none() && report.freeze_authority.is_none()
        }))
    })
}

/// Announce the entries one lift removed.
async fn announce_lifts(cause: &str, result: TokenResult<Vec<TokenBlacklistRecord>>) {
    let lifted = match result {
        Ok(lifted) => lifted,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("[CLEANUP] Failed to lift entries ({cause}): {e}"),
            );
            return;
        }
    };
    for entry in lifted {
        logger::info(
            LogTag::Tokens,
            &format!(
                "[CLEANUP] Lifted the {} blacklist entry on {} ({cause}): {}",
                entry.source, entry.mint, entry.reason
            ),
        );
        record_token_event(
            &entry.mint,
            "token_blacklist_lifted",
            Severity::Info,
            serde_json::json!({
                "reason": entry.reason,
                "source": entry.source,
                "cause": cause,
            }),
        )
        .await;
    }
}

/// Lift the automatic entries whose cause has ended: stale no-route entries and authority
/// entries whose authorities are renounced. Loss and LLM entries are never lifted here.
async fn lift_ended_entries(db: &Arc<TokenDatabase>) {
    let worker_db = db.clone();
    let lifted = tokio::task::spawn_blocking(move || {
        (
            lift_stale_route_entries(&worker_db, Utc::now().timestamp()),
            lift_renounced_authority_entries(&worker_db),
        )
    })
    .await;
    match lifted {
        Ok((route, authority)) => {
            announce_lifts("route_retest", route).await;
            announce_lifts("authority_renounced", authority).await;
        }
        Err(e) => logger::error(LogTag::Tokens, &format!("[CLEANUP] Lift task failed: {e}")),
    }
}

/// Result of a cleanup scan
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub checked: usize,
    pub blacklisted: usize,
    pub errors: usize,
}

/// Start cleanup loop (runs every hour)
pub fn start_cleanup_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = lift_automatic_quote_asset_entries(&db) {
            logger::error(
                LogTag::Tokens,
                &format!("[CLEANUP] Failed to lift quote asset entries: {e}"),
            );
        }
        lift_ended_entries(&db).await;
        loop {
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(3600)).await {
                break;
            }
            lift_ended_entries(&db).await;
            logger::info(LogTag::Tokens, "[CLEANUP] Starting cleanup scan...");
            // Race the scan against shutdown: it can iterate many tokens, and we must
            // stay parked on notified() so the one-shot shutdown broadcast is not missed.
            match run_or_shutdown(&shutdown, run_cleanup_scan(&db)).await {
                None => break,
                Some(Ok(result)) => {
                    logger::info(
                        LogTag::Tokens,
                        &format!(
                            "[CLEANUP] Scan complete: {} checked, {} blacklisted, {} errors",
                            result.checked, result.blacklisted, result.errors
                        ),
                    );
                }
                Some(Err(e)) => {
                    logger::error(LogTag::Tokens, &format!("[CLEANUP] Scan failed: {e}"));
                }
            }
        }
    })
}

// ============================================================================
// BLACKLIST OPERATIONS
// ============================================================================

/// Add a token to the blacklist.
///
/// `source` records WHO excluded the token and is not cosmetic:
/// [`get_blacklist_summary`] counts every `manual` entry as an owner decision,
/// so an automatic exclusion filed as `manual` misreports the dashboard's
/// blacklist breakdown. Use [`BlacklistSource::Manual`] only for an action the
/// owner took (API, Telegram), and an `auto_*` source otherwise. The write
/// rule (removal memory, firmer entries) lives in `TokenDatabase::add_to_blacklist`.
pub fn blacklist_token(
    mint: &str,
    reason: &str,
    source: &str,
    db: &TokenDatabase,
) -> TokenResult<BlacklistWrite> {
    let write = db.add_to_blacklist(mint, reason, source)?;
    if write.is_written() {
        crate::tokens::store::invalidate_token_snapshot(db.chain(), mint);
    }
    Ok(write)
}

/// Remove a token from the blacklist at the user's request.
///
/// Deletes the token's entry and remembers the removal, so no automatic source files the
/// token again; clears the token's pool and account blacklist rows so it leaves the
/// Blacklisted view; records an event; and rebuilds the filtering snapshot so the views
/// update at once. Only the entry's own removal can fail the call: a pool clear or a
/// snapshot rebuild that fails is logged, since the removal has committed. Returns the
/// removed token entry, `None` when the token had none (it was listed through its pools).
pub async fn remove_blacklisted_token(
    mint: &str,
    db: &Arc<TokenDatabase>,
) -> TokenResult<Option<TokenBlacklistRecord>> {
    let chain = db.chain();
    let worker_db = db.clone();
    let worker_mint = mint.to_owned();
    let removed =
        tokio::task::spawn_blocking(move || worker_db.clear_blacklist_entry(&worker_mint))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))??;
    crate::tokens::store::invalidate_token_snapshot(chain, mint);

    let (pools, accounts) = match crate::pools::db::clear_token_blacklists(chain, mint).await {
        Ok(cleared) => cleared,
        Err(e) => {
            logger::warning(
                LogTag::Tokens,
                &format!("[BLACKLIST] Failed to clear pool blacklist rows of {mint}: {e}"),
            );
            (0, 0)
        }
    };

    if removed.is_some() || pools > 0 || accounts > 0 {
        logger::info(
            LogTag::Tokens,
            &format!(
                "[BLACKLIST] Removed {mint} from the blacklist ({pools} pool, {accounts} account rows cleared)"
            ),
        );
        record_token_event(
            mint,
            "token_blacklist_cleared",
            Severity::Info,
            serde_json::json!({
                "reason": removed.as_ref().map(|entry| entry.reason.as_str()),
                "source": removed.as_ref().map(|entry| entry.source.as_str()),
                "pools_cleared": pools,
                "accounts_cleared": accounts,
            }),
        )
        .await;
    }

    if let Err(e) = crate::filtering::refresh(ChainScope::One(chain)).await {
        logger::warning(
            LogTag::Tokens,
            &format!("[BLACKLIST] Filtering refresh after removing {mint} failed: {e}"),
        );
    }
    Ok(removed)
}

/// Blacklist summary for dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlacklistSummary {
    pub total_count: usize,
    pub authority_mint_count: usize,
    pub authority_freeze_count: usize,
    pub manual_count: usize,
    pub non_authority_auto_count: usize,
    pub non_authority_breakdown: HashMap<String, usize>,
}

/// Get blacklist summary
pub fn get_blacklist_summary(db: &TokenDatabase) -> TokenResult<BlacklistSummary> {
    let conn = db.conn()?;

    let total_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM blacklist WHERE chain_id = ?1",
            params![db.chain_id()],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let mut authority_mint_count = 0;
    let mut authority_freeze_count = 0;
    let mut manual_count = 0;
    let mut non_authority_auto_count = 0;
    let mut non_authority_breakdown: HashMap<String, usize> = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT reason, source FROM blacklist WHERE chain_id = ?1")
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

    let reasons = stmt
        .query_map(params![db.chain_id()], |row| {
            let reason: String = row.get(0)?;
            let source: String = row.get(1)?;
            Ok((reason, source))
        })
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })
        })?;

    for reason_result in reasons {
        if let Ok((reason, source)) = reason_result {
            if source.eq_ignore_ascii_case("manual") {
                manual_count += 1;
                continue;
            }

            let reason_lower = reason.to_ascii_lowercase();

            if reason_lower.starts_with("mint authority") {
                authority_mint_count += 1;
            } else if reason_lower.starts_with("freeze authority") {
                authority_freeze_count += 1;
            } else {
                non_authority_auto_count += 1;
                *non_authority_breakdown.entry(reason).or_default() += 1;
            }
        }
    }

    Ok(BlacklistSummary {
        total_count,
        authority_mint_count,
        authority_freeze_count,
        manual_count,
        non_authority_auto_count,
        non_authority_breakdown,
    })
}
