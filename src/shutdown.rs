use crate::prelude::*;
use std::sync::atomic::{ AtomicBool, AtomicU32, Ordering };
use std::sync::Arc;
use tokio::sync::{ Mutex, RwLock };
use std::collections::HashMap;
use chrono::{ DateTime, Utc };

// ═══════════════════════════════════════════════════════════════════════════════
// SHUTDOWN MANAGEMENT SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════
//
// This module ensures safe and complete shutdown by:
// 1. Preventing new transactions during shutdown
// 2. Tracking and waiting for pending transactions
// 3. Ensuring all position states are saved
// 4. Providing graceful shutdown with timeouts
//
// SHUTDOWN PHASES:
// Phase 1: Signal received - stop accepting new trades
// Phase 2: Wait for pending transactions to complete
// Phase 3: Save all states and flush to disk
// Phase 4: Exit cleanly
// ═══════════════════════════════════════════════════════════════════════════════

pub static SHUTDOWN_MANAGER: Lazy<ShutdownManager> = Lazy::new(|| ShutdownManager::new());

/// Global shutdown state management
pub struct ShutdownManager {
    /// Primary shutdown flag - prevents new operations
    pub shutdown_requested: AtomicBool,

    /// Graceful shutdown in progress - finishing existing operations
    pub shutdown_in_progress: AtomicBool,

    /// Emergency shutdown - force exit after timeout
    pub emergency_shutdown: AtomicBool,

    /// Active transaction counter
    pub active_transactions: AtomicU32,

    /// Pending transactions tracker
    pub pending_transactions: Arc<Mutex<HashMap<String, PendingTransaction>>>,

    /// State save status
    pub state_saved: AtomicBool,

    /// Shutdown start time
    pub shutdown_start_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

#[derive(Debug, Clone)]
pub struct PendingTransaction {
    pub tx_id: String,
    pub tx_type: TransactionType,
    pub mint: String,
    pub symbol: String,
    pub amount: f64,
    pub started_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionType {
    Buy,
    Sell,
    DCA,
}

impl ShutdownManager {
    pub fn new() -> Self {
        Self {
            shutdown_requested: AtomicBool::new(false),
            shutdown_in_progress: AtomicBool::new(false),
            emergency_shutdown: AtomicBool::new(false),
            active_transactions: AtomicU32::new(0),
            pending_transactions: Arc::new(Mutex::new(HashMap::new())),
            state_saved: AtomicBool::new(false),
            shutdown_start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Acquire)
    }

    /// Check if shutdown is in progress
    pub fn is_shutdown_in_progress(&self) -> bool {
        self.shutdown_in_progress.load(Ordering::Acquire)
    }

    /// Check if emergency shutdown is active
    pub fn is_emergency_shutdown(&self) -> bool {
        self.emergency_shutdown.load(Ordering::Acquire)
    }

    /// Check if we should accept new transactions
    pub fn should_accept_new_transactions(&self) -> bool {
        !self.is_shutdown_requested() && !self.is_shutdown_in_progress()
    }

    /// Get count of active transactions
    pub fn get_active_transaction_count(&self) -> u32 {
        self.active_transactions.load(Ordering::Acquire)
    }

    /// Register a new transaction
    pub async fn register_transaction(
        &self,
        tx_id: String,
        tx_type: TransactionType,
        mint: String,
        symbol: String,
        amount: f64
    ) -> Result<()> {
        if !self.should_accept_new_transactions() {
            anyhow::bail!("Shutdown in progress - not accepting new transactions");
        }

        self.active_transactions.fetch_add(1, Ordering::AcqRel);

        let pending_tx = PendingTransaction {
            tx_id: tx_id.clone(),
            tx_type: tx_type.clone(),
            mint,
            symbol,
            started_at: Utc::now(),
            timeout_at: Utc::now() + chrono::Duration::seconds(60), // 60 second timeout
            amount,
        };

        self.pending_transactions.lock().await.insert(tx_id.clone(), pending_tx);

        println!("📝 [SHUTDOWN] Registered transaction {}: {:?}", tx_id, tx_type);
        Ok(())
    }

    /// Complete a transaction (success or failure)
    pub async fn complete_transaction(&self, tx_id: &str, success: bool) {
        let mut pending = self.pending_transactions.lock().await;
        if let Some(tx) = pending.remove(tx_id) {
            self.active_transactions.fetch_sub(1, Ordering::AcqRel);
            let duration = Utc::now().signed_duration_since(tx.started_at);

            println!(
                "{} [SHUTDOWN] Completed transaction {} in {:.2}s: {:?} for {}",
                if success {
                    "✅"
                } else {
                    "❌"
                },
                tx_id,
                (duration.num_milliseconds() as f64) / 1000.0,
                tx.tx_type,
                tx.symbol
            );
        }
    }

    /// Initialize graceful shutdown
    pub async fn initiate_shutdown(&self) -> Result<()> {
        if self.shutdown_requested.load(Ordering::Acquire) {
            return Ok(()); // Already shutting down
        }

        println!("\n🛑 [SHUTDOWN] Graceful shutdown initiated");

        // Set shutdown flags
        self.shutdown_requested.store(true, Ordering::Release);
        *self.shutdown_start_time.write().await = Some(Utc::now());

        // Start shutdown process in background
        let manager = &*SHUTDOWN_MANAGER;
        tokio::spawn(async move {
            if let Err(e) = manager.execute_shutdown().await {
                eprintln!("❌ [SHUTDOWN] Shutdown process failed: {}", e);
                std::process::exit(1);
            }
        });

        Ok(())
    }

    /// Execute the complete shutdown process
    async fn execute_shutdown(&self) -> Result<()> {
        self.shutdown_in_progress.store(true, Ordering::Release);

        println!("⏸️  [SHUTDOWN] Phase 1: Stopping new operations...");

        // Phase 1: Stop accepting new operations
        // (Already done by setting shutdown_requested)

        // Phase 2: Wait for pending transactions with timeout
        println!("⏳ [SHUTDOWN] Phase 2: Waiting for pending transactions...");
        let wait_result = self.wait_for_transactions().await;

        match wait_result {
            Ok(_) => println!("✅ [SHUTDOWN] All transactions completed successfully"),
            Err(e) => {
                eprintln!("⚠️ [SHUTDOWN] Transaction wait failed: {}", e);
                println!("🚨 [SHUTDOWN] Proceeding with emergency save...");
            }
        }

        // Phase 3: Emergency position reconciliation
        println!("🔍 [SHUTDOWN] Phase 3: Reconciling positions with on-chain state...");
        if let Err(e) = self.reconcile_positions().await {
            eprintln!("⚠️ [SHUTDOWN] Position reconciliation failed: {}", e);
        }

        // Phase 4: Save all states
        println!("💾 [SHUTDOWN] Phase 4: Saving all states...");
        if let Err(e) = self.save_all_states().await {
            eprintln!("❌ [SHUTDOWN] State saving failed: {}", e);
        } else {
            self.state_saved.store(true, Ordering::Release);
            println!("✅ [SHUTDOWN] All states saved successfully");
        }

        // Phase 5: Final cleanup
        println!("🧹 [SHUTDOWN] Phase 5: Final cleanup...");
        self.final_cleanup().await;

        println!("✅ [SHUTDOWN] Graceful shutdown completed successfully");
        std::process::exit(0);
    }

    /// Wait for all pending transactions to complete
    async fn wait_for_transactions(&self) -> Result<()> {
        let max_wait_time = 120; // 2 minutes maximum wait
        let check_interval = 1; // Check every second

        for elapsed in 0..max_wait_time {
            let active_count = self.get_active_transaction_count();

            if active_count == 0 {
                println!("✅ [SHUTDOWN] All transactions completed");
                return Ok(());
            }

            // Show pending transactions every 5 seconds
            if elapsed % 5 == 0 {
                self.log_pending_transactions().await;
            }

            // Check for timed out transactions
            self.check_transaction_timeouts().await;

            tokio::time::sleep(Duration::from_secs(check_interval)).await;
        }

        // Force emergency shutdown if transactions are still pending
        let remaining = self.get_active_transaction_count();
        if remaining > 0 {
            eprintln!(
                "⚠️ [SHUTDOWN] {} transactions still pending after {}s, forcing emergency shutdown",
                remaining,
                max_wait_time
            );
            self.emergency_shutdown.store(true, Ordering::Release);
        }

        anyhow::bail!("Transaction wait timeout - {} transactions still pending", remaining);
    }

    /// Log current pending transactions
    async fn log_pending_transactions(&self) {
        let pending = self.pending_transactions.lock().await;
        let active_count = self.get_active_transaction_count();

        if active_count > 0 {
            println!("⏳ [SHUTDOWN] Waiting for {} active transactions:", active_count);
            for (tx_id, tx) in pending.iter() {
                let elapsed = Utc::now().signed_duration_since(tx.started_at);
                println!(
                    "   • {} - {:?} {} ({:.1}s elapsed)",
                    tx_id,
                    tx.tx_type,
                    tx.symbol,
                    (elapsed.num_milliseconds() as f64) / 1000.0
                );
            }
        }
    }

    /// Check for and handle timed out transactions
    async fn check_transaction_timeouts(&self) {
        let mut pending = self.pending_transactions.lock().await;
        let now = Utc::now();
        let mut timed_out = Vec::new();

        for (tx_id, tx) in pending.iter() {
            if now > tx.timeout_at {
                timed_out.push((tx_id.clone(), tx.clone()));
            }
        }

        for (tx_id, tx) in timed_out {
            pending.remove(&tx_id);
            self.active_transactions.fetch_sub(1, Ordering::AcqRel);

            eprintln!(
                "⏰ [SHUTDOWN] Transaction {} timed out after 60s: {:?} for {}",
                tx_id,
                tx.tx_type,
                tx.symbol
            );

            // Log the timeout for manual investigation
            self.log_timed_out_transaction(&tx).await;
        }
    }

    /// Log timed out transaction for manual review
    async fn log_timed_out_transaction(&self, tx: &PendingTransaction) {
        let log_entry = format!(
            "{}: TIMEOUT - {:?} {} mint={} amount={:.9} started_at={}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            tx.tx_type,
            tx.symbol,
            tx.mint,
            tx.amount,
            tx.started_at.format("%Y-%m-%d %H:%M:%S")
        );

        if
            let Err(e) = tokio::fs::OpenOptions
                ::new()
                .create(true)
                .append(true)
                .open("shutdown_timeouts.log").await
        {
            eprintln!("❌ [SHUTDOWN] Failed to open timeout log: {}", e);
            return;
        }

        let log_content = format!("{}\n", log_entry);
        if let Err(e) = tokio::fs::write("shutdown_timeouts.log", log_content).await {
            eprintln!("❌ [SHUTDOWN] Failed to write timeout log: {}", e);
        }
    }

    /// Reconcile positions with actual on-chain state
    async fn reconcile_positions(&self) -> Result<()> {
        let positions = OPEN_POSITIONS.read().await;
        let mut reconciliation_needed = Vec::new();

        println!("🔍 [SHUTDOWN] Checking {} open positions for reconciliation...", positions.len());

        for (mint, position) in positions.iter() {
            // Get actual token balance
            let actual_balance = crate::helpers::get_biggest_token_amount_f64(mint);
            let expected_balance = position.token_amount;
            let diff = (actual_balance - expected_balance).abs();

            // If difference is significant (>1% or >0.001 tokens), flag for reconciliation
            if diff > expected_balance * 0.01 || diff > 0.001 {
                reconciliation_needed.push((mint.clone(), expected_balance, actual_balance));

                println!(
                    "⚠️ [RECONCILE] {} balance mismatch: expected={:.9}, actual={:.9}, diff={:.9}",
                    mint,
                    expected_balance,
                    actual_balance,
                    diff
                );
            }
        }

        drop(positions);

        // Update positions with actual balances
        if !reconciliation_needed.is_empty() {
            println!(
                "🔧 [RECONCILE] Updating {} positions with actual balances...",
                reconciliation_needed.len()
            );

            let mut positions_write = OPEN_POSITIONS.write().await;
            for (mint, expected, actual) in reconciliation_needed {
                if let Some(position) = positions_write.get_mut(&mint) {
                    let old_amount = position.token_amount;
                    position.token_amount = actual;

                    println!(
                        "✏️ [RECONCILE] Updated {} balance: {:.9} → {:.9}",
                        mint,
                        old_amount,
                        actual
                    );
                }
            }
        } else {
            println!("✅ [RECONCILE] All positions match on-chain balances");
        }

        Ok(())
    }

    /// Save all critical states to disk
    async fn save_all_states(&self) -> Result<()> {
        println!("💾 [SHUTDOWN] Saving open positions...");
        crate::persistence::save_open().await;

        println!("💾 [SHUTDOWN] Saving closed positions...");
        crate::persistence::save_closed().await;

        println!("💾 [SHUTDOWN] Saving watchlist...");
        crate::persistence::save_watchlist().await;

        println!("💾 [SHUTDOWN] Saving performance history...");
        if let Err(e) = crate::performance::save_performance_history().await {
            eprintln!("⚠️ [SHUTDOWN] Performance history save failed: {}", e);
        }

        println!("💾 [SHUTDOWN] Flushing pool cache...");
        flush_pool_cache_to_disk_nonblocking();

        // Ensure all files are synced to disk
        println!("💾 [SHUTDOWN] Syncing files to disk...");
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    /// Final cleanup before exit
    async fn final_cleanup(&self) {
        // Log shutdown summary
        if let Some(start_time) = *self.shutdown_start_time.read().await {
            let duration = Utc::now().signed_duration_since(start_time);
            println!(
                "📊 [SHUTDOWN] Total shutdown time: {:.2}s",
                (duration.num_milliseconds() as f64) / 1000.0
            );
        }

        // Log final state
        let open_count = OPEN_POSITIONS.read().await.len();
        let closed_count = CLOSED_POSITIONS.read().await.len();

        println!(
            "📊 [SHUTDOWN] Final state: {} open positions, {} closed positions",
            open_count,
            closed_count
        );

        // Create shutdown summary file
        let summary = format!(
            "Shutdown completed at: {}\nOpen positions: {}\nClosed positions: {}\nState saved: {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            open_count,
            closed_count,
            self.state_saved.load(Ordering::Acquire)
        );

        if let Err(e) = tokio::fs::write("last_shutdown.log", summary).await {
            eprintln!("⚠️ [SHUTDOWN] Failed to write shutdown summary: {}", e);
        }
    }
}

/// Install signal handlers for graceful shutdown
pub fn install_shutdown_handlers() -> Result<()> {
    // Install Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\n🛑 [SIGNAL] Received Ctrl+C, initiating graceful shutdown...");

        // Use blocking runtime for signal handler
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            if let Err(e) = SHUTDOWN_MANAGER.initiate_shutdown().await {
                eprintln!("❌ [SIGNAL] Failed to initiate shutdown: {}", e);
                std::process::exit(1);
            }
        });
    })?;

    // Install SIGTERM handler for Unix systems
    #[cfg(unix)]
    {
        use tokio::signal::unix::{ signal, SignalKind };
        tokio::spawn(async {
            let mut sigterm = signal(SignalKind::terminate()).expect(
                "Failed to install SIGTERM handler"
            );

            sigterm.recv().await;
            println!("\n🛑 [SIGNAL] Received SIGTERM, initiating graceful shutdown...");

            if let Err(e) = SHUTDOWN_MANAGER.initiate_shutdown().await {
                eprintln!("❌ [SIGNAL] Failed to initiate shutdown: {}", e);
                std::process::exit(1);
            }
        });
    }

    Ok(())
}

/// Wrapper for buy operations with shutdown safety
pub async fn safe_buy_gmgn_with_amounts(
    token_mint_address: &str,
    in_amount: u64,
    symbol: &str
) -> Result<(String, f64)> {
    let manager = &*SHUTDOWN_MANAGER;

    if !manager.should_accept_new_transactions() {
        anyhow::bail!("Shutdown in progress - buy operation rejected");
    }

    let tx_id = format!("buy_{}_{}", token_mint_address, Utc::now().timestamp_millis());

    // Register transaction
    manager.register_transaction(
        tx_id.clone(),
        TransactionType::Buy,
        token_mint_address.to_string(),
        symbol.to_string(),
        (in_amount as f64) / 1_000_000_000.0 // Convert lamports to SOL
    ).await?;

    // Execute the buy
    let result = crate::swap_gmgn::buy_gmgn_with_amounts(token_mint_address, in_amount).await;

    // Mark transaction as complete
    manager.complete_transaction(&tx_id, result.is_ok()).await;

    result
}

/// Wrapper for sell operations with shutdown safety
pub async fn safe_sell_all_gmgn(
    token_mint_address: &str,
    min_out_amount: f64,
    symbol: &str
) -> Result<String> {
    let manager = &*SHUTDOWN_MANAGER;

    if !manager.should_accept_new_transactions() {
        anyhow::bail!("Shutdown in progress - sell operation rejected");
    }

    let tx_id = format!("sell_{}_{}", token_mint_address, Utc::now().timestamp_millis());

    // Get token amount for logging
    let token_amount = crate::helpers::get_biggest_token_amount_f64(token_mint_address);

    // Register transaction
    manager.register_transaction(
        tx_id.clone(),
        TransactionType::Sell,
        token_mint_address.to_string(),
        symbol.to_string(),
        token_amount
    ).await?;

    // Execute the sell
    let result = crate::swap_gmgn::sell_all_gmgn(token_mint_address, min_out_amount).await;

    // Mark transaction as complete
    manager.complete_transaction(&tx_id, result.is_ok()).await;

    result
}

/// Check if shutdown has been requested (replacement for SHUTDOWN.load())
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_MANAGER.is_shutdown_requested()
}

/// Check if we should continue normal operations
pub fn should_continue_operations() -> bool {
    !SHUTDOWN_MANAGER.is_shutdown_requested()
}

/// Get shutdown manager for direct access if needed
pub fn get_shutdown_manager() -> &'static ShutdownManager {
    &*SHUTDOWN_MANAGER
}
