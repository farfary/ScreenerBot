//// Pool pricing is enabled - use pool interface only
use crate::global::{ is_debug_decimals_enabled, is_debug_monitor_enabled };
use crate::logger::{ log, LogTag };
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::Notify;

pub mod blacklist;
pub mod cache;
pub mod decimals;
pub mod dexscreener;
pub mod discovery;
pub mod geckoterminal;
pub mod monitor;
pub mod ohlcv_db;
pub mod ohlcvs;
pub mod raydium;
pub mod security;
pub mod security_db;
pub mod types;

// Re-export main types and functions
pub use blacklist::{
    get_blacklist_stats_db,
    initialize_system_stable_blacklist,
    is_system_or_stable_token,
    is_token_blacklisted,
    is_token_excluded_from_trading,
    add_to_blacklist_db,
    track_liquidity_db,
};
pub use cache::{ DatabaseStats, TokenDatabase };
pub use decimals::{ batch_fetch_token_decimals, get_token_decimals_from_chain };
pub use dexscreener::{
    get_global_dexscreener_api,
    get_token_pairs_from_api,
    init_dexscreener_api,
    DexScreenerApi,
    API_CALLS_PER_MONITORING_CYCLE,
    DEXSCREENER_DISCOVERY_RATE_LIMIT,
    // API configuration constants
    DEXSCREENER_RATE_LIMIT_PER_MINUTE,
    MAX_TOKENS_PER_API_CALL,
};
pub use discovery::{ discover_tokens_once, start_token_discovery, TokenDiscovery };
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
pub use security::{
    SecurityAnalyzer,
    SecurityAnalysis,
    RiskLevel,
    is_token_safe,
    get_token_risk_level,
    // Re-export helpers used by bins/tools
    get_security_analyzer,
    initialize_security_analyzer,
    start_security_summary_task,
};
pub use security_db::{
    SecurityDatabase,
    SecurityInfo,
    SecurityRisk,
    MarketInfo,
    HolderInfo,
    parse_rugcheck_response,
};
pub use types::*;

// Re-export from pools module for compatibility
// Re-export from new pools system for compatibility
pub use crate::pools::types::{ PriceResult };

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
}

impl TokensSystem {
    /// Create new tokens system instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let discovery = TokenDiscovery::new()?;
        let database = TokenDatabase::new()?;

        log(LogTag::System, "INIT", "Tokens system initialized");

        Ok(Self {
            discovery,
            database,
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
        let blacklist_stats = get_blacklist_stats_db();

        Ok(TokensSystemStats {
            total_tokens: db_stats.total_tokens,
            tokens_with_liquidity: db_stats.tokens_with_liquidity,
            active_tokens: 0, // No monitoring system
            blacklisted_tokens: blacklist_stats.map(|s| s.total_blacklisted).unwrap_or(0),
            last_discovery_cycle: None,
            last_monitoring_cycle: None,
        })
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
    if let Some(decimals) = decimals::get_cached_decimals(mint) {
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
    // IMPORTANT: Cache-only by design. Never trigger RPC fetches in sync/runtime paths.
    decimals::get_cached_decimals(mint)
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
// TOKEN DISCOVERY INTEGRATION
// =============================================================================

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
