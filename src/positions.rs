use crate::{
    arguments::{
        get_max_exit_retries,
        is_debug_positions_enabled,
        is_debug_swaps_enabled,
        is_dry_run_enabled,
    },
    configs::read_configs,
    errors::blockchain::{ is_permanent_failure, parse_structured_solana_error },
    errors::{ BlockchainError, DataError, NetworkError, PositionError, ScreenerBotError },
    logger::{ log, log_price_change, LogTag },
    pools::get_pool_price,
    positions_db::{
        delete_position_by_id,
        force_database_sync,
        get_closed_positions as db_get_closed_positions,
        get_open_positions as db_get_open_positions,
        get_position_by_id as db_get_position_by_id,
        get_position_by_mint as db_get_position_by_mint,
        initialize_positions_database,
        load_all_positions,
        save_position,
        save_token_snapshot,
        update_position,
        PositionState,
        TokenSnapshot,
    },
    positions_lib::{
        add_signature_to_index,
        get_position_index_by_mint,
        remove_position_by_signature,
        save_position_token_snapshot,
        sync_position_to_database,
        update_mint_position_index,
    },
    positions_types::Position,
    rpc::{ get_rpc_client, lamports_to_sol, sol_to_lamports },
    swaps::{
        config::{ QUOTE_SLIPPAGE_PERCENT, SOL_MINT },
        execute_best_swap,
        get_best_quote,
        UnifiedQuote,
    },
    tokens::{ get_token_decimals, get_token_from_db, PriceResult, Token },
    trader::{ CriticalOperationGuard, MAX_OPEN_POSITIONS, PROFIT_EXTRA_NEEDED_SOL },
    transactions::{ get_global_transaction_manager, get_transaction },
    transactions_types::{ SwapPnLInfo, Transaction, TransactionStatus },
    utils::{ get_token_balance, get_wallet_address, safe_truncate },
};
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use serde::{ Deserialize, Serialize };
use std::sync::atomic::{ AtomicU64, Ordering };
use std::{ collections::{ HashMap, HashSet }, str::FromStr, sync::{ Arc, LazyLock } };
use tokio::{ sync::{ Mutex, Notify, OwnedMutexGuard, RwLock }, time::{ sleep, Duration } };

/// Verification data for tracking transaction status with slot-based expiration
#[derive(Debug, Clone)]
pub struct VerificationData {
    pub timestamp: DateTime<Utc>,
    pub last_valid_block_height: Option<u64>, // Block height when transaction expires
    pub retry_count: u32,
}

impl VerificationData {
    pub fn new(last_valid_block_height: Option<u64>) -> Self {
        Self {
            timestamp: Utc::now(),
            last_valid_block_height,
            retry_count: 0,
        }
    }

    pub fn with_retry_increment(&self) -> Self {
        Self {
            timestamp: self.timestamp,
            last_valid_block_height: self.last_valid_block_height,
            retry_count: self.retry_count + 1,
        }
    }
}

#[derive(Debug)]
pub struct PositionLockGuard {
    mint: String,
    // keep the owned guard inside the struct so it lives until drop
    _owned_guard: Option<OwnedMutexGuard<()>>,
}

impl PositionLockGuard {
    // Helper to create an "empty" guard (if you need that pattern elsewhere).
    pub fn empty(mint: String) -> Self {
        Self {
            mint,
            _owned_guard: None,
        }
    }
}

impl Drop for PositionLockGuard {
    fn drop(&mut self) {
        // When this struct is dropped, the OwnedMutexGuard is dropped and the lock is released.
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("🔓 Released position lock for mint: {}", safe_truncate(&self.mint, 8))
            );
        }
    }
}

// ==================== GLOBAL STATE (PHASE 2: SHARDED) ====================

// Phase 2: Split monolithic state into separate shards to reduce contention
// Each shard can be locked independently, allowing concurrent operations

// Core position data with concurrent read access
pub static POSITIONS: LazyLock<RwLock<Vec<Position>>> = LazyLock::new(|| RwLock::new(Vec::new()));

// Verification queue - isolated from position data for fast enqueue/dequeue
static PENDING_VERIFICATIONS: LazyLock<
    RwLock<HashMap<String, (String, i64, bool, VerificationData)>>
> = LazyLock::new(|| RwLock::new(HashMap::new()));

// Individual control maps, each with their own lock
static FROZEN_COOLDOWNS: LazyLock<RwLock<HashMap<String, DateTime<Utc>>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

static LAST_OPEN_TIME: LazyLock<RwLock<Option<DateTime<Utc>>>> = LazyLock::new(||
    RwLock::new(None)
);

// ==================== CONSTANT-TIME INDEXES ====================

// Phase 2: O(1) signature to mint lookup (eliminates position vector scans)
pub static SIG_TO_MINT_INDEX: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

// Phase 2: O(1) mint to position vector index lookup
pub static MINT_TO_POSITION_INDEX: LazyLock<RwLock<HashMap<String, usize>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

// ==================== GLOBAL STATICS ====================

// Per-position locks for operation safety
static POSITION_LOCKS: LazyLock<RwLock<HashMap<String, Arc<Mutex<()>>>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

// Global position creation lock to prevent race conditions on MAX_OPEN_POSITIONS
static GLOBAL_POSITION_CREATION_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

// Critical operations tracking to prevent race conditions with price updates
static CRITICAL_OPERATIONS: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(||
    RwLock::new(HashSet::new())
);

// ==================== PROCEEDS METRICS (EXIT QUALITY) ====================
#[derive(Debug, Clone, Default)]
pub struct ProceedsMetricsSnapshot {
    pub accepted_quotes: u64,
    pub rejected_quotes: u64,
    pub accepted_profit_quotes: u64,
    pub accepted_loss_quotes: u64,
    pub total_shortfall_bps_sum: u64, // sum of shortfall * 100 (bps relative to required) for accepted profit quotes
    pub worst_shortfall_bps: u64, // max shortfall bps observed (accepted profit quotes)
    pub average_shortfall_bps: f64, // derived
    pub last_update_unix: i64,
}

struct ProceedsMetricsInternal {
    accepted_quotes: AtomicU64,
    rejected_quotes: AtomicU64,
    accepted_profit_quotes: AtomicU64,
    accepted_loss_quotes: AtomicU64,
    total_shortfall_bps_sum: AtomicU64,
    worst_shortfall_bps: AtomicU64,
    last_update_unix: AtomicU64,
}

impl ProceedsMetricsInternal {
    const fn new() -> Self {
        Self {
            accepted_quotes: AtomicU64::new(0),
            rejected_quotes: AtomicU64::new(0),
            accepted_profit_quotes: AtomicU64::new(0),
            accepted_loss_quotes: AtomicU64::new(0),
            total_shortfall_bps_sum: AtomicU64::new(0),
            worst_shortfall_bps: AtomicU64::new(0),
            last_update_unix: AtomicU64::new(0),
        }
    }
    fn snapshot(&self) -> ProceedsMetricsSnapshot {
        let profit_count = self.accepted_profit_quotes.load(Ordering::Relaxed);
        let total_shortfall_sum = self.total_shortfall_bps_sum.load(Ordering::Relaxed);
        ProceedsMetricsSnapshot {
            accepted_quotes: self.accepted_quotes.load(Ordering::Relaxed),
            rejected_quotes: self.rejected_quotes.load(Ordering::Relaxed),
            accepted_profit_quotes: profit_count,
            accepted_loss_quotes: self.accepted_loss_quotes.load(Ordering::Relaxed),
            total_shortfall_bps_sum: total_shortfall_sum,
            worst_shortfall_bps: self.worst_shortfall_bps.load(Ordering::Relaxed),
            average_shortfall_bps: if profit_count > 0 {
                (total_shortfall_sum as f64) / (profit_count as f64)
            } else {
                0.0
            },
            last_update_unix: self.last_update_unix.load(Ordering::Relaxed) as i64,
        }
    }
}

static PROCEEDS_METRICS: LazyLock<ProceedsMetricsInternal> = LazyLock::new(||
    ProceedsMetricsInternal::new()
);

pub async fn get_proceeds_metrics_snapshot() -> ProceedsMetricsSnapshot {
    PROCEEDS_METRICS.snapshot()
}

// Safety constants for verification system
const VERIFICATION_BATCH_SIZE: usize = 10;
const POSITION_OPEN_COOLDOWN_SECS: i64 = 5; // No global cooldown (from backup)

// Verification safety windows - aligned with Solana transaction validity
const ENTRY_VERIFICATION_MAX_SECS: i64 = 180; // 3 minutes for time-based fallback
const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150; // ~150 slots ≈ 1.5 minutes for transaction expiration
const MAX_VERIFICATION_RETRIES: u32 = 3; // Maximum retry attempts for expired transactions

// Sell retry slippages now sourced from trader unified constants
use crate::trader::SLIPPAGE_EXIT_RETRY_STEPS_PCT;

// ==================== POSITION LOCKING ====================

/// Mark a position as undergoing critical operation (closing, verification, etc.)
async fn mark_critical_operation(mint: &str) {
    let mut critical_ops = CRITICAL_OPERATIONS.write().await;
    critical_ops.insert(mint.to_string());
}

/// Unmark a position from critical operation
async fn unmark_critical_operation(mint: &str) {
    let mut critical_ops = CRITICAL_OPERATIONS.write().await;
    critical_ops.remove(mint);
}

/// Check if a position is undergoing critical operation
pub async fn is_critical_operation_active(mint: &str) -> bool {
    let critical_ops = CRITICAL_OPERATIONS.read().await;
    critical_ops.contains(mint)
}

/// Acquire a position-level lock that will remain held until the returned guard is dropped.
/// Use like: let _lock = acquire_position_lock(mint).await;
pub async fn acquire_position_lock(mint: &str) -> PositionLockGuard {
    let mint_key = mint.to_string();

    // Get or create the lock for this mint
    let lock: Arc<tokio::sync::Mutex<()>> = {
        let mut locks = POSITION_LOCKS.write().await;
        locks
            .entry(mint_key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    // Acquire an *owned* guard that can be stored and will hold the lock across awaits/tasks
    let owned_guard = lock.clone().lock_owned().await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🔒 Acquired position lock for mint: {}", safe_truncate(&mint_key, 8))
        );
    }

    PositionLockGuard {
        mint: mint_key,
        _owned_guard: Some(owned_guard),
    }
}

// ==================== VERIFICATION QUEUE MANAGEMENT ====================

/// Add transaction to verification queue with proper block height tracking
async fn enqueue_for_verification(
    signature: String,
    mint: String,
    position_id: i64,
    is_entry: bool,
    last_valid_block_height: Option<u64>
) -> Result<(), String> {
    let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
    let already_present = pending_verifications.contains_key(&signature);

    if !already_present {
        let verification_data = VerificationData::new(last_valid_block_height);
        pending_verifications.insert(signature.clone(), (
            mint,
            position_id,
            is_entry,
            verification_data,
        ));

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "VERIFICATION_ENQUEUE",
                &format!(
                    "📥 Enqueued transaction {} for verification (height: {:?}, queue_size: {})",
                    safe_truncate(&signature, 8),
                    last_valid_block_height,
                    pending_verifications.len()
                )
            );
        }
    }

    Ok(())
}

/// Check if a transaction has expired based on current block height
async fn is_transaction_expired(verification_data: &VerificationData) -> Result<bool, String> {
    if let Some(last_valid_height) = verification_data.last_valid_block_height {
        let rpc_client = get_rpc_client();
        match rpc_client.get_block_height().await {
            Ok(current_height) => {
                let is_expired = current_height > last_valid_height;

                if is_expired && is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "VERIFICATION_EXPIRED",
                        &format!(
                            "❌ Transaction expired: current height {} > last valid height {}",
                            current_height,
                            last_valid_height
                        )
                    );
                }

                Ok(is_expired)
            }
            Err(e) => {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "VERIFICATION_HEIGHT_ERROR",
                        &format!("⚠️ Failed to get current block height: {}", e)
                    );
                }
                // Fallback to time-based check if block height lookup fails
                let age_seconds = Utc::now()
                    .signed_duration_since(verification_data.timestamp)
                    .num_seconds();
                Ok(age_seconds > ENTRY_VERIFICATION_MAX_SECS)
            }
        }
    } else {
        // No block height info available, use time-based fallback
        let age_seconds = Utc::now()
            .signed_duration_since(verification_data.timestamp)
            .num_seconds();
        Ok(age_seconds > ENTRY_VERIFICATION_MAX_SECS)
    }
}

// ==================== CORE POSITION OPERATIONS ====================

/// Open a new position directly
pub async fn open_position_direct(token_mint: &str) -> Result<String, String> {
    // Get token from database using mint
    let token = match crate::tokens::get_token_from_db(token_mint).await {
        Some(token) => token,
        None => {
            return Err(format!("Token not found in database: {}", token_mint));
        }
    };

    // Get price info from pool service
    let price_info = match get_pool_price(token_mint) {
        Some(price_info) => price_info,
        None => {
            return Err(format!("No price data available for token: {}", token_mint));
        }
    };

    // Calculate required values internally
    let entry_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid price for token: {}", token_mint));
        }
    };

    let percent_change = 0.0; // No change calculation needed
    let size_sol = crate::trader::TRADE_SIZE_SOL;
    let liquidity_tier = Some("UNKNOWN".to_string());
    let profit_min = 5.0; // Default 5% profit target
    let profit_max = 20.0; // Default 20% profit target

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
                token.mint,
                entry_price,
                percent_change
            )
        );
        return Err("DRY-RUN: Position would be opened".to_string());
    }

    // ATOMIC POSITION CREATION: Use global lock to prevent race conditions
    // This ensures position limit checks and creation happen atomically
    let _global_creation_lock = GLOBAL_POSITION_CREATION_LOCK.lock().await;

    // Check cooldowns and existing positions using sharded locks
    {
        // GLOBAL COOLDOWN CHECK - use dedicated last_open_time lock
        {
            let last_open_time = LAST_OPEN_TIME.read().await;
            if let Some(last_open) = *last_open_time {
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
        }

        // ATOMIC POSITION LIMIT CHECK - use read lock for concurrent access
        let (already_has_position, open_positions_count) = {
            let positions = POSITIONS.read().await;
            let has_position = positions
                .iter()
                .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_time.is_none());

            let count = positions
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
        *LAST_OPEN_TIME.write().await = Some(Utc::now());
    }

    // Execute the buy transaction (still under global creation lock)
    let _guard = CriticalOperationGuard::new(&format!("BUY {}", token.symbol));

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
        tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
            if let Some(price_info) = get_pool_price(&token.mint) {
                Some(price_info.price_sol)
            } else {
                None
            }
        }).await
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
        &token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(size_sol),
        best_quote
    ).await.map_err(|e| format!("Swap execution failed: {}", e))?;

    if let Some(ref signature) = swap_result.transaction_signature {
        log(
            LogTag::Positions,
            "TRANSACTION",
            &format!("Transaction {} will be monitored by positions manager", signature)
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
        return Err(format!("Invalid base58 format: {}", transaction_signature));
    }

    // Get current block height for transaction expiration tracking
    let last_valid_block_height = match get_rpc_client().get_block_height().await {
        Ok(current_height) => {
            // Add buffer for transaction validity (typical Solana transaction is valid for ~150 slots)
            Some(current_height + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        }
        Err(e) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "WARN",
                    &format!("Failed to get block height for transaction expiration tracking: {}", e)
                );
            }
            None // Fallback to time-based verification
        }
    };

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
                transaction_signature,
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
        profit_target_min: Some(profit_min),
        profit_target_max: Some(profit_max),
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

    // Add position to sharded state and database (still under global creation lock)
    let position_id = {
        // Save to database first to get the ID (with retries BEFORE mutating in-memory state)
        let mut position_id: i64 = -1;
        let mut attempt = 0;
        const MAX_DB_RETRIES: usize = 3;
        while attempt < MAX_DB_RETRIES {
            match save_position(&new_position).await {
                Ok(id) => {
                    position_id = id;
                    log(
                        LogTag::Positions,
                        "INSERT",
                        &format!("Inserted new position ID {} for mint {}", id, token.mint)
                    );
                    log(
                        LogTag::Positions,
                        "DB_SAVE",
                        &format!(
                            "Position saved to database with ID {} (attempt {} )",
                            id,
                            attempt + 1
                        )
                    );

                    // Save opening token snapshot (async, non-blocking)
                    {
                        let mint_clone = token.mint.clone();
                        tokio::spawn(async move {
                            if
                                let Err(e) = save_position_token_snapshot(
                                    id,
                                    &mint_clone,
                                    "opening"
                                ).await
                            {
                                log(
                                    LogTag::Positions,
                                    "SNAPSHOT_WARN",
                                    &format!(
                                        "Failed to save opening snapshot for {}: {}",
                                        safe_truncate(&mint_clone, 8),
                                        e
                                    )
                                );
                            }
                        });
                    }
                    break;
                }
                Err(e) => {
                    attempt += 1;
                    log(
                        LogTag::Positions,
                        "DB_ERROR",
                        &format!(
                            "Failed to save position to database (attempt {}/{}): {}",
                            attempt,
                            MAX_DB_RETRIES,
                            e
                        )
                    );
                    if attempt >= MAX_DB_RETRIES {
                        // Abort opening to avoid inconsistent in-memory only position
                        return Err(
                            format!(
                                "Failed to persist new position after {} attempts: {}",
                                MAX_DB_RETRIES,
                                e
                            )
                        );
                    }
                    // small backoff
                    sleep(Duration::from_millis(150 * (attempt as u64))).await;
                }
            }
        }

        // Update position with database ID if successful
        let mut position_with_id = new_position.clone();
        if position_id > 0 {
            position_with_id.id = Some(position_id);

            // Enqueue entry transaction for verification now that we have position_id
            if
                let Err(e) = enqueue_for_verification(
                    transaction_signature.clone(),
                    token.mint.clone(),
                    position_id,
                    true, // is_entry: true for entry transaction
                    last_valid_block_height
                ).await
            {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!("Failed to enqueue entry transaction for verification: {}", e)
                );
            }
        }

        // Add position to in-memory list with correct ID
        let position_index = {
            let mut positions = POSITIONS.write().await;
            positions.push(position_with_id.clone());
            positions.len() - 1
        };

        // Update indexes for constant-time lookups
        {
            SIG_TO_MINT_INDEX.write().await.insert(
                transaction_signature.clone(),
                token.mint.clone()
            );
            MINT_TO_POSITION_INDEX.write().await.insert(token.mint.clone(), position_index);
        }

        // Add token to priority pool service for fast price updates
        // Priority token management removed - no longer needed

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "✅ Position created for {} (ID: {}) with signature {} - profit targets: {:.2}%-{:.2}% | Added to priority pool service",
                    token.symbol,
                    position_id,
                    transaction_signature,
                    profit_min,
                    profit_max
                )
            );
        }

        position_id
    };

    // Log entry transaction with comprehensive verification
    log(
        LogTag::Positions,
        "POSITION_ENTRY",
        &format!("📝 Entry transaction {} added to comprehensive verification queue (RPC + transaction analysis)", transaction_signature)
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
            "✅ POSITION CREATED: {} (ID: {}) | TX: {} | Signal Price: {:.12} SOL | Verification: Pending",
            token.symbol,
            position_id,
            transaction_signature,
            entry_price
        )
    );

    Ok(transaction_signature)
}

/// Close an existing position directly
pub async fn close_position_direct(
    token_mint: &str,
    exit_reason: String
) -> Result<String, String> {
    // Get token from database using mint
    let token = match crate::tokens::get_token_from_db(token_mint).await {
        Some(token) => token,
        None => {
            return Err(format!("Token not found in database: {}", token_mint));
        }
    };

    // Get price info from pool service
    let price_info = match get_pool_price(token_mint) {
        Some(price_info) => price_info,
        None => {
            return Err(format!("No price data available for token: {}", token_mint));
        }
    };

    // Calculate exit price from PriceResult
    let exit_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid exit price for token: {}", token_mint));
        }
    };

    let mint = token_mint;
    let exit_time = chrono::Utc::now();
    let _lock = acquire_position_lock(mint).await;

    // Mark this position as undergoing critical operation
    mark_critical_operation(mint).await;

    // Ensure cleanup happens even if function exits early
    let cleanup_critical_op = || async {
        unmark_critical_operation(mint).await;
    };

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
            let positions = POSITIONS.read().await;
            positions
                .iter()
                .find(|p| p.mint == *mint && p.exit_price.is_none())
                .map(|p| format!("{} ({})", p.symbol, p.mint))
        };

        if let Some(info) = position_info {
            log(
                LogTag::Positions,
                "DRY-RUN",
                &format!("🚫 DRY-RUN: Would close position for {}", info)
            );
            cleanup_critical_op().await;
            return Err("DRY-RUN: Position would be closed".to_string());
        }
    }

    // Find position and validate with enhanced state checking
    let position_info = {
        let positions = POSITIONS.read().await;
        positions
            .iter()
            .find(|p| {
                let matches_mint = p.mint == *mint;
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
            .map(|p| (p.symbol.clone(), p.entry_size_sol, p.entry_price, p.id))
    };

    let (symbol, entry_size_sol, entry_price, position_id): (String, f64, f64, Option<i64>) = match
        position_info
    {
        Some(info) => info,
        None => {
            cleanup_critical_op().await;
            return Err(format!("No open position found for token {}", mint));
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

    // Clear failed exit transaction data if retrying (check transaction existence first)
    {
        let mut positions = POSITIONS.write().await;
        if let Some(position) = positions.iter_mut().find(|p| p.mint == *mint) {
            if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
                let sig = position.exit_transaction_signature.as_ref().unwrap();

                // Check if transaction actually exists on blockchain
                let transaction_exists = get_transaction(sig).await
                    .map(|opt| opt.is_some())
                    .unwrap_or(false);

                if transaction_exists {
                    log(
                        LogTag::Positions,
                        "RETRY_EXIT",
                        &format!(
                            "🔄 Previous exit transaction exists for {} - keeping signature, clearing other exit data",
                            position.symbol
                        )
                    );
                    // Keep signature since transaction exists, just retry verification
                } else {
                    log(
                        LogTag::Positions,
                        "RETRY_EXIT",
                        &format!(
                            "🔄 Previous exit transaction not found for {} - clearing all exit data for retry",
                            position.symbol
                        )
                    );
                    // Clear signature since transaction doesn't exist
                    position.exit_transaction_signature = None;
                }

                // Clear other failed exit data regardless
                position.exit_price = None;
                position.exit_time = None;
                position.transaction_exit_verified = false;
                position.sol_received = None;
                position.effective_exit_price = None;
                position.exit_fee_lamports = None;
            }
        }
    }

    // NOTE: Critical operation guard is handled at the trader level to avoid duplicate guards
    // Removed: let _guard = CriticalOperationGuard::new(&format!("SELL {}", symbol));

    // ✅ ENSURE token remains in watch list during sell process
    let _price_service_result = match
        tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
            if let Some(price_info) = get_pool_price(&token.mint) {
                Some(price_info.price_sol)
            } else {
                None
            }
        }).await
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
        &format!(
            "🔴 SELLING all {} tokens (ID: {}) (mint: {}) for SOL",
            symbol,
            position_id.unwrap_or(-1),
            mint
        )
    );

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            unmark_critical_operation(mint).await;
            log(LogTag::Positions, "ERROR", &format!("❌ Failed to get wallet address: {}", e));
            return Err(format!("Failed to get wallet address: {}", e));
        }
    };

    // Get token balance using cached function for better performance
    let token_balance = match get_token_balance_safe(mint, &wallet_address).await {
        Some(balance) if balance > 0 => balance,
        Some(_) => {
            // Zero tokens found - check if tokens were already sold in a recent transaction
            log(
                LogTag::Positions,
                "RECOVERY_CHECK",
                &format!("🔍 Zero {} tokens found - checking for recent sell transactions to recover position", symbol)
            );

            // Try to recover position from recent transactions
            match attempt_position_recovery_from_transactions(mint, &symbol).await {
                Ok(recovered_signature) => {
                    unmark_critical_operation(mint).await;
                    log(
                        LogTag::Positions,
                        "RECOVERY_SUCCESS",
                        &format!(
                            "✅ Position recovered for {} using transaction {}",
                            symbol,
                            recovered_signature
                        )
                    );
                    return Ok(
                        format!("Position recovered from transaction {}", recovered_signature)
                    );
                }
                Err(recovery_error) => {
                    log(
                        LogTag::Positions,
                        "RECOVERY_FAILED",
                        &format!("❌ Recovery failed for {}: {}", symbol, recovery_error)
                    );
                    unmark_critical_operation(mint).await;
                    return Err(
                        format!(
                            "No {} tokens to sell (recovery failed: {})",
                            symbol,
                            recovery_error
                        )
                    );
                }
            }
        }
        None => {
            unmark_critical_operation(mint).await;
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
    // Compute classification of exit (profit vs loss) for slippage policy & proceeds validation
    let profit_percent_for_exit = if entry_price > 0.0 {
        ((exit_price - entry_price) / entry_price) * 100.0
    } else {
        0.0
    };
    let is_loss_exit = profit_percent_for_exit < 0.0;
    // Max allowed slippage for profitable exits (user requirement)
    // Configurable slippage caps imported from trader module
    use crate::trader::{
        MAX_LOSS_EXIT_SLIPPAGE_PCT,
        MAX_PROFIT_EXIT_SLIPPAGE_PCT,
        PROFIT_EXTRA_NEEDED_SOL,
    };
    let max_slippage_allowed = if is_loss_exit {
        MAX_LOSS_EXIT_SLIPPAGE_PCT
    } else {
        MAX_PROFIT_EXIT_SLIPPAGE_PCT
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🎯 Exit classification for {}: profit_pct={:.2} is_loss_exit={} max_slippage_allowed={:.1}%",
                symbol,
                profit_percent_for_exit,
                is_loss_exit,
                max_slippage_allowed
            )
        );
    }

    'outer: for &slippage in SLIPPAGE_EXIT_RETRY_STEPS_PCT.iter() {
        // Each slippage level is attempted up to TWO times before advancing.
        for repeat in 1..=2 {
            // Enforce dynamic slippage ceiling based on exit classification
            if slippage > max_slippage_allowed {
                if is_debug_swaps_enabled() && repeat == 1 {
                    // log once per slippage tier
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "⛔ Skipping slippage {:.1}% (exceeds allowed {:.1}% for this exit type)",
                            slippage,
                            max_slippage_allowed
                        )
                    );
                }
                continue 'outer; // proceed to next slippage tier
            }
            if is_debug_swaps_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🔄 Attempting sell with {:.1}% slippage (try {}/2) for {}",
                        slippage,
                        repeat,
                        symbol
                    )
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
                    // Proceeds validation (two-attempt logic variant)
                    let quoted_output_sol = lamports_to_sol(quote.output_amount);
                    let expected_profit_sol = if profit_percent_for_exit > 0.0 {
                        (entry_size_sol * (profit_percent_for_exit / 100.0)).max(0.0)
                    } else {
                        0.0
                    };
                    let target_required_sol = if profit_percent_for_exit > 0.0 {
                        entry_size_sol + expected_profit_sol + PROFIT_EXTRA_NEEDED_SOL
                    } else {
                        quoted_output_sol
                    };
                    let shortfall_pct = if
                        profit_percent_for_exit > 0.0 &&
                        target_required_sol > 0.0
                    {
                        ((target_required_sol - quoted_output_sol) / target_required_sol) * 100.0
                    } else {
                        0.0
                    };
                    let within_tolerance =
                        is_loss_exit || shortfall_pct <= MAX_PROFIT_EXIT_SLIPPAGE_PCT;

                    if is_debug_swaps_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "📐 Proceeds check: quoted={:.8} SOL required={:.8} SOL shortfall={:.2}% within_tolerance={} (profit_exit={} try={}/2)",
                                quoted_output_sol,
                                target_required_sol,
                                shortfall_pct.max(0.0),
                                within_tolerance,
                                !is_loss_exit,
                                repeat
                            )
                        );
                    }

                    if within_tolerance {
                        PROCEEDS_METRICS.accepted_quotes.fetch_add(1, Ordering::Relaxed);
                        PROCEEDS_METRICS.last_update_unix.store(
                            Utc::now().timestamp() as u64,
                            Ordering::Relaxed
                        );
                        if is_loss_exit {
                            PROCEEDS_METRICS.accepted_loss_quotes.fetch_add(1, Ordering::Relaxed);
                        } else {
                            PROCEEDS_METRICS.accepted_profit_quotes.fetch_add(1, Ordering::Relaxed);
                            let shortfall_bps = (shortfall_pct.max(0.0) * 100.0).round() as u64;
                            PROCEEDS_METRICS.total_shortfall_bps_sum.fetch_add(
                                shortfall_bps,
                                Ordering::Relaxed
                            );
                            loop {
                                let current_worst = PROCEEDS_METRICS.worst_shortfall_bps.load(
                                    Ordering::Relaxed
                                );
                                if shortfall_bps <= current_worst {
                                    break;
                                }
                                if
                                    PROCEEDS_METRICS.worst_shortfall_bps.compare_exchange(
                                        current_worst,
                                        shortfall_bps,
                                        Ordering::Relaxed,
                                        Ordering::Relaxed
                                    ).is_ok()
                                {
                                    break;
                                }
                            }
                        }
                        if !is_loss_exit {
                            log(
                                LogTag::Positions,
                                "PROCEEDS_ACCEPT",
                                &format!(
                                    "📊 Proceeds ok: shortfall={:.2}% (<= {:.2}% cap) output={:.8} required={:.8} (slip {:.1}% try {}/2)",
                                    shortfall_pct.max(0.0),
                                    MAX_PROFIT_EXIT_SLIPPAGE_PCT,
                                    quoted_output_sol,
                                    target_required_sol,
                                    slippage,
                                    repeat
                                )
                            );
                        }
                        best_quote = Some(quote);
                        quote_slippage_used = slippage;
                        break 'outer; // success
                    } else {
                        last_error = format!(
                            "Quote shortfall {:.2}% exceeds allowed {:.2}% (quoted {:.8} < required {:.8} SOL)",
                            shortfall_pct,
                            MAX_PROFIT_EXIT_SLIPPAGE_PCT,
                            quoted_output_sol,
                            target_required_sol
                        );
                        log(
                            LogTag::Positions,
                            "PROCEEDS_REJECT",
                            &format!(
                                "❌ Rejecting quote at {:.1}% slippage (try {}/2) for {}: {}",
                                slippage,
                                repeat,
                                symbol,
                                last_error
                            )
                        );
                        PROCEEDS_METRICS.rejected_quotes.fetch_add(1, Ordering::Relaxed);
                        PROCEEDS_METRICS.last_update_unix.store(
                            Utc::now().timestamp() as u64,
                            Ordering::Relaxed
                        );
                        continue; // second attempt or next tier
                    }
                }
                Ok(Err(e)) => {
                    last_error = e.to_string();
                    if is_debug_swaps_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "❌ Quote failed with {:.1}% slippage (try {}/2): {}",
                                slippage,
                                repeat,
                                last_error
                            )
                        );
                    }
                    continue; // next repeat / tier
                }
                Err(_) => {
                    last_error = format!(
                        "Quote timeout with {:.1}% slippage (try {}/2)",
                        slippage,
                        repeat
                    );
                    if is_debug_swaps_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "⏰ Quote timeout with {:.1}% slippage (try {}/2)",
                                slippage,
                                repeat
                            )
                        );
                    }
                    continue; // next repeat / tier
                }
            }
        }
    }

    let quote = match best_quote {
        Some(q) => q,
        None => {
            unmark_critical_operation(mint).await;
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
    let swap_result = execute_best_swap(&token, mint, SOL_MINT, token_balance, quote).await;

    let transaction_signature = match swap_result {
        Ok(result) => {
            if let Some(ref signature) = result.transaction_signature {
                log(
                    LogTag::Positions,
                    "TRANSACTION",
                    &format!("Sell transaction {} will be monitored by positions manager", signature)
                );
                signature.clone()
            } else {
                unmark_critical_operation(mint).await;
                return Err(format!("Sell swap completed but no transaction signature returned"));
            }
        }
        Err(e) => {
            unmark_critical_operation(mint).await;
            return Err(format!("Sell swap execution failed: {}", e));
        }
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🔴 SELL operation completed for {} - TX: {}", symbol, transaction_signature)
        );
    }

    // CRITICAL VALIDATION: Verify transaction signature is valid before updating position
    if transaction_signature.is_empty() || transaction_signature.len() < 32 {
        unmark_critical_operation(mint).await;
        return Err(format!("Transaction signature is invalid or empty: {}", transaction_signature));
    }

    // Additional validation: Check if signature is valid base58
    if bs58::decode(&transaction_signature).into_vec().is_err() {
        unmark_critical_operation(mint).await;
        return Err(format!("Invalid base58 format: {}", transaction_signature));
    }

    // Get current block height for transaction expiration tracking
    let last_valid_block_height = match get_rpc_client().get_block_height().await {
        Ok(current_height) => {
            // Add buffer for transaction validity (typical Solana transaction is valid for ~150 slots)
            Some(current_height + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        }
        Err(e) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "WARN",
                    &format!("Failed to get block height for exit transaction expiration tracking: {}", e)
                );
            }
            None // Fallback to time-based verification
        }
    };

    // === Phase 2: Sharded locks + O(1) index lookup for exit signature update ===
    // 1. Find position index using O(1) mint lookup (no scan needed)
    let position_idx = get_position_index_by_mint(mint).await;

    // 2. Snapshot existing exit signature using minimal read lock
    let existing_exit_sig: Option<String> = if let Some(idx) = position_idx {
        let positions = POSITIONS.read().await;
        if let Some(position) = positions.get(idx) {
            if
                position.mint == *mint &&
                (position.exit_price.is_none() || !position.transaction_exit_verified)
            {
                position.exit_transaction_signature.clone()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 3. Validate existing exit signature (potential RPC) outside any lock
    if let Some(existing_sig) = &existing_exit_sig {
        if let Ok(Some(_)) = get_transaction(existing_sig).await {
            log(
                LogTag::Positions,
                "WARNING",
                &format!(
                    "⚠️ Position {} already has valid exit transaction {} - not overwriting with {}",
                    symbol,
                    existing_sig,
                    transaction_signature
                )
            );
            unmark_critical_operation(mint).await;
            return Err(format!("Position already has valid exit transaction: {}", existing_sig));
        }
    }

    // 4. Re-lock and set exit signature using O(1) index lookup
    let mut position_for_db: Option<Position> = None;
    if let Some(idx) = position_idx {
        let mut positions = POSITIONS.write().await;
        if let Some(position) = positions.get_mut(idx) {
            if
                position.mint == *mint &&
                (position.exit_price.is_none() || !position.transaction_exit_verified)
            {
                if position.exit_transaction_signature.is_none() {
                    position.exit_transaction_signature = Some(transaction_signature.clone());
                    position.closed_reason = Some(format!("{}_pending_verification", exit_reason));

                    // Add to signature index for future O(1) lookups
                    drop(positions); // Release write lock before index update
                    add_signature_to_index(&transaction_signature, mint).await;

                    log(
                        LogTag::Positions,
                        "EXIT_SIG_SET",
                        &format!(
                            "✳️ Set exit signature {} for {} (will persist to DB & enqueue)",
                            transaction_signature,
                            symbol
                        )
                    );

                    // Re-acquire for clone
                    let positions = POSITIONS.read().await;
                    if let Some(pos) = positions.get(idx) {
                        position_for_db = Some(pos.clone());
                    }
                } else {
                    position_for_db = Some(position.clone());
                }
            }
        }
    }

    if position_for_db.is_none() {
        log(
            LogTag::Positions,
            "WARNING",
            &format!("⚠️ Position for {} not found during exit update", symbol)
        );
    }

    // 4. Proceed with database persistence (outside global lock)
    let mut db_update_succeeded = true; // track for enqueue logic
    if let Some(position) = position_for_db {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🗄️ Starting database update for position with ID: {}",
                    position.id.unwrap_or(-1)
                )
            );
        }

        if position.id.is_some() {
            let position_id = position.id.unwrap();

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("🔄 Starting database retry loop for position ID {}", position_id)
                );
            }

            // Retry database update with read-after-write verification
            let mut retry_count = 0;
            let max_retries = 3;

            loop {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "📝 Database update attempt {}/{} for position ID {}",
                            retry_count + 1,
                            max_retries,
                            position_id
                        )
                    );
                }

                // Attempt database update
                match update_position(&position).await {
                    Ok(_) => {
                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "DEBUG",
                                &format!("✅ Database update succeeded for position ID {}, verifying write...", position_id)
                            );
                        }

                        // Verify write succeeded by reading back the exit signature
                        match db_get_position_by_id(position_id).await {
                            Ok(Some(updated_position)) => {
                                if is_debug_positions_enabled() {
                                    log(
                                        LogTag::Positions,
                                        "DEBUG",
                                        &format!("🔍 Read back position ID {}, comparing signatures", position_id)
                                    );
                                }

                                if
                                    updated_position.exit_transaction_signature.as_ref() ==
                                    Some(&transaction_signature)
                                {
                                    if is_debug_positions_enabled() {
                                        log(
                                            LogTag::Positions,
                                            "DEBUG",
                                            &format!("✅ Exit signature verified in database for position ID {}", position_id)
                                        );
                                    }

                                    log(
                                        LogTag::Positions,
                                        "DB_UPDATE",
                                        &format!("Position {} exit signature verified in database", position_id)
                                    );

                                    // Force database sync to ensure all connections see the update immediately
                                    // This prevents race conditions with concurrent verification processes
                                    if let Err(sync_err) = force_database_sync().await {
                                        log(
                                            LogTag::Positions,
                                            "DB_SYNC_WARNING",
                                            &format!("Failed to sync database after exit signature update: {}", sync_err)
                                        );
                                    } else {
                                        if is_debug_positions_enabled() {
                                            log(
                                                LogTag::Positions,
                                                "DEBUG",
                                                &format!("✅ Database sync completed for position ID {}", position_id)
                                            );
                                        }
                                        log(
                                            LogTag::Positions,
                                            "DB_SYNC_SUCCESS",
                                            &format!("Database synchronized after position {} exit signature update", position_id)
                                        );
                                    }

                                    if is_debug_positions_enabled() {
                                        log(
                                            LogTag::Positions,
                                            "DEBUG",
                                            &format!("🚀 Breaking from database retry loop - success for position ID {}", position_id)
                                        );
                                    }

                                    break; // Success - exit signature confirmed persisted
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "DB_VERIFY_FAILED",
                                        &format!(
                                            "Exit signature not found in database for position {}, retry {}/{}",
                                            position_id,
                                            retry_count + 1,
                                            max_retries
                                        )
                                    );
                                }
                            }
                            Ok(None) => {
                                log(
                                    LogTag::Positions,
                                    "DB_VERIFY_FAILED",
                                    &format!(
                                        "Position {} not found in database during verification, retry {}/{}",
                                        position_id,
                                        retry_count + 1,
                                        max_retries
                                    )
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Positions,
                                    "DB_VERIFY_ERROR",
                                    &format!(
                                        "Failed to verify position {} update: {}, retry {}/{}",
                                        position_id,
                                        e,
                                        retry_count + 1,
                                        max_retries
                                    )
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "DB_ERROR",
                            &format!(
                                "Failed to update position {} in database: {}, retry {}/{}",
                                position_id,
                                e,
                                retry_count + 1,
                                max_retries
                            )
                        );
                    }
                }

                retry_count += 1;
                if retry_count >= max_retries {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "❌ Max retries reached ({}) for position ID {}, returning error",
                                max_retries,
                                position_id
                            )
                        );
                    }
                    // Do NOT abort here: continue with enqueue so verification can still proceed
                    log(
                        LogTag::Positions,
                        "EXIT_DB_PERSIST_DEFERRED",
                        &format!("❌ Failed to persist exit signature after {} attempts (will proceed & retry in background)", max_retries)
                    );
                    db_update_succeeded = false;
                    break;
                }

                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "⏳ Retry {}/{} failed, sleeping before next attempt",
                            retry_count,
                            max_retries
                        )
                    );
                }

                // Brief delay before retry
                sleep(Duration::from_millis(100 * (retry_count as u64))).await;
            }

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("✅ Database retry loop completed successfully for position ID {}", position_id)
                );
            }
        } else {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("⚠️ Position has no ID, skipping database update")
                );
            }
        }
    } else {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("⚠️ No position_for_db found, skipping database update")
            );
        }
    }

    // DEBUG: Log that we're about to start verification enqueue
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🔄 About to enqueue verification for transaction {}", transaction_signature)
        );
    }

    // Only add to verification queue after confirming database persistence
    // This must happen regardless of whether database update succeeded or failed
    // CRITICAL: Verification enqueue MUST succeed - retry until it works
    let mut enqueue_attempt = 1;
    let max_enqueue_attempts = 5;
    let mut enqueue_success = false;

    while !enqueue_success && enqueue_attempt <= max_enqueue_attempts {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔄 Verification enqueue attempt {}/{} for transaction {}",
                    enqueue_attempt,
                    max_enqueue_attempts,
                    transaction_signature
                )
            );
        }

        match
            tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("🔒 Acquiring PENDING_VERIFICATIONS lock for verification enqueue (attempt {})...", enqueue_attempt)
                    );
                }

                // Phase 2: Use dedicated verification queue lock (micro-contention)
                enqueue_for_verification(
                    transaction_signature.clone(),
                    mint.to_string(),
                    position_id.unwrap_or(0),
                    false, // is_entry: false for exit
                    last_valid_block_height
                ).await;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("📝 Enqueued exit transaction {} for verification using block height system", transaction_signature)
                    );
                }

                log(
                    LogTag::Positions,
                    "VERIFICATION_ENQUEUE_EXIT",
                    &format!(
                        "📥 Enqueued EXIT tx {} using block height system (attempt={})",
                        transaction_signature,
                        enqueue_attempt
                    )
                );

                Ok::<(), String>(())
            }).await
        {
            Ok(Ok(())) => {
                // Verification enqueue succeeded
                enqueue_success = true;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "✅ Verification enqueue completed successfully for transaction {} (attempt {})",
                            transaction_signature,
                            enqueue_attempt
                        )
                    );
                }
            }
            Ok(Err(e)) => {
                // Verification enqueue failed with error
                log(
                    LogTag::Positions,
                    "WARN",
                    &format!(
                        "❌ Verification enqueue failed for transaction {} (attempt {}): {}",
                        transaction_signature,
                        enqueue_attempt,
                        e
                    )
                );
            }
            Err(_) => {
                // Verification enqueue timed out
                log(
                    LogTag::Positions,
                    "WARN",
                    &format!(
                        "⏰ Verification enqueue timed out (5s) for transaction {} (attempt {})",
                        transaction_signature,
                        enqueue_attempt
                    )
                );
            }
        }

        if !enqueue_success {
            enqueue_attempt += 1;
            if enqueue_attempt <= max_enqueue_attempts {
                // Wait before retrying (exponential backoff)
                let wait_ms = (enqueue_attempt - 1) * 500; // 500ms, 1000ms, 1500ms, 2000ms
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "⏳ Waiting {}ms before verification enqueue retry {}/{}",
                            wait_ms,
                            enqueue_attempt,
                            max_enqueue_attempts
                        )
                    );
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms as u64)).await;
            }
        }
    }

    // CRITICAL CHECK: If verification enqueue failed after all attempts, log critical error
    if !enqueue_success {
        log(
            LogTag::Positions,
            "ERROR",
            &format!(
                "🚨 CRITICAL: Verification enqueue FAILED after {} attempts for transaction {}! Position will be stuck!",
                max_enqueue_attempts,
                transaction_signature
            )
        );

        // Spawn a background task to keep retrying indefinitely
        let bg_signature = transaction_signature.clone();
        let bg_mint = mint.to_string();
        let bg_position_id = position_id.unwrap_or(0);
        let bg_last_valid_block_height = last_valid_block_height;
        tokio::spawn(async move {
            let mut bg_attempt = 1;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                log(
                    LogTag::Positions,
                    "RETRY_BACKGROUND",
                    &format!(
                        "🔁 Background verification enqueue retry {} for transaction {}",
                        bg_attempt,
                        bg_signature
                    )
                );

                match
                    tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
                        // Use new block height system for background enqueue
                        enqueue_for_verification(
                            bg_signature.clone(),
                            bg_mint.clone(),
                            bg_position_id,
                            false, // is_entry: false for exit
                            bg_last_valid_block_height
                        ).await?;

                        log(
                            LogTag::Positions,
                            "VERIFICATION_ENQUEUE_EXIT_BACKGROUND",
                            &format!(
                                "📥 Background enqueued EXIT tx {} using block height system (bg_attempt={})",
                                bg_signature,
                                bg_attempt
                            )
                        );
                        Ok::<(), String>(())
                    }).await
                {
                    Ok(Ok(())) => {
                        log(
                            LogTag::Positions,
                            "SUCCESS",
                            &format!(
                                "✅ Background verification enqueue succeeded for transaction {} after {} attempts",
                                bg_signature,
                                bg_attempt
                            )
                        );
                        break; // Success - exit background retry loop
                    }
                    Ok(Err(e)) => {
                        log(
                            LogTag::Positions,
                            "WARN",
                            &format!(
                                "⚠️ Background verification enqueue failed for transaction {} (attempt {}): {}",
                                bg_signature,
                                bg_attempt,
                                e
                            )
                        );
                        // Continue retrying
                        bg_attempt += 1;
                    }
                    Err(_) => {
                        // Timeout - continue retrying
                        bg_attempt += 1;
                    }
                }
            }
        });
    }

    // IMPORTANT: Release the per-position lock BEFORE attempting quick verification
    // so verify_position_transaction can acquire it. Without this, the quick
    // verification would block until timeout, slowing closure flow.
    if is_debug_positions_enabled() {
        log(LogTag::Positions, "DEBUG", &format!("🔓 Releasing position lock for {}", mint));
    }
    drop(_lock);

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🏁 Exiting position update block for {}", token.symbol)
        );
    }

    if is_debug_positions_enabled() {
        log(LogTag::Positions, "DEBUG", &format!("🧹 Starting cleanup for {}", token.symbol));
    }

    unmark_critical_operation(mint).await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("📝 About to log POSITION_EXIT for {}", token.symbol)
        );
    }

    // Log exit transaction with comprehensive verification
    log(
        LogTag::Positions,
        "POSITION_EXIT",
        &format!("📝 Exit transaction {} added to comprehensive verification queue (RPC + transaction analysis)", transaction_signature)
    );

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🏊 Removing {} from priority pool service", token.symbol)
        );
    }

    // Remove token from priority pool service (no longer need fast updates)
    // Priority token management removed - no longer needed

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🚀 Spawning background transaction fetch for {}", transaction_signature)
        );
    }

    // Immediately attempt to fetch transaction to accelerate verification
    let sig_for_fetch = transaction_signature.clone();
    tokio::spawn(async move {
        let _ = get_transaction(&sig_for_fetch).await;
    });

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("⚡ Starting quick verification attempt for {} (with 3s propagation delay)", transaction_signature)
        );
    }

    // CRITICAL: Add initial delay to allow for transaction propagation
    // This prevents verification from failing immediately due to RPC propagation delays
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Schedule background DB retry if needed
    if !db_update_succeeded {
        let mint_clone = mint.to_string();
        let signature_clone = transaction_signature.clone();
        tokio::spawn(async move {
            let mut retry = 0;
            while retry < 10 {
                // up to ~10 retries with backoff
                tokio::time::sleep(Duration::from_secs(5 * ((retry + 1) as u64))).await;
                // fetch position and retry update
                let pos_opt = {
                    let positions = POSITIONS.read().await;
                    positions
                        .iter()
                        .find(|p| p.mint == mint_clone)
                        .cloned()
                };
                if let Some(pos) = pos_opt {
                    if let Some(id) = pos.id {
                        // only if still missing persisted exit sig
                        match update_position(&pos).await {
                            Ok(_) => {
                                log(
                                    LogTag::Positions,
                                    "EXIT_DB_RETRY_SUCCESS",
                                    &format!(
                                        "✅ Background retry succeeded updating exit signature {} for mint {} (retry {})",
                                        signature_clone,
                                        safe_truncate(&mint_clone, 8),
                                        retry + 1
                                    )
                                );
                                break;
                            }
                            Err(e) => {
                                log(
                                    LogTag::Positions,
                                    "EXIT_DB_RETRY_FAIL",
                                    &format!(
                                        "⚠️ Background retry {} failed updating exit signature for mint {}: {}",
                                        retry + 1,
                                        safe_truncate(&mint_clone, 8),
                                        e
                                    )
                                );
                            }
                        }
                    }
                }
                retry += 1;
            }
        });
    }

    // Quick verification attempt (30 seconds timeout)
    let quick_verification_result = tokio::time::timeout(
        Duration::from_secs(30),
        verify_position_transaction(&transaction_signature)
    ).await;

    let position_status = match quick_verification_result {
        Ok(Ok(true)) => {
            // Verification succeeded quickly - position is truly closed
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("✅ Quick verification succeeded for {}", transaction_signature)
                );
            }
            log(
                LogTag::Positions,
                "QUICK_VERIFICATION_SUCCESS",
                &format!("✅ {} exit verified immediately", symbol)
            );
            "CLOSED"
        }
        _ => {
            // Verification failed or timed out - keep as "closing in progress"
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("⏳ Quick verification failed/timed out for {}, will verify in background", transaction_signature)
                );
            }
            log(
                LogTag::Positions,
                "QUICK_VERIFICATION_PENDING",
                &format!("⏳ {} exit pending verification (normal - will retry)", symbol)
            );
            "CLOSING"
        }
    };

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🎯 Final position status for {}: {}", token.symbol, position_status)
        );
    }

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ POSITION {}: {} (ID: {}) | TX: {} | Reason: {} | Status: {} | Removed from priority pool service",
            position_status,
            symbol,
            position_id.unwrap_or(-1),
            transaction_signature,
            exit_reason,
            position_status
        )
    );

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🔓 Final cleanup of critical operation marking for {}", mint)
        );
    }

    // Final cleanup of critical operation marking
    unmark_critical_operation(mint).await;

    // Update OHLCV watch list - mark as no longer open position (lower priority)
    if let Ok(ohlcv_service) = crate::tokens::get_ohlcv_service_clone().await {
        ohlcv_service.add_to_watch_list(mint, false).await; // false = no longer open position
    }

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "✅ close_position_direct completed successfully for {} with transaction {}",
                token.symbol,
                transaction_signature
            )
        );
    }

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

    // CRITICAL: Skip price updates if critical operations are in progress for this position
    // This prevents race conditions with closing, verification, and other critical state changes
    if is_critical_operation_active(mint).await {
        return false; // Don't interfere with critical operations
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

    // Double-check critical operations after acquiring lock
    if is_critical_operation_active(mint).await {
        return false; // Still in critical operation, abort
    }

    let mut positions = POSITIONS.write().await;

    if let Some(position) = positions.iter_mut().find(|p| p.mint == mint) {
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

        // Database update for price tracking (can be async since not critical for verification)
        if position.id.is_some() {
            let position_clone = position.clone();
            tokio::spawn(async move {
                if let Err(e) = sync_position_to_database(&position_clone).await {
                    // Only log if debug is enabled to avoid spam
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("Price sync failed for {}: {}", position_clone.symbol, e)
                        );
                    }
                }
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
            "DEBUG",
            &format!("🔍 Starting comprehensive verification for transaction {}", signature)
        );
    }

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "VERIFY",
            &format!("🔍 Performing comprehensive verification for transaction {}", signature)
        );
    }

    // Get the transaction with comprehensive verification using priority processing
    // This ensures we get a fully analyzed transaction even when the manager is busy
    let transaction = match get_transaction(signature).await {
        Ok(Some(transaction)) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🔍 Transaction {} found, checking status: {:?}",
                        signature,
                        transaction.status
                    )
                );
            }

            // Check transaction status and success
            match transaction.status {
                TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                    if transaction.success {
                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "DEBUG",
                                &format!(
                                    "✅ Transaction {} status: {:?}, success: true",
                                    signature,
                                    transaction.status
                                )
                            );
                        }

                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "VERIFY_SUCCESS",
                                &format!(
                                    "✅ Transaction {} verified successfully: fee={:.6} SOL, sol_change={:.6} SOL",
                                    signature,
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
                                    signature,
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
                            &format!("⏳ Transaction {} still pending verification", signature)
                        );
                    }
                    return Err("Transaction still pending".to_string());
                }
                TransactionStatus::Failed(error) => {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "VERIFY_FAILED",
                            &format!("❌ Transaction {} failed: {}", signature, error)
                        );
                    }
                    return Err(format!("Transaction failed: {}", error));
                }
            }
        }
        Ok(None) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("🔍 Transaction {} not found in system, checking verification age", signature)
                );
            }

            // Transaction not found - check verification age
            let verification_age_seconds = {
                let pending_verifications = PENDING_VERIFICATIONS.read().await;
                if let Some((_, _, _, verification_data)) = pending_verifications.get(signature) {
                    Utc::now().signed_duration_since(verification_data.timestamp).num_seconds()
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
                        signature,
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
                            signature,
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
                            signature,
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
                    &format!("❌ Error getting transaction {}: {}", signature, e)
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
                            info.token_mint,
                            info.sol_amount,
                            info.token_amount,
                            info.calculated_price_sol
                        )
                    );
                } else {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("⚠️ No swap analysis result for transaction {}", signature)
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

    // Find the position mint FIRST using O(1) index lookup
    let position_mint = {
        let sig_to_mint = SIG_TO_MINT_INDEX.read().await;
        let mint = sig_to_mint.get(signature).cloned();

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔍 Index lookup for signature {}: found_mint={:?}",
                    signature,
                    mint
                        .as_ref()
                        .map(|m| m.as_str())
                        .unwrap_or("None")
                )
            );
        }

        mint
    };

    let position_mint_for_lock = match position_mint {
        Some(mint) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("✅ Position mint found for {}: {}", signature, mint)
                );
            }
            mint
        }
        None => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!("❌ No position mint found for signature {} in index", signature)
                );
            }
            return Err("No matching position found for transaction".to_string());
        }
    };

    // NOW acquire the position lock for the correct mint
    let _lock = acquire_position_lock(&position_mint_for_lock).await;

    // Update position verification status using O(1) index lookup with fallback
    let mut verified = false;
    let mut position_for_db_update: Option<Position> = None;
    let mut position_mint: Option<String> = None;
    {
        // Get position index using O(1) mint lookup
        let position_index = {
            let mint_to_index = MINT_TO_POSITION_INDEX.read().await;
            mint_to_index.get(&position_mint_for_lock).copied()
        };

        let position_index = match position_index {
            Some(index) => index,
            None => {
                // CRITICAL FIX: Index lookup failed - attempt recovery
                log(
                    LogTag::Positions,
                    "INDEX_RECOVERY",
                    &format!("❌ Position index not found for mint {}, attempting recovery", position_mint_for_lock)
                );

                // Try to rebuild the index and find the position
                update_mint_position_index().await;

                // Retry the lookup after rebuilding
                let mint_to_index = MINT_TO_POSITION_INDEX.read().await;
                if let Some(recovered_index) = mint_to_index.get(&position_mint_for_lock).copied() {
                    log(
                        LogTag::Positions,
                        "INDEX_RECOVERY_SUCCESS",
                        &format!(
                            "✅ Position index recovered for mint {} at index {}",
                            position_mint_for_lock,
                            recovered_index
                        )
                    );
                    recovered_index
                } else {
                    // Still not found after recovery - try linear search as last resort
                    log(
                        LogTag::Positions,
                        "INDEX_RECOVERY_FALLBACK",
                        &format!("⚠️ Index recovery failed for mint {}, falling back to linear search", position_mint_for_lock)
                    );

                    let positions = POSITIONS.read().await;
                    if
                        let Some((found_index, _)) = positions
                            .iter()
                            .enumerate()
                            .find(|(_, p)| p.mint == position_mint_for_lock)
                    {
                        log(
                            LogTag::Positions,
                            "LINEAR_SEARCH_SUCCESS",
                            &format!(
                                "✅ Position found via linear search for mint {} at index {}",
                                position_mint_for_lock,
                                found_index
                            )
                        );
                        found_index
                    } else {
                        log(
                            LogTag::Positions,
                            "POSITION_NOT_FOUND",
                            &format!("❌ Position not found for mint {} even with linear search", position_mint_for_lock)
                        );
                        return Err(
                            "Position not found even after index recovery and linear search".to_string()
                        );
                    }
                }
            }
        };

        // Get mutable access to the specific position using minimal write lock
        let mut positions = POSITIONS.write().await;

        if position_index >= positions.len() {
            log(
                LogTag::Positions,
                "INDEX_OUT_OF_BOUNDS",
                &format!(
                    "❌ Position index {} out of bounds (positions.len()={})",
                    position_index,
                    positions.len()
                )
            );
            return Err("Position index out of bounds".to_string());
        }

        let position = &mut positions[position_index];

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔍 O(1) position lookup for verification - found position {} at index {}",
                    position.symbol,
                    position_index
                )
            );
        }

        let is_entry =
            position.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature);
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
                    .map(|s| s.as_str())
                    .unwrap_or("None"),
                position.exit_transaction_signature
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("None"),
                is_entry,
                is_exit
            )
        );

        if is_entry {
            // Mark position for critical operation during verification
            position_mint = Some(position.mint.clone());

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
                            signature,
                            position.symbol,
                            position.mint,
                            swap_info.swap_type,
                            swap_info.token_mint
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
                        signature,
                        position.symbol
                    )
                );
                // Don't mark as failed - let it retry (same as backup)
                return Err("No valid swap analysis - will retry".to_string());
            }
        } else if is_exit {
            // Mark position for critical operation during verification
            position_mint = Some(position.mint.clone());

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
                            signature,
                            position.symbol,
                            position.mint,
                            swap_info.swap_type,
                            swap_info.token_mint
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
                        signature,
                        position.symbol
                    )
                );
                // Don't mark as failed - let it retry (same as backup)
                return Err("No valid swap analysis - will retry".to_string());
            }
        } else {
            return Err("Transaction signature does not match position entry or exit".to_string());
        }

        // NOTE: Do NOT remove from pending verifications yet - only after successful database update

        log(
            LogTag::Positions,
            "VERIFY_RESULT",
            &format!(
                "✅ O(1) position verification completed for {}: verified={}, position_for_db_update={}",
                signature,
                verified,
                position_for_db_update.is_some()
            )
        );
    }

    // If we found a position, mark it for critical operation
    if let Some(ref mint) = position_mint {
        mark_critical_operation(mint).await;
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
                // Cleanup critical operation marking before returning error
                if let Some(ref mint) = position_mint {
                    unmark_critical_operation(mint).await;
                }
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

                // Force database sync after verification updates to prevent race conditions
                // This is critical when exit_verified changes from 0 to 1
                if let Err(sync_err) = force_database_sync().await {
                    log(
                        LogTag::Positions,
                        "DB_SYNC_WARNING",
                        &format!(
                            "Failed to sync database after verification update for {}: {}",
                            position.symbol,
                            sync_err
                        )
                    );
                } else {
                    log(
                        LogTag::Positions,
                        "DB_SYNC_SUCCESS",
                        &format!(
                            "Database synchronized after verification update for {}",
                            position.symbol
                        )
                    );
                }

                // CRITICAL FIX: Update in-memory position to match database state
                // This prevents the "UNVERIFIED" display bug where database shows verified=1
                // but memory shows verified=false, causing UI to display stale state
                {
                    let mut positions = POSITIONS.write().await;
                    if
                        let Some(in_memory_position) = positions
                            .iter_mut()
                            .find(|p| { p.id == position.id && p.mint == position.mint })
                    {
                        // Update memory state to match database state
                        in_memory_position.transaction_entry_verified =
                            position.transaction_entry_verified;
                        in_memory_position.transaction_exit_verified =
                            position.transaction_exit_verified;

                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "MEMORY_SYNC",
                                &format!(
                                    "✅ Synchronized in-memory position {} verification status: entry={}, exit={}",
                                    position.symbol,
                                    in_memory_position.transaction_entry_verified,
                                    in_memory_position.transaction_exit_verified
                                )
                            );
                        }
                    } else {
                        log(
                            LogTag::Positions,
                            "MEMORY_SYNC_WARNING",
                            &format!(
                                "⚠️ Could not find in-memory position {} (ID: {}) to sync verification status",
                                position.symbol,
                                position.id.unwrap_or(0)
                            )
                        );
                    }
                }

                // Save closing token snapshot if this is an exit transaction verification
                if position.transaction_exit_verified && position.id.is_some() {
                    let position_id = position.id.unwrap();
                    let mint_clone = position.mint.clone();
                    tokio::spawn(async move {
                        if
                            let Err(e) = save_position_token_snapshot(
                                position_id,
                                &mint_clone,
                                "closing"
                            ).await
                        {
                            log(
                                LogTag::Positions,
                                "SNAPSHOT_WARN",
                                &format!(
                                    "Failed to save closing snapshot for {}: {}",
                                    safe_truncate(&mint_clone, 8),
                                    e
                                )
                            );
                        }
                    });
                }

                // Only remove from pending verifications AFTER successful database update
                // Note: Final cleanup at function end ensures this always happens
                {
                    let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
                    pending_verifications.remove(signature);

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🗑️ Removed {} from pending verifications after successful DB update", signature)
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

    // CRITICAL: Always ensure pending verification cleanup and critical operation cleanup
    // regardless of verification outcome or any earlier failures
    {
        let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
        let was_pending = pending_verifications.remove(signature);

        if was_pending.is_some() && is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "CLEANUP",
                &format!("🗑️ Final cleanup: Removed {} from pending verifications", signature)
            );
        }
    }

    // Always cleanup critical operation marking for any discovered mint
    if let Some(ref mint) = position_mint {
        unmark_critical_operation(mint).await;

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "CLEANUP",
                &format!(
                    "🧹 Final cleanup: Unmarked critical operation for mint {}",
                    safe_truncate(mint, 8)
                )
            );
        }
    }

    if verified {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "SUCCESS",
                &format!("✅ Comprehensive verification completed for transaction {}", signature)
            );
        }

        Ok(true)
    } else {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "WARNING",
                &format!("⚠️ No matching position found for transaction {}", signature)
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
        Ok(positions) => positions,
        Err(e) => {
            log(
                LogTag::Positions,
                "DB_FALLBACK",
                &format!("Database query failed, using memory: {}", e)
            );
            let positions = POSITIONS.read().await;
            positions
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
        Ok(positions) => positions,
        Err(e) => {
            log(
                LogTag::Positions,
                "DB_FALLBACK",
                &format!("Database query failed, using memory: {}", e)
            );
            let positions = POSITIONS.read().await;
            positions
                .iter()
                .filter(|p| {
                    // Only truly closed positions (verified exit)
                    p.transaction_exit_verified
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
            let positions = POSITIONS.read().await;
            positions.iter().any(|p| {
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
    let frozen_cooldowns = FROZEN_COOLDOWNS.read().await;
    let now = Utc::now();
    frozen_cooldowns
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

// ==================== BACKGROUND TASKS ====================

/// Start background position management tasks
pub async fn run_background_position_tasks(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🚀 Starting background position tasks");

    // Task 1: Verify pending transactions in parallel
    tokio::spawn(async move {
        verify_pending_transactions_parallel(shutdown).await;
    });
}

/// Verify pending transactions with parallel processing
async fn verify_pending_transactions_parallel(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🔍 Starting parallel transaction verification task");

    // Helper: classify transient (retryable) verification errors that should KEEP the signature queued
    // These represent propagation delays, incomplete analysis, or missing swap parsing that can succeed later.
    fn is_transient_verification_error(msg: &str) -> bool {
        let m = msg.to_lowercase();
        return (
            m.contains("within propagation grace") ||
            m.contains("still pending") ||
            m.contains("within propagation") ||
            m.contains("not found in system") || // pre-timeout missing tx
            m.contains("will retry") || // explicit retry hint (swap analysis missing)
            m.contains("no valid swap analysis") ||
            m.contains("error getting transaction") || // intermittent RPC fetch issues
            m.contains("transaction manager not available") ||
            m.contains("transaction manager not initialized")
        );
    }

    let mut first_cycle = true;
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping transaction verification task");
                break;
            }
            _ = sleep(if first_cycle { Duration::from_secs(5) } else { Duration::from_secs(15) }) => {
                if first_cycle {
                    first_cycle = false;
                    log(LogTag::Positions, "VERIFICATION_ACCELERATE", "🚀 Running accelerated first verification cycle (5s)");
                } else {
                    log(LogTag::Positions, "VERIFICATION_CYCLE", "🔄 Starting verification cycle (15s interval for responsive processing)");
                }
                // GUARD: Re-enqueue any exit signatures that are set but not yet verified and missing from pending queue
                {
                    // Collect missing exit sigs with their position metadata
                    let mut to_enqueue: Vec<(String, String, i64)> = Vec::new(); // (signature, mint, position_id)
                    {
                        let positions = POSITIONS.read().await;
                        let pending_verifications = PENDING_VERIFICATIONS.read().await;
                        for p in &*positions {
                            if let Some(sig) = &p.exit_transaction_signature {
                                if !p.transaction_exit_verified && !pending_verifications.contains_key(sig) {
                                    to_enqueue.push((sig.clone(), p.mint.clone(), p.id.unwrap_or(0)));
                                }
                            }
                        }
                    }
                    if !to_enqueue.is_empty() {
                        for (sig, mint, position_id) in &to_enqueue {
                            enqueue_for_verification(
                                sig.clone(),
                                mint.clone(),
                                *position_id,
                                false, // is_entry: false for exit
                                None, // No block height available for guard re-enqueue
                            ).await;
                        }
                        log(
                            LogTag::Positions,
                            "VERIFICATION_GUARD_REQUEUE",
                            &format!(
                                "🛡️ Re-enqueued {} missing exit verifications using new system: {}",
                                to_enqueue.len(),
                                to_enqueue.iter().map(|(sig, _, _)| sig.as_str()).collect::<Vec<_>>().join(", ")
                            )
                        );
                    }
                }
                // First, cleanup stale pending verifications using block height expiration
                let expired_sigs: Vec<String> = {
                    let mut to_remove = Vec::new();
                    let pending_verifications = PENDING_VERIFICATIONS.read().await;
                    
                    for (sig, (mint, position_id, is_entry, verification_data)) in pending_verifications.iter() {
                        if is_transaction_expired(verification_data).await.unwrap_or(false) {
                            to_remove.push(sig.clone());
                        }
                    }
                    to_remove
                };

                if !expired_sigs.is_empty() {
                    let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
                    for sig in &expired_sigs {
                        pending_verifications.remove(sig);
                    }
                    log(
                        LogTag::Positions,
                        "CLEANUP",
                        &format!("🧹 Cleaned up {} expired pending verifications using block height expiration", expired_sigs.len())
                    );

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🗑️ Expired signatures removed: {}",
                                expired_sigs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
                        );
                    }
                }

                // Get batch of pending verifications with prioritization for recent transactions
                let pending_sigs: Vec<String> = {
                    let pending_verifications = PENDING_VERIFICATIONS.read().await;
                    let now = Utc::now();

                    // Sort by age - prioritize newest transactions (within 60 seconds) for responsive verification
                    let mut sig_ages: Vec<(String, i64)> = pending_verifications
                        .iter()
                        .map(|(sig, (mint, position_id, is_entry, verification_data))| {
                            let age_seconds = now.signed_duration_since(verification_data.timestamp).num_seconds();
                            (sig.clone(), age_seconds)
                        })
                        .collect();

                    // Sort by age: recent transactions (0-60s) first, then older ones
                    sig_ages.sort_by(|a, b| {
                        match (a.1 <= 60, b.1 <= 60) {
                            (true, false) => std::cmp::Ordering::Less,  // a is recent, b is old -> a first
                            (false, true) => std::cmp::Ordering::Greater, // a is old, b is recent -> b first
                            _ => a.1.cmp(&b.1) // both recent or both old -> sort by age
                        }
                    });

                    let sigs: Vec<String> = sig_ages.iter().map(|(sig, _)| sig.clone()).collect();

                    // Always log pending verifications for debugging
                    if !sigs.is_empty() {
                        let recent_count = sig_ages.iter().filter(|(_, age)| *age <= 60).count();
                        log(
                            LogTag::Positions,
                            "VERIFICATION_QUEUE",
                            &format!("📋 Found {} pending verifications ({} recent): {}",
                                sigs.len(),
                                recent_count,
                                sigs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                            )
                        );

                        if is_debug_positions_enabled() {
                            // Log detailed verification queue information with ages
                            let pending_verifications = PENDING_VERIFICATIONS.read().await;
                            for (i, sig) in sigs.iter().enumerate() {
                                if let Some((mint, position_id, is_entry, verification_data)) = pending_verifications.get(sig) {
                                    let age_seconds = now.signed_duration_since(verification_data.timestamp).num_seconds();
                                    log(
                                        LogTag::Positions,
                                        "DEBUG",
                                        &format!("📋 Queue item {}: {} (age: {}s, mint: {}, pos_id: {}, is_entry: {})",
                                            i + 1, sig, age_seconds, mint, position_id, is_entry)
                                    );
                                }
                            }
                        }
                    } else {
                        log(
                            LogTag::Positions,
                            "VERIFICATION_QUEUE",
                            "📋 No pending verifications found"
                        );
                    }

                    sigs
                };

                if !pending_sigs.is_empty() {
                    log(
                        LogTag::Positions,
                        "VERIFICATION_BATCH_START",
                        &format!("🔍 Processing {} pending verifications", pending_sigs.len())
                    );

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🔍 Starting verification processing for {} transactions", pending_sigs.len())
                        );
                    }

                    // Process verifications in batches
                    for (batch_index, batch) in pending_sigs.chunks(VERIFICATION_BATCH_SIZE).enumerate() {
                        log(
                            LogTag::Positions,
                            "VERIFICATION_BATCH",
                            &format!("🔄 Processing batch {} of {} transactions: {}",
                                batch_index + 1,
                                batch.len(),
                                batch.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                            )
                        );

                        if is_debug_positions_enabled() {
                            log(
                                LogTag::Positions,
                                "DEBUG",
                                &format!("🔄 Batch {} details: {} transactions, batch size limit: {}",
                                    batch_index + 1, batch.len(), VERIFICATION_BATCH_SIZE)
                            );
                        }

                        let batch_futures: Vec<_> = batch.iter().map(|sig| {
                            let sig_clone = sig.clone();
                            async move {
                                if is_debug_positions_enabled() {
                                    log(
                                        LogTag::Positions,
                                        "DEBUG",
                                        &format!("🔍 Starting verification attempt for {}", sig_clone)
                                    );
                                }

                                log(
                                    LogTag::Positions,
                                    "VERIFICATION_ATTEMPT",
                                    &format!("🔍 Attempting verification for {}", sig_clone)
                                );

                                match verify_position_transaction(&sig_clone).await {
                                    Ok(verified) => {
                                        if verified {
                                            if is_debug_positions_enabled() {
                                                log(
                                                    LogTag::Positions,
                                                    "DEBUG",
                                                    &format!("✅ Verification completed successfully for {}", sig_clone)
                                                );
                                            }
                                            log(
                                                LogTag::Positions,
                                                "VERIFICATION_SUCCESS",
                                                &format!("✅ Transaction {} verified", sig_clone)
                                            );
                                            Some(sig_clone)
                                        } else {
                                            if is_debug_positions_enabled() {
                                                log(
                                                    LogTag::Positions,
                                                    "DEBUG",
                                                    &format!("⚠️ Verification returned false for {}", sig_clone)
                                                );
                                            }
                                            None
                                        }
                                    }
                                    Err(e) => {
                                        // CRITICAL: Always log verification failures to catch silent issues
                                        log(
                                            LogTag::Positions,
                                            "VERIFICATION_ERROR",
                                            &format!("❌ Verification failed for {}: {}", sig_clone, e)
                                        );

                                        if is_debug_positions_enabled() {
                                            log(
                                                LogTag::Positions,
                                                "DEBUG",
                                                &format!("❌ Verification failed for {}: {}", sig_clone, e)
                                            );
                                        }

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
                                            // NEW LOGIC: distinguish entry vs exit. Never delete position on exit permanent failure; instead revert state if tokens remain.
                                            let is_exit_permanent = {
                                                let positions = POSITIONS.read().await;
                                                positions.iter().any(|p| p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone))
                                            };

                                            if is_exit_permanent {
                                                if is_debug_positions_enabled() {
                                                    log(
                                                        LogTag::Positions,
                                                        "DEBUG",
                                                        &format!("🛑 Permanent EXIT tx failure detected for {} ({}). Retaining position & scheduling retry.", sig_clone, e)
                                                    );
                                                }

                                                log(
                                                    LogTag::Positions,
                                                    "PERMANENT_EXIT_FAILURE",
                                                    &format!("🛑 Exit transaction permanent failure: {} (error: {}). Will revert exit attempt if tokens still present and retry.", sig_clone, e)
                                                );

                                                // Revert / clear exit signature if tx failed and tokens still in wallet so we can attempt a new close later
                                                let mut cleared_for_retry = false;
                                                {
                                                    let mut positions = POSITIONS.write().await;
                                                    if let Some(position) = positions.iter_mut().find(|p| p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)) {
                                                        // Check wallet balance to decide whether to clear signature
                                                        let mut clear_sig = false;
                                                        if let Ok(wallet_address) = get_wallet_address() {
                                                            if let Ok(balance) = get_token_balance(&wallet_address, &position.mint).await {
                                                                if balance > 0 { clear_sig = true; }
                                                            }
                                                        }
                                                        if clear_sig {
                                                            position.exit_transaction_signature = None; // allow re-close attempt
                                                            position.transaction_exit_verified = false;
                                                            position.closed_reason = Some("exit_permanent_failure_retry".to_string());
                                                            cleared_for_retry = true;
                                                        } else {
                                                            // No tokens left -> treat as synthetic exit (we effectively sold or tokens gone)
                                                            position.synthetic_exit = true;
                                                            position.transaction_exit_verified = true; // mark as verified to prevent infinite loop
                                                            position.closed_reason = Some("synthetic_exit_permanent_failure".to_string());
                                                            position.exit_time = Some(Utc::now());
                                                        }
                                                    }
                                                }

                                                if cleared_for_retry {
                                                    // Persist cleared signature + closed_reason update to DB and remove old signature from index
                                                    {
                                                        let positions = POSITIONS.read().await;
                                                        if let Some(position) = positions.iter().find(|p| p.closed_reason.as_deref() == Some("exit_permanent_failure_retry")) {
                                                            if let Some(id) = position.id {
                                                                let _ = update_position(position).await; // best-effort
                                                            }
                                                        }
                                                    }
                                                    {
                                                        // Remove the failed signature mapping (if still present)
                                                        let mut sig_index = SIG_TO_MINT_INDEX.write().await;
                                                        sig_index.remove(&sig_clone);
                                                    }

                                                    // Spawn background retry attempt
                                                    tokio::spawn(async move {
                                                        sleep(Duration::from_secs(5)).await;
                                                        // Snapshot positions and pick one flagged for retry
                                                        let positions_snapshot = POSITIONS.read().await;
                                                        if let Some(position) = positions_snapshot.iter().find(|p| p.closed_reason.as_deref() == Some("exit_permanent_failure_retry") && p.exit_transaction_signature.is_none()) {
                                                            if let Some(token_obj) = get_token_from_db(&position.mint).await {
                                                                if let Some(current_price_info) = get_pool_price(&position.mint) {
                                                                    let price = current_price_info.price_sol;
                                                                    if price > 0.0 && price.is_finite() {
                                                                    let reason = format!("Retry after permanent exit failure for {}", position.symbol);
                                                                    let _ = close_position_direct(&position.mint, reason).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    });
                                                }

                                                // Return signature to remove from pending queue only (position retained)
                                                Some(sig_clone)
                                            } else {
                                                // ENTRY permanent failure: keep legacy behavior (remove orphan position to free capital)
                                                if is_debug_positions_enabled() {
                                                    log(
                                                        LogTag::Positions,
                                                        "DEBUG",
                                                        &format!("🗑️ Permanent ENTRY failure detected for {}, initiating cleanup", sig_clone)
                                                    );
                                                }
                                                log(
                                                    LogTag::Positions,
                                                    "PERMANENT_FAILURE_CLEANUP",
                                                    &format!("🗑️ Removing position with permanent entry failure: {} (error: {})", sig_clone, e)
                                                );
                                                tokio::spawn({
                                                    let sig_for_cleanup = sig_clone.clone();
                                                    async move {
                                                        if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                            log(
                                                                LogTag::Positions,
                                                                "CLEANUP_ERROR",
                                                                &format!("Failed to remove position with signature {}: {}", sig_for_cleanup, cleanup_err)
                                                            );
                                                        }
                                                    }
                                                });
                                                Some(sig_clone)
                                            }
                                        } else {
                                            // Check verification age before removing position
                                            let verification_age_seconds = {
                                                let now = Utc::now();
                                                let pending_verifications = PENDING_VERIFICATIONS.read().await;
                                                if let Some((_, _, _, verification_data)) = pending_verifications.get(&sig_clone) {
                                                    now.signed_duration_since(verification_data.timestamp).num_seconds()
                                                } else {
                                                    0 // If not found, treat as new
                                                }
                                            };

                                            if is_debug_positions_enabled() {
                                                log(
                                                    LogTag::Positions,
                                                    "DEBUG",
                                                    &format!("⏰ Verification age for {}: {}s", sig_clone, verification_age_seconds)
                                                );
                                            }

                                            // Progressive timeout handling - different timeouts for different situations
                                            let is_exit_transaction = {
                                                let positions = POSITIONS.read().await;
                                                positions.iter().any(|p| {
                                                    p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)
                                                })
                                            };

                                            if is_debug_positions_enabled() {
                                                log(
                                                    LogTag::Positions,
                                                    "DEBUG",
                                                    &format!("🔍 Transaction type for {}: {} (timeout: {}s)",
                                                        sig_clone,
                                                        if is_exit_transaction { "Exit" } else { "Entry" },
                                                        if is_exit_transaction { 60 } else { 90 })
                                                );
                                            }

                                            // EARLY CLASSIFICATION: transient / retryable errors stay queued (return None)
                                            if is_transient_verification_error(&e) {
                                                if is_debug_positions_enabled() {
                                                    log(
                                                        LogTag::Positions,
                                                        "DEBUG",
                                                        &format!("🔄 Transient error detected for {}, keeping in queue", sig_clone)
                                                    );
                                                }

                                                log(
                                                    LogTag::Positions,
                                                    "VERIFICATION_RETRY_KEEP",
                                                    &format!(
                                                        "🔄 Keeping {} in pending queue (transient error, age {}s): {}",
                                                        sig_clone,
                                                        verification_age_seconds,
                                                        e
                                                    )
                                                );
                                                return None; // Keep signature in pending_verifications
                                            }

                                            let timeout_threshold = if is_exit_transaction {
                                                60 // 1 minute for exit transactions
                                            } else {
                                                90 // 1.5 minutes for entry transactions
                                            };

                                            // Only remove if truly timed out (beyond progressive grace period)
                                            if e.contains("Verification timeout:") || e.contains("verification timeout") ||
                                               (e.contains("Transaction not found in system") && verification_age_seconds > timeout_threshold) {

                                                if is_debug_positions_enabled() {
                                                    log(
                                                        LogTag::Positions,
                                                        "DEBUG",
                                                        &format!("⏰ Timeout condition met for {} (age: {}s, threshold: {}s)",
                                                            sig_clone, verification_age_seconds, timeout_threshold)
                                                    );
                                                }

                                                // Use the is_exit_transaction we already determined above

                                                if is_exit_transaction {
                                                    // For exit transaction failures, check wallet balance before removing position
                                                    if is_debug_positions_enabled() {
                                                        log(
                                                            LogTag::Positions,
                                                            "DEBUG",
                                                            &format!("🔍 Exit transaction timeout for {}, checking wallet balance", sig_clone)
                                                        );
                                                    }

                                                    log(
                                                        LogTag::Positions,
                                                        "EXIT_VERIFICATION_TIMEOUT",
                                                        &format!("⏰ Exit transaction {} verification timeout - checking wallet balance before cleanup", sig_clone)
                                                    );

                                                    // Find the position and check wallet balance
                                                    let should_remove_position = {
                                                        let positions = POSITIONS.read().await;
                                                        if let Some(position) = positions.iter().find(|p| {
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
                                                            &format!("🗑️ Removing position with failed exit verification: {} (error: {}, age: {}s)", sig_clone, e, verification_age_seconds)
                                                        );

                                                        tokio::spawn({
                                                            let sig_for_cleanup = sig_clone.clone();
                                                            async move {
                                                                if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                                    log(
                                                                        LogTag::Positions,
                                                                        "CLEANUP_ERROR",
                                                                        &format!("Failed to remove position with signature {}: {}", sig_for_cleanup, cleanup_err)
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
                                                            &format!("🔄 Keeping position with failed exit verification: {} - tokens still in wallet", sig_clone)
                                                        );

                                                                                                // Mark the exit transaction as failed but keep the position and schedule retry
                                        {
                                            let mut positions = POSITIONS.write().await;
                                            if let Some(position) = positions.iter_mut().find(|p| {
                                                p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(&sig_clone)
                                            }) {
                                                // FIXED: Check if transaction actually exists before preserving signature
                                                let transaction_exists = get_transaction(&sig_clone).await
                                                    .map(|opt| opt.is_some())
                                                    .unwrap_or(false);

                                                if transaction_exists {
                                                    // Transaction exists on blockchain - preserve signature, retry verification only
                                                    position.closed_reason = Some("exit_verification_retry_pending".to_string());
                                                    log(
                                                        LogTag::Positions,
                                                        "EXIT_VERIFICATION_RETRY",
                                                        &format!("🔄 {} exit transaction exists but verification failed - preserving signature", position.symbol)
                                                    );
                                                } else {
                                                    // Transaction doesn't exist - clear signature and allow new sell attempt
                                                    position.exit_transaction_signature = None;
                                                    position.closed_reason = Some("exit_retry_pending".to_string());
                                                    log(
                                                        LogTag::Positions,
                                                        "EXIT_RETRY_SCHEDULED",
                                                        &format!("🔄 {} exit transaction not found - clearing signature for retry", position.symbol)
                                                    );
                                                }
                                                            }
                                                        }

                                                        // Spawn background retry if signature was cleared (meaning tokens still present and we plan to re-attempt close)
                                                        {
                                                            let positions = POSITIONS.read().await;
                                                            if let Some(pos) = positions.iter().find(|p| p.exit_transaction_signature.is_none() && p.closed_reason.as_deref() == Some("exit_retry_pending")) {
                                                                let mint_retry = pos.mint.clone();
                                                                let symbol_retry = pos.symbol.clone();

                                                                // Persist state change (cleared signature) to DB
                                                                if let Some(id) = pos.id { let _ = update_position(pos).await; }
                                                                // Remove old failed signature from index
                                                                {
                                                                    let mut sig_index = SIG_TO_MINT_INDEX.write().await;
                                                                    sig_index.remove(&sig_clone);
                                                                }

                                                                tokio::spawn(async move {
                                                                    sleep(Duration::from_secs(5)).await; // small delay
                                                                    if let Some(token_obj) = get_token_from_db(&mint_retry).await {
                                                                        if let Some(current_price_info) = get_pool_price(&mint_retry) {
                                                                            let price = current_price_info.price_sol;
                                                                            if price > 0.0 && price.is_finite() {
                                                                            let reason = format!("Retry after failed exit verification for {}", symbol_retry);
                                                                            let _ = close_position_direct(&mint_retry, reason).await;
                                                                            }
                                                                        }
                                                                    }
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
                                                        &format!("🗑️ Removing position with failed entry verification: {} (error: {}, age: {}s)", sig_clone, e, verification_age_seconds)
                                                    );

                                                    tokio::spawn({
                                                        let sig_for_cleanup = sig_clone.clone();
                                                        async move {
                                                            if let Err(cleanup_err) = remove_position_by_signature(&sig_for_cleanup).await {
                                                                log(
                                                                    LogTag::Positions,
                                                                    "CLEANUP_ERROR",
                                                                    &format!("Failed to remove position with signature {}: {}", sig_for_cleanup, cleanup_err)
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
                                                    &format!("❌ Failed to verify {}: {}", sig_clone, e)
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
                            let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
                            for sig in &completed_sigs {
                                pending_verifications.remove(sig);
                            }
                            log(
                                LogTag::Positions,
                                "VERIFICATION_CLEANUP",
                                &format!("🧹 Removed {} completed verifications from pending queue: {}",
                                    completed_sigs.len(),
                                    completed_sigs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                                )
                            );
                        }
                    }
                } else {
                    log(
                        LogTag::Positions,
                        "VERIFICATION_QUEUE_EMPTY",
                        "🆗 No pending verifications to process"
                    );
                }
            }
        }
    }
}

/// Retry failed operations with parallel processing
// Removed - simplified architecture to focus only on verification

// ==================== HELPER FUNCTIONS ====================

/// Get token balance safely with error handling
async fn get_token_balance_safe(mint: &str, wallet_address: &str) -> Option<u64> {
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => Some(balance),
        Err(e) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "WARNING",
                    &format!("Failed to get token balance for {}: {}", mint, e)
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
    initialize_positions_database().await.map_err(|e|
        format!("Failed to initialize positions database: {}", e)
    )?;

    // Load existing positions from database into memory
    match load_all_positions().await {
        Ok(positions) => {
            // Get all necessary write locks
            let mut global_positions = POSITIONS.write().await;
            let mut pending_verifications = PENDING_VERIFICATIONS.write().await;
            let mut sig_to_mint_index = SIG_TO_MINT_INDEX.write().await;
            let mut mint_to_position_index = MINT_TO_POSITION_INDEX.write().await;

            // Add unverified positions to pending verification queue
            let mut unverified_count = 0;
            for position in &positions {
                // Check if entry transaction needs verification
                if !position.transaction_entry_verified {
                    if let Some(entry_sig) = &position.entry_transaction_signature {
                        // For startup requeue, we don't have the original block height,
                        // so we'll use None to fall back to time-based verification
                        enqueue_for_verification(
                            entry_sig.clone(),
                            position.mint.clone(),
                            position.id.unwrap_or(0),
                            true, // is_entry: true for entry
                            None // No block height available from startup
                        ).await;
                        log(
                            LogTag::Positions,
                            "VERIFICATION_REQUEUE_ENTRY",
                            &format!(
                                "♻️ Startup requeue ENTRY {} for {} using new system",
                                entry_sig,
                                safe_truncate(&position.symbol, 8)
                            )
                        );
                        unverified_count += 1;
                    }
                }
                // Check if exit transaction needs verification
                if !position.transaction_exit_verified {
                    if let Some(exit_sig) = &position.exit_transaction_signature {
                        // For startup requeue, we don't have the original block height,
                        // so we'll use None to fall back to time-based verification
                        enqueue_for_verification(
                            exit_sig.clone(),
                            position.mint.clone(),
                            position.id.unwrap_or(0),
                            false, // is_entry: false for exit
                            None // No block height available from startup
                        ).await;
                        log(
                            LogTag::Positions,
                            "VERIFICATION_REQUEUE_EXIT",
                            &format!(
                                "♻️ Startup requeue EXIT {} for {} using new system",
                                exit_sig,
                                safe_truncate(&position.symbol, 8)
                            )
                        );
                        unverified_count += 1;
                    }
                }
            }

            // Populate positions and rebuild indexes
            *global_positions = positions;

            // Rebuild signature-to-mint index
            sig_to_mint_index.clear();
            for position in global_positions.iter() {
                if let Some(ref entry_sig) = position.entry_transaction_signature {
                    sig_to_mint_index.insert(entry_sig.clone(), position.mint.clone());
                }
                if let Some(ref exit_sig) = position.exit_transaction_signature {
                    sig_to_mint_index.insert(exit_sig.clone(), position.mint.clone());
                }
            }

            // Rebuild mint-to-position index
            mint_to_position_index.clear();
            for (index, position) in global_positions.iter().enumerate() {
                mint_to_position_index.insert(position.mint.clone(), index);
            }

            log(
                LogTag::Positions,
                "STARTUP",
                &format!("✅ Loaded {} positions from database", global_positions.len())
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

    log(LogTag::Positions, "STARTUP", "✅ Positions system initialized");
    Ok(())
}

/// Start the positions manager service
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) -> Result<(), String> {
    log(LogTag::Positions, "STARTUP", "🚀 Starting positions manager service");

    // Initialize the system first
    initialize_positions_system().await?;

    // Start background tasks
    run_background_position_tasks(shutdown).await;

    Ok(())
}

// ==================== SAFETY HELPERS ====================

/// Attempt to recover a position by finding its sell transaction and using full verification flow
/// This handles cases where tokens were sold but position wasn't properly closed
/// Uses the same verification workflow as normal position closing for consistency and full P&L calculation
pub async fn attempt_position_recovery_from_transactions(
    mint: &str,
    symbol: &str
) -> Result<String, String> {
    let _lock = acquire_position_lock(mint).await;

    log(
        LogTag::Positions,
        "RECOVERY_START",
        &format!(
            "🔍 Starting position recovery for {} (mint: {})",
            symbol,
            crate::utils::safe_truncate(mint, 8)
        )
    );

    // First, find the position that needs recovery
    let position = {
        let positions = POSITIONS.read().await;
        positions
            .iter()
            .find(|p| p.mint == mint && p.exit_transaction_signature.is_none())
            .cloned()
    };

    let position = match position {
        Some(pos) => pos,
        None => {
            return Err("No open position found for this token".to_string());
        }
    };

    log(
        LogTag::Positions,
        "RECOVERY_POSITION",
        &format!(
            "🎯 Found position to recover: {} (ID: {}, token_amount: {:?})",
            symbol,
            position.id.unwrap_or(0),
            position.token_amount
        )
    );

    // Search for recent SwapTokenToSol transactions for this token
    let transaction_manager = match get_global_transaction_manager().await {
        Some(manager_guard) => manager_guard,
        None => {
            return Err("Transaction manager not available".to_string());
        }
    };

    // Get recent sell transactions for this token using transaction manager
    let signatures = {
        let manager = transaction_manager.lock().await;
        if let Some(ref manager) = *manager {
            if let Some(ref db) = manager.transaction_database {
                // Try standard search first
                match
                    db.get_swap_signatures_for_token(mint, Some("SwapTokenToSol"), Some(20)).await
                {
                    Ok(sigs) if !sigs.is_empty() => sigs,
                    _ => {
                        // Fall back to broader search if standard search fails
                        log(
                            LogTag::Positions,
                            "RECOVERY_FALLBACK_SEARCH",
                            &format!("Standard search failed for {}, trying broader search", symbol)
                        );

                        // Search more broadly in transaction_type field for tokens with missing metadata
                        match db.get_swap_signatures_for_token_fallback(mint, Some(20)).await {
                            Ok(sigs) => sigs,
                            Err(e) => {
                                return Err(
                                    format!("Failed to search transactions (fallback): {}", e)
                                );
                            }
                        }
                    }
                }
            } else {
                return Err("Transaction database not available".to_string());
            }
        } else {
            return Err("Transaction manager not initialized".to_string());
        }
    };

    if signatures.is_empty() {
        return Err("No recent sell transactions found".to_string());
    }

    log(
        LogTag::Positions,
        "RECOVERY_SEARCH",
        &format!("🔍 Found {} potential sell transactions to check", signatures.len())
    );

    // Check each transaction to find the one that matches our position
    // ENHANCED FILTERING: Use comprehensive transaction-to-position matching
    let mut candidate_transactions = Vec::new();

    for signature in signatures.iter() {
        log(
            LogTag::Positions,
            "RECOVERY_CHECK_TX",
            &format!("🔍 Checking transaction {}", signature)
        );

        // Validate transaction exists and is successful using priority transaction access
        // This bypasses the busy manager issue and ensures we get properly analyzed transactions
        match get_transaction(&signature).await {
            Ok(Some(transaction)) => {
                // Verify transaction is successful and finalized
                if
                    !transaction.success ||
                    !matches!(
                        transaction.status,
                        TransactionStatus::Confirmed | TransactionStatus::Finalized
                    )
                {
                    log(
                        LogTag::Positions,
                        "RECOVERY_SKIP_TX",
                        &format!("⚠️ Skipping failed/pending transaction {}", signature)
                    );
                    continue;
                }

                // Convert transaction to SwapPnLInfo using the same method as verification
                let swap_pnl_info = {
                    let manager = transaction_manager.lock().await;
                    if let Some(ref manager) = *manager {
                        let empty_cache = std::collections::HashMap::new();
                        manager.convert_to_swap_pnl_info(&transaction, &empty_cache, false)
                    } else {
                        None
                    }
                };

                // Check if this transaction is a valid sell for our token
                if let Some(swap_info) = swap_pnl_info {
                    if swap_info.swap_type == "Sell" && swap_info.token_mint == mint {
                        // CRITICAL TIME-BASED FILTERING: Only consider transactions AFTER position entry
                        if swap_info.timestamp <= position.entry_time {
                            log(
                                LogTag::Positions,
                                "RECOVERY_SKIP_TIME",
                                &format!(
                                    "⏰ Skipping pre-entry transaction: {} (tx: {}, pos entry: {})",
                                    signature,
                                    swap_info.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                    position.entry_time.format("%Y-%m-%d %H:%M:%S")
                                )
                            );
                            continue;
                        }

                        // AMOUNT-BASED SCORING: Calculate how well the transaction amount matches position
                        let expected_tokens = position.token_amount.unwrap_or(0) as f64; // Position token amount
                        let actual_tokens = swap_info.token_amount.abs(); // Transaction token amount
                        let amount_diff = (actual_tokens - expected_tokens).abs();
                        let amount_ratio = if expected_tokens > 0.0 {
                            amount_diff / expected_tokens
                        } else {
                            f64::INFINITY
                        };

                        // TIME PROXIMITY SCORING: Prefer transactions closer to entry time
                        let time_diff_seconds = (
                            swap_info.timestamp - position.entry_time
                        ).num_seconds() as f64;

                        // WALLET ADDRESS VERIFICATION: Ensure this transaction is from our wallet
                        let wallet_address = match get_wallet_address() {
                            Ok(addr) => addr,
                            Err(e) => {
                                log(
                                    LogTag::Positions,
                                    "ERROR",
                                    &format!("Failed to load wallet address for verification: {}", e)
                                );
                                continue;
                            }
                        };

                        // Verify the transaction involves our wallet address
                        // Check if our wallet is involved in any token transfers
                        let mut is_our_transaction = false;
                        for transfer in &transaction.token_transfers {
                            if transfer.from == wallet_address || transfer.to == wallet_address {
                                is_our_transaction = true;
                                break;
                            }
                        }

                        // Also check SOL balance changes
                        if !is_our_transaction {
                            for balance_change in &transaction.sol_balance_changes {
                                if balance_change.account == wallet_address {
                                    is_our_transaction = true;
                                    break;
                                }
                            }
                        }

                        if !is_our_transaction {
                            log(
                                LogTag::Positions,
                                "RECOVERY_SKIP_WALLET",
                                &format!("🚫 Skipping transaction from different wallet: {}", signature)
                            );
                            continue;
                        }

                        // Calculate composite score: prioritize amount accuracy, then time proximity
                        let composite_score = amount_ratio + (time_diff_seconds / 86400.0) * 0.1; // Time factor

                        candidate_transactions.push((
                            swap_info.clone(),
                            amount_ratio,
                            time_diff_seconds,
                            composite_score,
                            signature.clone(),
                            transaction.clone(),
                        ));

                        log(
                            LogTag::Positions,
                            "RECOVERY_CANDIDATE",
                            &format!(
                                "📊 Candidate transaction: {} - Amount: {:.2} vs pos {:.2} (ratio: {:.4}), Time: +{:.0}s, Score: {:.4}",
                                signature,
                                actual_tokens,
                                expected_tokens,
                                amount_ratio,
                                time_diff_seconds,
                                composite_score
                            )
                        );
                    }
                }
            }
            Ok(None) => {
                log(
                    LogTag::Positions,
                    "RECOVERY_TX_NOT_FOUND",
                    &format!("⚠️ Transaction {} not found in database", signature)
                );
                continue;
            }
            Err(e) => {
                log(
                    LogTag::Positions,
                    "RECOVERY_ERROR_TX",
                    &format!("❌ Failed to get transaction {}: {}", signature, e)
                );
                continue;
            }
        }
    }

    // Sort candidates by composite score (best match first)
    candidate_transactions.sort_by(|a, b|
        a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal)
    );

    log(
        LogTag::Positions,
        "RECOVERY_CANDIDATES",
        &format!(
            "🎯 Found {} candidate transactions for position {} ({}), sorted by best match",
            candidate_transactions.len(),
            position.id.unwrap_or(0),
            symbol
        )
    );

    // Use the best matching candidate if it meets quality criteria
    if
        let Some(
            (best_swap_info, amount_ratio, time_diff, score, best_signature, best_transaction),
        ) = candidate_transactions.first()
    {
        // Quality threshold: require reasonable amount matching (allow 15% difference)
        if *amount_ratio < 0.15 {
            log(
                LogTag::Positions,
                "RECOVERY_BEST_MATCH",
                &format!(
                    "🏆 Using best match {} for position {}: amount ratio {:.4}, time +{:.0}s, score {:.4}",
                    best_signature,
                    position.id.unwrap_or(0),
                    amount_ratio,
                    time_diff,
                    score
                )
            );

            // CRITICAL: Set the exit transaction signature and use the NORMAL verification flow
            // This ensures the position gets fully verified with all the same calculations
            // as a normal position close, including proper P&L calculation

            // Update position with exit transaction signature
            {
                let mut positions = POSITIONS.write().await;
                if let Some(pos) = positions.iter_mut().find(|p| p.mint == mint) {
                    pos.exit_transaction_signature = Some(best_signature.clone());

                    log(
                        LogTag::Positions,
                        "RECOVERY_SET_EXIT_SIGNATURE",
                        &format!("🔄 Set exit signature for {}: {}", symbol, best_signature)
                    );
                }
            }

            // Update signature index
            {
                let mut sig_to_mint = SIG_TO_MINT_INDEX.write().await;
                sig_to_mint.insert(best_signature.clone(), mint.to_string());
            }

            // Add to verification queue using new block height system
            enqueue_for_verification(
                best_signature.clone(),
                mint.to_string(),
                position.id.unwrap_or(0),
                false, // is_entry: false for exit
                None // Recovery mode - no block height available
            ).await;
            log(
                LogTag::Positions,
                "VERIFICATION_ENQUEUE_EXIT_RECOVERY",
                &format!(
                    "📥 Enqueued EXIT (recovery) {} for {} using new system",
                    best_signature,
                    safe_truncate(&symbol, 8)
                )
            );

            // Update database with exit signature immediately
            if let Some(position_id) = position.id {
                let mut updated_position = position.clone();
                updated_position.exit_transaction_signature = Some(best_signature.clone());

                match update_position(&updated_position).await {
                    Ok(_) => {
                        log(
                            LogTag::Positions,
                            "RECOVERY_EXIT_SIGNATURE_SAVED",
                            &format!("✅ Exit signature saved for {} in database", symbol)
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "RECOVERY_EXIT_SIGNATURE_ERROR",
                            &format!("❌ Failed to save exit signature for {}: {}", symbol, e)
                        );
                        // Continue with verification anyway
                    }
                }
            }

            log(
                LogTag::Positions,
                "RECOVERY_START_VERIFICATION",
                &format!("🔍 Starting full verification workflow for recovered position {}", symbol)
            );

            // CRITICAL FIX: Release the position lock BEFORE calling verification
            // to prevent deadlock when verification tries to acquire the same lock
            drop(_lock);

            // Use the same comprehensive verification as normal position closing
            let verification_result = verify_position_transaction(&best_signature).await;
            match verification_result {
                Ok(true) => {
                    log(
                        LogTag::Positions,
                        "RECOVERY_VERIFICATION_SUCCESS",
                        &format!("✅ Position recovery completed successfully for {}", symbol)
                    );
                    return Ok(best_signature.clone());
                }
                Ok(false) => {
                    log(
                        LogTag::Positions,
                        "RECOVERY_VERIFICATION_INCOMPLETE",
                        &format!("⚠️ Position recovery verification incomplete for {} - will retry", symbol)
                    );
                    // Don't return error - verification is in progress
                    return Ok(best_signature.clone());
                }
                Err(e) => {
                    log(
                        LogTag::Positions,
                        "RECOVERY_VERIFICATION_ERROR",
                        &format!("❌ Position recovery verification failed for {}: {}", symbol, e)
                    );
                    // Return error since we found the right transaction but verification failed
                    return Err(format!("Verification failed for matching transaction: {}", e));
                }
            }
        } else {
            log(
                LogTag::Positions,
                "RECOVERY_POOR_MATCH",
                &format!(
                    "❌ Best candidate {} has poor amount match (ratio: {:.4} > 0.15) for position {}",
                    best_signature,
                    amount_ratio,
                    position.id.unwrap_or(0)
                )
            );
        }
    } else {
        log(
            LogTag::Positions,
            "RECOVERY_NO_CANDIDATES",
            &format!(
                "❌ No valid candidate transactions found for position {} ({})",
                position.id.unwrap_or(0),
                symbol
            )
        );
    }

    Err("No matching sell transaction found for position recovery".to_string())
}
