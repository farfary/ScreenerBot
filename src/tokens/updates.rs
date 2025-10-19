/// Updates orchestrator - Priority-based background updates
///
/// Coordinates fetching from all sources (DexScreener, GeckoTerminal, Rugcheck)
/// with rate limiting and priority-based scheduling.
///
/// Priority levels:
/// - Critical (100): Open positions → Update every 30s
/// - High (50): Filtered/watched tokens → Update every 60s  
/// - Low (10): Oldest non-blacklisted → Update every 5min
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::{dexscreener, geckoterminal};
use crate::tokens::security::rugcheck;
use crate::tokens::types::{TokenError, TokenResult};
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
}

impl RateLimitCoordinator {
    pub fn new() -> Self {
        Self {
            dexscreener_sem: Arc::new(Semaphore::new(300)), // 300/min
            geckoterminal_sem: Arc::new(Semaphore::new(30)), // 30/min
            rugcheck_sem: Arc::new(Semaphore::new(60)),     // 60/min
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
        self.dexscreener_sem.add_permits(300);
        self.geckoterminal_sem.add_permits(30);
        self.rugcheck_sem.add_permits(60);
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

/// Update a single token from all sources with partial failure handling
///
/// Attempts to fetch from all sources, tracks success/failure per source.
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

    // 3. Update Rugcheck security data
    match coordinator.acquire_rugcheck().await {
        Ok(_) => match rugcheck::fetch_rugcheck_data(mint, db).await {
            Ok(Some(_)) => successes.push("Rugcheck".to_string()),
            Ok(None) => failures.push(format!("Rugcheck: Token not analyzed")),
            Err(e) => failures.push(format!("Rugcheck: {}", e)),
        },
        Err(e) => failures.push(format!("Rugcheck rate limit: {}", e)),
    }

    // Update tracking timestamp
    if !successes.is_empty() {
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

// ============================================================================
// PRIORITY-BASED UPDATE LOOPS
// ============================================================================

/// Start the main update loop with all priority levels
pub fn start_update_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> Vec<JoinHandle<()>> {
    let coordinator = Arc::new(RateLimitCoordinator::new());

    let mut handles = Vec::new();

    // Semaphore refill task (every minute)
    let coord_refill = coordinator.clone();
    let shutdown_refill = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_refill.notified() => break,
                _ = sleep(Duration::from_secs(60)) => {
                    coord_refill.refill_all();
                }
            }
        }
    }));

    // Critical priority loop (every 30s)
    let db_critical = db.clone();
    let coord_critical = coordinator.clone();
    let shutdown_critical = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_critical.notified() => break,
                _ = sleep(Duration::from_secs(30)) => {
                    update_critical_tokens(&db_critical, &coord_critical).await;
                }
            }
        }
    }));

    // High priority loop (every 60s)
    let db_high = db.clone();
    let coord_high = coordinator.clone();
    let shutdown_high = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_high.notified() => break,
                _ = sleep(Duration::from_secs(60)) => {
                    update_high_priority_tokens(&db_high, &coord_high).await;
                }
            }
        }
    }));

    // Low priority loop (every 5min)
    let db_low = db.clone();
    let coord_low = coordinator.clone();
    let shutdown_low = shutdown.clone();
    handles.push(tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_low.notified() => break,
                _ = sleep(Duration::from_secs(300)) => {
                    update_low_priority_tokens(&db_low, &coord_low).await;
                }
            }
        }
    }));

    handles
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

    for mint in tokens {
        match update_token(&mint, db, coordinator).await {
            Ok(result) if result.is_success() => {
                // Success - no logging needed
            }
            Ok(result) if result.is_partial_failure() => {
                eprintln!(
                    "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                    mint,
                    result.successes.len(),
                    result.failures.len()
                );
            }
            Ok(result) => {
                eprintln!(
                    "[UPDATES] Total failure for {}: {:?}",
                    mint, result.failures
                );
            }
            Err(e) => {
                eprintln!("[UPDATES] Error updating {}: {}", mint, e);
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

    // Limit to 50 tokens per batch
    let batch = &tokens[..tokens.len().min(50)];
    println!("[UPDATES] Updating {} high priority tokens", batch.len());

    for mint in batch {
        match update_token(mint, db, coordinator).await {
            Ok(result) if result.is_success() => {}
            Ok(result) if result.is_partial_failure() => {
                eprintln!(
                    "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                    mint,
                    result.successes.len(),
                    result.failures.len()
                );
            }
            _ => {}
        }
    }
}

/// Update low priority tokens (oldest non-blacklisted)
async fn update_low_priority_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    // Get oldest 20 non-blacklisted tokens
    let tokens = match db.get_oldest_non_blacklisted(20) {
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

    for mint in tokens {
        match update_token(&mint, db, coordinator).await {
            Ok(result) if result.is_success() => {}
            Ok(result) if result.is_partial_failure() => {
                eprintln!(
                    "[UPDATES] Partial failure for {}: {} succeeded, {} failed",
                    mint,
                    result.successes.len(),
                    result.failures.len()
                );
            }
            _ => {}
        }
    }
}
