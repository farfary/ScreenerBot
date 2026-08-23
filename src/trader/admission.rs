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
    OpenCooldown {
        wait_secs: u64,
    },
    EntryReserved,
    Blacklisted,
    /// A check itself failed (DB/state error), not a rejection.
    CheckFailed(String),
}

/// Remaining global entry cooldown, kept separate from the strategy admission path so
/// existing strategy scheduling behavior does not change. Copy uses it to persist an
/// honest typed skip before reaching the string error in `open_position_impl`.
pub fn open_cooldown_wait_secs(
    last_open: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    cooldown_secs: u64,
) -> Option<u64> {
    let last_open = last_open?;
    let elapsed = now.signed_duration_since(last_open).num_seconds().max(0) as u64;
    (elapsed < cooldown_secs).then_some(cooldown_secs - elapsed)
}

pub async fn check_open_cooldown() -> Result<(), EntryBlock> {
    let cooldown_secs = crate::config::with_config(|config| {
        config.positions.position_open_cooldown_secs.max(0) as u64
    });
    let last_open = *crate::positions::state::LAST_OPEN_TIME.read().await;
    match open_cooldown_wait_secs(last_open, chrono::Utc::now(), cooldown_secs) {
        Some(wait_secs) => Err(EntryBlock::OpenCooldown { wait_secs }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::open_cooldown_wait_secs;

    #[test]
    fn open_cooldown_reports_the_boundary_without_rounding_it_away() {
        let now = Utc.timestamp_opt(100, 0).unwrap();
        assert_eq!(open_cooldown_wait_secs(None, now, 5), None);
        assert_eq!(open_cooldown_wait_secs(Some(now), now, 5), Some(5));
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(4)), now, 5),
            Some(1)
        );
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(5)), now, 5),
            None
        );
    }
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

    // 1. Connectivity check - no required endpoint may be CONFIRMED down.
    //
    // Confirmed-down, not "not known to be up": an endpoint whose monitor is disabled in
    // config is never probed and stays `Unknown` forever, so the stricter check turned
    // `connectivity.endpoints.rugcheck.enabled = false` into a permanent, silent halt on
    // all automated buying. The executors still refuse to send a swap without a healthy
    // RPC, which is where fail-closed belongs.
    if let Some(unhealthy) = crate::connectivity::check_endpoints_usable(required_endpoints).await {
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
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
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
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
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
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
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
