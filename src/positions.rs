use crate::{
    tokens::{ Token, get_token_decimals, get_token_price_safe },
    swaps::{
        get_best_quote,
        execute_best_swap,
        UnifiedQuote,
        config::{ SOL_MINT, QUOTE_SLIPPAGE_PERCENT },
    },
    transactions::{ get_transaction, Transaction, TransactionStatus },
    rpc::{ lamports_to_sol, sol_to_lamports },
    errors::{ ScreenerBotError, PositionError, DataError, BlockchainError, NetworkError },
    logger::{ log, LogTag },
    arguments::{
        is_dry_run_enabled,
        is_debug_positions_enabled,
        is_debug_swaps_enabled,
        get_max_exit_retries,
    },
    trader::{ CriticalOperationGuard },
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
        println!("🔓 Released position lock for mint: {}", &self.mint[..8]);
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
const MAX_OPEN_POSITIONS: usize = 10;
const POSITION_OPEN_COOLDOWN_SECS: i64 = 0; // No global cooldown (from backup)
const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 15; // Re-entry cooldown after closing (from backup)

// Sell retry slippages (progressive)
const SELL_RETRY_SLIPPAGES: &[f64] = &[3.0, 5.0, 8.0, 12.0, 20.0];

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

    println!("🔒 Acquired position lock for mint: {}", &mint_key[..8]);

    PositionLockGuard { mint: mint_key }
}

// ==================== CORE POSITION OPERATIONS ====================

/// Open a new position directly (replaces actor message)
pub async fn open_position_direct(
    token: &Token,
    entry_price: f64,
    percent_change: f64,
    size_sol: f64
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

    // Check cooldowns and existing positions
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

        // CHECK EXISTING POSITION
        let (already_has_position, open_positions_count) = {
            let has_position = state.positions
                .iter()
                .any(|p| {
                    p.mint == token.mint &&
                        p.position_type == "buy" &&
                        p.exit_price.is_none() &&
                        p.exit_transaction_signature.is_none()
                });

            let count = state.positions
                .iter()
                .filter(|p| {
                    p.position_type == "buy" &&
                        p.exit_price.is_none() &&
                        p.exit_transaction_signature.is_none()
                })
                .count();

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "📊 Position check - existing: {}, open count: {}/{}",
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

    // Execute the buy transaction
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
            LogTag::Swap,
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
        LogTag::Swap,
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
            get_token_price_safe(&token.mint)
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
            Some(0.0)
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
                LogTag::Swap,
                "QUOTE_TIMEOUT",
                &format!("⏰ Quote request timeout for {} after 20s", token.symbol)
            );
            return Err(format!("Quote request timeout for {}", token.symbol));
        }
    };

    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "QUOTE",
            &format!(
                "📊 Best quote from {:?}: {} SOL -> {} tokens",
                best_quote.router,
                lamports_to_sol(best_quote.input_amount),
                best_quote.output_amount
            )
        );
    }

    log(
        LogTag::Swap,
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
            LogTag::Swap,
            "TRANSACTION",
            &format!("Transaction {} will be monitored by positions manager", &signature[..8])
        );
    }

    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "BUY_COMPLETE",
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
    let (profit_min, profit_max) = get_profit_target(token).await;

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
        liquidity_tier: determine_liquidity_tier(&token.mint).await.ok(),
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

    // Add position to global state and database
    let position_id = {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        // Track for comprehensive verification
        state.pending_verifications.insert(transaction_signature.clone(), Utc::now());

        // Add position to in-memory list
        state.positions.push(new_position.clone());

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "✅ Position created for {} with signature {} - profit targets: {:.2}%-{:.2}%",
                    token.symbol,
                    get_signature_prefix(&transaction_signature),
                    profit_min,
                    profit_max
                )
            );
        }

        // Save to database
        match save_position(&new_position).await {
            Ok(id) => {
                log(
                    LogTag::Positions,
                    "DB_SAVE",
                    &format!("Position saved to database with ID {}", id)
                );
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
        }
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
    exit_price: f64,
    exit_reason: String
) -> Result<String, String> {
    let _lock = acquire_position_lock(mint).await;

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔄 Attempting to close position for {} - reason: {}",
                get_mint_prefix(mint),
                exit_reason
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

    // Find position and validate
    let position_info = {
        let state = GLOBAL_POSITIONS_STATE.lock().await;
        state.positions
            .iter()
            .find(|p| {
                p.mint == mint &&
                    p.position_type == "buy" &&
                    p.exit_price.is_none() &&
                    p.exit_transaction_signature.is_none()
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

    log(
        LogTag::Swap,
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

    // Get token balance
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
            LogTag::Swap,
            "BALANCE",
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
                LogTag::Swap,
                "RETRY",
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
                        LogTag::Swap,
                        "QUOTE_SUCCESS",
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
                        LogTag::Swap,
                        "QUOTE_FAIL",
                        &format!("❌ Quote failed with {:.1}% slippage: {}", slippage, last_error)
                    );
                }
                continue;
            }
            Err(_) => {
                last_error = format!("Quote timeout with {:.1}% slippage", slippage);
                if is_debug_swaps_enabled() {
                    log(
                        LogTag::Swap,
                        "QUOTE_TIMEOUT",
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
        LogTag::Swap,
        "SWAP",
        &format!(
            "🚀 Executing sell with {:.1}% slippage via {:?}...",
            quote_slippage_used,
            quote.router
        )
    );

    // Create token object for swap execution
    let token = Token {
        mint: mint.to_string(),
        symbol: symbol.clone(),
        name: symbol.clone(), // Use symbol as name if not available
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

    // Execute the swap
    let swap_result = execute_best_swap(&token, mint, SOL_MINT, token_balance, quote).await;

    let transaction_signature = match swap_result {
        Ok(result) => {
            if let Some(ref signature) = result.transaction_signature {
                log(
                    LogTag::Swap,
                    "TRANSACTION",
                    &format!(
                        "Sell transaction {} will be monitored by positions manager",
                        &signature[..8]
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
            LogTag::Swap,
            "SELL_COMPLETE",
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

    // Update position with exit transaction
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;
        let mut position_for_db: Option<Position> = None;

        if
            let Some(position) = state.positions
                .iter_mut()
                .find(|p| p.mint == mint && p.exit_price.is_none())
        {
            position.exit_transaction_signature = Some(transaction_signature.clone());
            position.exit_time = Some(Utc::now());
            position.exit_price = Some(exit_price);
            position.closed_reason = Some(exit_reason.clone());

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "✅ Position updated with exit transaction {} for {}",
                        get_signature_prefix(&transaction_signature),
                        symbol
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

    // Immediately attempt to fetch transaction to accelerate verification
    let sig_for_fetch = transaction_signature.clone();
    tokio::spawn(async move {
        let _ = get_transaction(&sig_for_fetch).await;
    });

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ POSITION CLOSED: {} | TX: {} | Reason: {} | Verification: Pending",
            symbol,
            get_signature_prefix(&transaction_signature),
            exit_reason
        )
    );

    Ok(transaction_signature)
}

/// Update position tracking data independently
pub async fn update_position_tracking(
    mint: &str,
    current_price: f64,
    market_cap: Option<f64>,
    liquidity_tier: Option<String>
) -> Result<(), String> {
    let _lock = acquire_position_lock(mint).await;

    if is_debug_positions_enabled() {
        println!("📊 Updating tracking for {} - Price: ${:.8}", &mint[..8], current_price);
    }

    if current_price <= 0.0 || !current_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "WARN",
                &format!(
                    "Skipping position tracking update for mint {}: invalid price {:.10}",
                    get_mint_prefix(mint),
                    current_price
                )
            );
        }
        return Err("Invalid price for tracking update".to_string());
    }

    // Find and update position in global state
    let mut updated = false;
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        if let Some(position) = state.positions.iter_mut().find(|p| p.mint == mint) {
            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

            // Initialize price tracking if not set
            if position.price_highest == 0.0 {
                position.price_highest = entry_price;
                position.price_lowest = entry_price;
            }

            // Check for price change and log if significant
            let old_price = position.current_price.unwrap_or(entry_price);
            let price_change = current_price - old_price;
            let price_change_percent = if old_price != 0.0 {
                (price_change / old_price) * 100.0
            } else {
                0.0
            };

            // Log price change if significant (0.01% threshold)
            let change_threshold = if old_price > 0.0 {
                (old_price * 0.0001).max(f64::EPSILON * 100.0)
            } else {
                f64::EPSILON * 100.0
            };

            let price_diff = (old_price - current_price).abs();

            // Check if enough time has passed since last log (30 seconds)
            let time_since_last_log = position.current_price_updated
                .map(|last| (Utc::now() - last).num_seconds())
                .unwrap_or(999);
            let should_log_periodic = time_since_last_log >= 30;

            if price_diff > change_threshold || should_log_periodic {
                // Calculate current P&L for logging
                let (pnl_sol, pnl_percent) = calculate_position_pnl(
                    position,
                    Some(current_price)
                ).await;

                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "TRACKING",
                        &format!(
                            "📊 {} price: {:.10} SOL ({:+.2}%) | P&L: {:+.6} SOL ({:+.1}%)",
                            position.symbol,
                            current_price,
                            price_change_percent,
                            pnl_sol,
                            pnl_percent
                        )
                    );
                }
            }

            // Update tracking data
            position.current_price = Some(current_price);
            position.current_price_updated = Some(Utc::now());

            // Update price high/low
            if current_price > position.price_highest {
                position.price_highest = current_price;
            }
            if current_price < position.price_lowest {
                position.price_lowest = current_price;
            }

            // Update additional fields if provided
            if let Some(tier) = liquidity_tier {
                position.liquidity_tier = Some(tier);
            }

            updated = true;

            // Save to database
            if let Err(e) = sync_position_to_database(position).await {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!("Failed to sync tracking update to database: {}", e)
                );
            }
        }
    }

    if !updated {
        return Err(format!("Position not found for mint: {}", get_mint_prefix(mint)));
    }

    Ok(())
}

/// Verify a position transaction independently
pub async fn verify_position_transaction(signature: &str) -> Result<bool, String> {
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔍 Starting comprehensive verification for transaction {}",
                get_signature_prefix(signature)
            )
        );
    }

    // Get the transaction with timeout
    let transaction = match
        tokio::time::timeout(tokio::time::Duration::from_secs(30), get_transaction(signature)).await
    {
        Ok(Ok(Some(tx))) => tx,
        Ok(Ok(None)) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("❌ Transaction {} not found via RPC", get_signature_prefix(signature))
                );
            }
            return Err("Transaction not found".to_string());
        }
        Ok(Err(e)) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "❌ RPC error for transaction {}: {}",
                        get_signature_prefix(signature),
                        e
                    )
                );
            }
            return Err(format!("RPC error: {}", e));
        }
        Err(_) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("⏰ RPC timeout for transaction {}", get_signature_prefix(signature))
                );
            }
            return Err("RPC timeout".to_string());
        }
    };

    // Check transaction status
    if !transaction.success {
        if let Some(ref error_msg) = transaction.error_message {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "❌ Transaction {} failed with error: {}",
                        get_signature_prefix(signature),
                        error_msg
                    )
                );
            }
            return Err(format!("Transaction failed: {}", error_msg));
        } else {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "❌ Transaction {} failed with unknown error",
                        get_signature_prefix(signature)
                    )
                );
            }
            return Err("Transaction failed with unknown error".to_string());
        }
    }

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "✅ RPC verification successful for transaction {}",
                get_signature_prefix(signature)
            )
        );
    }

    // Process transaction through transaction system for swap analysis
    // Note: Transaction processing happens automatically through the background service
    // We mainly need RPC verification which was already completed above

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("✅ Transaction processing completed for {}", get_signature_prefix(signature))
        );
    }

    // Update position verification status
    {
        let mut state = GLOBAL_POSITIONS_STATE.lock().await;

        // Mark position as verified
        for position in &mut state.positions {
            if position.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature) {
                position.transaction_entry_verified = true;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("✅ Entry transaction verified for position {}", position.symbol)
                    );
                }

                // Update in database
                if let Some(position_id) = position.id {
                    let position_clone = position.clone();
                    tokio::spawn(async move {
                        if let Err(e) = update_position(&position_clone).await {
                            log(
                                LogTag::Positions,
                                "DB_ERROR",
                                &format!("Failed to update entry verification in database: {}", e)
                            );
                        }
                    });
                }
                break;
            } else if
                position.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
            {
                position.transaction_exit_verified = true;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("✅ Exit transaction verified for position {}", position.symbol)
                    );
                }

                // Update in database
                if let Some(position_id) = position.id {
                    let position_clone = position.clone();
                    tokio::spawn(async move {
                        if let Err(e) = update_position(&position_clone).await {
                            log(
                                LogTag::Positions,
                                "DB_ERROR",
                                &format!("Failed to update exit verification in database: {}", e)
                            );
                        }
                    });
                }
                break;
            }
        }

        // Remove from pending verifications
        state.pending_verifications.remove(signature);
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
                .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
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
                .filter(|p| p.exit_price.is_some())
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
        Ok(Some(position)) => position.exit_price.is_none(),
        Ok(None) => false,
        Err(_) => {
            // Fallback to memory
            let state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions
                .iter()
                .any(|p| p.mint == mint && p.position_type == "buy" && p.exit_price.is_none())
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

/// Calculate position P&L with optional current price
pub async fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "📊 Calculating P&L for position {} ({})",
                position.symbol,
                get_mint_prefix(&position.mint)
            )
        );
    }

    // Determine price to use (exit_price, provided current_price, or fetch current price)
    let price_to_use = if let Some(exit_price) = position.exit_price {
        // Use effective exit price if available, otherwise signal exit price
        position.effective_exit_price.unwrap_or(exit_price)
    } else if let Some(provided_price) = current_price {
        provided_price
    } else {
        // Fetch current price for open position
        match get_token_price_safe(&position.mint).await {
            Some(price) if price > 0.0 => price,
            _ => {
                // Fallback to entry price if current price unavailable
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "WARNING",
                        &format!(
                            "⚠️ Could not get current price for {} - using entry price for P&L calculation",
                            position.symbol
                        )
                    );
                }
                position.entry_price
            }
        }
    };

    // Use effective entry price if available, otherwise signal entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

    // Calculate gross P&L based on price movement
    let price_change_ratio = if entry_price > 0.0 {
        price_to_use / entry_price
    } else {
        1.0 // No change if entry price is invalid
    };

    // Gross P&L is the change in position value
    let gross_pnl_sol = position.entry_size_sol * (price_change_ratio - 1.0);

    // Calculate total fees (excluding ATA rent from trading costs)
    let total_fees_sol = calculate_position_total_fees(position);

    // Net P&L subtracts fees from gross P&L
    let net_pnl_sol = gross_pnl_sol - total_fees_sol;

    // Calculate percentage return
    let pnl_percent = if position.entry_size_sol > 0.0 {
        (net_pnl_sol / position.entry_size_sol) * 100.0
    } else {
        0.0
    };

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "📊 P&L calculated for {} - Entry: {:.8}, Current/Exit: {:.8}, Gross: {:.4} SOL, Fees: {:.4} SOL, Net: {:.4} SOL ({:.2}%)",
                position.symbol,
                entry_price,
                price_to_use,
                gross_pnl_sol,
                total_fees_sol,
                net_pnl_sol,
                pnl_percent
            )
        );
    }

    (net_pnl_sol, pnl_percent)
}

/// Calculate total fees for a position
pub fn calculate_position_total_fees(position: &Position) -> f64 {
    // Sum entry and exit fees in SOL (excluding ATA rent from trading costs)
    let entry_fees_sol = (position.entry_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    let exit_fees_sol = (position.exit_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    entry_fees_sol + exit_fees_sol
}

// ==================== LIQUIDITY TIER ====================

/// Determine liquidity tier for a token
pub async fn determine_liquidity_tier(mint: &str) -> Result<String, String> {
    use crate::tokens::get_pool_service;

    // Get pool service to fetch liquidity information
    let pool_service = get_pool_service();

    // Try to get pool data for the token
    let liquidity_usd = if let Some(pool_result) = pool_service.get_pool_price(mint, None).await {
        pool_result.liquidity_usd
    } else {
        return Ok("UNKNOWN".to_string());
    };

    if liquidity_usd < 0.0 {
        return Ok("INVALID".to_string());
    }

    // Liquidity tier classification based on USD value (same as backup file)
    let tier = match liquidity_usd {
        x if x < 1_000.0 => "MICRO", // < $1K
        x if x < 10_000.0 => "SMALL", // $1K - $10K
        x if x < 50_000.0 => "MEDIUM", // $10K - $50K
        x if x < 250_000.0 => "LARGE", // $50K - $250K
        x if x < 1_000_000.0 => "XLARGE", // $250K - $1M
        _ => "MEGA", // > $1M
    };

    Ok(tier.to_string())
}

// ==================== BACKGROUND TASKS ====================

/// Start background position management tasks
pub async fn run_background_position_tasks(shutdown: Arc<Notify>) {
    println!("🚀 Starting background position tasks");

    // Spawn independent background tasks
    let shutdown_clone1 = shutdown.clone();
    let shutdown_clone2 = shutdown.clone();
    let shutdown_clone3 = shutdown.clone();

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
}

/// Verify pending transactions with parallel processing
async fn verify_pending_transactions_parallel(shutdown: Arc<Notify>) {
    println!("🔍 Starting parallel transaction verification task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                println!("🛑 Stopping transaction verification task");
                break;
            }
            _ = sleep(Duration::from_secs(30)) => {
                // Get batch of pending verifications
                let pending_sigs: Vec<String> = {
                    let state = GLOBAL_POSITIONS_STATE.lock().await;
                    state.pending_verifications.keys().cloned().collect()
                };

                if !pending_sigs.is_empty() {
                    if is_debug_positions_enabled() {
                        println!("🔍 Processing {} pending verifications", pending_sigs.len());
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
                                        log(
                                            LogTag::Positions,
                                            "VERIFICATION_ERROR",
                                            &format!("❌ Failed to verify {}: {}", get_signature_prefix(&sig_clone), e)
                                        );
                                        None
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
                    println!("🔍 No pending verifications to process");
                }
            }
        }
    }
}

/// Cleanup phantom positions with parallel processing
async fn cleanup_phantom_positions_parallel(shutdown: Arc<Notify>) {
    println!("🧹 Starting parallel phantom cleanup task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                println!("🛑 Stopping phantom cleanup task");
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
                    match get_token_balance(&position.mint, &wallet_address).await {
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
                                // Check if entry transaction actually failed
                                if let Some(entry_sig) = &pos.entry_transaction_signature {
                                    match get_transaction(entry_sig).await {
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
                    println!("🧹 No phantom positions detected");
                }
            }
        }
    }
}

/// Retry failed operations with parallel processing
async fn retry_failed_operations_parallel(shutdown: Arc<Notify>) {
    println!("🔄 Starting parallel retry task");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                println!("🛑 Stopping retry task");
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
                                    let current_price = get_token_price_safe(&mint_clone).await.unwrap_or(position.entry_price);
                                    
                                    match close_position_direct(&mint_clone, current_price, "retry_attempt".to_string()).await {
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
                    println!("🔄 No retry operations ready");
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

/// Get profit targets for a token based on its characteristics
async fn get_profit_target(token: &Token) -> (f64, f64) {
    // Default profit targets - could be made configurable or dynamic
    let base_min = 10.0; // 10% minimum
    let base_max = 50.0; // 50% maximum

    // Could adjust based on token liquidity, market cap, etc.
    // For now, return defaults
    (base_min, base_max)
}

/// Get token balance safely with error handling
async fn get_token_balance_safe(mint: &str, wallet_address: &str) -> Option<u64> {
    match get_token_balance(mint, wallet_address).await {
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
    println!("🚀 Initializing positions system");

    // Initialize database first
    initialize_positions_database().await.map_err(|e| {
        format!("Failed to initialize positions database: {}", e)
    })?;

    // Load existing positions from database into memory
    match load_all_positions().await {
        Ok(positions) => {
            let mut state = GLOBAL_POSITIONS_STATE.lock().await;
            state.positions = positions;
            println!("✅ Loaded {} positions from database", state.positions.len());
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

    println!("✅ Positions system initialized");
    Ok(())
}

/// Start the positions manager service (replaces actor spawn)
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) -> Result<(), String> {
    println!("🚀 Starting positions manager service");

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

// ==================== DATABASE SYNC HELPERS ====================

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
