use crate::apis::dexscreener::RATE_LIMIT_PER_MINUTE as DEX_DEFAULT_PER_MINUTE;
use crate::apis::geckoterminal::RATE_LIMIT_PER_MINUTE as GECKO_DEFAULT_PER_MINUTE;
use crate::apis::rugcheck::RATE_LIMIT_PER_MINUTE as RUG_DEFAULT_PER_MINUTE;
use crate::config::with_config;
/// Updates orchestrator - Priority-based background updates
///
/// Coordinates fetching from all sources (DexScreener, GeckoTerminal, Rugcheck)
/// with rate limiting and priority-based scheduling.
///
/// Priority levels (actual loop intervals):
/// - Critical (100): Open positions → Update every 5s
/// - High (50): Filtered/watched tokens → Update every 10s  
/// - Low (10): Oldest non-blacklisted → Update every 30s
///
/// Security data (Rugcheck) is fetched in a separate loop, one token per interval (configurable, default 60s),
/// and only for tokens that don't have security data yet.
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::{dexscreener, geckoterminal};
use crate::tokens::security::rugcheck;
use crate::tokens::types::{TokenError, TokenResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::sleep;

// ============================================================================
// RATE LIMIT COORDINATOR
// ============================================================================

/// Global rate limit coordinator for all API sources
///
/// Uses semaphores to enforce API rate limits:
/// - DexScreener: 300/min
/// - GeckoTerminal: 30/min
/// - Rugcheck: 60/min
pub struct RateLimitCoordinator {
    dexscreener_sem: Arc<Semaphore>,
    geckoterminal_sem: Arc<Semaphore>,
    rugcheck_sem: Arc<Semaphore>,
    dexscreener_budget: usize,
    geckoterminal_budget: usize,
    rugcheck_budget: usize,
}

impl RateLimitCoordinator {
    pub fn new() -> Self {
        // Read limits from config; fall back to API defaults if unset (0)
        let (dex_limit, gecko_limit, rug_limit) = with_config(|cfg| {
            let s = &cfg.tokens.sources;
            let dex = if s.dexscreener.rate_limit_per_minute == 0 {
                DEX_DEFAULT_PER_MINUTE
            } else {
                s.dexscreener.rate_limit_per_minute as usize
            };
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
            (dex, gecko, rug)
        });

        Self {
            dexscreener_sem: Arc::new(Semaphore::new(dex_limit)),
            geckoterminal_sem: Arc::new(Semaphore::new(gecko_limit)),
            rugcheck_sem: Arc::new(Semaphore::new(rug_limit)),
            dexscreener_budget: dex_limit,
            geckoterminal_budget: gecko_limit,
            rugcheck_budget: rug_limit,
        }
    }

    /// Acquire permit for DexScreener API call
    pub async fn acquire_dexscreener(&self) -> Result<(), TokenError> {
        self.dexscreener_sem
            .clone()
            .acquire_owned()
            .await
            .map(|permit| {
                // Do not release permits early; refill task restores capacity each minute
                permit.forget();
            })
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener".to_string(),
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
        if self.dexscreener_budget > 0 {
            self.dexscreener_sem.add_permits(self.dexscreener_budget);
        }
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
// UPDATE FUNCTIONS
// ============================================================================

/// Update a single token from market data sources (DexScreener + GeckoTerminal)
///
/// Note: Security data (Rugcheck) is handled separately in update_security_data()
/// and is fetched only once per token, not on every update cycle.
///
/// Returns overall success if at least one source succeeds.
pub async fn update_token(
    mint: &str,
    db: &TokenDatabase,
    coordinator: &RateLimitCoordinator,
) -> TokenResult<UpdateResult> {
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    // 1. Update DexScreener market data
    match coordinator.acquire_dexscreener().await {
        Ok(_) => match dexscreener::fetch_dexscreener_data(mint, db).await {
            Ok(Some(_)) => successes.push("DexScreener".to_string()),
            Ok(None) => failures.push(format!("DexScreener: Token not listed")),
            Err(e) => failures.push(format!("DexScreener: {}", e)),
        },
        Err(e) => failures.push(format!("DexScreener rate limit: {}", e)),
    }

    // 2. Update GeckoTerminal market data
    match coordinator.acquire_geckoterminal().await {
        Ok(_) => match geckoterminal::fetch_geckoterminal_data(mint, db).await {
            Ok(Some(_)) => successes.push("GeckoTerminal".to_string()),
            Ok(None) => failures.push(format!("GeckoTerminal: Token not listed")),
            Err(e) => failures.push(format!("GeckoTerminal: {}", e)),
        },
        Err(e) => failures.push(format!("GeckoTerminal rate limit: {}", e)),
    }

    // Update tracking timestamp for market data
    let market_data_updated = !successes.is_empty();

    if market_data_updated {
        let had_errors = !failures.is_empty();
        let _ = db.mark_updated(mint, had_errors);
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

    // Acquire rate limit permits for both sources in parallel
    let (dex_permit, gecko_permit) = tokio::join!(
        coordinator.acquire_dexscreener(),
        coordinator.acquire_geckoterminal()
    );

    // 1 & 2. Fetch DexScreener and GeckoTerminal in PARALLEL
    let (dex_result, gecko_result) = tokio::join!(
        async {
            match dex_permit {
                Ok(_) => dexscreener::fetch_dexscreener_data_batch(mints, db).await,
                Err(e) => Err(TokenError::RateLimit {
                    source: "DexScreener".to_string(),
                    message: e.to_string(),
                }),
            }
        },
        async {
            match gecko_permit {
                Ok(_) => geckoterminal::fetch_geckoterminal_data_batch(mints, db).await,
                Err(e) => Err(TokenError::RateLimit {
                    source: "GeckoTerminal".to_string(),
                    message: e.to_string(),
                }),
            }
        }
    );

    // Process DexScreener results
    let (dex_results, dex_global_err): (HashMap<String, Option<()>>, Option<String>) = match dex_result {
        Ok(data) => (data.into_iter().map(|(k,v)| (k, v.map(|_| ()))) .collect(), None),
        Err(e) => {
            let msg = format!("DexScreener batch failed: {}", e);
            eprintln!("[UPDATES] {}", msg);
            (HashMap::new(), Some(msg))
        }
    };

    // Process GeckoTerminal results
    let (gecko_results, gecko_global_err): (HashMap<String, Option<()>>, Option<String>) = match gecko_result {
        Ok(data) => (data.into_iter().map(|(k,v)| (k, v.map(|_| ()))) .collect(), None),
        Err(e) => {
            let msg = format!("GeckoTerminal batch failed: {}", e);
            eprintln!("[UPDATES] {}", msg);
            (HashMap::new(), Some(msg))
        }
    };

    // 3. Process each token with batch results (market data only)
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

        // GeckoTerminal result from batch
        if let Some(Some(_)) = gecko_results.get(mint) {
            successes.push("GeckoTerminal".to_string());
        } else if gecko_results.contains_key(mint) {
            failures.push("GeckoTerminal: Token not listed".to_string());
        } else if let Some(err) = &gecko_global_err {
            failures.push(err.clone());
        }

        // If both maps are empty and no global errors, still mark as failure to avoid ambiguity
        if successes.is_empty() && failures.is_empty() {
            failures.push("No market sources responded".to_string());
        }

        // Update tracking timestamp
        let market_data_updated = !successes.is_empty();

        if market_data_updated {
            let had_errors = !failures.is_empty();
            let _ = db.mark_updated(mint, had_errors);
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
            eprintln!("[UPDATES] Failed to load tokens without security data: {}", e);
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
                println!("[UPDATES] Security data fetched for {}", mint);
            }
            Ok(None) => {
                // Token not analyzed by Rugcheck - this is expected for many tokens
            }
            Err(e) => {
                eprintln!("[UPDATES] Rugcheck error for {}: {}", mint, e);
            }
        },
        Err(e) => {
            eprintln!("[UPDATES] Rugcheck rate limit: {}", e);
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

    // Critical priority loop (configurable)
    let db_critical = db.clone();
    let coord_critical = coordinator.clone();
    let shutdown_critical = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_critical.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.critical_seconds))) => {
                    update_critical_tokens(&db_critical, &coord_critical).await;
                }
            }
        }
    }));

    // High priority loop (configurable)
    let db_high = db.clone();
    let coord_high = coordinator.clone();
    let shutdown_high = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_high.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.high_seconds))) => {
                    update_high_priority_tokens(&db_high, &coord_high).await;
                }
            }
        }
    }));

    // Low priority loop (configurable)
    let db_low = db.clone();
    let coord_low = coordinator.clone();
    let shutdown_low = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_low.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.low_seconds))) => {
                    update_low_priority_tokens(&db_low, &coord_low).await;
                }
            }
        }
    }));

    handles
}

/// Seed market data for tokens that have never been updated
async fn update_uninitialized_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    const MAX_INITIAL_BATCH: usize = 30;

    let tokens = match db.get_tokens_without_market_data(MAX_INITIAL_BATCH) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("[UPDATES] Failed to load uninitialized tokens: {}", e);
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    println!(
        "[UPDATES] Seeding market data for {} newly discovered tokens",
        tokens.len()
    );

    // Process in batches of 30
    for chunk in tokens.chunks(30) {
        match update_tokens_batch(chunk, db, coordinator).await {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            eprintln!(
                                "[UPDATES] Failed to record seed error for {}: {}",
                                result.mint, err
                            );
                        }
                    } else if result.is_partial_failure() {
                        eprintln!(
                            "[UPDATES] Partial failure while seeding {}: {:?}",
                            result.mint, result.failures
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("[UPDATES] Batch error during seeding: {}", e);
            }
        }
    }
}

/// Update critical priority tokens (open positions)
async fn update_critical_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    // Get tokens with priority = 100 (open positions)
    let tokens = match db.get_tokens_by_priority(100, 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("[UPDATES] Failed to get critical tokens: {}", e);
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    println!(
        "[UPDATES] Updating {} critical priority tokens",
        tokens.len()
    );

    // Process in batches of 30
    for chunk in tokens.chunks(30) {
        match update_tokens_batch(chunk, db, coordinator).await {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        eprintln!(
                            "[UPDATES] Total failure for {}: {:?}",
                            result.mint, result.failures
                        );
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            eprintln!("[UPDATES] Failed to record error for {}: {}", result.mint, err);
                        }
                    } else if result.is_partial_failure() {
                        eprintln!(
                            "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                            result.mint,
                            result.successes.len(),
                            result.failures.len()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("[UPDATES] Batch error for critical tokens: {}", e);
            }
        }
    }
}

/// Update high priority tokens (filtered/watched)
async fn update_high_priority_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    // Get tokens with priority = 50
    let tokens = match db.get_tokens_by_priority(50, 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("[UPDATES] Failed to get high priority tokens: {}", e);
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    // Limit to 60 tokens total, process in batches of 30
    let batch = &tokens[..tokens.len().min(60)];
    println!("[UPDATES] Updating {} high priority tokens", batch.len());

    for chunk in batch.chunks(30) {
        match update_tokens_batch(chunk, db, coordinator).await {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        let message = result.failures.join(" | ");
                        if let Err(err) =
                            db.record_market_error(result.mint.as_str(), message.as_str())
                        {
                            eprintln!("[UPDATES] Failed to record error for {}: {}", result.mint, err);
                        }
                    } else if result.is_partial_failure() {
                        eprintln!(
                            "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                            result.mint,
                            result.successes.len(),
                            result.failures.len()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("[UPDATES] Batch error for high priority tokens: {}", e);
            }
        }
    }
}

/// Update low priority tokens (oldest non-blacklisted)
async fn update_low_priority_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    // Get oldest 30 non-blacklisted tokens (batch size)
    let tokens = match db.get_oldest_non_blacklisted(30) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("[UPDATES] Failed to get low priority tokens: {}", e);
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    println!("[UPDATES] Updating {} low priority tokens", tokens.len());

    // Process all in one batch (already limited to 30)
    match update_tokens_batch(&tokens, db, coordinator).await {
        Ok(results) => {
            for result in results {
                if result.is_total_failure() {
                    let message = result.failures.join(" | ");
                    if let Err(err) = db.record_market_error(result.mint.as_str(), message.as_str())
                    {
                        eprintln!("[UPDATES] Failed to record error for {}: {}", result.mint, err);
                    }
                } else if result.is_partial_failure() {
                    eprintln!(
                        "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                        result.mint,
                        result.successes.len(),
                        result.failures.len()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("[UPDATES] Batch error for low priority tokens: {}", e);
        }
    }
}
