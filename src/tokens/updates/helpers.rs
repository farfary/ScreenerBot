use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;

/// Number of consecutive failures before marking a token as permanently failed for market data
/// "Token not listed" errors are considered permanent after this many attempts
pub(super) const PERMANENT_FAILURE_THRESHOLD: u32 = 3;

/// In-flight token tracking to prevent duplicate fetches across loops
pub(super) static IN_FLIGHT_TOKENS: LazyLock<StdMutex<HashSet<String>>> =
    LazyLock::new(|| StdMutex::new(HashSet::new()));

/// Try to mark a token as in-flight. Returns true if marked, false if already in-flight.
pub(super) fn try_mark_in_flight(mint: &str) -> bool {
    if let Ok(mut set) = IN_FLIGHT_TOKENS.lock() {
        set.insert(mint.to_string())
    } else {
        true // If lock poisoned, allow fetch
    }
}

/// Clear in-flight marker for a token
pub(super) fn clear_in_flight(mint: &str) {
    if let Ok(mut set) = IN_FLIGHT_TOKENS.lock() {
        set.remove(mint);
    }
}

/// Check if updates should be paused due to active tools
pub(super) fn should_skip_for_tools() -> bool {
    crate::global::are_tools_active()
}

/// Determine the market error type based on failure messages
/// - "Token not listed" → "not_listed" (potentially permanent after threshold)
/// - Other errors → "temporary" (transient, will retry with backoff)
pub(super) fn classify_market_error(failures: &[String]) -> &'static str {
    // Check if ALL failures are "not listed" type
    let all_not_listed = failures
        .iter()
        .all(|f| f.contains("not listed") || f.contains("Token not listed"));

    if all_not_listed && !failures.is_empty() {
        "not_listed"
    } else {
        "temporary"
    }
}

/// Record a market error and potentially mark as permanent failure
/// Returns true if the token was marked as permanently failed
pub(super) fn handle_market_failure(db: &TokenDatabase, mint: &str, failures: &[String]) -> bool {
    let error_type = classify_market_error(failures);
    let message = failures.join(" | ");

    match db.record_market_error(mint, &message, error_type) {
        Ok(error_count) => {
            // Mark as permanent if it's a "not listed" error and we've hit the threshold
            if error_type == "not_listed" && error_count >= PERMANENT_FAILURE_THRESHOLD {
                // Update to permanent status (without incrementing count again)
                if let Err(e) = db.mark_market_permanent(mint) {
                    logger::error(
                        LogTag::Tokens,
                        &format!("Failed to mark {mint} as permanent failure: {e}"),
                    );
                    return false;
                }
                logger::info(
                    LogTag::Tokens,
                    &format!(
                        "Marked {} as permanently failed for market data after {} attempts (not listed on any exchange)",
                        mint, error_count
                    ),
                );
                return true;
            }
        }
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to record market error for {mint}: {e}"),
            );
        }
    }
    false
}

/// Filter out the token currently being viewed in the dashboard
/// Dashboard-active tokens get priority updates via the UI, so skip them in batch updates
pub(super) fn filter_dashboard_active_token(tokens: Vec<String>) -> Vec<String> {
    if let Some(active_mint) = crate::global::get_dashboard_active_token() {
        let original_count = tokens.len();
        let filtered: Vec<String> = tokens.into_iter().filter(|m| m != &active_mint).collect();
        if filtered.len() < original_count {
            logger::debug(
                LogTag::Tokens,
                &format!(
                    "Skipping dashboard-active token {} in batch update (getting priority updates via UI)",
                    active_mint
                ),
            );
        }
        filtered
    } else {
        tokens
    }
}
