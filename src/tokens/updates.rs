use crate::apis::dexscreener::{
    RATE_LIMIT_LATEST_BOOSTS_PER_MINUTE as DEX_BOOSTS_PER_MINUTE,
    RATE_LIMIT_LATEST_PROFILES_PER_MINUTE as DEX_PROFILES_PER_MINUTE,
    RATE_LIMIT_TOKEN_BATCH_PER_MINUTE as DEX_BATCH_PER_MINUTE,
    RATE_LIMIT_TOKEN_POOLS_PER_MINUTE as DEX_POOLS_PER_MINUTE,
};
use crate::apis::geckoterminal::RATE_LIMIT_PER_MINUTE as GECKO_DEFAULT_PER_MINUTE;
use crate::apis::rugcheck::RATE_LIMIT_PER_MINUTE as RUG_DEFAULT_PER_MINUTE;
use crate::config::with_config;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::pools;
/// Updates orchestrator - State-based priority updates
///
/// Coordinates fetching from all sources (DexScreener, GeckoTerminal, Rugcheck)
/// with rate limiting and state-based priority scheduling.
///
/// Priority levels (named by token state):
/// - OpenPosition (100): Tokens with active trading positions → Update every 5s
/// - PoolTracked (75): Tokens tracked by Pool Service → Update every 7s
/// - FilterPassed (60): Tokens that passed filtering criteria → Update every 8s
/// - Uninitialized (55): New tokens without market data yet → Update every 10s (immediate seeding)
/// - Stale (40): Tokens with outdated market data → Update every 15s
/// - Standard (25): Regular tokens with fresh data → Update every 20s
/// - Background (10): Oldest tokens being refreshed in background → Update every 30s
///
/// Security data (Rugcheck) is fetched in a separate loop, one token per interval (configurable, default 60s),
/// and only for tokens that don't have security data yet.
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::{dexscreener, geckoterminal};
use crate::tokens::priorities::Priority;
use crate::tokens::security::rugcheck;
use crate::tokens::types::{TokenError, TokenResult};
use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Check if updates should be paused due to active tools
fn should_skip_for_tools() -> bool {
    crate::global::are_tools_active()
}

/// Filter out the token currently being viewed in the dashboard
/// Dashboard-active tokens get priority updates via the UI, so skip them in batch updates
fn filter_dashboard_active_token(tokens: Vec<String>) -> Vec<String> {
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

// ============================================================================
// RATE LIMIT COORDINATOR
// ============================================================================

/// Global rate limit coordinator for all API sources
///
/// Uses separate semaphores per endpoint to prevent different operations from blocking each other:
/// - DexScreener token batch (market data): 300/min
/// - DexScreener profiles (discovery): 60/min
/// - DexScreener boosts (discovery): 60/min
/// - DexScreener token pools (full pool fetch): 300/min
/// - GeckoTerminal: 30/min
/// - Rugcheck: 60/min
pub struct RateLimitCoordinator {
    // DexScreener endpoints (separate limits per endpoint)
    dexscreener_batch_sem: Arc<Semaphore>,
    dexscreener_profiles_sem: Arc<Semaphore>,
    dexscreener_boosts_sem: Arc<Semaphore>,
    dexscreener_pools_sem: Arc<Semaphore>,
    dexscreener_batch_budget: usize,
    dexscreener_profiles_budget: usize,
    dexscreener_boosts_budget: usize,
    dexscreener_pools_budget: usize,
    // Other API endpoints
    geckoterminal_sem: Arc<Semaphore>,
    rugcheck_sem: Arc<Semaphore>,
    geckoterminal_budget: usize,
    rugcheck_budget: usize,
}

impl RateLimitCoordinator {
    pub fn new() -> Self {
        // Read limits from config; fall back to API defaults if unset (0)
        let (gecko_limit, rug_limit) = with_config(|cfg| {
            let s = &cfg.tokens.sources;
            let gecko = if s.geckoterminal.rate_limit_per_minute == 0 {
                GECKO_DEFAULT_PER_MINUTE
            } else {
                s.geckoterminal.rate_limit_per_minute as usize
            };
            let rug = if s.rugcheck.rate_limit_per_minute == 0 {
                RUG_DEFAULT_PER_MINUTE
            } else {
                s.rugcheck.rate_limit_per_minute as usize
            };
            (gecko, rug)
        });

        Self {
            // DexScreener endpoints with separate limits
            dexscreener_batch_sem: Arc::new(Semaphore::new(DEX_BATCH_PER_MINUTE)),
            dexscreener_profiles_sem: Arc::new(Semaphore::new(DEX_PROFILES_PER_MINUTE)),
            dexscreener_boosts_sem: Arc::new(Semaphore::new(DEX_BOOSTS_PER_MINUTE)),
            dexscreener_pools_sem: Arc::new(Semaphore::new(DEX_POOLS_PER_MINUTE)),
            dexscreener_batch_budget: DEX_BATCH_PER_MINUTE,
            dexscreener_profiles_budget: DEX_PROFILES_PER_MINUTE,
            dexscreener_boosts_budget: DEX_BOOSTS_PER_MINUTE,
            dexscreener_pools_budget: DEX_POOLS_PER_MINUTE,
            // Other API endpoints
            geckoterminal_sem: Arc::new(Semaphore::new(gecko_limit)),
            rugcheck_sem: Arc::new(Semaphore::new(rug_limit)),
            geckoterminal_budget: gecko_limit,
            rugcheck_budget: rug_limit,
        }
    }

    /// Acquire permit for DexScreener token batch API call (market data updates)
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_batch(&self) -> Result<(), TokenError> {
        self.dexscreener_batch_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| {
                // Do not release permits early; refill task restores capacity each minute
                permit.forget();
            })
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Batch".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Acquire permit for DexScreener profiles API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_profiles(&self) -> Result<(), TokenError> {
        self.dexscreener_profiles_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| {
                permit.forget();
            })
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Profiles".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Acquire permit for DexScreener boosts API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_boosts(&self) -> Result<(), TokenError> {
        self.dexscreener_boosts_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| {
                permit.forget();
            })
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Boosts".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Acquire permit for DexScreener full pool fetch API call
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_pools(&self) -> Result<(), TokenError> {
        self.dexscreener_pools_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| {
                permit.forget();
            })
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Pools".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Acquire permit for GeckoTerminal API call
    pub async fn acquire_geckoterminal(&self) -> Result<(), TokenError> {
        self.geckoterminal_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| permit.forget())
            .map_err(|e| TokenError::RateLimit {
                source: "GeckoTerminal".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Acquire permit for Rugcheck API call
    pub async fn acquire_rugcheck(&self) -> Result<(), TokenError> {
        self.rugcheck_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| permit.forget())
            .map_err(|e| TokenError::RateLimit {
                source: "Rugcheck".to_string(),
                message: format!("Failed to acquire permit: {}", e),
            })
    }

    /// Refill all semaphores (called every minute)
    pub fn refill_all(&self) {
        // DexScreener endpoints
        if self.dexscreener_batch_budget > 0 {
            self.dexscreener_batch_sem
                .add_permits(self.dexscreener_batch_budget);
        }
        if self.dexscreener_profiles_budget > 0 {
            self.dexscreener_profiles_sem
                .add_permits(self.dexscreener_profiles_budget);
        }
        if self.dexscreener_boosts_budget > 0 {
            self.dexscreener_boosts_sem
                .add_permits(self.dexscreener_boosts_budget);
        }
        if self.dexscreener_pools_budget > 0 {
            self.dexscreener_pools_sem
                .add_permits(self.dexscreener_pools_budget);
        }
        // Other API endpoints
        if self.geckoterminal_budget > 0 {
            self.geckoterminal_sem
                .add_permits(self.geckoterminal_budget);
        }
        if self.rugcheck_budget > 0 {
            self.rugcheck_sem.add_permits(self.rugcheck_budget);
        }
    }
}

impl Default for RateLimitCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// POOL PRIORITY COORDINATOR
// ============================================================================

struct PoolPriorityState {
    last_seen: Instant,
    previous_priority: i32,
}

struct PoolPriorityManager {
    state: Mutex<HashMap<String, PoolPriorityState>>,
    demote_after: Duration,
}

impl PoolPriorityManager {
    fn new(demote_after: Duration) -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            demote_after,
        }
    }

    async fn sync(&self, db: &TokenDatabase) {
        let now = Instant::now();
        let pool_tokens = pools::get_available_tokens();
        let pool_set: HashSet<String> = pool_tokens.iter().cloned().collect();

        let priorities = match db.get_priorities_for_tokens(&pool_tokens) {
            Ok(map) => map,
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Failed to load priorities for pool tokens: {}", e),
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
                    state.remove(mint);
                    continue;
                }

                let entry = state
                    .entry(mint.clone())
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
            state.retain(|mint, info| {
                if pool_set.contains(mint) {
                    true
                } else if now.duration_since(info.last_seen) >= demote_after {
                    demotion_candidates.push((mint.clone(), info.previous_priority));
                    false
                } else {
                    true
                }
            });
        }

        if !promotions.is_empty() {
            let mut promoted = Vec::new();
            for mint in promotions {
                if let Err(e) = db.update_priority(&mint, Priority::PoolTracked.to_value()) {
                    logger::error(
                        LogTag::Tokens,
                        &format!("Failed to promote {} to PoolTracked priority: {}", mint, e),
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
                    .map(|(mint, prev)| format!("{} (from={})", mint, prev))
                    .collect();
                let extra = count.saturating_sub(sample_entries.len());
                let mut message = format!("Promoted {} tokens to pool priority", count);
                if !sample_entries.is_empty() {
                    message.push_str(&format!("; details: {}", sample_entries.join(", ")));
                }
                if extra > 0 {
                    message.push_str(&format!(" (+{} more)", extra));
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
                    &format!("Failed to load priorities for demotion candidates: {}", e),
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
                    &format!("Failed to demote {} from PoolTracked priority: {}", mint, e),
                );
            } else {
                demoted.push((mint, target_priority));
            }
        }

        if !demoted.is_empty() {
            let count = demoted.len();
            let sample_entries: Vec<String> = demoted
                .iter()
                .take(3)
                .map(|(mint, target)| format!("{} (to={})", mint, target))
                .collect();
            let extra = count.saturating_sub(sample_entries.len());
            let mut message = format!("Demoted {} tokens from pool priority after timeout", count);
            if !sample_entries.is_empty() {
                message.push_str(&format!("; details: {}", sample_entries.join(", ")));
            }
            if extra > 0 {
                message.push_str(&format!(" (+{} more)", extra));
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
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    // Update DexScreener market data only
    match coordinator.acquire_dexscreener_batch().await {
        Ok(_) => match dexscreener::fetch_dexscreener_data(mint, db).await {
            Ok(Some(_)) => successes.push("DexScreener".to_string()),
            Ok(None) => failures.push(format!("DexScreener: Token not listed")),
            Err(e) => failures.push(format!("DexScreener: {}", e)),
        },
        Err(e) => failures.push(format!("DexScreener rate limit: {}", e)),
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
    pub fn is_success(&self) -> bool {
        !self.successes.is_empty()
    }

    pub fn is_partial_failure(&self) -> bool {
        !self.successes.is_empty() && !self.failures.is_empty()
    }

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

    let mut results = Vec::new();

    // Acquire rate limit permit for DexScreener batch endpoint (market data)
    let dex_permit = coordinator.acquire_dexscreener_batch().await;

    // Fetch DexScreener data
    let dex_result = match dex_permit {
        Ok(_) => dexscreener::fetch_dexscreener_data_batch(mints, db).await,
        Err(e) => Err(TokenError::RateLimit {
            source: "DexScreener-Batch".to_string(),
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
                let msg = format!("DexScreener batch failed: {}", e);
                logger::error(LogTag::Tokens, &msg);
                (HashMap::new(), Some(msg))
            }
        };

    // Process each token with batch results (market data from DexScreener only)
    for mint in mints {
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        // DexScreener result from batch
        if let Some(Some(_)) = dex_results.get(mint) {
            successes.push("DexScreener".to_string());
        } else if dex_results.contains_key(mint) {
            failures.push("DexScreener: Token not listed".to_string());
        } else if let Some(err) = &dex_global_err {
            failures.push(err.clone());
        }

        // If no results at all, mark as failure
        if successes.is_empty() && failures.is_empty() {
            failures.push("No market sources responded".to_string());
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

    Ok(results)
}

// ============================================================================
// SECURITY DATA UPDATES (ONE-TIME FETCH)
// ============================================================================

/// Update Rugcheck security data for tokens that don't have it yet
///
/// Security data is static/rarely changing - fetch once and cache.
/// Processes ONE token per cycle for better performance with large backlogs.
async fn update_security_data(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    let tokens = match db.get_tokens_without_security_data(1) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to load tokens without security data: {}", e),
            );
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    let mint = &tokens[0];

    // Fetch security data for single token
    match coordinator.acquire_rugcheck().await {
        Ok(_) => match rugcheck::fetch_rugcheck_data(mint, db).await {
            Ok(Some(_)) => {
                logger::debug(
                    LogTag::Tokens,
                    &format!("Security data fetched for {}", mint),
                );
                // Clear any previous error tracking
                let _ = db.clear_security_error(mint);
            }
            Ok(None) => {
                // Token not analyzed by Rugcheck - this is PERMANENT (404/400 not found)
                let _ = db.record_security_error(
                    mint,
                    "Token not analyzed by Rugcheck (404/400)",
                    "permanent",
                );
            }
            Err(e) => {
                // Classify error type
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
                    &format!("Rugcheck error ({}) for {}: {}", error_type, mint, e),
                );
                let _ = db.record_security_error(mint, &e.to_string(), error_type);
            }
        },
        Err(e) => {
            logger::error(LogTag::Tokens, &format!("Rugcheck rate limit: {}", e));
        }
    }
}

// ============================================================================
// PRIORITY-BASED UPDATE LOOPS
// ============================================================================

/// Start the main update loop with all priority levels
pub fn start_update_loop(
    db: Arc<TokenDatabase>,
    shutdown: Arc<Notify>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::new();

    // Security data loop (one-time fetch for tokens without Rugcheck data)
    let db_security = db.clone();
    let coord_security = coordinator.clone();
    let shutdown_security = shutdown.clone();
    handles.push(tokio::spawn(async move {
        update_security_data(&db_security, &coord_security).await;
        loop {
            tokio::select! {
                _ = shutdown_security.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.security_seconds))) => {
                    update_security_data(&db_security, &coord_security).await;
                }
            }
        }
    }));

    // Immediate seeding loop for tokens that have no market data yet
    let db_seed = db.clone();
    let coord_seed = coordinator.clone();
    let shutdown_seed = shutdown.clone();
    handles.push(tokio::spawn(async move {
        update_uninitialized_tokens(&db_seed, &coord_seed).await;
        loop {
            tokio::select! {
                _ = shutdown_seed.notified() => break,
                _ = sleep(Duration::from_secs(10)) => {
                    update_uninitialized_tokens(&db_seed, &coord_seed).await;
                }
            }
        }
    }));

    // Pool priority sync loop (every 5s)
    let pool_priority_manager = Arc::new(PoolPriorityManager::new(Duration::from_secs(60)));
    let manager_sync = pool_priority_manager.clone();
    let db_pool_state = db.clone();
    let shutdown_pool_sync = shutdown.clone();
    handles.push(tokio::spawn(async move {
        manager_sync.sync(db_pool_state.as_ref()).await;
        loop {
            tokio::select! {
                _ = shutdown_pool_sync.notified() => break,
                _ = sleep(Duration::from_secs(5)) => {
                    manager_sync.sync(db_pool_state.as_ref()).await;
                }
            }
        }
    }));

    // Pool-tracked tokens loop (configurable)
    let db_pool_update = db.clone();
    let coord_pool = coordinator.clone();
    let shutdown_pool_update = shutdown.clone();
    handles.push(tokio::spawn(async move {
        update_pool_tracked_tokens(&db_pool_update, &coord_pool).await;
        loop {
            tokio::select! {
                _ = shutdown_pool_update.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.pool_tracked_seconds))) => {
                    update_pool_tracked_tokens(&db_pool_update, &coord_pool).await;
                }
            }
        }
    }));

    // Open position tokens loop (configurable)
    let db_open_pos = db.clone();
    let coord_open_pos = coordinator.clone();
    let shutdown_open_pos = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_open_pos.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.open_position_seconds))) => {
                    update_open_position_tokens(&db_open_pos, &coord_open_pos).await;
                }
            }
        }
    }));

    // Filter-passed tokens loop (configurable)
    let db_filter_passed = db.clone();
    let coord_filter_passed = coordinator.clone();
    let shutdown_filter_passed = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_filter_passed.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.filter_passed_seconds))) => {
                    update_filter_passed_tokens(&db_filter_passed, &coord_filter_passed).await;
                }
            }
        }
    }));

    // Background tokens loop (configurable)
    let db_background = db.clone();
    let coord_background = coordinator.clone();
    let shutdown_background = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_background.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.background_seconds))) => {
                    update_background_tokens(&db_background, &coord_background).await;
                }
            }
        }
    }));

    handles
}

/// Seed market data for tokens that have never been updated
async fn update_uninitialized_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (uninitialized) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    const MAX_INITIAL_BATCH: usize = 30;

    let tokens = match db.get_tokens_without_market_data(MAX_INITIAL_BATCH) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to load uninitialized tokens: {}", e),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!(
            "Seeding market data for {} newly discovered tokens",
            tokens.len()
        ),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = tokens
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            logger::error(
                                LogTag::Tokens,
                                &format!(
                                    "Failed to record seed error for {}: {}",
                                    result.mint, err
                                ),
                            );
                        }
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure while seeding {}: {:?}",
                                result.mint, result.failures
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error during seeding: {}", e),
                );
            }
        }
    }
}

/// Update open position tokens (tokens with active trading positions)
async fn update_open_position_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (open positions) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::OpenPosition.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get open position tokens: {}", e),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} open position tokens", tokens.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = tokens
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        logger::error(
                            LogTag::Tokens,
                            &format!("Total failure for {}: {:?}", result.mint, result.failures),
                        );
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            logger::error(
                                LogTag::Tokens,
                                &format!("Failed to record error for {}: {}", result.mint, err),
                            );
                        }
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for open position tokens: {}", e),
                );
            }
        }
    }
}

/// Update pool-tracked tokens (Pool Service tracked tokens)
async fn update_pool_tracked_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (pool tracked) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::PoolTracked.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get pool-tracked tokens: {}", e),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    let batch = &tokens[..tokens.len().min(90)];
    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} pool-tracked tokens", batch.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = batch
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            logger::error(
                                LogTag::Tokens,
                                &format!("Failed to record error for {}: {}", result.mint, err),
                            );
                        }
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    } else if result.is_success() {
                        // Success: Demote from PoolTracked (75) to Stale (40) priority
                        // After fresh update, token returns to normal priority rotation
                        // Using Stale (40) instead of non-existent "High" (50)
                        if let Err(e) = db.update_priority(&result.mint, Priority::Stale.to_value())
                        {
                            logger::warning(
                                LogTag::Tokens,
                                &format!(
                                    "Failed to demote {} from PoolTracked to Stale priority: {}",
                                    result.mint, e
                                ),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for pool priority tokens: {}", e),
                );
            }
        }
    }
}

/// Update filter-passed tokens (tokens that passed filtering criteria)
async fn update_filter_passed_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (filter passed) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::FilterPassed.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get filter-passed tokens: {}", e),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    // Limit to 60 tokens total, process in batches of 30
    let batch = &tokens[..tokens.len().min(60)];
    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} filter-passed tokens", batch.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = batch
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            logger::error(
                                LogTag::Tokens,
                                &format!("Failed to record error for {}: {}", result.mint, err),
                            );
                        }
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for passed priority tokens: {}", e),
                );
            }
        }
    }
}

/// Update background tokens (oldest non-blacklisted tokens)
async fn update_background_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (background) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    // Get oldest 30 non-blacklisted tokens (batch size)
    let tokens = match db.get_oldest_non_blacklisted(30) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get background tokens: {}", e),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} background tokens", tokens.len()),
    );

    // Process all in one batch (already limited to 30)
    match update_tokens_batch(&tokens, db, coordinator).await {
        Ok(results) => {
            for result in results {
                if result.is_total_failure() {
                    let message = result.failures.join(" | ");
                    if let Err(err) = db.record_market_error(result.mint.as_str(), message.as_str())
                    {
                        logger::error(
                            LogTag::Tokens,
                            &format!("Failed to record error for {}: {}", result.mint, err),
                        );
                    }
                } else if result.is_partial_failure() {
                    logger::warning(
                        LogTag::Tokens,
                        &format!(
                            "Partial failure for {}: {} succeeded, {} failed",
                            result.mint,
                            result.successes.len(),
                            result.failures.len()
                        ),
                    );
                }
            }
        }
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Batch error for low priority tokens: {}", e),
            );
        }
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
pub async fn force_update_token(
    mint: &str,
    db: Arc<TokenDatabase>,
    coordinator: Arc<RateLimitCoordinator>,
) -> TokenResult<UpdateResult> {
    logger::debug(
        LogTag::Tokens,
        &format!("Force update (full) requested for mint={}", mint),
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
            match coord_ref.acquire_dexscreener_batch().await {
                Ok(_) => dexscreener::fetch_dexscreener_data(&mint_str, db_ref).await,
                Err(e) => Err(e),
            }
        },
        // GeckoTerminal market data
        async {
            match coord_ref.acquire_geckoterminal().await {
                Ok(_) => geckoterminal::fetch_geckoterminal_data(&mint_str, db_ref).await,
                Err(e) => Err(e),
            }
        },
        // Rugcheck security data
        async {
            match coord_ref.acquire_rugcheck().await {
                Ok(_) => rugcheck::fetch_rugcheck_data(&mint_str, db_ref).await,
                Err(e) => Err(e),
            }
        }
    );

    // Process DexScreener result
    match dex_result {
        Ok(Some(_)) => successes.push("DexScreener".to_string()),
        Ok(None) => failures.push("DexScreener: Token not listed".to_string()),
        Err(e) => failures.push(format!("DexScreener: {}", e)),
    }

    // Process GeckoTerminal result
    match gecko_result {
        Ok(Some(_)) => successes.push("GeckoTerminal".to_string()),
        Ok(None) => failures.push("GeckoTerminal: Token not listed".to_string()),
        Err(e) => failures.push(format!("GeckoTerminal: {}", e)),
    }

    // Process Rugcheck result
    match rug_result {
        Ok(Some(_)) => successes.push("Rugcheck".to_string()),
        Ok(None) => failures.push("Rugcheck: No security data available".to_string()),
        Err(e) => failures.push(format!("Rugcheck: {}", e)),
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
