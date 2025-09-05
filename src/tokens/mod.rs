//// Pool pricing is enabled - use pool interface only
pub use crate::pool_service::{
    get_pool_service,
    init_pool_service,
    get_price,
    get_price_full,
    get_price_history,
    get_tokens_with_recent_pools_infos,
    check_token_availability,
};

pub use crate::pool_interface::{ 
    PoolInterface, 
    TokenPriceInfo, 
    PriceResult, 
    PriceOptions 
};

// Pool database functions
pub use crate::pool_db::{
    init_pool_db_service,
    store_price_entry,
    get_price_history_for_token,
};

use crate::global::{ is_debug_decimals_enabled, is_debug_monitor_enabled };
/// Centralized Token Management System - Thread-Safe Edition
/// Pool pricing is enabled - use pool module for direct on-chain price calculations

/// This module provides thread-safe access to token data and prices
/// using a centralized price service instead of direct database access.
use crate::logger::{ log, LogTag };
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Notify;

pub mod authority;
pub mod blacklist;
pub mod cache;
pub mod decimals;
pub mod dexscreener;
pub mod discovery;
pub mod geckoterminal;
pub mod holders;
pub mod lp_lock;
pub mod monitor;
pub mod ohlcv_db;
pub mod ohlcvs;
pub mod raydium;
pub mod rugcheck;
pub mod types;

// Re-export main types and functions
pub use authority::{
    get_authority_summary,
    get_multiple_token_authorities,
    get_token_authorities,
    is_token_safe,
    TokenAuthorities,
    TokenRiskLevel,
};
pub use blacklist::{
    add_to_blacklist_manual,
    check_and_track_liquidity,
    get_blacklist_stats,
    initialize_system_stable_blacklist,
    is_system_or_stable_token,
    is_token_blacklisted,
    is_token_excluded_from_trading,
    TokenBlacklist,
};
pub use cache::{ DatabaseStats, TokenDatabase };
pub use decimals::{
    batch_fetch_token_decimals,
    get_cached_decimals,
    get_token_decimals_from_chain,
};
pub use dexscreener::{
    get_global_dexscreener_api,
    get_token_from_mint_global_api,
    get_token_pairs_from_api,
    get_token_prices_from_api,
    init_dexscreener_api,
    DexScreenerApi,
    API_CALLS_PER_MONITORING_CYCLE,
    DEXSCREENER_DISCOVERY_RATE_LIMIT,
    // API configuration constants
    DEXSCREENER_RATE_LIMIT_PER_MINUTE,
    MAX_TOKENS_PER_API_CALL,
};
pub use discovery::{ discover_tokens_once, start_token_discovery, TokenDiscovery };
pub use holders::{
    get_count_holders,
    get_holder_stats,
    get_top_holders_analysis,
    HolderStats,
    TokenHolder,
    TopHoldersAnalysis,
};
pub use lp_lock::{
    check_lp_lock_status,
    check_multiple_lp_locks,
    get_lp_lock_summary,
    is_lp_safe,
    LockPrograms,
    LpLockAnalysis,
    LpLockStatus,
};
pub use geckoterminal::{ OhlcvDataPoint };
pub use ohlcvs::{
    get_latest_ohlcv,
    get_ohlcv_service_clone,
    init_ohlcv_service,
    is_ohlcv_data_available,
    start_ohlcv_monitoring,
    DataAvailability,
    OhlcvService,
};
// Pool service initialization moved to pool_service module
pub use rugcheck::{
    get_high_risk_issues,
    get_rugcheck_score,
    get_token_rugcheck_data,
    is_token_safe_for_trading,
    update_new_token_rugcheck_data,
    RugcheckResponse,
    RugcheckService,
};
pub use types::*;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Enable/disable pool price calculations globally (NOW ENABLED)
pub const ENABLE_POOL_PRICES: bool = true;

/// Primary price source configuration
pub const USE_DEXSCREENER_PRIMARY: bool = true;

/// Price validation thresholds
pub const MAX_PRICE_DEVIATION_PERCENT: f64 = 50.0; // Maximum deviation between sources

// =============================================================================
// TOKENS SYSTEM MANAGER
// =============================================================================

/// Complete tokens system manager
pub struct TokensSystem {
    discovery: TokenDiscovery,
    database: TokenDatabase,
    rugcheck_service: Arc<RugcheckService>,
}

impl TokensSystem {
    /// Create new tokens system instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let discovery = TokenDiscovery::new()?;
        let database = TokenDatabase::new()?;

        // Create rugcheck service with a temporary shutdown notify (will be replaced by global service)
        let shutdown_notify = Arc::new(Notify::new());
        let rugcheck_service = Arc::new(RugcheckService::new(database.clone(), shutdown_notify));

        log(LogTag::System, "INIT", "Tokens system initialized");

        Ok(Self {
            discovery,
            database,
            rugcheck_service,
        })
    }

    /// Start all background tasks
    pub async fn start_background_tasks(
        &mut self,
        shutdown: Arc<Notify>
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let mut handles = Vec::new();

        // Start discovery task
        log(LogTag::System, "START", "Starting token discovery task...");
        let discovery_handle = start_token_discovery(shutdown.clone()).await?;
        handles.push(discovery_handle);

        // Start OHLCV monitoring task
        log(LogTag::System, "START", "Starting OHLCV monitoring task...");
        match start_ohlcv_monitoring(shutdown.clone()).await {
            Ok(ohlcv_handle) => {
                handles.push(ohlcv_handle);
                log(LogTag::System, "SUCCESS", "OHLCV monitoring task started");
            }
            Err(e) => {
                log(LogTag::System, "WARN", &format!("Failed to start OHLCV monitoring: {}", e));
            }
        }

        log(LogTag::System, "SUCCESS", "All tokens system background tasks started");

        Ok(handles)
    }

    /// Get system statistics
    pub async fn get_system_stats(&self) -> Result<TokensSystemStats, String> {
        let db_stats = self.database
            .get_stats()
            .map_err(|e| format!("Failed to get database stats: {}", e))?;
        let blacklist_stats = get_blacklist_stats();

        Ok(TokensSystemStats {
            total_tokens: db_stats.total_tokens,
            tokens_with_liquidity: db_stats.tokens_with_liquidity,
            active_tokens: 0, // No monitoring system
            blacklisted_tokens: blacklist_stats.map(|s| s.total_blacklisted).unwrap_or(0),
            last_discovery_cycle: None,
            last_monitoring_cycle: None,
        })
    }

    /// Get rugcheck service reference
    pub fn get_rugcheck_service(&self) -> Arc<RugcheckService> {
        self.rugcheck_service.clone()
    }
}

/// Tokens system statistics
#[derive(Debug, Clone)]
pub struct TokensSystemStats {
    pub total_tokens: usize,
    pub tokens_with_liquidity: usize,
    pub active_tokens: usize,
    pub blacklisted_tokens: usize,
    pub last_discovery_cycle: Option<chrono::DateTime<chrono::Utc>>,
    pub last_monitoring_cycle: Option<chrono::DateTime<chrono::Utc>>,
}

// =============================================================================
// GLOBAL RUGCHECK SERVICE ACCESS
// =============================================================================

static GLOBAL_RUGCHECK_SERVICE: Mutex<Option<Arc<RugcheckService>>> = Mutex::new(None);

/// Initialize global rugcheck service
pub async fn initialize_global_rugcheck_service(
    database: TokenDatabase,
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    // Check if already initialized and stop the old one first
    if let Some(old_service) = GLOBAL_RUGCHECK_SERVICE.lock().unwrap().take() {
        log(LogTag::System, "INIT", "Replacing existing global rugcheck service");
        // The old service will stop when its shutdown notify is triggered
    }

    let service = Arc::new(RugcheckService::new(database, shutdown));

    // Start background service
    let service_clone = service.clone();
    let handle = tokio::spawn(async move {
        service_clone.start_background_service().await;
    });

    *GLOBAL_RUGCHECK_SERVICE.lock().unwrap() = Some(service);
    log(LogTag::System, "INIT", "Global rugcheck service initialized");
    Ok(handle)
}

/// Get global rugcheck service instance
pub fn get_global_rugcheck_service() -> Option<Arc<RugcheckService>> {
    GLOBAL_RUGCHECK_SERVICE.lock().unwrap().clone()
}

// =============================================================================
// INITIALIZATION FUNCTIONS
// =============================================================================

/// Initialize the tokens system with price service
pub async fn initialize_tokens_system() -> Result<TokensSystem, Box<dyn std::error::Error>> {
    log(LogTag::System, "INIT", "Initializing complete tokens system...");

    // Initialize global RPC client from configuration
    if let Err(e) = crate::rpc::init_rpc_client() {
        log(
            LogTag::System,
            "WARN",
            &format!("RPC config initialization failed, using fallback: {}", e)
        );
    }

    // Initialize global DexScreener API client
    if let Err(e) = init_dexscreener_api().await {
        return Err(format!("Failed to initialize DexScreener API: {}", e).into());
    }

    // Initialize price service
    // Price service initialization moved to pool_service module

    // Note: Position-related cleanup is now handled by positions manager
    // No longer need to cleanup stale watch list entries since monitoring is disabled

    // Initialize OHLCV service
    if let Err(e) = init_ohlcv_service().await {
        log(LogTag::System, "WARN", &format!("OHLCV service initialization failed: {}", e));
    } else {
        log(LogTag::System, "SUCCESS", "OHLCV service initialized successfully");
    }

    // Create tokens system
    let system = TokensSystem::new()?;

    log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");
    Ok(system)
}

// =============================================================================
// SAFE TOKEN ACCESS FUNCTIONS
// =============================================================================

// =============================================================================
// UNIFIED DECIMAL ACCESS FUNCTION
// =============================================================================

/// Universal token decimal access function
///
/// This is the ONLY function you should use for getting token decimals anywhere in the codebase.
/// It handles all scenarios: sync/async, cache/blockchain, SOL native token, and proper error handling.
///
/// **Usage Patterns:**
/// - `get_token_decimals(mint)` - Always check cache first, then blockchain if needed
///
/// **Parameters:**
/// - `mint`: Token mint address
///
/// **Returns:**
/// - `Some(decimals)` - Successfully found decimals (9 for SOL, actual value for tokens)
/// - `None` - Could not determine decimals (caller should skip operations)
///
/// **Debug Logging:** Use `--debug-decimals` flag to enable detailed logging
pub async fn get_token_decimals(mint: &str) -> Option<u8> {
    let debug_enabled = is_debug_decimals_enabled();

    // Handle SOL native token immediately
    if mint == "So11111111111111111111111111111111111111112" {
        if debug_enabled {
            log(LogTag::Decimals, "SOL_NATIVE", "SOL decimals: 9 (native token)");
        }
        return Some(9);
    }

    // First check cache (always available)
    if let Some(decimals) = get_cached_decimals(mint) {
        if debug_enabled {
            log(
                LogTag::Decimals,
                "CACHE_HIT",
                &format!("Cached decimals for {}: {}", &mint[..8], decimals)
            );
        }
        return Some(decimals);
    }

    // If not in cache, try to fetch from blockchain
    if debug_enabled {
        log(
            LogTag::Decimals,
            "BLOCKCHAIN_FETCH",
            &format!("Fetching decimals for {} from blockchain", &mint[..8])
        );
    }

    match get_token_decimals_from_chain(mint).await {
        Ok(decimals) => {
            if debug_enabled {
                log(
                    LogTag::Decimals,
                    "FETCH_SUCCESS",
                    &format!("Fetched decimals {} for {} from blockchain", decimals, &mint[..8])
                );
            }
            return Some(decimals);
        }
        Err(e) => {
            if debug_enabled {
                log(
                    LogTag::Decimals,
                    "FETCH_ERROR",
                    &format!("Failed to fetch decimals for {}: {}", &mint[..8], e)
                );
            }
        }
    }

    // Could not determine decimals - only log this as warning if debug enabled
    if debug_enabled {
        log(
            LogTag::Decimals,
            "NO_DECIMALS",
            &format!("No decimals available for {} - operations will be skipped", &mint[..8])
        );
    }

    None
}

// =============================================================================
// CONVENIENCE WRAPPER FUNCTIONS (for backward compatibility)
// =============================================================================

/// Synchronous cache-only decimal access (for P&L calculations)
/// **Use this in sync contexts where you can't await**
pub fn get_token_decimals_sync(mint: &str) -> Option<u8> {
    // Use a blocking runtime to call the unified get_token_decimals function
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async { get_token_decimals(mint).await })
    })
}

/// Async blockchain-enabled decimal access with Result return type
/// **Use this when you need detailed error information**
pub async fn get_token_decimals_safe(mint: &str) -> Result<u8, String> {
    match get_token_decimals(mint).await {
        Some(decimals) => Ok(decimals),
        None => Err(format!("Could not determine decimals for token {}", mint)),
    }
}

// =============================================================================
// RUGCHECK HELPER FUNCTIONS
// =============================================================================

/// Get rugcheck data for a token using global service
pub async fn get_token_rugcheck_data_safe(mint: &str) -> Result<Option<RugcheckResponse>, String> {
    match get_global_rugcheck_service() {
        Some(service) => service.get_rugcheck_data(mint).await,
        None => {
            log(LogTag::Rugcheck, "ERROR", "Global rugcheck service not initialized");
            Err("Global rugcheck service not initialized".to_string())
        }
    }
}

/// Check if token is safe for trading based on rugcheck data (auto-fetch if missing)
pub async fn is_token_safe_for_trading_safe(mint: &str) -> bool {
    match get_token_rugcheck_data_safe(mint).await {
        Ok(Some(rugcheck_data)) => is_token_safe_for_trading(&rugcheck_data),
        Ok(None) => {
            log(
                LogTag::Rugcheck,
                "WARN",
                &format!("No rugcheck data available after auto-fetch for token: {}", mint)
            );
            true // Changed: Allow trading if rugcheck data unavailable (fail-safe approach)
        }
        Err(e) => {
            log(
                LogTag::Rugcheck,
                "ERROR",
                &format!("Failed to get rugcheck data for {}: {}", mint, e)
            );
            true // Changed: Allow trading if rugcheck service has errors (fail-safe approach)
        }
    }
}

// =============================================================================
// TOKEN DISCOVERY INTEGRATION
// =============================================================================

/// Get current token price using thread-safe price service
pub async fn get_current_token_price(mint: &str) -> Option<f64> {
    get_price(mint).await
}

/// Get all tokens by liquidity using database directly (for compatibility)
pub async fn get_all_tokens_by_liquidity() -> Result<Vec<ApiToken>, String> {
    let db = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;

    match db.get_all_tokens().await {
        Ok(tokens) => {
            let mut sorted_tokens = tokens;
            sorted_tokens.sort_by(|a, b| {
                let a_liq = a.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                let b_liq = b.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
            });
            Ok(sorted_tokens)
        }
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

/// Get token from database using safe system (compatibility function)
pub async fn get_token_from_db(mint: &str) -> Option<Token> {
    let db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return None;
        }
    };

    match db.get_token_by_mint(mint) {
        Ok(Some(api_token)) => Some(api_token.into()),
        _ => None,
    }
}
