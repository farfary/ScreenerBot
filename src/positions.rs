use crate::{
    tokens::{
        Token,
        get_token_decimals,
        PriceResult,
        get_price,
        PriceOptions,
        get_token_from_db,
        pool::{ add_priority_token, remove_priority_token },
    },
    swaps::{
        get_best_quote,
        execute_best_swap,
        UnifiedQuote,
        config::{ SOL_MINT, QUOTE_SLIPPAGE_PERCENT },
    },
    transactions::{
        get_transaction,
        get_priority_transaction,
        Transaction,
        TransactionStatus,
        get_global_transaction_manager,
    },
    rpc::{ lamports_to_sol, sol_to_lamports },
    errors::{ ScreenerBotError, PositionError, DataError, BlockchainError, NetworkError },
    errors::blockchain::{ parse_structured_solana_error, is_permanent_failure },
    logger::{ log, LogTag, log_price_change },
    arguments::{
        is_dry_run_enabled,
        is_debug_positions_enabled,
        is_debug_swaps_enabled,
        get_max_exit_retries,
    },
    trader::{ CriticalOperationGuard, PROFIT_EXTRA_NEEDED_SOL, MAX_OPEN_POSITIONS },
    utils::{ get_wallet_address, get_token_balance, safe_truncate },
    configs::{ read_configs },
    positions_db::{
        initialize_positions_database,
        PositionState,
        save_position,
        load_all_positions,
        delete_position_by_id,
        update_position,
        get_open_positions as db_get_open_positions,
        get_closed_positions as db_get_closed_positions,
        get_position_by_mint as db_get_position_by_mint,
        save_token_snapshot,
        TokenSnapshot,
    },
    tokens::{
        dexscreener::get_token_from_mint_global_api,
        rugcheck::RugcheckResponse,
        get_token_rugcheck_data_safe,
    },
};
use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use serde::{ Deserialize, Serialize };
use std::{ collections::{ HashMap, HashSet }, sync::{ Arc, LazyLock }, str::FromStr };
use tokio::{ sync::{ Mutex, RwLock, Notify }, time::{ sleep, Duration } };

// ==================== POSITION STRUCTURES ====================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub id: Option<i64>, // Database ID - None for new positions
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_price: Option<f64>,
    pub exit_time: Option<DateTime<Utc>>,
    pub position_type: String, // "buy" or "sell"
    pub entry_size_sol: f64,
    pub total_size_sol: f64,
    pub price_highest: f64,
    pub price_lowest: f64,
    // Transaction signatures
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>, // Amount of tokens bought/sold
    pub effective_entry_price: Option<f64>, // Actual price from on-chain transaction
    pub effective_exit_price: Option<f64>, // Actual exit price from on-chain transaction
    pub sol_received: Option<f64>, // Actual SOL received after sell (lamports converted to SOL)
    // Profit targets
    pub profit_target_min: Option<f64>, // Minimum profit target percentage
    pub profit_target_max: Option<f64>, // Maximum profit target percentage
    pub liquidity_tier: Option<String>, // Liquidity tier for reference
    // Verification status
    pub transaction_entry_verified: bool, // Whether entry transaction is fully verified
    pub transaction_exit_verified: bool, // Whether exit transaction is fully verified
    // Fee tracking
    pub entry_fee_lamports: Option<u64>, // Actual entry transaction fee
    pub exit_fee_lamports: Option<u64>, // Actual exit transaction fee
    // Price tracking
    pub current_price: Option<f64>, // Current market price (updated by monitoring system)
    pub current_price_updated: Option<DateTime<Utc>>, // When current_price was last updated
    // Phantom position handling
    pub phantom_remove: bool,
    pub phantom_confirmations: u32, // How many times we confirmed zero wallet balance while still open
    pub phantom_first_seen: Option<DateTime<Utc>>, // When first confirmed phantom
    pub synthetic_exit: bool, // True if we synthetically closed due to missing exit tx
    pub closed_reason: Option<String>, // Optional reason for closure (e.g., "synthetic_phantom_closure")
}

#[derive(Debug, Clone)]
pub struct PositionLockGuard {
    mint: String,
}

impl Drop for PositionLockGuard {
    fn drop(&mut self) {
        // Lock will be automatically cleaned up when the guard is dropped
        // The Arc<Mutex<()>> will be removed from POSITION_LOCKS when no longer referenced
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("🔓 Released position lock for mint: {}", safe_truncate(&self.mint, 8))
            );
        }
    }
}

// ==================== GLOBAL STATE ====================

#[derive(Debug)]
pub struct GlobalPositionsState {
    pub positions: Vec<Position>,
    pub pending_verifications: HashMap<String, DateTime<Utc>>, // signature -> timestamp
    pub retry_queue: HashMap<String, (DateTime<Utc>, u32)>, // signature -> (next_retry, count)
    pub frozen_cooldowns: HashMap<String, DateTime<Utc>>, // mint -> cooldown_until
    pub last_open_time: Option<DateTime<Utc>>, // Global open cooldown
    pub reentry_cooldowns: HashMap<String, DateTime<Utc>>, // mint -> cooldown_until
    pub exit_verification_deadlines: HashMap<String, DateTime<Utc>>, // signature -> deadline
    pub failed_exit_retries: HashMap<String, (DateTime<Utc>, u32)>, // mint -> (next_retry, attempt_count)
}

impl GlobalPositionsState {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            pending_verifications: HashMap::new(),
            retry_queue: HashMap::new(),
            frozen_cooldowns: HashMap::new(),
            last_open_time: None,
            reentry_cooldowns: HashMap::new(),
            exit_verification_deadlines: HashMap::new(),
            failed_exit_retries: HashMap::new(),
        }
    }
}

// ==================== GLOBAL STATICS ====================

// Global positions state (replaces actor)
static GLOBAL_POSITIONS_STATE: LazyLock<Mutex<GlobalPositionsState>> = LazyLock::new(|| {
    Mutex::new(GlobalPositionsState::new())
});

// Per-position locks for operation safety
static POSITION_LOCKS: LazyLock<RwLock<HashMap<String, Arc<Mutex<()>>>>> = LazyLock::new(|| {
    RwLock::new(HashMap::new())
});

// Global position creation lock to prevent race conditions on MAX_OPEN_POSITIONS
static GLOBAL_POSITION_CREATION_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| { Mutex::new(()) });

// Safety mechanisms from original implementation
static RECENT_SWAP_ATTEMPTS: LazyLock<RwLock<HashMap<String, DateTime<Utc>>>> = LazyLock::new(|| {
    RwLock::new(HashMap::new())
});
static ACTIVE_SELLS: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(|| {
    RwLock::new(HashSet::new())
});
static BALANCE_CACHE: LazyLock<RwLock<HashMap<String, (f64, DateTime<Utc>)>>> = LazyLock::new(|| {
    RwLock::new(HashMap::new())
});

// Safety constants from original implementation
const PHANTOM_TIMEOUT_MINUTES: i64 = 5;
const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_DELAY_MINUTES: u64 = 2;
const VERIFICATION_BATCH_SIZE: usize = 10;
const CLEANUP_BATCH_SIZE: usize = 20;
const SWAP_ATTEMPT_COOLDOWN_SECONDS: i64 = 30;
const BALANCE_CACHE_DURATION_SECONDS: i64 = 30;
const DUPLICATE_SWAP_PREVENTION_SECS: i64 = 30;
const POSITION_OPEN_COOLDOWN_SECS: i64 = 0; // No global cooldown (from backup)
const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 6 * 60; // Re-entry cooldown after closing (from backup)

// Verification safety windows - reduced for better UX
const ENTRY_VERIFICATION_MAX_SECS: i64 = 90; // hard cap for entry verification age before treating as timeout
const EXIT_VERIFICATION_MAX_SECS: i64 = 60; // 1 minute for exit verification (faster than entry)
const VERIFICATION_GRACE_PERIOD_SECS: i64 = 120; // grace period before aggressive cleanup (2 minutes)

// Sell retry slippages (progressive)
const SELL_RETRY_SLIPPAGES: &[f64] = &[3.0, 5.0, 8.0, 12.0, 20.0];

// Failed exit retry configuration
const FAILED_EXIT_RETRY_DELAY_MINUTES: u64 = 2; // Wait 2 minutes before retrying failed exit
const MAX_FAILED_EXIT_RETRIES: u32 = 5; // Maximum 5 retry attempts for failed exits
const FAILED_EXIT_RETRY_SLIPPAGES: &[f64] = &[5.0, 8.0, 12.0, 15.0, 20.0]; // Progressive slippage for failed exits

// ==================== POSITION LOCKING ====================

/// Acquire a per-position lock to ensure safe concurrent operations
pub async fn acquire_position_lock(mint: &str) -> PositionLockGuard {
    let mint_key = mint.to_string();

    // Get or create the lock for this mint
    let lock = {
        let mut locks = POSITION_LOCKS.write().await;
        locks
            .entry(mint_key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };

    // Acquire the lock
    let _guard = lock.lock().await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🔒 Acquired position lock for mint: {}", safe_truncate(&mint_key, 8))
        );
    }

    PositionLockGuard { mint: mint_key }
}

// ==================== TOKEN SNAPSHOT FUNCTIONS ====================

/// Fetch latest token data from APIs and create a snapshot
async fn fetch_and_create_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str,
) -> Result<TokenSnapshot, String> {
    let fetch_start = Utc::now();
    
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "SNAPSHOT_FETCH",
            &format!("Fetching latest token data for {} snapshot of {}", snapshot_type, safe_truncate(mint, 8))
        );
    }

    // Fetch latest data from DexScreener API
    let dex_token = match get_token_from_mint_global_api(mint).await {
        Ok(Some(token)) => Some(token),
        Ok(None) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_NO_DEX_DATA",
                &format!("No DexScreener data found for {}", safe_truncate(mint, 8))
            );
            None
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_DEX_ERROR",
                &format!("Error fetching DexScreener data for {}: {}", safe_truncate(mint, 8), e)
            );
            None
        }
    };

    // Fetch latest rugcheck data
    let rugcheck_data = match get_token_rugcheck_data_safe(mint).await {
        Ok(Some(data)) => Some(data),
        Ok(None) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_NO_RUGCHECK",
                &format!("No rugcheck data found for {}", safe_truncate(mint, 8))
            );
            None
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_RUGCHECK_ERROR",
                &format!("Error fetching rugcheck data for {}: {}", safe_truncate(mint, 8), e)
            );
            None
        }
    };

    // Calculate data freshness score (0-100)
    let fetch_duration_ms = Utc::now().signed_duration_since(fetch_start).num_milliseconds();
    let freshness_score = if fetch_duration_ms < 1000 {
        100 // Very fresh, under 1 second
    } else if fetch_duration_ms < 5000 {
        80  // Good, under 5 seconds
    } else if fetch_duration_ms < 10000 {
        60  // OK, under 10 seconds
    } else if fetch_duration_ms < 30000 {
        40  // Slow, under 30 seconds
    } else {
        20  // Very slow, over 30 seconds
    };

    // Extract DexScreener data
    let (symbol, name, price_sol, price_usd, price_native, dex_id, pair_address, pair_url,
         fdv, market_cap, pair_created_at, liquidity_usd, liquidity_base, liquidity_quote,
         volume_h24, volume_h6, volume_h1, volume_m5,
         txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
         txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
         price_change_h24, price_change_h6, price_change_h1, price_change_m5) = 
        if let Some(ref token) = dex_token {
            (
                Some(token.symbol.clone()),
                Some(token.name.clone()),
                token.price_dexscreener_sol,
                token.price_dexscreener_usd,
                token.price_dexscreener_sol, // Use SOL price as native
                token.dex_id.clone(),
                token.pair_address.clone(),
                token.pair_url.clone(),
                token.fdv,
                token.market_cap,
                token.created_at.map(|dt| dt.timestamp()),
                token.liquidity.as_ref().and_then(|l| l.usd),
                token.liquidity.as_ref().and_then(|l| l.base),
                token.liquidity.as_ref().and_then(|l| l.quote),
                token.volume.as_ref().and_then(|v| v.h24),
                token.volume.as_ref().and_then(|v| v.h6),
                token.volume.as_ref().and_then(|v| v.h1),
                token.volume.as_ref().and_then(|v| v.m5),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.sells)),
                token.price_change.as_ref().and_then(|pc| pc.h24),
                token.price_change.as_ref().and_then(|pc| pc.h6),
                token.price_change.as_ref().and_then(|pc| pc.h1),
                token.price_change.as_ref().and_then(|pc| pc.m5),
            )
        } else {
            (None, None, None, None, None, None, None, None, None, None, None, None, None, None,
             None, None, None, None, None, None, None, None, None, None, None, None,
             None, None, None, None)
        };

    // Extract rugcheck data
    let (rugcheck_score, rugcheck_score_normalised, rugcheck_rugged, rugcheck_risks_json,
         rugcheck_mint_authority, rugcheck_freeze_authority, rugcheck_creator, rugcheck_creator_balance,
         rugcheck_total_holders, rugcheck_total_market_liquidity, rugcheck_total_stable_liquidity,
         rugcheck_total_lp_providers, rugcheck_lp_locked_pct, rugcheck_lp_locked_usd,
         rugcheck_transfer_fee_pct, rugcheck_transfer_fee_max_amount, rugcheck_jup_verified, rugcheck_jup_strict,
         token_uri, token_description, token_image, token_website, token_twitter, token_telegram) = 
        if let Some(ref data) = rugcheck_data {
            let risks_json = if let Some(risks) = &data.risks {
                match serde_json::to_string(risks) {
                    Ok(json) => Some(json),
                    Err(_) => None,
                }
            } else {
                None
            };

            let lp_data = data.markets.as_ref()
                .and_then(|markets| markets.first())
                .and_then(|market| market.lp.as_ref());

            (
                data.score,
                data.score_normalised,
                data.rugged,
                risks_json,
                data.mint_authority.as_ref().and_then(|ma| serde_json::to_string(ma).ok()),
                data.freeze_authority.as_ref().and_then(|fa| serde_json::to_string(fa).ok()),
                data.creator.clone(),
                data.creator_balance.clone(),
                data.total_holders,
                data.total_market_liquidity,
                data.total_stable_liquidity,
                data.total_lp_providers,
                lp_data.and_then(|lp| lp.lp_locked_pct),
                lp_data.and_then(|lp| lp.lp_locked_usd),
                data.transfer_fee.as_ref().and_then(|tf| tf.pct),
                data.transfer_fee.as_ref().and_then(|tf| tf.max_amount.clone()),
                data.verification.as_ref().and_then(|v| v.jup_verified),
                data.verification.as_ref().and_then(|v| v.jup_strict),
                data.token_meta.as_ref().and_then(|tm| tm.uri.clone()),
                data.file_meta.as_ref().and_then(|fm| fm.description.clone()),
                data.file_meta.as_ref().and_then(|fm| fm.image.clone()),
                None, // website - extract from verification links if needed
                None, // twitter - extract from verification links if needed
                None, // telegram - extract from verification links if needed
            )
        } else {
            (None, None, None, None, None, None, None, None, None, None, None, None, None, None,
             None, None, None, None, None, None, None, None, None, None)
        };

    // Create snapshot
    let snapshot = TokenSnapshot {
        id: None,
        position_id,
        snapshot_type: snapshot_type.to_string(),
        mint: mint.to_string(),
        symbol,
        name,
        price_sol,
        price_usd,
        price_native,
        dex_id,
        pair_address,
        pair_url,
        fdv,
        market_cap,
        pair_created_at,
        liquidity_usd,
        liquidity_base,
        liquidity_quote,
        volume_h24,
        volume_h6,
        volume_h1,
        volume_m5,
        txns_h24_buys,
        txns_h24_sells,
        txns_h6_buys,
        txns_h6_sells,
        txns_h1_buys,
        txns_h1_sells,
        txns_m5_buys,
        txns_m5_sells,
        price_change_h24,
        price_change_h6,
        price_change_h1,
        price_change_m5,
        rugcheck_score,
        rugcheck_score_normalised,
        rugcheck_rugged,
        rugcheck_risks_json,
        rugcheck_mint_authority,
        rugcheck_freeze_authority,
        rugcheck_creator,
        rugcheck_creator_balance,
        rugcheck_total_holders,
        rugcheck_total_market_liquidity,
        rugcheck_total_stable_liquidity,
        rugcheck_total_lp_providers,
        rugcheck_lp_locked_pct,
        rugcheck_lp_locked_usd,
        rugcheck_transfer_fee_pct,
        rugcheck_transfer_fee_max_amount,
        rugcheck_jup_verified,
        rugcheck_jup_strict,
        token_uri,
        token_description,
        token_image,
        token_website,
        token_twitter,
        token_telegram,
        snapshot_time: Utc::now(),
        api_fetch_time: fetch_start,
        data_freshness_score: freshness_score,
    };

    log(
        LogTag::Positions,
        "SNAPSHOT_CREATED",
        &format!(
            "Created {} snapshot for {} - freshness: {}/100, price: {:?} SOL, rugcheck: {:?}",
            snapshot_type,
            safe_truncate(mint, 8),
            freshness_score,
            price_sol,
            rugcheck_score_normalised.or(rugcheck_score)
        )
    );

    Ok(snapshot)
}

/// Save token snapshot for a position
pub async fn save_position_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str,
) -> Result<(), String> {
    // Fetch and create snapshot
    let snapshot = fetch_and_create_token_snapshot(position_id, mint, snapshot_type).await?;
    
    // Save to database
    match save_token_snapshot(&snapshot).await {
        Ok(snapshot_id) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_SAVED",
                &format!(
                    "Saved {} snapshot for {} with ID {}",
                    snapshot_type,
                    safe_truncate(mint, 8),
                    snapshot_id
                )
            );
            Ok(())
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_SAVE_ERROR",
                &format!(
                    "Failed to save {} snapshot for {}: {}",
                    snapshot_type,
                    safe_truncate(mint, 8),
                    e
                )
            );
            Err(e)
        }
    }
}

// ==================== CORE POSITION OPERATIONS ====================

/// Open a new position directly (replaces actor message)
pub async fn open_position_direct(
    token: &Token,
    entry_price: f64,
    percent_change: f64,
    size_sol: f64,
    liquidity_tier: Option<String>,
    profit_target_min: f64,
    profit_target_max: f64
) -> Result<String, String> {
    let _lock = acquire_position_lock(&token.mint).await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🎯 Starting open_position for {} at price {:.8} SOL ({}% change) with size {} SOL",
                token.symbol,
                entry_price,
                percent_change,
                size_sol
            )
        );
    }

    // CRITICAL SAFETY CHECK: Validate price
    if entry_price <= 0.0 || !entry_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("❌ Invalid price validation failed: {}", entry_price)
            );
        }
        return Err(format!("Price must be positive and finite: {}", entry_price));
    }

    // DRY-RUN MODE CHECK
    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would open position for {} ({}) at {:.6} SOL ({}%)",
                token.symbol,
                get_mint_prefix(&token.mint),
                entry_price,
                percent_change
            )
        );
        return Err("DRY-RUN: Position would be opened".to_string());
    }

    // ATOMIC POSITION CREATION: Use global lock to prevent race conditions
    // This ensures position limit checks and creation happen atomically
    let _global_creation_lock = GLOBAL_POSITION_CREATION_LOCK.lock().await;

    // Check cooldowns and existing positions under global lock
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        // RE-ENTRY COOLDOWN CHECK
        if let Some(cooldown_until) = state.reentry_cooldowns.get(&token.mint) {
            let now = Utc::now();
            if *cooldown_until > now {
                let remaining = (*cooldown_until - now).num_minutes();
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "⏳ Re-entry cooldown active for {} - {} minutes remaining",
                            token.symbol,
                            remaining
                        )
                    );
                }
                return Err(
                    format!(
                        "Re-entry cooldown active for {} ({}): wait {}m",
                        token.symbol,
                        get_mint_prefix(&token.mint),
                        remaining
                    )
                );
            }
        }

        // GLOBAL COOLDOWN CHECK
        if let Some(last_open) = state.last_open_time {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(last_open).num_seconds();
            if elapsed < POSITION_OPEN_COOLDOWN_SECS {
                let remaining = POSITION_OPEN_COOLDOWN_SECS - elapsed;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("⏳ Global open cooldown active - {} seconds remaining", remaining)
                    );
                }
                return Err(format!("Opening positions cooldown active: wait {}s", remaining));
            }
        }

        // ATOMIC POSITION LIMIT CHECK
        let (already_has_position, open_positions_count) = {
            let has_position = state.positions
                .iter()
                .any(|p| {
                    p.mint == token.mint && p.position_type == "buy" && p.exit_time.is_none()
                });

            let count = state.positions
                .iter()
                .filter(|p| {
                    p.position_type == "buy" && 
                    p.exit_time.is_none() &&
                    // Only count truly open positions for limit checks
                    (!p.exit_transaction_signature.is_some() || !p.transaction_exit_verified)
                })
                .count();

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "📊 ATOMIC position check - existing: {}, open count: {}/{}",
                        has_position,
                        count,
                        MAX_OPEN_POSITIONS
                    )
                );
            }

            (has_position, count)
        };

        if already_has_position {
            return Err("Already have open position for this token".to_string());
        }

        if open_positions_count >= MAX_OPEN_POSITIONS {
            return Err(
                format!(
                    "Maximum open positions reached ({}/{})",
                    open_positions_count,
                    MAX_OPEN_POSITIONS
                )
            );
        }

        // Update global open time
        state.last_open_time = Some(Utc::now());
    }

    // Execute the buy transaction (still under global creation lock)
    let _guard = CriticalOperationGuard::new(&format!("BUY {}", token.symbol));

    // DUPLICATE SWAP PREVENTION
    if is_duplicate_swap_attempt(&token.mint, size_sol, "BUY").await {
        return Err(
            format!(
                "Duplicate swap prevented for {} - similar buy attempted within last {}s",
                token.symbol,
                DUPLICATE_SWAP_PREVENTION_SECS
            )
        );
    }

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "💸 Executing swap for {} with {} SOL at price {:.8}",
                token.symbol,
                size_sol,
                entry_price
            )
        );
    }

    // Validate expected price
    if entry_price <= 0.0 || !entry_price.is_finite() {
        log(
            LogTag::Positions,
            "ERROR",
            &format!(
                "❌ REFUSING TO BUY: Invalid expected_price for {} ({}). Price = {:.10}",
                token.symbol,
                token.mint,
                entry_price
            )
        );
        return Err(format!("Invalid expected price: {:.10}", entry_price));
    }

    log(
        LogTag::Positions,
        "BUY_START",
        &format!(
            "🟢 BUYING {} SOL worth of {} tokens (mint: {})",
            size_sol,
            token.symbol,
            token.mint
        )
    );

    // Add token to watch list before opening position
    let _price_service_result = match
        tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            get_price(&token.mint, Some(PriceOptions::simple()), false)
        ).await
    {
        Ok(result) => result,
        Err(_) => {
            log(
                LogTag::Positions,
                "TIMEOUT",
                &format!(
                    "⏰ Price service timeout for {} after 10s - continuing without price check",
                    token.symbol
                )
            );
            None
        }
    };

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "WATCH_LIST",
            &format!("✅ Added {} to price monitoring watch list before swap", token.symbol)
        );
    }

    // Get wallet address
    let wallet_address = get_wallet_address().map_err(|e| {
        log(LogTag::Positions, "ERROR", &format!("❌ Failed to get wallet address: {}", e));
        format!("Failed to get wallet address: {}", e)
    })?;

    // Get best quote with timeout
    let best_quote = match
        tokio::time::timeout(
            tokio::time::Duration::from_secs(20),
            get_best_quote(
                SOL_MINT,
                &token.mint,
                sol_to_lamports(size_sol),
                &wallet_address,
                QUOTE_SLIPPAGE_PERCENT
            )
        ).await
    {
        Ok(Ok(quote)) => quote,
        Ok(Err(e)) => {
            return Err(format!("Quote request failed: {}", e));
        }
        Err(_) => {
            log(
                LogTag::Positions,
                "TIMEOUT",
                &format!("⏰ Quote request timeout for {} after 20s", token.symbol)
            );
            return Err(format!("Quote request timeout for {}", token.symbol));
        }
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "📊 Best quote from {:?}: {} SOL -> {} tokens",
                best_quote.router,
                lamports_to_sol(best_quote.input_amount),
                best_quote.output_amount
            )
        );
    }

    log(
        LogTag::Positions,
        "SWAP",
        &format!("🚀 Executing swap with best quote via {:?}...", best_quote.router)
    );

    // Execute the swap
    let swap_result = execute_best_swap(
        token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(size_sol),
        best_quote
    ).await.map_err(|e| format!("Swap execution failed: {}", e))?;

    if let Some(ref signature) = swap_result.transaction_signature {
        log(
            LogTag::Positions,
            "TRANSACTION",
            &format!("Transaction {} will be monitored by positions manager", safe_truncate(&signature, 8))
        );
    }

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🟢 BUY operation completed for {} - Success: {} | TX: {}",
                token.symbol,
                swap_result.success,
                swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );
    }

    let transaction_signature = swap_result.transaction_signature.clone().unwrap_or_default();

    // CRITICAL VALIDATION: Verify transaction signature is valid before creating position
    if transaction_signature.is_empty() || transaction_signature.len() < 32 {
        return Err(format!("Transaction signature is invalid or empty: {}", transaction_signature));
    }

    // Additional validation: Check if signature is valid base58
    if bs58::decode(&transaction_signature).into_vec().is_err() {
        return Err(
            format!("Invalid base58 format: {}", get_signature_prefix(&transaction_signature))
        );
    }

    // Log swap execution details
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "✅ Swap executed via {:?} - signature: {}, success: {}",
                swap_result.router_used
                    .as_ref()
                    .map(|r| format!("{:?}", r))
                    .unwrap_or_else(|| "Unknown".to_string()),
                get_signature_prefix(&transaction_signature),
                swap_result.success
            )
        );
    }

    // Create position optimistically
    let new_position = Position {
        id: None, // Will be set by database after insertion
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        entry_price,
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: size_sol,
        total_size_sol: size_sol,
        price_highest: entry_price,
        price_lowest: entry_price,
        entry_transaction_signature: Some(transaction_signature.clone()),
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: None,
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: Some(profit_target_min),
        profit_target_max: Some(profit_target_max),
        liquidity_tier,
        transaction_entry_verified: false,
        transaction_exit_verified: false,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: Some(entry_price), // Initialize with entry price
        current_price_updated: Some(Utc::now()),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: None,
    };

    // Add position to global state and database (still under global creation lock)
    let position_id = {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        // Position limits already checked atomically above - no need for redundant checks

        // Track for comprehensive verification
        state.pending_verifications.insert(transaction_signature.clone(), Utc::now());

        // Save to database first to get the ID
        let position_id = match save_position(&new_position).await {
            Ok(id) => {
                log(
                    LogTag::Positions,
                    "INSERT",
                    &format!(
                        "Inserted new position ID {} for mint {}",
                        id,
                        get_mint_prefix(&token.mint)
                    )
                );
                log(
                    LogTag::Positions,
                    "DB_SAVE",
                    &format!("Position saved to database with ID {}", id)
                );

                // Save opening token snapshot (async, non-blocking)
                {
                    let mint_clone = token.mint.clone();
                    tokio::spawn(async move {
                        if let Err(e) = save_position_token_snapshot(id, &mint_clone, "opening").await {
                            log(
                                LogTag::Positions,
                                "SNAPSHOT_WARN",
                                &format!("Failed to save opening snapshot for {}: {}", safe_truncate(&mint_clone, 8), e)
                            );
                        }
                    });
                }

                id
            }
            Err(e) => {
                log(
                    LogTag::Positions,
                    "DB_ERROR",
                    &format!("Failed to save position to database: {}", e)
                );
                // Continue without database ID - position is still in memory
                -1
            }
        };

        // Update position with database ID if successful
        let mut position_with_id = new_position.clone();
        if position_id > 0 {
            position_with_id.id = Some(position_id);
        }

        // Add position to in-memory list with correct ID
        state.positions.push(position_with_id);

        // Add token to priority pool service for fast price updates
        add_priority_token(&token.mint).await;

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "✅ Position created for {} with signature {} - profit targets: {:.2}%-{:.2}% | Added to priority pool service",
                    token.symbol,
                    get_signature_prefix(&transaction_signature),
                    profit_target_min,
                    profit_target_max
                )
            );
        }

        position_id
    };

    // Log entry transaction with comprehensive verification
    log(
        LogTag::Positions,
        "POSITION_ENTRY",
        &format!(
            "📝 Entry transaction {} added to comprehensive verification queue (RPC + transaction analysis)",
            get_signature_prefix(&transaction_signature)
        )
    );

    // Immediately attempt to fetch transaction to accelerate verification
    let sig_for_fetch = transaction_signature.clone();
    tokio::spawn(async move {
        let _ = get_transaction(&sig_for_fetch).await;
    });

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ POSITION CREATED: {} | TX: {} | Signal Price: {:.12} SOL | Verification: Pending",
            token.symbol,
            get_signature_prefix(&transaction_signature),
            entry_price
        )
    );

    Ok(transaction_signature)
}

/// Close an existing position directly (replaces actor message)
pub async fn close_position_direct(
    mint: &str,
    token: &Token,
    exit_price: f64,
    exit_reason: String,
    exit_time: DateTime<Utc>
) -> Result<String, String> {
    let _lock = acquire_position_lock(mint).await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔄 Attempting to close position for {} - reason: {} at price {:.8} SOL",
                token.symbol,
                exit_reason,
                exit_price
            )
        );
    }

    // DRY-RUN MODE CHECK
    if is_dry_run_enabled() {
        let position_info = {
            let state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions
                .iter()
                .find(|p| p.mint == mint && p.exit_price.is_none())
                .map(|p| format!("{} ({})", p.symbol, get_mint_prefix(&p.mint)))
        };

        if let Some(info) = position_info {
            log(
                LogTag::Positions,
                "DRY-RUN",
                &format!("🚫 DRY-RUN: Would close position for {}", info)
            );
            return Err("DRY-RUN: Position would be closed".to_string());
        }
    }

    // Find position and validate with enhanced state checking
    let position_info = {
        let state = GLOBAL_POSITIONS_STATE.lock().await;
        state.positions
            .iter()
            .find(|p| {
                let matches_mint = p.mint == mint;
                let no_exit_sig = p.exit_transaction_signature.is_none();
                let failed_exit =
                    p.exit_transaction_signature.is_some() && !p.transaction_exit_verified;
                let can_close = no_exit_sig || failed_exit;

                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "🎯 Position check: mint_match={}, no_exit_sig={}, failed_exit={}, can_close={}",
                            matches_mint,
                            no_exit_sig,
                            failed_exit,
                            can_close
                        )
                    );
                }

                matches_mint && can_close
            })
            .map(|p| (p.symbol.clone(), p.entry_size_sol, p.entry_price))
    };

    let (symbol, entry_size_sol, entry_price) = match position_info {
        Some(info) => info,
        None => {
            return Err(format!("No open position found for token {}", get_mint_prefix(mint)));
        }
    };

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "📊 Found position for {} - entry: {:.8} SOL, size: {} SOL",
                symbol,
                entry_price,
                entry_size_sol
            )
        );
    }

    // Clear failed exit transaction data if retrying
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;
        if let Some(position) = state.positions.iter_mut().find(|p| p.mint == mint) {
            if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
                log(
                    LogTag::Positions,
                    "RETRY_EXIT",
                    &format!(
                        "🔄 Previous exit transaction failed for {} - clearing failed exit data and retrying",
                        position.symbol
                    )
                );
                // Clear failed exit transaction data
                position.exit_price = None;
                position.exit_time = None;
                position.transaction_exit_verified = false;
                position.sol_received = None;
                position.effective_exit_price = None;
                position.exit_fee_lamports = None;
                position.exit_transaction_signature = None; // Clear failed signature
            }
        }
    }

    // Check active sells to prevent duplicates
    {
        let active_sells = ACTIVE_SELLS.read().await;
        if active_sells.contains(mint) {
            return Err(
                format!("Sell already in progress for {} ({})", symbol, get_mint_prefix(mint))
            );
        }
    }

    // Mark as actively selling
    {
        let mut active_sells = ACTIVE_SELLS.write().await;
        active_sells.insert(mint.to_string());
    }

    // Clean up function for consistent exit handling
    let cleanup = || async {
        let mut active_sells = ACTIVE_SELLS.write().await;
        active_sells.remove(mint);
    };

    let _guard = CriticalOperationGuard::new(&format!("SELL {}", symbol));

    // DUPLICATE SWAP PREVENTION
    if is_duplicate_swap_attempt(mint, entry_size_sol, "SELL").await {
        cleanup().await;
        return Err(
            format!(
                "Duplicate swap prevented for {} - similar sell attempted within last {}s",
                symbol,
                DUPLICATE_SWAP_PREVENTION_SECS
            )
        );
    }

    // ✅ ENSURE token remains in watch list during sell process
    let _price_service_result = match
        tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            get_price(&token.mint, Some(PriceOptions::simple()), false)
        ).await
    {
        Ok(result) => result,
        Err(_) => {
            log(
                LogTag::Positions,
                "TIMEOUT",
                &format!(
                    "⏰ Price service timeout for {} during sell after 10s - continuing without price check",
                    token.symbol
                )
            );
            None
        }
    };

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "WATCH_LIST",
            &format!("✅ Refreshed {} in watch list before sell execution", token.symbol)
        );
    }

    log(
        LogTag::Positions,
        "SELL_START",
        &format!("🔴 SELLING all {} tokens (mint: {}) for SOL", symbol, mint)
    );

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            cleanup().await;
            log(LogTag::Positions, "ERROR", &format!("❌ Failed to get wallet address: {}", e));
            return Err(format!("Failed to get wallet address: {}", e));
        }
    };

    // Get token balance using cached function for better performance
    let token_balance = match get_token_balance_safe(mint, &wallet_address).await {
        Some(balance) if balance > 0 => balance,
        Some(_) => {
            cleanup().await;
            return Err(format!("No {} tokens to sell", symbol));
        }
        None => {
            cleanup().await;
            return Err(format!("Failed to get token balance for {}", symbol));
        }
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("📊 Token balance for {}: {} tokens", symbol, token_balance)
        );
    }

    // Multi-slippage sell attempt with retries
    let mut last_error = String::new();
    let mut best_quote: Option<UnifiedQuote> = None;
    let mut quote_slippage_used = 0.0;

    for &slippage in SELL_RETRY_SLIPPAGES.iter() {
        if is_debug_swaps_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("🔄 Attempting sell with {:.1}% slippage for {}", slippage, symbol)
            );
        }

        // Get quote with current slippage
        match
            tokio::time::timeout(
                tokio::time::Duration::from_secs(20),
                get_best_quote(mint, SOL_MINT, token_balance, &wallet_address, slippage)
            ).await
        {
            Ok(Ok(quote)) => {
                best_quote = Some(quote);
                quote_slippage_used = slippage;
                if is_debug_swaps_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "✅ Quote obtained with {:.1}% slippage: {} tokens -> {} SOL",
                            slippage,
                            token_balance,
                            lamports_to_sol(best_quote.as_ref().unwrap().output_amount)
                        )
                    );
                }
                break;
            }
            Ok(Err(e)) => {
                last_error = e.to_string();
                if is_debug_swaps_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("❌ Quote failed with {:.1}% slippage: {}", slippage, last_error)
                    );
                }
                continue;
            }
            Err(_) => {
                last_error = format!("Quote timeout with {:.1}% slippage", slippage);
                if is_debug_swaps_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("⏰ Quote timeout with {:.1}% slippage", slippage)
                    );
                }
                continue;
            }
        }
    }

    let quote = match best_quote {
        Some(q) => q,
        None => {
            cleanup().await;
            return Err(format!("All sell quotes failed for {}: {}", symbol, last_error));
        }
    };

    log(
        LogTag::Positions,
        "SWAP",
        &format!(
            "🚀 Executing sell with {:.1}% slippage via {:?}...",
            quote_slippage_used,
            quote.router
        )
    );

    // Execute the swap using the provided token object (no manual creation needed)
    let swap_result = execute_best_swap(token, mint, SOL_MINT, token_balance, quote).await;

    let transaction_signature = match swap_result {
        Ok(result) => {
            if let Some(ref signature) = result.transaction_signature {
                log(
                    LogTag::Positions,
                    "TRANSACTION",
                    &format!(
                        "Sell transaction {} will be monitored by positions manager",
                        safe_truncate(&signature, 8)
                    )
                );
                signature.clone()
            } else {
                cleanup().await;
                return Err(format!("Sell swap completed but no transaction signature returned"));
            }
        }
        Err(e) => {
            cleanup().await;
            return Err(format!("Sell swap execution failed: {}", e));
        }
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔴 SELL operation completed for {} - TX: {}",
                symbol,
                get_signature_prefix(&transaction_signature)
            )
        );
    }

    // CRITICAL VALIDATION: Verify transaction signature is valid before updating position
    if transaction_signature.is_empty() || transaction_signature.len() < 32 {
        cleanup().await;
        return Err(format!("Transaction signature is invalid or empty: {}", transaction_signature));
    }

    // Additional validation: Check if signature is valid base58
    if bs58::decode(&transaction_signature).into_vec().is_err() {
        cleanup().await;
        return Err(
            format!("Invalid base58 format: {}", get_signature_prefix(&transaction_signature))
        );
    }

    // Update position with exit transaction using provided exit_time
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;
        let mut position_for_db: Option<Position> = None;

        if
            let Some(position) = state.positions
                .iter_mut()
                .find(
                    |p| p.mint == mint && (p.exit_price.is_none() || !p.transaction_exit_verified)
                )
        {
            position.exit_transaction_signature = Some(transaction_signature.clone());
            // Don't set exit_time and exit_price until verified - keep position as "closing in progress"
            position.closed_reason = Some(format!("{}_pending_verification", exit_reason));

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "✅ Position updated with exit transaction {} for {} at {}",
                        get_signature_prefix(&transaction_signature),
                        symbol,
                        exit_time.format("%H:%M:%S%.3f")
                    )
                );
            }

            // Clone position for database update
            position_for_db = Some(position.clone());
        } else {
            log(
                LogTag::Positions,
                "WARNING",
                &format!("⚠️ Position for {} not found during exit update", symbol)
            );
        }

        // Track for comprehensive verification
        state.pending_verifications.insert(transaction_signature.clone(), Utc::now());

        // Add re-entry cooldown
        let cooldown_duration = chrono::Duration::minutes(POSITION_CLOSE_COOLDOWN_MINUTES);
        state.reentry_cooldowns.insert(mint.to_string(), Utc::now() + cooldown_duration);

        // Update in database (after releasing the lock)
        if let Some(position) = position_for_db {
            if position.id.is_some() {
                tokio::spawn(async move {
                    if let Err(e) = update_position(&position).await {
                        log(
                            LogTag::Positions,
                            "DB_ERROR",
                            &format!("Failed to update position in database: {}", e)
                        );
                    } else {
                        log(
                            LogTag::Positions,
                            "DB_UPDATE",
                            &format!("Position {} updated in database", position.id.unwrap())
                        );
                    }
                });
            }
        }
    }

    cleanup().await;

    // Log exit transaction with comprehensive verification
    log(
        LogTag::Positions,
        "POSITION_EXIT",
        &format!(
            "📝 Exit transaction {} added to comprehensive verification queue (RPC + transaction analysis)",
            get_signature_prefix(&transaction_signature)
        )
    );

    // Remove token from priority pool service (no longer need fast updates)
    remove_priority_token(mint).await;

    // Immediately attempt to fetch transaction to accelerate verification
    let sig_for_fetch = transaction_signature.clone();
    tokio::spawn(async move {
        let _ = get_transaction(&sig_for_fetch).await;
    });

    // Quick verification attempt (30 seconds timeout)
    let quick_verification_result = tokio::time::timeout(
        Duration::from_secs(30),
        verify_position_transaction(&transaction_signature)
    ).await;

    let position_status = match quick_verification_result {
        Ok(Ok(true)) => {
            // Verification succeeded quickly - position is truly closed
            log(LogTag::Positions, "QUICK_VERIFICATION_SUCCESS", 
                &format!("✅ {} exit verified immediately", symbol));
            "CLOSED"
        },
        _ => {
            // Verification failed or timed out - keep as "closing in progress"
            log(LogTag::Positions, "QUICK_VERIFICATION_PENDING", 
                &format!("⏳ {} exit pending verification (normal - will retry)", symbol));
            "CLOSING"
        }
    };

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ POSITION {}: {} | TX: {} | Reason: {} | Status: {} | Removed from priority pool service",
            position_status,
            symbol,
            get_signature_prefix(&transaction_signature),
            exit_reason,
            position_status
        )
    );

    Ok(transaction_signature)
}

/// Update position tracking data independently
pub async fn update_position_tracking(
    mint: &str,
    current_price: f64,
    price_result: &PriceResult
) -> bool {
    if current_price <= 0.0 || !current_price.is_finite() {
        return false;
    }

    // Use timeout-based lock to avoid blocking tracking updates
    let _lock = match
        tokio::time::timeout(Duration::from_millis(100), acquire_position_lock(mint)).await
    {
        Ok(lock) => lock,
        Err(_) => {
            return false; // Don't block tracking updates
        }
    };

    let mut state = GLOBAL_POSITIONS_STATE.lock().await;

    if let Some(position) = state.positions.iter_mut().find(|p| p.mint == mint) {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

        // Initialize price tracking if not set
        if position.price_highest == 0.0 {
            position.price_highest = entry_price;
            position.price_lowest = entry_price;
        }

        // Track new highs and lows
        if current_price > position.price_highest {
            position.price_highest = current_price;
        }
        if current_price < position.price_lowest {
            position.price_lowest = current_price;
        }

        // Update current price (always, regardless of high/low changes)
        position.current_price = Some(current_price);
        position.current_price_updated = Some(Utc::now());

        // Async database update without blocking
        if position.id.is_some() {
            let position_clone = position.clone();
            tokio::spawn(async move {
                let _ = sync_position_to_database(&position_clone).await;
            });
        }

        // Return true since current_price was updated (always meaningful for tracking)
        true
    } else {
        false
    }
}

/// Verify a position transaction with comprehensive analysis and field population
pub async fn verify_position_transaction(signature: &str) -> Result<bool, String> {
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "VERIFY",
            &format!(
                "🔍 Performing comprehensive verification for transaction {}",
                get_signature_prefix(signature)
            )
        );
    }

    // Get the transaction with comprehensive verification using priority processing
    // This ensures we get a fully analyzed transaction even when the manager is busy
    let transaction = match get_priority_transaction(signature).await {
        Ok(Some(transaction)) => {
            // Check transaction status and success
            match transaction.status {
                TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                    if transaction.success {
                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "VERIFY_SUCCESS",
                                &format!(
                                    "✅ Transaction {} verified successfully: fee={:.6} SOL, sol_change={:.6} SOL",
                                    get_signature_prefix(signature),
                                    transaction.fee_sol,
                                    transaction.sol_balance_change
                                )
                            );
                        }
                        transaction
                    } else {
                        let error_msg = transaction.error_message.unwrap_or(
                            "Unknown error".to_string()
                        );
                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "VERIFY_FAILED",
                                &format!(
                                    "❌ Transaction {} failed on-chain: {}",
                                    get_signature_prefix(signature),
                                    error_msg
                                )
                            );
                        }
                        return Err(format!("Transaction failed on-chain: {}", error_msg));
                    }
                }
                TransactionStatus::Pending => {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "VERIFY_PENDING",
                            &format!(
                                "⏳ Transaction {} still pending verification",
                                get_signature_prefix(signature)
                            )
                        );
                    }
                    return Err("Transaction still pending".to_string());
                }
                TransactionStatus::Failed(error) => {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "VERIFY_FAILED",
                            &format!(
                                "❌ Transaction {} failed: {}",
                                get_signature_prefix(signature),
                                error
                            )
                        );
                    }
                    return Err(format!("Transaction failed: {}", error));
                }
            }
        }
        Ok(None) => {
            // Transaction not found - check verification age
            let verification_age_seconds = {
                let state = GLOBAL_POSITIONS_STATE.lock().await;
                if let Some(added_at) = state.pending_verifications.get(signature) {
                    Utc::now().signed_duration_since(*added_at).num_seconds()
                } else {
                    0
                }
            };

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🔍 Transaction {} not found in system - age: {}s",
                        get_signature_prefix(signature),
                        verification_age_seconds
                    )
                );
            }

            // Extended propagation grace: allow up to 15s for propagation
            if verification_age_seconds <= 15 {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "VERIFY_PENDING",
                        &format!(
                            "⏳ Transaction {} still within propagation grace ({}s <= 15s)",
                            get_signature_prefix(signature),
                            verification_age_seconds
                        )
                    );
                }
                return Err("Transaction within propagation grace".to_string());
            }

            // Check if we've exceeded maximum verification time
            if verification_age_seconds > ENTRY_VERIFICATION_MAX_SECS {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "VERIFY_TIMEOUT",
                        &format!(
                            "⏰ Transaction {} verification timeout ({}s > {}s)",
                            get_signature_prefix(signature),
                            verification_age_seconds,
                            ENTRY_VERIFICATION_MAX_SECS
                        )
                    );
                }
                return Err(format!("Verification timeout: {}s", verification_age_seconds));
            }

            return Err("Transaction not found in system".to_string());
        }
        Err(e) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "VERIFY_ERROR",
                    &format!(
                        "❌ Error getting transaction {}: {}",
                        get_signature_prefix(signature),
                        e
                    )
                );
            }
            return Err(format!("Error getting transaction: {}", e));
        }
    };

    // Get transaction manager for swap analysis (same as backup system)
    let transaction_manager = match get_global_transaction_manager().await {
        Some(manager_guard) => manager_guard,
        None => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "ERROR",
                    "❌ Transaction manager not available for verification"
                );
            }
            return Err("Transaction manager not available".to_string());
        }
    };

    // Perform swap analysis using transaction manager (BACKUP SYSTEM APPROACH)
    // Use convert_to_swap_pnl_info directly without requiring swap_analysis field
    let swap_pnl_info = {
        let manager = transaction_manager.lock().await;
        if let Some(ref manager) = *manager {
            let empty_cache = std::collections::HashMap::new();
            // Use the same method as backup system - works with transaction_type and balance changes
            let swap_info = manager.convert_to_swap_pnl_info(&transaction, &empty_cache, false);

            if is_debug_positions_enabled() {
                if let Some(ref info) = swap_info {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "🔍 Swap analysis result: type={}, token_mint={}, sol_amount={}, token_amount={}, price={:.9}",
                            info.swap_type,
                            get_mint_prefix(&info.token_mint),
                            info.sol_amount,
                            info.token_amount,
                            info.calculated_price_sol
                        )
                    );
                } else {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "⚠️ No swap analysis result for transaction {}",
                            get_signature_prefix(signature)
                        )
                    );
                }
            }

            swap_info
        } else {
            if is_debug_positions_enabled() {
                log(LogTag::Positions, "ERROR", "❌ Transaction manager not initialized");
            }
            return Err("Transaction manager not initialized".to_string());
        }
    };

    // Update position verification status and populate fields from transaction data
    let mut verified = false;
    let mut position_for_db_update: Option<Position> = None;
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔍 Starting position search for verification - {} positions in memory",
                    state.positions.len()
                )
            );
        }

        // Find and update the position
        for position in &mut state.positions {
            let is_entry =
                position.entry_transaction_signature.as_ref().map(|s| s.as_str()) ==
                Some(signature);
            let is_exit =
                position.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature);

            log(
                LogTag::Positions,
                "VERIFY_CHECK",
                &format!(
                    "🔍 Checking position {} (ID: {}): entry_sig={}, exit_sig={}, is_entry={}, is_exit={}",
                    position.symbol,
                    position.id.unwrap_or(0),
                    position.entry_transaction_signature
                        .as_ref()
                        .map(|s| get_signature_prefix(s))
                        .unwrap_or("None".to_string()),
                    position.exit_transaction_signature
                        .as_ref()
                        .map(|s| get_signature_prefix(s))
                        .unwrap_or("None".to_string()),
                    is_entry,
                    is_exit
                )
            );

            if is_entry {
                // Entry transaction verification
                if let Some(ref swap_info) = swap_pnl_info {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "POSITION_ENTRY_SWAP_INFO",
                            &format!(
                                "📊 Entry swap info for {}: type={}, token_mint={}, sol_amount={}, token_amount={}, price={:.9}",
                                position.symbol,
                                swap_info.swap_type,
                                safe_truncate(&swap_info.token_mint, 8),
                                swap_info.sol_amount,
                                swap_info.token_amount,
                                swap_info.calculated_price_sol
                            )
                        );
                    }

                    if swap_info.swap_type == "Buy" && swap_info.token_mint == position.mint {
                        // Update position with actual transaction data
                        position.transaction_entry_verified = true;

                        // Calculate effective entry price using effective SOL spent (excludes ATA rent)
                        let effective_price = if
                            swap_info.token_amount.abs() > 0.0 &&
                            swap_info.effective_sol_spent > 0.0
                        {
                            swap_info.effective_sol_spent / swap_info.token_amount.abs()
                        } else {
                            swap_info.calculated_price_sol // Fallback to regular price
                        };

                        position.effective_entry_price = Some(effective_price);
                        position.total_size_sol = swap_info.sol_amount;

                        // Convert token amount from float to units (with decimals)
                        if let Some(token_decimals) = get_token_decimals(&position.mint).await {
                            let token_amount_units = (swap_info.token_amount.abs() *
                                (10_f64).powi(token_decimals as i32)) as u64;
                            position.token_amount = Some(token_amount_units);

                            if is_debug_positions_enabled() {
                                log(
                                    LogTag::Positions,
                                    "POSITION_ENTRY_TOKEN_AMOUNT",
                                    &format!(
                                        "🔢 Token amount for {}: {} tokens ({} units with {} decimals)",
                                        position.symbol,
                                        swap_info.token_amount,
                                        token_amount_units,
                                        token_decimals
                                    )
                                );
                            }
                        }

                        // Convert fee from SOL to lamports
                        position.entry_fee_lamports = Some(sol_to_lamports(swap_info.fee_sol));

                        verified = true;

                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "POSITION_ENTRY_VERIFIED",
                                &format!(
                                    "✅ Entry transaction verified for {}: price={:.9} SOL, effective_price={:.9} SOL",
                                    position.symbol,
                                    swap_info.calculated_price_sol,
                                    effective_price
                                )
                            );
                        }

                        // Store position for database update (after releasing lock)
                        if position.id.is_some() {
                            position_for_db_update = Some(position.clone());
                            log(
                                LogTag::Positions,
                                "DB_PREP",
                                &format!(
                                    "🔄 Position {} (ID: {}) prepared for database update with entry_verified={}",
                                    position.symbol,
                                    position.id.unwrap(),
                                    position.transaction_entry_verified
                                )
                            );
                        } else {
                            log(
                                LogTag::Positions,
                                "DB_PREP_ERROR",
                                &format!(
                                    "⚠️ Position {} has no ID - cannot update database",
                                    position.symbol
                                )
                            );
                        }
                    } else {
                        // Type/token mismatch (same as backup system)
                        position.transaction_entry_verified = false;
                        log(
                            LogTag::Positions,
                            "POSITION_ENTRY_MISMATCH",
                            &format!(
                                "⚠️ Entry transaction {} type/token mismatch for position {}: expected Buy {}, got {} {} - PENDING TRANSACTION SHOULD BE REMOVED",
                                get_signature_prefix(signature),
                                position.symbol,
                                get_mint_prefix(&position.mint),
                                swap_info.swap_type,
                                get_mint_prefix(&swap_info.token_mint)
                            )
                        );
                        return Err("Transaction type/token mismatch".to_string());
                    }
                } else {
                    // No swap analysis available (same handling as backup system)
                    log(
                        LogTag::Positions,
                        "POSITION_ENTRY_NO_SWAP",
                        &format!(
                            "⚠️ Entry transaction {} has no valid swap analysis for position {} - will retry on next verification cycle",
                            get_signature_prefix(signature),
                            position.symbol
                        )
                    );
                    // Don't mark as failed - let it retry (same as backup)
                    return Err("No valid swap analysis - will retry".to_string());
                }
                break;
            } else if is_exit {
                // Exit transaction verification
                if let Some(ref swap_info) = swap_pnl_info {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "POSITION_EXIT_SWAP_INFO",
                            &format!(
                                "📊 Exit swap info for {}: type={}, token_mint={}, sol_amount={}, token_amount={}, price={:.9}",
                                position.symbol,
                                swap_info.swap_type,
                                safe_truncate(&swap_info.token_mint, 8),
                                swap_info.sol_amount,
                                swap_info.token_amount,
                                swap_info.calculated_price_sol
                            )
                        );
                    }

                    if swap_info.swap_type == "Sell" && swap_info.token_mint == position.mint {
                        // Update position with actual exit transaction data
                        position.transaction_exit_verified = true;

                        // Use actual SOL received from swap analysis (for sells, use effective_sol_received)
                        position.sol_received = Some(swap_info.effective_sol_received.abs()); // For sell, this is SOL received
                        position.effective_exit_price = Some(swap_info.calculated_price_sol);
                        
                        // CRITICAL FIX: Set exit_time and exit_price when exit transaction is verified
                        // Use accurate blockchain time if available, fallback to current time
                        let exit_time = if let Some(block_time) = transaction.block_time {
                            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
                        } else {
                            Utc::now()
                        };
                        position.exit_time = Some(exit_time);
                        position.exit_price = Some(swap_info.calculated_price_sol);

                        // Convert fee from SOL to lamports
                        position.exit_fee_lamports = Some(sol_to_lamports(swap_info.fee_sol));

                        verified = true;

                        log(
                            LogTag::Positions,
                            "POSITION_EXIT_VERIFIED",
                            &format!(
                                "✅ Exit transaction verified for {}: price={:.9} SOL, sol_received={:.6} SOL, exit_time={} - POSITION NOW CLOSED",
                                position.symbol,
                                swap_info.calculated_price_sol,
                                swap_info.effective_sol_received.abs(),
                                exit_time.format("%H:%M:%S")
                            )
                        );

                        // Store position for database update (after releasing lock)
                        if position.id.is_some() {
                            position_for_db_update = Some(position.clone());
                            if is_debug_positions_enabled() {
                                log(
                                    LogTag::Positions,
                                    "DEBUG",
                                    &format!(
                                        "🔄 Exit position {} (ID: {}) prepared for database update",
                                        position.symbol,
                                        position.id.unwrap()
                                    )
                                );
                            }
                        } else {
                            if is_debug_positions_enabled() {
                                log(
                                    LogTag::Positions,
                                    "WARNING",
                                    &format!(
                                        "⚠️ Exit position {} has no ID - cannot update database",
                                        position.symbol
                                    )
                                );
                            }
                        }
                    } else {
                        // Type/token mismatch (same as backup system)
                        position.transaction_exit_verified = false;
                        log(
                            LogTag::Positions,
                            "POSITION_EXIT_MISMATCH",
                            &format!(
                                "⚠️ Exit transaction {} type/token mismatch for position {}: expected Sell {}, got {} {} - PENDING TRANSACTION SHOULD BE REMOVED",
                                get_signature_prefix(signature),
                                position.symbol,
                                get_mint_prefix(&position.mint),
                                swap_info.swap_type,
                                get_mint_prefix(&swap_info.token_mint)
                            )
                        );
                        return Err("Transaction type/token mismatch".to_string());
                    }
                } else {
                    // No swap analysis available (same handling as backup system)
                    log(
                        LogTag::Positions,
                        "POSITION_EXIT_NO_SWAP",
                        &format!(
                            "⚠️ Exit transaction {} has no valid swap analysis for position {} - will retry on next verification cycle",
                            get_signature_prefix(signature),
                            position.symbol
                        )
                    );
                    // Don't mark as failed - let it retry (same as backup)
                    return Err("No valid swap analysis - will retry".to_string());
                }
                break;
            }
        }

        // NOTE: Do NOT remove from pending verifications yet - only after successful database update

        log(
            LogTag::Positions,
            "VERIFY_RESULT",
            &format!(
                "� Position search completed for {}: verified={}, position_for_db_update={}, positions_checked={}",
                get_signature_prefix(signature),
                verified,
                position_for_db_update.is_some(),
                state.positions.len()
            )
        );
    }

    // Update database AFTER releasing the state lock to prevent deadlock/contention
    if let Some(position) = position_for_db_update {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔄 Attempting database update for position {} (ID: {}) - entry_verified={}, exit_verified={}",
                    position.symbol,
                    position.id.unwrap_or(0),
                    position.transaction_entry_verified,
                    position.transaction_exit_verified
                )
            );
        }

        // Log the update_position call explicitly
        log(
            LogTag::Positions,
            "DB_UPDATE_CALL",
            &format!(
                "📝 Calling update_position for {} with verification status: entry={}, exit={}",
                position.symbol,
                position.transaction_entry_verified,
                position.transaction_exit_verified
            )
        );

        match update_position(&position).await {
            Err(e) => {
                log(
                    LogTag::Positions,
                    "DB_ERROR",
                    &format!(
                        "❌ Failed to update verification in database for {}: {}",
                        position.symbol,
                        e
                    )
                );
                // Return error to prevent marking as verified if database update failed
                return Err(format!("Database update failed: {}", e));
            }
            Ok(_) => {
                log(
                    LogTag::Positions,
                    "DB_UPDATE_SUCCESS",
                    &format!(
                        "✅ Verification status saved to database for {} - entry_verified={}, exit_verified={}",
                        position.symbol,
                        position.transaction_entry_verified,
                        position.transaction_exit_verified
                    )
                );

                // Save closing token snapshot if this is an exit transaction verification
                if position.transaction_exit_verified && position.id.is_some() {
                    let position_id = position.id.unwrap();
                    let mint_clone = position.mint.clone();
                    tokio::spawn(async move {
                        if let Err(e) = save_position_token_snapshot(position_id, &mint_clone, "closing").await {
                            log(
                                LogTag::Positions,
                                "SNAPSHOT_WARN",
                                &format!("Failed to save closing snapshot for {}: {}", safe_truncate(&mint_clone, 8), e)
                            );
                        }
                    });
                }

                // Only remove from pending verifications AFTER successful database update
                {
                    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                    state.pending_verifications.remove(signature);

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "🗑️ Removed {} from pending verifications after successful DB update",
                                get_signature_prefix(signature)
                            )
                        );
                    }
                }
            }
        }
    } else {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("⚠️ No position prepared for database update - verified={}, will not update DB", verified)
            );
        }
    }

    if verified {
        // Reload positions from database to ensure perfect sync after verification update
        if let Err(e) = reload_positions_from_database().await {
            log(
                LogTag::Positions,
                "WARNING",
                &format!("Failed to reload positions after verification: {}", e)
            );
        } else {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "SYNC_SUCCESS",
                    &format!(
                        "✅ Positions reloaded from database after verification of {}",
                        get_signature_prefix(signature)
                    )
                );
            }
        }

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "SUCCESS",
                &format!(
                    "✅ Comprehensive verification completed for transaction {}",
                    get_signature_prefix(signature)
                )
            );
        }
        Ok(true)
    } else {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "WARNING",
                &format!(
                    "⚠️ No matching position found for transaction {}",
                    get_signature_prefix(signature)
                )
            );
        }
        Err("No matching position found for transaction".to_string())
    }
}

// ==================== POSITION QUERIES ====================

/// Get all open positions
pub async fn get_open_positions() -> Vec<Position> {
    // Try database first, fallback to memory
    match db_get_open_positions().await {
        Ok(positions) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DB_QUERY",
                    &format!("Retrieved {} open positions from database", positions.len())
                );
            }
            positions
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "DB_FALLBACK",
                &format!("Database query failed, using memory: {}", e)
            );
            let state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions
                .iter()
                .filter(|p| {
                    p.position_type == "buy" && 
                    p.exit_price.is_none() && 
                    // Include positions with unverified exit transactions as "open" (closing in progress)
                    (!p.exit_transaction_signature.is_some() || !p.transaction_exit_verified)
                })
                .cloned()
                .collect()
        }
    }
}

/// Get all closed positions
pub async fn get_closed_positions() -> Vec<Position> {
    // Try database first, fallback to memory
    match db_get_closed_positions().await {
        Ok(positions) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DB_QUERY",
                    &format!("Retrieved {} closed positions from database", positions.len())
                );
            }
            positions
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "DB_FALLBACK",
                &format!("Database query failed, using memory: {}", e)
            );
            let state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions
                .iter()
                .filter(|p| {
                    // Only truly closed positions (with verified exit or explicit exit_price)
                    p.exit_price.is_some() && 
                    (p.transaction_exit_verified || p.exit_transaction_signature.is_none())
                })
                .cloned()
                .collect()
        }
    }
}

/// Get count of open positions
pub async fn get_open_positions_count() -> usize {
    // Get open positions and count them
    get_open_positions().await.len()
}

/// Check if a position is open for given mint
pub async fn is_open_position(mint: &str) -> bool {
    // Try database first, fallback to memory
    match db_get_position_by_mint(mint).await {
        Ok(Some(position)) => position.exit_time.is_none(),
        Ok(None) => false,
        Err(_) => {
            // Fallback to memory
            let state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions
                .iter()
                .any(|p| {
                    p.mint == mint && 
                    p.position_type == "buy" && 
                    p.exit_time.is_none() &&
                    // Include positions that are closing but not yet verified
                    (!p.exit_transaction_signature.is_some() || !p.transaction_exit_verified)
                })
        }
    }
}

/// Get list of open position mints
pub async fn get_open_mints() -> Vec<String> {
    // Get open positions and extract their mints
    get_open_positions().await
        .iter()
        .map(|p| p.mint.clone())
        .collect()
}

/// Get active frozen cooldowns
pub async fn get_active_frozen_cooldowns() -> Vec<(String, i64)> {
    let state = GLOBAL_POSITIONS_STATE.lock().await;
    let now = Utc::now();
    state.frozen_cooldowns
        .iter()
        .filter_map(|(mint, cooldown_until)| {
            if *cooldown_until > now {
                let remaining_minutes = (*cooldown_until - now).num_minutes();
                Some((mint.clone(), remaining_minutes))
            } else {
                None
            }
        })
        .collect()
}

// ==================== P&L CALCULATION ====================

/// Unified profit/loss calculation for both open and closed positions
/// Uses effective prices and actual token amounts when available
/// For closed positions with sol_received, uses actual SOL invested vs SOL received
/// NOTE: sol_received should contain ONLY the SOL from token sale, excluding ATA rent reclaim
pub async fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    // Safety check: validate position has valid entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("❌ Invalid entry price for {}: {}", position.symbol, entry_price)
            );
        }
        // Invalid entry price - return neutral P&L to avoid triggering emergency exits
        return (0.0, 0.0);
    }

    // For open positions, validate current price if provided
    if let Some(current) = current_price {
        if current <= 0.0 || !current.is_finite() {
            // Invalid current price - return neutral P&L to avoid false emergency signals
            return (0.0, 0.0);
        }
    }

    // For positions with pending exit transactions (closing in progress), use current price for estimation
    if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
        if let Some(current) = current_price {
            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            let entry_cost = position.entry_size_sol;
            
            // Calculate estimated P&L based on current price (closing in progress)
            if let Some(token_amount) = position.token_amount {
                let token_decimals_opt = get_token_decimals(&position.mint).await;
                if let Some(token_decimals) = token_decimals_opt {
                    let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
                    let current_value = ui_token_amount * current;
                    
                    // Account for fees (estimated)
                    let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
                    let estimated_sell_fee = buy_fee;
                    let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL;
                    let net_pnl_sol = current_value - entry_cost - total_fees;
                    let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;
                    
                    return (net_pnl_sol, net_pnl_percent);
                }
            }
            
            // Fallback calculation for closing positions
            let price_change = (current - entry_price) / entry_price;
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee;
            let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL;
            let fee_percent = (total_fees / entry_cost) * 100.0;
            let net_pnl_percent = price_change * 100.0 - fee_percent;
            let net_pnl_sol = (net_pnl_percent / 100.0) * entry_cost;
            
            return (net_pnl_sol, net_pnl_percent);
        }
    }
    
    // For closed positions, prioritize sol_received for most accurate P&L
    if let (Some(exit_price), Some(sol_received)) = (position.exit_price, position.sol_received) {
        // Use actual SOL invested vs SOL received for closed positions
        let sol_invested = position.entry_size_sol;

        // Use actual transaction fees plus profit buffer for P&L calculation
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer in P&L calculation

        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let safe_invested = if sol_invested < 0.00001 { 0.00001 } else { sol_invested };
        let net_pnl_percent = (net_pnl_sol / safe_invested) * 100.0;

        return (net_pnl_sol, net_pnl_percent);
    }

    // Fallback for closed positions without sol_received (backward compatibility)
    if let Some(exit_price) = position.exit_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let effective_exit = position.effective_exit_price.unwrap_or(exit_price);

        // For closed positions: actual transaction-based calculation
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let entry_cost = position.entry_size_sol;
            let exit_value = ui_token_amount * effective_exit;

            // Account for actual buy + sell fees plus profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // For open positions, use current price
    if let Some(current) = current_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

        // For open positions: current value vs entry cost
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let current_value = ui_token_amount * current;
            let entry_cost = position.entry_size_sol;

            // Account for actual buy fee (already paid) + estimated sell fee + profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
            let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = current_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
        let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // No price available
    (0.0, 0.0)
}

/// Calculate total fees for a position
pub fn calculate_position_total_fees(position: &Position) -> f64 {
    // Sum entry and exit fees in SOL (excluding ATA rent from trading costs)
    let entry_fees_sol = (position.entry_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    let exit_fees_sol = (position.exit_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    entry_fees_sol + exit_fees_sol
}

// ==================== BACKGROUND TASKS ====================

/// Start background position management tasks
pub async fn run_background_position_tasks(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🚀 Starting background position tasks");

    // Spawn independent background tasks
    let shutdown_clone1 = shutdown.clone();
    let shutdown_clone2 = shutdown.clone();
    let shutdown_clone3 = shutdown.clone();
    let shutdown_clone4 = shutdown.clone();

    // Task 1: Verify pending transactions in parallel
    tokio::spawn(async move {
        verify_pending_transactions_parallel(shutdown_clone1).await;
    });

    // Task 2: Cleanup phantom positions in parallel
    tokio::spawn(async move {
        cleanup_phantom_positions_parallel(shutdown_clone2).await;
    });

    // Task 3: Retry failed operations in parallel
    tokio::spawn(async move {
        retry_failed_operations_parallel(shutdown_clone3).await;
    });

    // Task 4: Process failed exit retries in parallel
    tokio::spawn(async move {
        process_failed_exit_retries_parallel(shutdown_clone4).await;
    });
}

/// Verify pending transactions with parallel processing
async fn verify_pending_transactions_parallel(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🔍 Starting parallel transaction verification task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping transaction verification task");
                break;
            }
            _ = sleep(Duration::from_secs(30)) => {
                // First, cleanup stale pending verifications
                let now = Utc::now();
                let stale_sigs: Vec<String> = {
                    let state = GLOBAL_POSITIONS_STATE.lock().await;
                    state.pending_verifications
                        .iter()
                        .filter_map(|(sig, added_at)| {
                            let age_seconds = now.signed_duration_since(*added_at).num_seconds();
                            if age_seconds > ENTRY_VERIFICATION_MAX_SECS * 2 { // 180 seconds = 3 minutes
                                Some(sig.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                if !stale_sigs.is_empty() {
                    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                    for sig in &stale_sigs {
                        state.pending_verifications.remove(sig);
                    }
                    log(
                        LogTag::Positions,
                        "CLEANUP",
                        &format!("🧹 Cleaned up {} stale pending verifications (age > {}s)", stale_sigs.len(), ENTRY_VERIFICATION_MAX_SECS * 2)
                    );
                }

                // Get batch of pending verifications
                let pending_sigs: Vec<String> = {
                    let state = GLOBAL_POSITIONS_STATE.lock().await;
                    state.pending_verifications.keys().cloned().collect()
                };

                if !pending_sigs.is_empty() {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🔍 Processing {} pending verifications", pending_sigs.len())
                        );
                    }

                    // Process verifications in batches
                    for batch in pending_sigs.chunks(VERIFICATION_BATCH_SIZE) {
                        let batch_futures: Vec<_> = batch.iter().map(|sig| {
                            let sig_clone = sig.clone();
                            async move {
                                match verify_position_transaction(&sig_clone).await {
                                    Ok(verified) => {
                                        if verified {
                                            if is_debug_positions_enabled() {
                                                log(
                                                    LogTag::Positions,
                                                    "VERIFICATION_SUCCESS",
                                                    &format!("✅ Transaction {} verified", get_signature_prefix(&sig_clone))
                                                );
                                            }
                                            Some(sig_clone)
                                        } else {
                                            None
                                        }
                                    }
                                    Err(e) => {
                                        // Check for permanent failures that should be cleaned up immediately
                                        
                                        let should_cleanup_immediately = if e.contains("[PERMANENT]") {
                                            // Error already contains permanent failure indicator
                                            true
                                        } else {
                                            // Try to parse error for permanent failure detection
                                            if let Ok(Some(transaction)) = crate::transactions::get_transaction(&sig_clone).await {
                                                if let Some(error_msg) = &transaction.error_message {
                                                    error_msg.contains("[PERMANENT]")
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        };
                                        
                                        if should_cleanup_immediately {
                                            log(
                                                LogTag::Positions,
                                                "PERMANENT_FAILURE_CLEANUP",
                                                &format!("🗑️ Immediately removing position with permanent failure: {} (error: {})", get_signature_prefix(&sig_clone), e)
                                            );
                                            
                                            // Remove the position with the permanently failed transaction
                                            tokio::spawn({
                                                let sig_for_cleanup = sig_clone.clone();
                                                async move {
                                                    if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                        log(
                                                            LogTag::Positions,
                                                            "CLEANUP_ERROR",
                                                            &format!("Failed to remove position with signature {}: {}", get_signature_prefix(&sig_for_cleanup), cleanup_err)
                                                        );
                                                    }
                                                }
                                            });
                                            
                                            // Return the signature to remove it from pending verifications
                                            Some(sig_clone)
                                        } else {
                                            // Check verification age before removing position
                                            let verification_age_seconds = {
                                                let state = GLOBAL_POSITIONS_STATE.lock().await;
                                                if let Some(added_at) = state.pending_verifications.get(&sig_clone) {
                                                    now.signed_duration_since(*added_at).num_seconds()
                                                } else {
                                                    0 // If not found, treat as new
                                                }
                                            };
                                            
                                            // Progressive timeout handling - different timeouts for different situations
                                            let is_exit_transaction = {
                                                let state = GLOBAL_POSITIONS_STATE.lock().await;
                                                state.positions.iter().any(|p| {
                                                    p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)
                                                })
                                            };
                                            
                                            let timeout_threshold = if is_exit_transaction {
                                                60 // 1 minute for exit transactions
                                            } else {
                                                90 // 1.5 minutes for entry transactions  
                                            };
                                            
                                            // Only remove if truly timed out (beyond progressive grace period)
                                            if e.contains("Verification timeout:") || e.contains("verification timeout") || 
                                               (e.contains("Transaction not found in system") && verification_age_seconds > timeout_threshold) {
                                                
                                                // Use the is_exit_transaction we already determined above
                                                
                                                if is_exit_transaction {
                                                    // For exit transaction failures, check wallet balance before removing position
                                                    log(
                                                        LogTag::Positions,
                                                        "EXIT_VERIFICATION_TIMEOUT",
                                                        &format!("⏰ Exit transaction {} verification timeout - checking wallet balance before cleanup", get_signature_prefix(&sig_clone))
                                                    );
                                                    
                                                    // Find the position and check wallet balance
                                                    let should_remove_position = {
                                                        let state = GLOBAL_POSITIONS_STATE.lock().await;
                                                        if let Some(position) = state.positions.iter().find(|p| {
                                                            p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)
                                                        }) {
                                                            // Check wallet balance for this token
                                                            match get_wallet_address() {
                                                                Ok(wallet_address) => {
                                                                    match get_token_balance(&wallet_address, &position.mint).await {
                                                                        Ok(balance) => {
                                                                            log(
                                                                                LogTag::Positions,
                                                                                "WALLET_BALANCE_CHECK",
                                                                                &format!("💰 Wallet balance for {}: {} tokens", position.symbol, balance)
                                                                            );
                                                                            
                                                                            if balance > 0 {
                                                                                log(
                                                                                    LogTag::Positions,
                                                                                    "POSITION_KEPT_WITH_TOKENS",
                                                                                    &format!("✅ Keeping position {} - tokens still in wallet ({})", position.symbol, balance)
                                                                                );
                                                                                false // Keep position - tokens still in wallet
                                                                            } else {
                                                                                log(
                                                                                    LogTag::Positions,
                                                                                    "POSITION_REMOVED_ZERO_BALANCE",
                                                                                    &format!("🗑️ Removing position {} - zero balance confirmed", position.symbol)
                                                                                );
                                                                                true // Remove position - no tokens in wallet
                                                                            }
                                                                        }
                                                                        Err(err) => {
                                                                            log(
                                                                                LogTag::Positions,
                                                                                "BALANCE_CHECK_ERROR",
                                                                                &format!("❌ Could not check balance for {}: {} - keeping position to be safe", position.symbol, err)
                                                                            );
                                                                            false // Keep position if balance check fails
                                                                        }
                                                                    }
                                                                }
                                                                Err(err) => {
                                                                    log(
                                                                        LogTag::Positions,
                                                                        "WALLET_ADDRESS_ERROR",
                                                                        &format!("❌ Could not get wallet address: {} - keeping position to be safe", err)
                                                                    );
                                                                    false // Keep position if wallet address fails
                                                                }
                                                            }
                                                        } else {
                                                            true // Position not found, safe to remove
                                                        }
                                                    };
                                                    
                                                    if should_remove_position {
                                                        log(
                                                            LogTag::Positions,
                                                            "VERIFICATION_TIMEOUT_CLEANUP",
                                                            &format!("🗑️ Removing position with failed exit verification: {} (error: {}, age: {}s)", get_signature_prefix(&sig_clone), e, verification_age_seconds)
                                                        );
                                                        
                                                        tokio::spawn({
                                                            let sig_for_cleanup = sig_clone.clone();
                                                            async move {
                                                                if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                                    log(
                                                                        LogTag::Positions,
                                                                        "CLEANUP_ERROR",
                                                                        &format!("Failed to remove position with signature {}: {}", get_signature_prefix(&sig_for_cleanup), cleanup_err)
                                                                    );
                                                                }
                                                            }
                                                        });
                                                        
                                                        // Return the signature to remove it from pending verifications
                                                        Some(sig_clone)
                                                    } else {
                                                        // Don't remove from pending - keep trying or mark exit as failed
                                                        log(
                                                            LogTag::Positions,
                                                            "EXIT_VERIFICATION_KEPT",
                                                            &format!("🔄 Keeping position with failed exit verification: {} - tokens still in wallet", get_signature_prefix(&sig_clone))
                                                        );
                                                        
                                                                                                // Mark the exit transaction as failed but keep the position and schedule retry
                                        {
                                            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                                            if let Some(position) = state.positions.iter_mut().find(|p| {
                                                p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)
                                            }) {
                                                position.exit_transaction_signature = None; // Clear failed exit transaction
                                                position.closed_reason = Some("exit_retry_pending".to_string()); // Better status tracking
                                                log(
                                                    LogTag::Positions,
                                                    "EXIT_RETRY_SCHEDULED",
                                                    &format!("🔄 {} exit failed, retry scheduled", position.symbol)
                                                );
                                                                
                                                                // Schedule failed exit retry
                                                                let mint_for_retry = position.mint.clone();
                                                                tokio::spawn(async move {
                                                                    schedule_failed_exit_retry(&mint_for_retry, 0).await;
                                                                });
                                                            }
                                                        }
                                                        
                                                        Some(sig_clone) // Remove from pending verifications
                                                    }
                                                } else {
                                                    // For entry transaction failures, always remove the position
                                                    log(
                                                        LogTag::Positions,
                                                        "VERIFICATION_TIMEOUT_CLEANUP",
                                                        &format!("🗑️ Removing position with failed entry verification: {} (error: {}, age: {}s)", get_signature_prefix(&sig_clone), e, verification_age_seconds)
                                                    );
                                                    
                                                    tokio::spawn({
                                                        let sig_for_cleanup = sig_clone.clone();
                                                        async move {
                                                            if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                                log(
                                                                    LogTag::Positions,
                                                                    "CLEANUP_ERROR",
                                                                    &format!("Failed to remove position with signature {}: {}", get_signature_prefix(&sig_for_cleanup), cleanup_err)
                                                                );
                                                            }
                                                        }
                                                    });
                                                    
                                                    Some(sig_clone)
                                                }
                                            } else {
                                                log(
                                                    LogTag::Positions,
                                                    "VERIFICATION_ERROR",
                                                    &format!("❌ Failed to verify {}: {}", get_signature_prefix(&sig_clone), e)
                                                );
                                                None
                                            }
                                        }
                                    }
                                }
                            }
                        }).collect();

                        // Process verification batch in parallel
                        let results = futures::future::join_all(batch_futures).await;
                        
                        // Remove completed verifications from pending list
                        let completed_sigs: Vec<String> = results.into_iter().filter_map(|r| r).collect();
                        if !completed_sigs.is_empty() {
                            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                            for sig in completed_sigs {
                                state.pending_verifications.remove(&sig);
                            }
                        }
                    }
                } else {
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🔍 No pending verifications to process");
                    }
                }
            }
        }
    }
}

/// Cleanup phantom positions with parallel processing
async fn cleanup_phantom_positions_parallel(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🧹 Starting parallel phantom cleanup task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping phantom cleanup task");
                break;
            }
            _ = sleep(Duration::from_secs(60)) => {
                // Find potential phantom positions (open positions with zero wallet balance)
                let wallet_address = match get_wallet_address() {
                    Ok(addr) => addr,
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!("Failed to get wallet address for phantom cleanup: {}", e)
                        );
                        continue;
                    }
                };

                let open_positions = get_open_positions().await;
                let mut phantom_candidates = Vec::new();

                for position in open_positions {
                    // Skip very recent positions (less than 5 minutes old)
                    let age_minutes = (Utc::now() - position.entry_time).num_minutes();
                    if age_minutes < PHANTOM_TIMEOUT_MINUTES {
                        continue;
                    }

                    // Check token balance for this position
                    match get_token_balance(&wallet_address, &position.mint).await {
                        Ok(balance) => {
                            if balance == 0 {
                                phantom_candidates.push((position, age_minutes));
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Positions,
                                "WARN",
                                &format!("Failed to check balance for {}: {}", position.symbol, e)
                            );
                        }
                    }
                }

                if !phantom_candidates.is_empty() {
                    log(
                        LogTag::Positions,
                        "PHANTOM_DETECTION",
                        &format!("🔍 Found {} potential phantom positions", phantom_candidates.len())
                    );

                    // Process phantom candidates in batches
                    for batch in phantom_candidates.chunks(CLEANUP_BATCH_SIZE) {
                        let cleanup_futures: Vec<_> = batch.iter().map(|(position, age_minutes)| {
                            let pos = position.clone();
                            let age = *age_minutes;
                            async move {
                                // Check if entry transaction actually failed using priority processing
                                if let Some(entry_sig) = &pos.entry_transaction_signature {
                                    match get_priority_transaction(entry_sig).await {
                                        Ok(transaction) => {
                                            if let Some(tx) = transaction {
                                                if !tx.success || !matches!(tx.status, TransactionStatus::Confirmed | TransactionStatus::Finalized) {
                                                    log(
                                                        LogTag::Positions,
                                                        "PHANTOM_CLEANUP",
                                                        &format!(
                                                            "🧹 Removing phantom position {} - entry transaction failed or unconfirmed",
                                                            pos.symbol
                                                        )
                                                    );
                                                    return Some(pos.mint.clone());
                                                }
                                            } else {
                                                log(
                                                    LogTag::Positions,
                                                    "PHANTOM_CLEANUP",
                                                    &format!(
                                                        "🧹 Removing phantom position {} - entry transaction not found",
                                                        pos.symbol
                                                    )
                                                );
                                                return Some(pos.mint.clone());
                                            }
                                        }
                                        Err(_) => {
                                            // Transaction not found after timeout - likely failed
                                            if age > 10 {
                                                log(
                                                    LogTag::Positions,
                                                    "PHANTOM_CLEANUP",
                                                    &format!(
                                                        "🧹 Removing phantom position {} - transaction not found after {}min",
                                                        pos.symbol,
                                                        age
                                                    )
                                                );
                                                return Some(pos.mint.clone());
                                            }
                                        }
                                    }
                                }
                                None
                            }
                        }).collect();

                        // Process cleanup batch in parallel
                        let results = futures::future::join_all(cleanup_futures).await;
                        
                        // Remove phantom positions
                        let phantom_mints: Vec<String> = results.into_iter().filter_map(|r| r).collect();
                        if !phantom_mints.is_empty() {
                            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                            let original_count = state.positions.len();
                            state.positions.retain(|p| !phantom_mints.contains(&p.mint));
                            let removed_count = original_count - state.positions.len();
                            
                            if removed_count > 0 {
                                log(
                                    LogTag::Positions,
                                    "PHANTOM_CLEANUP",
                                    &format!("🧹 Removed {} phantom positions", removed_count)
                                );
                            }
                        }
                    }
                } else {
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🧹 No phantom positions detected");
                    }
                }
            }
        }
    }
}

/// Retry failed operations with parallel processing
async fn retry_failed_operations_parallel(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🔄 Starting parallel retry task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping retry task");
                break;
            }
            _ = sleep(Duration::from_secs(120)) => {
                // Get operations ready for retry
                let retry_candidates: Vec<(String, u32)> = {
                    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                    let now = Utc::now();
                    let mut candidates = Vec::new();

                    // Find retry candidates that are ready (past their retry time)
                    let ready_mints: Vec<String> = state.retry_queue
                        .iter()
                        .filter_map(|(mint, (next_retry, attempt_count))| {
                            if now >= *next_retry && *attempt_count < MAX_RETRY_ATTEMPTS {
                                Some((mint.clone(), *attempt_count))
                            } else {
                                None
                            }
                        })
                        .map(|(mint, count)| {
                            candidates.push((mint.clone(), count));
                            mint
                        })
                        .collect();

                    // Remove ready candidates from retry queue (they'll be re-added if they fail again)
                    for mint in ready_mints {
                        state.retry_queue.remove(&mint);
                    }

                    candidates
                };

                if !retry_candidates.is_empty() {
                    log(
                        LogTag::Positions,
                        "RETRY_OPERATIONS",
                        &format!("🔄 Processing {} retry operations", retry_candidates.len())
                    );

                    // Process retries in batches
                    for batch in retry_candidates.chunks(5) { // Smaller batches for retries
                        let retry_futures: Vec<_> = batch.iter().map(|(mint, attempt_count)| {
                            let mint_clone = mint.clone();
                            let attempts = *attempt_count;
                            async move {
                                // Try to close the position again
                                if let Some(position) = {
                                    let state = GLOBAL_POSITIONS_STATE.lock().await;
                                    state.positions.iter().find(|p| p.mint == mint_clone && p.exit_price.is_none()).cloned()
                                } {
                                    log(
                                        LogTag::Positions,
                                        "RETRY_SELL",
                                        &format!(
                                            "🔄 Retrying sell for {} (attempt {}/{})",
                                            position.symbol,
                                            attempts + 1,
                                            MAX_RETRY_ATTEMPTS
                                        )
                                    );

                                    // Get current price for exit
                                    let current_price = get_price(&mint_clone, Some(PriceOptions::simple()), false)
                                        .await
                                        .and_then(|r| r.best_sol_price())
                                        .unwrap_or(position.entry_price);
                                    
                                    // Create token object for retry
                                    let token = Token {
                                        mint: mint_clone.clone(),
                                        symbol: position.symbol.clone(),
                                        name: position.name.clone(),
                                        chain: "solana".to_string(),
                                        logo_url: None,
                                        coingecko_id: None,
                                        website: None,
                                        description: None,
                                        tags: vec![],
                                        is_verified: false,
                                        created_at: None,
                                        price_dexscreener_sol: None,
                                        price_dexscreener_usd: None,
                                        price_pool_sol: None,
                                        price_pool_usd: None,
                                        dex_id: None,
                                        pair_address: None,
                                        pair_url: None,
                                        labels: vec![],
                                        fdv: None,
                                        market_cap: None,
                                        txns: None,
                                        volume: None,
                                        price_change: None,
                                        liquidity: None,
                                        info: None,
                                        boosts: None,
                                    };
                                    
                                    match close_position_direct(&mint_clone, &token, current_price, "retry_attempt".to_string(), Utc::now()).await {
                                        Ok(signature) => {
                                            log(
                                                LogTag::Positions,
                                                "RETRY_SUCCESS",
                                                &format!("✅ Retry successful for {} with signature {}", position.symbol, get_signature_prefix(&signature))
                                            );
                                            None // Success, no need to retry again
                                        }
                                        Err(e) => {
                                            log(
                                                LogTag::Positions,
                                                "RETRY_FAILED",
                                                &format!("❌ Retry failed for {}: {}", position.symbol, e)
                                            );
                                            Some((mint_clone, attempts + 1)) // Failed, will retry if under limit
                                        }
                                    }
                                } else {
                                    None // Position no longer exists
                                }
                            }
                        }).collect();

                        // Process retry batch in parallel
                        let results = futures::future::join_all(retry_futures).await;
                        
                        // Re-schedule failed retries
                        let failed_retries: Vec<(String, u32)> = results.into_iter().filter_map(|r| r).collect();
                        if !failed_retries.is_empty() {
                            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                            let now = Utc::now();
                            
                            for (mint, attempts) in failed_retries {
                                if attempts < MAX_RETRY_ATTEMPTS {
                                    let next_retry = now + chrono::Duration::minutes(RETRY_DELAY_MINUTES as i64);
                                    state.retry_queue.insert(mint, (next_retry, attempts));
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "RETRY_EXHAUSTED",
                                        &format!("❌ Maximum retry attempts reached for {}", get_mint_prefix(&mint))
                                    );
                                }
                            }
                        }
                    }
                } else {
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🔄 No retry operations ready");
                    }
                }
            }
        }
    }
}

/// Process failed exit retries with parallel processing
async fn process_failed_exit_retries_parallel(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🔄 Starting parallel failed exit retry task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping failed exit retry task");
                break;
            }
            _ = sleep(Duration::from_secs(60)) => {
                // Get failed exit retries ready for processing
                let retry_candidates: Vec<(String, u32)> = {
                    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                    let now = Utc::now();
                    let mut candidates = Vec::new();

                    // Find retry candidates that are ready (past their retry time)
                    let ready_mints: Vec<String> = state.failed_exit_retries
                        .iter()
                        .filter_map(|(mint, (next_retry, attempt_count))| {
                            if now >= *next_retry && *attempt_count < MAX_FAILED_EXIT_RETRIES {
                                Some((mint.clone(), *attempt_count))
                            } else {
                                None
                            }
                        })
                        .map(|(mint, count)| {
                            candidates.push((mint.clone(), count));
                            mint
                        })
                        .collect();

                    // Remove ready candidates from retry queue (they'll be re-added if they fail again)
                    for mint in ready_mints {
                        state.failed_exit_retries.remove(&mint);
                    }

                    candidates
                };

                if !retry_candidates.is_empty() {
                    log(
                        LogTag::Positions,
                        "FAILED_EXIT_RETRY_PROCESSING",
                        &format!("🔄 Processing {} failed exit retries", retry_candidates.len())
                    );

                    // Process retries in batches
                    for batch in retry_candidates.chunks(3) { // Smaller batches for failed exit retries
                        let retry_futures: Vec<_> = batch.iter().map(|(mint, attempt_count)| {
                            let mint_clone = mint.clone();
                            let attempts = *attempt_count;
                            async move {
                                match retry_failed_exit(&mint_clone, attempts).await {
                                    Ok(signature) => {
                                        log(
                                            LogTag::Positions,
                                            "FAILED_EXIT_RETRY_SUCCESS",
                                            &format!("✅ Failed exit retry successful for {} with signature {}", get_mint_prefix(&mint_clone), get_signature_prefix(&signature))
                                        );
                                        None // Success, no need to retry again
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Positions,
                                            "FAILED_EXIT_RETRY_FAILED",
                                            &format!("❌ Failed exit retry failed for {}: {}", get_mint_prefix(&mint_clone), e)
                                        );
                                        Some((mint_clone, attempts + 1)) // Failed, will retry if under limit
                                    }
                                }
                            }
                        }).collect();

                        // Process retry batch in parallel
                        let results = futures::future::join_all(retry_futures).await;
                        
                        // Re-schedule failed retries
                        let failed_retries: Vec<(String, u32)> = results.into_iter().filter_map(|r| r).collect();
                        if !failed_retries.is_empty() {
                            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
                            
                            for (mint, attempts) in failed_retries {
                                if attempts < MAX_FAILED_EXIT_RETRIES {
                                    let next_retry = Utc::now() + chrono::Duration::minutes(FAILED_EXIT_RETRY_DELAY_MINUTES as i64);
                                    state.failed_exit_retries.insert(mint, (next_retry, attempts));
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "FAILED_EXIT_RETRY_EXHAUSTED",
                                        &format!("❌ Maximum failed exit retry attempts reached for {}", get_mint_prefix(&mint))
                                    );
                                }
                            }
                        }
                    }
                } else {
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🔄 No failed exit retries ready");
                    }
                }
            }
        }
    }
}

// ==================== HELPER FUNCTIONS ====================

/// Get safe truncated mint prefix for logging
fn get_mint_prefix(mint: &str) -> String {
    safe_truncate(mint, 8).to_string()
}

/// Get safe truncated signature prefix for logging
fn get_signature_prefix(signature: &str) -> String {
    safe_truncate(signature, 8).to_string()
}

/// Check if a swap attempt is a duplicate within the prevention window
async fn is_duplicate_swap_attempt(mint: &str, size_sol: f64, swap_type: &str) -> bool {
    let key = format!(
        "{}_{}_{}_{}",
        mint,
        size_sol,
        swap_type,
        Utc::now().timestamp() / (DUPLICATE_SWAP_PREVENTION_SECS as i64)
    );

    {
        let attempts = RECENT_SWAP_ATTEMPTS.read().await;
        if attempts.contains_key(&key) {
            return true;
        }
    }

    {
        let mut attempts = RECENT_SWAP_ATTEMPTS.write().await;
        attempts.insert(key, Utc::now());
    }

    false
}

/// Get token balance safely with error handling
async fn get_token_balance_safe(mint: &str, wallet_address: &str) -> Option<u64> {
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => Some(balance),
        Err(e) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "WARNING",
                    &format!("Failed to get token balance for {}: {}", get_mint_prefix(mint), e)
                );
            }
            None
        }
    }
}

// ==================== INITIALIZATION ====================

/// Initialize the positions manager system
pub async fn initialize_positions_system() -> Result<(), String> {
    log(LogTag::Positions, "STARTUP", "🚀 Initializing positions system");

    // Initialize database first
    initialize_positions_database().await.map_err(|e| {
        format!("Failed to initialize positions database: {}", e)
    })?;

    // Load existing positions from database into memory
    match load_all_positions().await {
        Ok(positions) => {
            let mut state = GLOBAL_POSITIONS_STATE.lock().await;

            // Add unverified positions to pending verification queue
            let mut unverified_count = 0;
            for position in &positions {
                // Check if entry transaction needs verification
                if !position.transaction_entry_verified {
                    if let Some(entry_sig) = &position.entry_transaction_signature {
                        state.pending_verifications.insert(entry_sig.clone(), Utc::now());
                        unverified_count += 1;
                    }
                }
                // Check if exit transaction needs verification
                if !position.transaction_exit_verified {
                    if let Some(exit_sig) = &position.exit_transaction_signature {
                        state.pending_verifications.insert(exit_sig.clone(), Utc::now());
                        unverified_count += 1;
                    }
                }
            }

            state.positions = positions;
            log(
                LogTag::Positions,
                "STARTUP",
                &format!("✅ Loaded {} positions from database", state.positions.len())
            );

            if unverified_count > 0 {
                log(
                    LogTag::Positions,
                    "STARTUP",
                    &format!("🔍 Added {} unverified transactions to verification queue", unverified_count)
                );
            }
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "WARNING",
                &format!("Failed to load positions from database: {}", e)
            );
            // Continue with empty state
        }
    }

    // Global state is already initialized by LazyLock

    // Fix any existing failed exit positions (like CHAMP)
    match fix_failed_exit_positions().await {
        Ok(fixed_count) => {
            if fixed_count > 0 {
                log(
                    LogTag::Positions,
                    "STARTUP",
                    &format!("🔧 Fixed {} failed exit positions during startup", fixed_count)
                );
            }
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "STARTUP_WARNING",
                &format!("⚠️ Failed to fix failed exit positions during startup: {}", e)
            );
        }
    }

    log(LogTag::Positions, "STARTUP", "✅ Positions system initialized");
    Ok(())
}

/// Start the positions manager service (replaces actor spawn)
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) -> Result<(), String> {
    log(LogTag::Positions, "STARTUP", "🚀 Starting positions manager service");

    // Initialize the system first
    initialize_positions_system().await?;

    // Start background tasks
    run_background_position_tasks(shutdown).await;

    Ok(())
}

// ==================== SAFETY HELPERS ====================

/// Check if swap attempt is allowed (prevents duplicates)
async fn is_swap_attempt_allowed(mint: &str) -> bool {
    let recent_attempts = RECENT_SWAP_ATTEMPTS.read().await;
    let now = Utc::now();

    if let Some(last_attempt) = recent_attempts.get(mint) {
        let elapsed = now.signed_duration_since(*last_attempt).num_seconds();
        elapsed >= SWAP_ATTEMPT_COOLDOWN_SECONDS
    } else {
        true
    }
}

/// Mark swap attempt to prevent duplicates
async fn mark_swap_attempt(mint: &str) {
    let mut recent_attempts = RECENT_SWAP_ATTEMPTS.write().await;
    let now = Utc::now();

    // Add current attempt
    recent_attempts.insert(mint.to_string(), now);

    // Clean up old entries (older than cooldown period)
    recent_attempts.retain(|_, last_attempt| {
        now.signed_duration_since(*last_attempt).num_seconds() < SWAP_ATTEMPT_COOLDOWN_SECONDS
    });
}

/// Check if position is actively being sold
async fn is_actively_selling(mint: &str) -> bool {
    let active_sells = ACTIVE_SELLS.read().await;
    active_sells.contains(mint)
}

/// Mark position as actively being sold
async fn mark_actively_selling(mint: &str) {
    let mut active_sells = ACTIVE_SELLS.write().await;
    active_sells.insert(mint.to_string());
}

/// Schedule a failed exit retry for a position
async fn schedule_failed_exit_retry(mint: &str, attempt_count: u32) {
    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
    
    if attempt_count < MAX_FAILED_EXIT_RETRIES {
        let next_retry = Utc::now() + chrono::Duration::minutes(FAILED_EXIT_RETRY_DELAY_MINUTES as i64);
        state.failed_exit_retries.insert(mint.to_string(), (next_retry, attempt_count));
        
        log(
            LogTag::Positions,
            "FAILED_EXIT_SCHEDULED",
            &format!(
                "🔄 Scheduled failed exit retry for {} (attempt {}/{}), next retry in {} minutes",
                get_mint_prefix(mint),
                attempt_count + 1,
                MAX_FAILED_EXIT_RETRIES,
                FAILED_EXIT_RETRY_DELAY_MINUTES
            )
        );
    } else {
        log(
            LogTag::Positions,
            "FAILED_EXIT_MAX_RETRIES",
            &format!(
                "❌ Maximum failed exit retries reached for {} ({} attempts)",
                get_mint_prefix(mint),
                MAX_FAILED_EXIT_RETRIES
            )
        );
    }
}

/// Remove position from actively selling
async fn unmark_actively_selling(mint: &str) {
    let mut active_sells = ACTIVE_SELLS.write().await;
    active_sells.remove(mint);
}

/// Get cached balance if fresh enough
async fn get_cached_balance(mint: &str) -> Option<f64> {
    let balance_cache = BALANCE_CACHE.read().await;
    let now = Utc::now();

    if let Some((balance, cached_at)) = balance_cache.get(mint) {
        let elapsed = now.signed_duration_since(*cached_at).num_seconds();
        if elapsed < BALANCE_CACHE_DURATION_SECONDS {
            Some(*balance)
        } else {
            None
        }
    } else {
        None
    }
}

/// Cache balance for mint
async fn cache_balance(mint: &str, balance: f64) {
    let mut balance_cache = BALANCE_CACHE.write().await;
    let now = Utc::now();

    // Store balance with timestamp
    balance_cache.insert(mint.to_string(), (balance, now));

    // Clean up old cache entries (older than cache duration)
    balance_cache.retain(|_, (_, cached_at)| {
        now.signed_duration_since(*cached_at).num_seconds() < BALANCE_CACHE_DURATION_SECONDS
    });
}

/// Retry a failed exit transaction for a position
async fn retry_failed_exit(mint: &str, attempt_count: u32) -> Result<String, String> {
    log(
        LogTag::Positions,
        "FAILED_EXIT_RETRY",
        &format!(
            "🔄 Retrying failed exit for {} (attempt {}/{})",
            get_mint_prefix(mint),
            attempt_count + 1,
            MAX_FAILED_EXIT_RETRIES
        )
    );

    // Get the position
    let position = {
        let state = GLOBAL_POSITIONS_STATE.lock().await;
        state.positions
            .iter()
            .find(|p| {
                p.mint == mint && 
                // Position should have failed exit transaction OR be truly open
                ((p.exit_transaction_signature.is_some() && !p.transaction_exit_verified) ||
                 (p.exit_transaction_signature.is_none() && p.exit_price.is_none()))
            })
            .cloned()
    };

    let position = match position {
        Some(pos) => pos,
        None => {
            return Err(format!("Position not found for failed exit retry: {}", get_mint_prefix(mint)));
        }
    };

    // Get current price for exit
    let current_price = get_price(&mint, Some(PriceOptions::simple()), false)
        .await
        .and_then(|r| r.best_sol_price())
        .unwrap_or(position.entry_price);

    // Get token object from database
    let token = match get_token_from_db(mint).await {
        Some(token) => token,
        None => {
            log(
                LogTag::Positions,
                "FAILED_EXIT_RETRY_TOKEN_ERROR",
                &format!("❌ Could not retrieve token data for {} from database", get_mint_prefix(mint))
            );
            return Err(format!("Token not found in database: {}", get_mint_prefix(mint)));
        }
    };

    // Use higher slippage for failed exit retries
    let slippage = FAILED_EXIT_RETRY_SLIPPAGES
        .get(attempt_count as usize)
        .copied()
        .unwrap_or(20.0);

    log(
        LogTag::Positions,
        "FAILED_EXIT_RETRY_SLIPPAGE",
        &format!(
            "📊 Using {:.1}% slippage for failed exit retry of {}",
            slippage,
            position.symbol
        )
    );

    // Try to close the position again with higher slippage
    match close_position_direct(&mint, &token, current_price, "failed_exit_retry".to_string(), Utc::now()).await {
        Ok(signature) => {
            log(
                LogTag::Positions,
                "FAILED_EXIT_RETRY_SUCCESS",
                &format!(
                    "✅ Failed exit retry successful for {} with signature {}",
                    position.symbol,
                    get_signature_prefix(&signature)
                )
            );
            Ok(signature)
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "FAILED_EXIT_RETRY_FAILED",
                &format!(
                    "❌ Failed exit retry failed for {}: {}",
                    position.symbol,
                    e
                )
            );
            Err(e)
        }
    }
}

// ==================== DATABASE SYNC HELPERS ====================

/// Remove a position by its transaction signature (for cleanup of failed positions)
async fn remove_position_by_signature(signature: &str) -> Result<(), String> {
    log(
        LogTag::Positions,
        "CLEANUP_START",
        &format!(
            "🗑️ Starting cleanup of position with signature {}",
            get_signature_prefix(signature)
        )
    );

    let position_to_remove = {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        // Find position with matching entry or exit signature
        let position_index = state.positions
            .iter()
            .position(|p| {
                p.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature) ||
                    p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
            });

        if let Some(index) = position_index {
            let position = state.positions.remove(index);
            log(
                LogTag::Positions,
                "CLEANUP_REMOVED",
                &format!(
                    "🗑️ Removed position {} from memory (signature: {})",
                    position.symbol,
                    get_signature_prefix(signature)
                )
            );

            // Remove token from priority pool service since position is being cleaned up
            let mint_for_cleanup = position.mint.clone();
            tokio::spawn(async move {
                remove_priority_token(&mint_for_cleanup).await;
            });

            Some(position)
        } else {
            log(
                LogTag::Positions,
                "CLEANUP_NOT_FOUND",
                &format!("⚠️ No position found with signature {}", get_signature_prefix(signature))
            );
            None
        }
    };

    // Remove from database if position had an ID
    if let Some(position) = position_to_remove {
        if let Some(position_id) = position.id {
            match crate::positions_db::delete_position_by_id(position_id).await {
                Ok(_) => {
                    log(
                        LogTag::Positions,
                        "CLEANUP_DB_SUCCESS",
                        &format!(
                            "🗑️ Removed position {} (ID: {}) from database",
                            position.symbol,
                            position_id
                        )
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Positions,
                        "CLEANUP_DB_ERROR",
                        &format!(
                            "❌ Failed to remove position {} (ID: {}) from database: {}",
                            position.symbol,
                            position_id,
                            e
                        )
                    );
                    return Err(format!("Database cleanup failed: {}", e));
                }
            }
        }

        log(
            LogTag::Positions,
            "CLEANUP_COMPLETE",
            &format!(
                "✅ Successfully cleaned up failed position {} with signature {}",
                position.symbol,
                get_signature_prefix(signature)
            )
        );
    }

    Ok(())
}

/// Sync a position between memory and database
pub async fn sync_position_to_database(position: &Position) -> Result<(), String> {
    if let Some(_position_id) = position.id {
        // Update existing position
        update_position(position).await
    } else {
        // Insert new position
        let new_id = save_position(position).await?;
        log(
            LogTag::Positions,
            "DB_SYNC",
            &format!("Position synced to database with new ID {}", new_id)
        );
        Ok(())
    }
}

/// Sync all memory positions to database
pub async fn sync_all_positions_to_database() -> Result<(), String> {
    let state = GLOBAL_POSITIONS_STATE.lock().await;
    let positions = state.positions.clone();
    drop(state); // Release lock early

    let mut success_count = 0;
    let mut error_count = 0;

    for position in positions {
        match sync_position_to_database(&position).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error_count += 1;
                log(
                    LogTag::Positions,
                    "DB_SYNC_ERROR",
                    &format!("Failed to sync position {}: {}", position.symbol, e)
                );
            }
        }
    }

    log(
        LogTag::Positions,
        "DB_SYNC_COMPLETE",
        &format!("Database sync completed: {} successful, {} errors", success_count, error_count)
    );

    if error_count > 0 {
        Err(format!("Database sync had {} errors", error_count))
    } else {
        Ok(())
    }
}

/// Load positions from database and replace memory state
pub async fn reload_positions_from_database() -> Result<(), String> {
    let positions = load_all_positions().await?;

    let mut state = GLOBAL_POSITIONS_STATE.lock().await;
    state.positions = positions;

    log(
        LogTag::Positions,
        "DB_RELOAD",
        &format!("Reloaded {} positions from database", state.positions.len())
    );

    Ok(())
}

/// Fix positions with failed exit transactions that still have tokens in wallet
pub async fn fix_failed_exit_positions() -> Result<usize, String> {
    log(LogTag::Positions, "FIX_START", "🔧 Starting failed exit position fix");

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(format!("Failed to get wallet address: {}", e));
        }
    };

    let mut fixed_count = 0;
    let mut state = GLOBAL_POSITIONS_STATE.lock().await;

    for position in &mut state.positions {
        // Check for positions that have exit transaction but are not verified
        if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
            // Check if tokens are still in wallet
            match get_token_balance(&wallet_address, &position.mint).await {
                Ok(balance) => {
                    if balance > 0 {
                        log(
                            LogTag::Positions,
                            "FIX_DETECTED",
                            &format!(
                                "🔧 Found failed exit position {} with {} tokens still in wallet",
                                position.symbol,
                                balance
                            )
                        );

                        // Clear failed exit transaction data
                        position.exit_transaction_signature = None;
                        position.exit_time = None;
                        position.exit_price = None;
                        position.transaction_exit_verified = false;
                        position.sol_received = None;
                        position.effective_exit_price = None;
                        position.exit_fee_lamports = None;
                        position.closed_reason = None;

                        // Update database
                        if let Some(position_id) = position.id {
                            tokio::spawn({
                                let position_clone = position.clone();
                                async move {
                                    if let Err(e) = update_position(&position_clone).await {
                                        log(
                                            LogTag::Positions,
                                            "FIX_DB_ERROR",
                                            &format!("❌ Failed to update position {} in database: {}", position_clone.symbol, e)
                                        );
                                    } else {
                                        log(
                                            LogTag::Positions,
                                            "FIX_DB_SUCCESS",
                                            &format!("✅ Updated position {} in database", position_clone.symbol)
                                        );
                                    }
                                }
                            });
                        }

                        // Schedule retry
                        let mint_for_retry = position.mint.clone();
                        tokio::spawn(async move {
                            schedule_failed_exit_retry(&mint_for_retry, 0).await;
                        });

                        fixed_count += 1;
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Positions,
                        "FIX_BALANCE_ERROR",
                        &format!("❌ Could not check balance for {}: {}", position.symbol, e)
                    );
                }
            }
        }
    }

    if fixed_count > 0 {
        log(
            LogTag::Positions,
            "FIX_COMPLETE",
            &format!("✅ Fixed {} failed exit positions", fixed_count)
        );
    } else {
        log(LogTag::Positions, "FIX_COMPLETE", "✅ No failed exit positions found");
    }

    Ok(fixed_count)
}
