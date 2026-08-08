//! Source-independent entry admission.
//!
//! Everything that decides whether *any* new entry may happen right now, regardless of
//! what produced the signal. Extracted from `evaluators::entry` so a second entry source
//! runs the identical gauntlet instead of a copy of it.

use crate::trader::safety;

/// Why an entry may not proceed. Every variant is a value a caller can record, count and
/// render — the checks used to be log lines only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum EntryBlock {
    ForceStopped,
    LossLimit,
    Connectivity(String),
    PositionLimit,
    AlreadyOpen,
    ReentryCooldown,
    Blacklisted,
    /// A check itself failed (DB/state error), not a rejection.
    CheckFailed(String),
}

/// Run the source-independent admission gauntlet for a potential entry.
///
/// Order: force stop -> loss limit -> connectivity -> position limits -> existing
/// position -> re-entry cooldown -> blacklist.
///
/// `required_endpoints` differs by source on purpose: strategy entries depend on filter
/// freshness and require rpc + dexscreener + rugcheck; a copy entry requires rpc only,
/// matching `execute_buy_managed`.
pub async fn check_entry_admission(
    mint: &str,
    required_endpoints: &[&str],
) -> Result<(), EntryBlock> {
    // Early exit: Force stop is active
    if crate::global::is_force_stopped() {
        crate::logger::debug(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by force_stop",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::ForceStopped);
    }

    // Early exit: Loss limit reached
    if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by loss_limit",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::LossLimit);
    }

    // 1. Connectivity check - critical endpoints must be healthy
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(required_endpoints).await
    {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by unhealthy: {}",
                &mint[..8.min(mint.len())],
                unhealthy
            ),
        );
        return Err(EntryBlock::Connectivity(unhealthy));
    }

    // 2. Position limits - check if we can open more positions
    match safety::check_position_limits().await {
        Ok(true) => {}
        Ok(false) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by position_limits",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::PositionLimit);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e)),
    }

    // 3. Existing position check - prevent duplicate entries
    match safety::has_open_position(mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by existing_position",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::AlreadyOpen);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e)),
    }

    // 4. Re-entry cooldown - prevent immediate re-entry after exit
    match safety::is_in_reentry_cooldown(mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by reentry_cooldown",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::ReentryCooldown);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e)),
    }

    // 5. Blacklist check - token-level only (not pool-level)
    if safety::is_blacklisted(mint).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by blacklist",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::Blacklisted);
    }

    Ok(())
}
