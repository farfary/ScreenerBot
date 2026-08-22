//! Token update core — main update loop that refreshes token data from multiple sources.

use super::helpers::{clear_in_flight, try_mark_in_flight};
use super::rate_limiter::RateLimitCoordinator;
use crate::chains::ChainId;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::pools;
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::{dexscreener, geckoterminal};
use crate::tokens::priorities::Priority;
use crate::tokens::security::rugcheck;
use crate::tokens::types::{TokenError, TokenResult};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// ============================================================================
// POOL PRIORITY MANAGER
// ============================================================================

pub(super) struct PoolPriorityState {
    pub(super) last_seen: Instant,
    pub(super) previous_priority: i32,
}

pub(super) struct PoolPriorityManager {
    pub(super) state: Mutex<HashMap<(ChainId, String), PoolPriorityState>>,
    pub(super) demote_after: Duration,
}

impl PoolPriorityManager {
    pub(super) fn new(demote_after: Duration) -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            demote_after,
        }
    }

    pub(super) async fn sync(&self, db: &TokenDatabase) {
        let now = Instant::now();
        let chain = db.chain();
        let pool_tokens = pools::get_available_tokens();
        let pool_set: HashSet<String> = pool_tokens.iter().cloned().collect();

        let priorities = match db.get_priorities_for_tokens(&pool_tokens) {
            Ok(map) => map,
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Failed to load priorities for pool tokens: {e}"),
                );
                return;
            }
        };

        let mut promotions: Vec<String> = Vec::new();
        let mut demotion_candidates: Vec<(String, i32)> = Vec::new();

        {
            let mut state = self.state.lock().await;

            for mint in pool_tokens.iter() {
                let current_priority = priorities
                    .get(mint)
                    .copied()
                    .unwrap_or(Priority::Standard.to_value());

                if current_priority == Priority::OpenPosition.to_value() {
                    state.remove(&(chain, mint.clone()));
                    continue;
                }

                let entry =
                    state
                        .entry((chain, mint.clone()))
                        .or_insert_with(|| PoolPriorityState {
                            last_seen: now,
                            previous_priority: current_priority,
                        });

                if current_priority != Priority::PoolTracked.to_value() {
                    entry.previous_priority = current_priority;
                    promotions.push(mint.clone());
                }

                entry.last_seen = now;
            }

            let demote_after = self.demote_after;
            // Collect demotion candidates WITHOUT removing from state yet
            for ((state_chain, mint), info) in state.iter() {
                if *state_chain == chain
                    && !pool_set.contains(mint)
                    && now.duration_since(info.last_seen) >= demote_after
                {
                    demotion_candidates.push((mint.clone(), info.previous_priority));
                }
            }
        }

        if !promotions.is_empty() {
            let mut promoted = Vec::new();
            for mint in promotions {
                if let Err(e) = db.update_priority(&mint, Priority::PoolTracked.to_value()) {
                    logger::error(
                        LogTag::Tokens,
                        &format!("Failed to promote {mint} to PoolTracked priority: {e}"),
                    );
                } else {
                    let previous_priority = priorities
                        .get(&mint)
                        .copied()
                        .unwrap_or(Priority::Standard.to_value());
                    promoted.push((mint, previous_priority));
                }
            }

            if !promoted.is_empty() {
                let count = promoted.len();
                let sample_entries: Vec<String> = promoted
                    .iter()
                    .take(3)
                    .map(|(mint, prev)| format!("{mint} (from={prev})"))
                    .collect();
                let extra = count.saturating_sub(sample_entries.len());
                let mut message = format!("Promoted {count} tokens to pool priority");
                if !sample_entries.is_empty() {
                    message.push_str(&format!("; details: {}", sample_entries.join(", ")));
                }
                if extra > 0 {
                    message.push_str(&format!(" (+{extra} more)"));
                }
                logger::info(LogTag::Tokens, &message);
            }
        }

        if demotion_candidates.is_empty() {
            return;
        }

        let demotion_mints: Vec<String> = demotion_candidates
            .iter()
            .map(|(mint, _)| mint.clone())
            .collect();

        let current_priorities = match db.get_priorities_for_tokens(&demotion_mints) {
            Ok(map) => map,
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Failed to load priorities for demotion candidates: {e}"),
                );
                return;
            }
        };

        let mut demoted = Vec::new();

        for (mint, previous_priority) in demotion_candidates {
            let current_priority = current_priorities
                .get(&mint)
                .copied()
                .unwrap_or(Priority::Standard.to_value());

            if current_priority != Priority::PoolTracked.to_value() {
                continue;
            }

            let mut target_priority = previous_priority;
            if target_priority == Priority::PoolTracked.to_value() {
                target_priority = Priority::Standard.to_value();
            }

            if let Err(e) = db.update_priority(&mint, target_priority) {
                logger::error(
                    LogTag::Tokens,
                    &format!("Failed to demote {mint} from PoolTracked priority: {e}"),
                );
            } else {
                demoted.push((mint.clone(), target_priority));
            }
        }

        // NOW remove successfully demoted tokens from state (after DB writes succeed)
        if !demoted.is_empty() {
            let mut state = self.state.lock().await;
            for (mint, _) in &demoted {
                state.remove(&(chain, mint.clone()));
            }

            let count = demoted.len();
            let sample_entries: Vec<String> = demoted
                .iter()
                .take(3)
                .map(|(mint, target)| format!("{mint} (to={target})"))
                .collect();
            let extra = count.saturating_sub(sample_entries.len());
            let mut message = format!("Demoted {count} tokens from pool priority after timeout");
            if !sample_entries.is_empty() {
                message.push_str(&format!("; details: {}", sample_entries.join(", ")));
            }
            if extra > 0 {
                message.push_str(&format!(" (+{extra} more)"));
            }
            logger::info(LogTag::Tokens, &message);
        }
    }
}

// ============================================================================
// UPDATE FUNCTIONS
// ============================================================================

/// Update a single token from market data sources (DexScreener only)
///
/// Note: Security data (Rugcheck) is handled separately in update_security_data()
/// and is fetched only once per token, not on every update cycle.
///
/// GeckoTerminal is no longer used for market data updates due to strict rate limits (30/min).
/// It is still used in discovery for finding new tokens.
///
/// Returns overall success if at least one source succeeds.
pub async fn update_token(
    mint: &str,
    db: &TokenDatabase,
    coordinator: &RateLimitCoordinator,
) -> TokenResult<UpdateResult> {
    // Skip the network update while the internet is confirmed offline — the
    // DexScreener fetch would only time out on DNS and spam the log. Reported as
    // a no-op result; the loops resume automatically on reconnect. Never
    // triggers at startup (Unknown state).
    if crate::connectivity::is_network_offline() {
        return Ok(UpdateResult {
            mint: mint.to_string(),
            successes: Vec::new(),
            failures: Vec::new(),
        });
    }

    let mut successes = Vec::new();
    let mut failures = Vec::new();

    // Update DexScreener market data only
    match coordinator.acquire_dexscreener_batch().await {
        Ok(permit) => match dexscreener::fetch_dexscreener_data(mint, db).await {
            Ok(Some(_)) => {
                permit.forget();
                successes.push("DexScreener".to_owned());
            }
            Ok(None) => failures.push("DexScreener: Token not listed".to_owned()),
            Err(e) => failures.push(format!("DexScreener: {e}")),
        },
        Err(e) => failures.push(format!("DexScreener rate limit: {e}")),
    }

    // Update tracking timestamp for market data
    let market_data_updated = !successes.is_empty();

    if market_data_updated {
        let _ = db.mark_market_data_updated(mint);

        // Record market data update event (sampled - every 50th token to avoid spam)
        let hash = mint.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
        if hash % 50 == 0 {
            tokio::spawn({
                let mint = mint.to_string();
                let successes = successes.clone();
                let failures = failures.clone();
                async move {
                    record_token_event(
                        &mint,
                        "market_data_updated",
                        Severity::Debug,
                        serde_json::json!({
                            "sources": successes,
                            "failures": failures,
                            "partial_failure": !failures.is_empty(),
                        }),
                    )
                    .await;
                }
            });
        }
    } else if !failures.is_empty() {
        // Record total failure (no successful updates)
        tokio::spawn({
            let mint = mint.to_string();
            let failures = failures.clone();
            async move {
                record_token_event(
                    &mint,
                    "market_data_update_failed",
                    Severity::Warn,
                    serde_json::json!({
                        "failures": failures,
                    }),
                )
                .await;
            }
        });
    }

    Ok(UpdateResult {
        mint: mint.to_string(),
        successes,
        failures,
    })
}

/// Result of updating a single token
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub mint: String,
    pub successes: Vec<String>,
    pub failures: Vec<String>,
}

impl UpdateResult {
    /// Whether all sources updated successfully
    pub fn is_success(&self) -> bool {
        !self.successes.is_empty()
    }

    /// Whether some but not all sources failed
    pub fn is_partial_failure(&self) -> bool {
        !self.successes.is_empty() && !self.failures.is_empty()
    }

    /// Whether all sources failed
    pub fn is_total_failure(&self) -> bool {
        self.successes.is_empty() && !self.failures.is_empty()
    }
}

/// Update multiple tokens in batch (DexScreener + GeckoTerminal batch endpoints)
///
/// Uses batch API for both DexScreener and GeckoTerminal (up to 30 tokens per request),
/// individual calls only for Rugcheck (no batch endpoint available).
///
/// # Arguments
/// * `mints` - Token addresses to update (up to 30 recommended)
/// * `db` - Database instance
/// * `coordinator` - Rate limit coordinator
///
/// # Returns
/// Vec<UpdateResult> - One result per token
pub async fn update_tokens_batch(
    mints: &[String],
    db: &TokenDatabase,
    coordinator: &RateLimitCoordinator,
) -> TokenResult<Vec<UpdateResult>> {
    if mints.is_empty() {
        return Ok(Vec::new());
    }

    // Skip the batch network update while the internet is confirmed offline (see
    // update_token). Resumes automatically on reconnect; no-op at startup.
    if crate::connectivity::is_network_offline() {
        return Ok(Vec::new());
    }

    // Filter out tokens already being fetched by other loops
    let mints_to_fetch: Vec<String> = mints
        .iter()
        .filter(|mint| try_mark_in_flight(db.chain(), mint))
        .cloned()
        .collect();

    if mints_to_fetch.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    // Acquire rate limit permit for DexScreener batch endpoint (market data)
    let dex_permit = coordinator.acquire_dexscreener_batch().await;

    // Fetch DexScreener data
    let dex_result = match dex_permit {
        Ok(permit) => {
            let result = dexscreener::fetch_dexscreener_data_batch(&mints_to_fetch, db).await;
            // Only forget permit if API call succeeded
            if result.is_ok() {
                permit.forget();
            }
            result
        }
        Err(e) => Err(TokenError::RateLimit {
            source: "DexScreener-Batch".to_owned(),
            message: e.to_string(),
        }),
    };

    // Process DexScreener results
    let (dex_results, dex_global_err): (HashMap<String, Option<()>>, Option<String>) =
        match dex_result {
            Ok(data) => (
                data.into_iter().map(|(k, v)| (k, v.map(|_| ()))).collect(),
                None,
            ),
            Err(e) => {
                let msg = format!("DexScreener batch failed: {e}");
                logger::error(LogTag::Tokens, &msg);
                (HashMap::new(), Some(msg))
            }
        };

    // Process each token with batch results (market data from DexScreener only)
    for mint in &mints_to_fetch {
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        // DexScreener result from batch
        if let Some(Some(_)) = dex_results.get(mint) {
            successes.push("DexScreener".to_owned());
        } else if dex_results.contains_key(mint) {
            failures.push("DexScreener: Token not listed".to_owned());
        } else if let Some(err) = &dex_global_err {
            failures.push(err.clone());
        }

        // If no results at all, mark as failure
        if successes.is_empty() && failures.is_empty() {
            failures.push("No market sources responded".to_owned());
        }

        // Update tracking timestamp
        let market_data_updated = !successes.is_empty();

        if market_data_updated {
            let _ = db.mark_market_data_updated(mint);
        }

        results.push(UpdateResult {
            mint: mint.clone(),
            successes,
            failures,
        });
    }

    // Clear in-flight markers for all tokens
    for mint in &mints_to_fetch {
        clear_in_flight(db.chain(), mint);
    }

    Ok(results)
}

// ============================================================================
// SECURITY DATA UPDATES (ONE-TIME FETCH)
// ============================================================================

/// Update Rugcheck security data for tokens that don't have it yet
///
/// Security data is static/rarely changing - fetch once and cache.
/// Processes ONE token per cycle for better performance with large backlogs.
pub(super) async fn update_security_data(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    // Skip the Rugcheck fetch while the internet is confirmed offline — it would
    // only time out and log errors. Resumes automatically on reconnect.
    if crate::connectivity::is_network_offline() {
        return;
    }

    // Load a whole batch of tokens lacking security data (was one-at-a-time). The
    // shared data server can return up to 30 tokens' reports in a single
    // un-throttled call, so we warm the backlog far faster.
    const SECURITY_BATCH: usize = 30;
    /// Cap on direct Rugcheck API fetches per cycle (for tokens the server didn't
    /// have) so the direct per-IP budget is never bursted.
    const MAX_DIRECT_PER_CYCLE: usize = 3;

    let tokens = match db.get_tokens_without_security_data(SECURITY_BATCH) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to load tokens without security data: {e}"),
            );
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    // 1) Batch-warm from the shared server in ONE call (server-first, no direct
    //    Rugcheck rate-limit permit consumed). Tokens the server had cached are
    //    persisted here and drop out of the backlog immediately.
    let warmed = rugcheck::warm_security_from_server(&tokens, db).await;
    for mint in &warmed {
        let _ = db.clear_security_error(mint);
    }
    if !warmed.is_empty() {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Security data batch-warmed for {} token(s) from data server",
                warmed.len()
            ),
        );
    }

    // 2) Direct Rugcheck fallback for a BOUNDED number of the misses (the server
    //    didn't have them yet — it has scheduled its own background fetch, but we
    //    still fetch directly now so the data is available immediately).
    let misses: Vec<String> = tokens
        .into_iter()
        .filter(|m| !warmed.contains(m))
        .take(MAX_DIRECT_PER_CYCLE)
        .collect();

    for mint in &misses {
        if !try_mark_in_flight(db.chain(), mint) {
            continue; // Another loop is already fetching this token
        }
        match coordinator.acquire_rugcheck().await {
            Ok(permit) => match rugcheck::fetch_rugcheck_data(mint, db).await {
                Ok(Some(_)) => {
                    permit.forget();
                    logger::debug(LogTag::Tokens, &format!("Security data fetched for {mint}"));
                    let _ = db.clear_security_error(mint);
                }
                Ok(None) => {
                    // Token not analyzed by Rugcheck - this is PERMANENT (404/400).
                    let _ = db.record_security_error(
                        mint,
                        "Token not analyzed by Rugcheck (404/400)",
                        "permanent",
                    );
                }
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    let error_type = if err_str.contains("404")
                        || err_str.contains("NotFound")
                        || err_str.contains("not found")
                    {
                        "permanent"
                    } else {
                        "temporary"
                    };
                    logger::error(
                        LogTag::Tokens,
                        &format!("Rugcheck error ({error_type}) for {mint}: {e}"),
                    );
                    let _ = db.record_security_error(mint, &e.to_string(), error_type);
                }
            },
            Err(e) => {
                logger::error(LogTag::Tokens, &format!("Rugcheck rate limit: {e}"));
            }
        }
        clear_in_flight(db.chain(), mint);
    }
}

// ============================================================================
// FORCE UPDATE API (for immediate fetching outside scheduled loops)
// ============================================================================

/// Force immediate update for a single token (bypasses normal scheduling)
///
/// This function is designed for on-demand updates when user explicitly
/// requests fresh data (e.g., viewing token details dialog).
///
/// Fetches from ALL sources in parallel:
/// - DexScreener (market data)
/// - GeckoTerminal (market data)
/// - Rugcheck (security data)
///
/// Uses the same rate limit coordinator as scheduled updates but executes
/// immediately without waiting for next loop iteration.
///
/// # Arguments
/// * `mint` - Token address to update
/// * `db` - Database instance
/// * `coordinator` - Rate limit coordinator
///
/// # Returns
/// UpdateResult with success/failure details from each source
/// Max time an on-demand (user-initiated) force update waits for a per-minute
/// rate-limit permit before proceeding without one. The background loop drains
/// the shared budget across thousands of tokens, so a blocking `acquire().await`
/// here could stall a freshly-opened token-details dialog for up to a minute.
/// Proceeding after a short wait is safe: each API client self-rate-limits
/// internally (1 concurrent request + min interval), so we never burst the API.
const ON_DEMAND_PERMIT_WAIT: Duration = Duration::from_secs(3);

/// Hard ceiling on each individual source fetch during an on-demand force update.
/// `force_update_token` fans the three sources out concurrently with `join!`, so
/// the endpoint blocks on the SLOWEST arm. A source that is rate-limited (e.g.
/// GeckoTerminal returning 429 and entering its internal retry/backoff) can stall
/// for 40s+ — long past the dashboard's 10s client-side abort — which leaves the
/// token-details dialog stuck on a loading spinner even though the other sources
/// already returned. Bounding each fetch turns a slow arm into a plain failure so
/// the fast sources still populate the UI promptly.
const ON_DEMAND_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// Run an on-demand source fetch under [`ON_DEMAND_FETCH_TIMEOUT`], mapping a
/// timeout to an `Api` failure tagged with the source name.
async fn fetch_with_deadline<T, F>(source: &str, fut: F) -> TokenResult<Option<T>>
where
    F: std::future::Future<Output = TokenResult<Option<T>>>,
{
    match tokio::time::timeout(ON_DEMAND_FETCH_TIMEOUT, fut).await {
        Ok(result) => result,
        Err(_) => Err(TokenError::Api {
            source: source.to_owned(),
            message: format!(
                "on-demand fetch timed out after {}s",
                ON_DEMAND_FETCH_TIMEOUT.as_secs()
            ),
        }),
    }
}

pub async fn force_update_token(
    mint: &str,
    db: std::sync::Arc<TokenDatabase>,
    coordinator: std::sync::Arc<RateLimitCoordinator>,
) -> TokenResult<UpdateResult> {
    logger::debug(
        LogTag::Tokens,
        &format!("Force update (full) requested for mint={mint}"),
    );

    let mut successes = Vec::new();
    let mut failures = Vec::new();

    // Clone what we need for the async blocks
    let mint_str = mint.to_string();
    let db_ref = &db;
    let coord_ref = &coordinator;

    // Fetch from ALL sources in parallel using tokio::join!
    let (dex_result, gecko_result, rug_result) = tokio::join!(
        // DexScreener market data
        async {
            let permit = match tokio::time::timeout(
                ON_DEMAND_PERMIT_WAIT,
                coord_ref.acquire_dexscreener_batch(),
            )
            .await
            {
                Ok(Ok(p)) => Some(p),
                _ => None,
            };
            let result = fetch_with_deadline(
                "DexScreener",
                dexscreener::fetch_dexscreener_data(&mint_str, db_ref),
            )
            .await;
            if let Some(p) = permit {
                if result.is_ok() && matches!(result, Ok(Some(_))) {
                    p.forget();
                }
            }
            result
        },
        // GeckoTerminal market data
        async {
            let permit = match tokio::time::timeout(
                ON_DEMAND_PERMIT_WAIT,
                coord_ref.acquire_geckoterminal(),
            )
            .await
            {
                Ok(Ok(p)) => Some(p),
                _ => None,
            };
            let result = fetch_with_deadline(
                "GeckoTerminal",
                geckoterminal::fetch_geckoterminal_data(&mint_str, db_ref),
            )
            .await;
            if let Some(p) = permit {
                if result.is_ok() && matches!(result, Ok(Some(_))) {
                    p.forget();
                }
            }
            result
        },
        // Rugcheck security data
        async {
            let permit =
                match tokio::time::timeout(ON_DEMAND_PERMIT_WAIT, coord_ref.acquire_rugcheck())
                    .await
                {
                    Ok(Ok(p)) => Some(p),
                    _ => None,
                };
            let result =
                fetch_with_deadline("Rugcheck", rugcheck::fetch_rugcheck_data(&mint_str, db_ref))
                    .await;
            if let Some(p) = permit {
                if result.is_ok() && matches!(result, Ok(Some(_))) {
                    p.forget();
                }
            }
            result
        }
    );

    // Process DexScreener result
    match dex_result {
        Ok(Some(_)) => successes.push("DexScreener".to_owned()),
        Ok(None) => failures.push("DexScreener: Token not listed".to_owned()),
        Err(e) => failures.push(format!("DexScreener: {e}")),
    }

    // Process GeckoTerminal result
    match gecko_result {
        Ok(Some(_)) => successes.push("GeckoTerminal".to_owned()),
        Ok(None) => failures.push("GeckoTerminal: Token not listed".to_owned()),
        Err(e) => failures.push(format!("GeckoTerminal: {e}")),
    }

    // Process Rugcheck result
    match rug_result {
        Ok(Some(_)) => successes.push("Rugcheck".to_owned()),
        Ok(None) => failures.push("Rugcheck: No security data available".to_owned()),
        Err(e) => failures.push(format!("Rugcheck: {e}")),
    }

    // Update tracking timestamp if any market data source succeeded
    let market_data_updated = successes
        .iter()
        .any(|s| s == "DexScreener" || s == "GeckoTerminal");
    if market_data_updated {
        let _ = db.mark_market_data_updated(mint);
    }

    // Log result summary
    if successes.is_empty() {
        logger::warning(
            LogTag::Tokens,
            &format!(
                "Force update failed for mint={}: all sources failed - {:?}",
                mint, failures
            ),
        );
    } else if !failures.is_empty() {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Force update partial success for mint={}: {} succeeded ({:?}), {} failed ({:?})",
                mint,
                successes.len(),
                successes,
                failures.len(),
                failures
            ),
        );
    } else {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Force update complete for mint={}: all sources succeeded ({:?})",
                mint, successes
            ),
        );
    }

    // Record event for force update (not sampled - user-initiated action)
    tokio::spawn({
        let mint = mint.to_string();
        let successes = successes.clone();
        let failures = failures.clone();
        async move {
            record_token_event(
                &mint,
                "force_update_complete",
                if successes.is_empty() {
                    Severity::Warn
                } else {
                    Severity::Info
                },
                serde_json::json!({
                    "sources_succeeded": successes,
                    "sources_failed": failures,
                    "is_partial": !successes.is_empty() && !failures.is_empty(),
                }),
            )
            .await;
        }
    });

    Ok(UpdateResult {
        mint: mint.to_string(),
        successes,
        failures,
    })
}
