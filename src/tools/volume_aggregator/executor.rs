//! Volume Aggregator Executor
//!
//! Handles the execution of volume generation sessions with:
//! - Strategy-based wallet distribution
//! - Database persistence for sessions and swaps
//! - Resume support for interrupted sessions

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::time::{sleep, Duration};

use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::tools::database::{
    get_va_session, get_va_swaps, insert_va_session, insert_va_swap, update_va_session_metrics,
    update_va_session_status, update_va_swap_result,
};
use crate::tools::swap_executor::{tool_buy, tool_sell};
use crate::tools::types::{DistributionStrategy, WalletMode};
use crate::tools::ToolStatus;
use crate::wallets::{self, WalletRole, WalletWithKey};

use super::strategies::{calculate_amount_clamped, calculate_delay, StrategyExecutor};
use super::types::{
    SessionStatus, TransactionStatus, VolumeConfig, VolumeSession, VolumeTransaction,
};

/// Minimum SOL balance required per wallet for gas fees
const MIN_WALLET_BALANCE_SOL: f64 = 0.01;

/// Volume Aggregator for generating trading volume
pub struct VolumeAggregator {
    /// Configuration for this session
    config: VolumeConfig,
    /// Strategy executor for wallet distribution
    strategy_executor: Option<StrategyExecutor>,
    /// Current execution status
    status: ToolStatus,
    /// Flag to abort execution
    abort_flag: Arc<AtomicBool>,
    /// Current session (if running)
    current_session: Option<VolumeSession>,
}

impl VolumeAggregator {
    /// Create a new volume aggregator with the given configuration
    pub fn new(config: VolumeConfig) -> Self {
        Self {
            config,
            strategy_executor: None,
            status: ToolStatus::Ready,
            abort_flag: Arc::new(AtomicBool::new(false)),
            current_session: None,
        }
    }

    /// Get current status
    pub fn status(&self) -> ToolStatus {
        self.status.clone()
    }

    /// Get the configuration
    pub fn config(&self) -> &VolumeConfig {
        &self.config
    }

    /// Get the abort flag for external control
    pub fn get_abort_flag(&self) -> Arc<AtomicBool> {
        self.abort_flag.clone()
    }

    /// Get current session (if any)
    pub fn current_session(&self) -> Option<&VolumeSession> {
        self.current_session.as_ref()
    }

    /// Prepare for execution by loading wallets and validating config
    pub async fn prepare(&mut self) -> Result<(), String> {
        logger::info(
            LogTag::Tools,
            &format!(
                "Preparing volume aggregator for token {} with {} SOL volume",
                self.config.token_mint, self.config.total_volume_sol
            ),
        );

        // Validate configuration
        self.config.validate()?;

        // Load wallets based on wallet mode
        let wallets = self.load_wallets().await?;

        // Create strategy executor with loaded wallets
        self.strategy_executor = Some(StrategyExecutor::new(wallets, self.config.strategy.clone()));

        self.status = ToolStatus::Ready;
        Ok(())
    }

    /// Load wallets based on wallet mode configuration
    async fn load_wallets(&self) -> Result<Vec<WalletWithKey>, String> {
        let all_wallets = wallets::get_wallets_with_keys().await?;

        let mut selected_wallets = match &self.config.wallet_mode {
            WalletMode::Single => {
                // Use first secondary wallet
                all_wallets
                    .into_iter()
                    .filter(|w| w.wallet.role == WalletRole::Secondary)
                    .take(1)
                    .collect::<Vec<_>>()
            }
            WalletMode::Selected => {
                // Use specific wallet addresses
                if let Some(ref addresses) = self.config.wallet_addresses {
                    all_wallets
                        .into_iter()
                        .filter(|w| addresses.contains(&w.wallet.address))
                        .collect()
                } else {
                    return Err("Selected wallet mode requires wallet_addresses".to_string());
                }
            }
            WalletMode::AutoSelect => {
                // Use all secondary wallets up to num_wallets
                all_wallets
                    .into_iter()
                    .filter(|w| w.wallet.role == WalletRole::Secondary)
                    .take(self.config.num_wallets)
                    .collect()
            }
        };

        if selected_wallets.is_empty() {
            return Err("No wallets available for volume aggregation".to_string());
        }

        // Validate wallet balances
        let rpc_client = get_rpc_client();
        let mut valid_wallets = Vec::new();

        for wallet in selected_wallets {
            match rpc_client.get_sol_balance(&wallet.wallet.address).await {
                Ok(balance) => {
                    if balance >= MIN_WALLET_BALANCE_SOL {
                        valid_wallets.push(wallet);
                    } else {
                        logger::warning(
                            LogTag::Tools,
                            &format!(
                                "Wallet {} has insufficient balance: {:.4} SOL (min: {} SOL)",
                                &wallet.wallet.address[..8],
                                balance,
                                MIN_WALLET_BALANCE_SOL
                            ),
                        );
                    }
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Tools,
                        &format!(
                            "Failed to check balance for wallet {}: {}",
                            &wallet.wallet.address[..8],
                            e
                        ),
                    );
                }
            }
        }

        if valid_wallets.is_empty() {
            return Err("No wallets with sufficient balance available".to_string());
        }

        logger::info(
            LogTag::Tools,
            &format!(
                "Loaded {} wallets with sufficient balance for volume aggregation",
                valid_wallets.len()
            ),
        );

        Ok(valid_wallets)
    }

    /// Execute the volume generation session
    pub async fn execute(&mut self) -> Result<VolumeSession, String> {
        let executor = self
            .strategy_executor
            .as_mut()
            .ok_or("No wallets available. Call prepare() first.")?;

        if !executor.has_wallets() {
            return Err("No wallets available. Call prepare() first.".to_string());
        }

        self.status = ToolStatus::Running;
        self.abort_flag.store(false, Ordering::SeqCst);

        // Create new session
        let mut session = VolumeSession::new(&self.config.token_mint, self.config.total_volume_sol);
        session.start();

        // Save session to database
        let wallet_addresses: Vec<String> = executor
            .wallets()
            .iter()
            .map(|w| w.wallet.address.clone())
            .collect();

        let db_id = insert_va_session(
            &session.session_id,
            &session.token_mint,
            session.target_volume_sol,
            &self.config.delay_config,
            &self.config.sizing_config,
            &self.config.strategy,
            &self.config.wallet_mode,
            Some(&wallet_addresses),
        )?;

        session.set_db_id(db_id);

        // Update session status to running
        update_va_session_status(&session.session_id, &ToolStatus::Running, None)?;

        logger::info(
            LogTag::Tools,
            &format!(
                "Starting volume generation session {} for token {} (db_id={})",
                session.session_id,
                &session.token_mint[..8],
                db_id
            ),
        );

        self.current_session = Some(session.clone());

        // Execute the session
        let result = self.execute_session_loop(&mut session).await;

        // Update final status
        match &result {
            Ok(_) => {
                update_va_session_status(&session.session_id, &ToolStatus::Completed, None)?;
            }
            Err(e) => {
                update_va_session_status(
                    &session.session_id,
                    &ToolStatus::Failed,
                    Some(e.as_str()),
                )?;
            }
        }

        // Update final metrics
        update_va_session_metrics(
            &session.session_id,
            session.actual_volume_sol,
            session.successful_buys as i32,
            session.successful_sells as i32,
            session.failed_count as i32,
        )?;

        self.current_session = Some(session.clone());
        result.map(|_| session)
    }

    /// Resume an interrupted session
    pub async fn resume(&mut self, session_id: &str) -> Result<VolumeSession, String> {
        // Load session from database
        let db_session = get_va_session(session_id)?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        // Check if session can be resumed
        if db_session.status != "running" {
            return Err(format!(
                "Session {} cannot be resumed (status: {})",
                session_id, db_session.status
            ));
        }

        // Load existing swaps
        let existing_swaps = get_va_swaps(session_id)?;

        // Reconstruct session
        let mut session = VolumeSession::with_id(
            db_session.session_id.clone(),
            db_session.token_mint.clone(),
            db_session.target_volume_sol,
        );
        session.set_db_id(db_session.id);
        session.actual_volume_sol = db_session.actual_volume_sol;
        session.successful_buys = db_session.successful_buys as usize;
        session.successful_sells = db_session.successful_sells as usize;
        session.failed_count = db_session.failed_count as usize;
        session.status = SessionStatus::Running;

        // Prepare wallets
        self.prepare().await?;

        let executor = self
            .strategy_executor
            .as_mut()
            .ok_or("Failed to prepare wallets for resume")?;

        // Set operation count to resume from correct point
        executor.set_operation_count(existing_swaps.len());

        self.status = ToolStatus::Running;
        self.abort_flag.store(false, Ordering::SeqCst);

        logger::info(
            LogTag::Tools,
            &format!(
                "Resuming volume session {} from tx {} ({:.2} SOL remaining)",
                session_id,
                existing_swaps.len(),
                session.remaining_volume()
            ),
        );

        self.current_session = Some(session.clone());

        // Continue execution
        let result = self.execute_session_loop(&mut session).await;

        // Update final status and metrics
        match &result {
            Ok(_) => {
                update_va_session_status(&session.session_id, &ToolStatus::Completed, None)?;
            }
            Err(e) => {
                update_va_session_status(
                    &session.session_id,
                    &ToolStatus::Failed,
                    Some(e.as_str()),
                )?;
            }
        }

        update_va_session_metrics(
            &session.session_id,
            session.actual_volume_sol,
            session.successful_buys as i32,
            session.successful_sells as i32,
            session.failed_count as i32,
        )?;

        self.current_session = Some(session.clone());
        result.map(|_| session)
    }

    /// Main execution loop for a session
    async fn execute_session_loop(&mut self, session: &mut VolumeSession) -> Result<(), String> {
        let executor = self
            .strategy_executor
            .as_mut()
            .ok_or("Strategy executor not initialized")?;

        let mut tx_id = session.last_completed_index();

        // Track token holdings per wallet for sells
        let mut wallet_token_balances: HashMap<String, u64> = HashMap::new();

        while session.remaining_volume() > 0.001 {
            // Check abort flag
            if self.abort_flag.load(Ordering::SeqCst) {
                logger::warning(
                    LogTag::Tools,
                    &format!("Volume session {} aborted by user", session.session_id),
                );
                self.status = ToolStatus::Aborted;
                session.abort();
                update_va_session_status(&session.session_id, &ToolStatus::Aborted, None)?;
                return Ok(());
            }

            // Calculate amount for this transaction
            let amount =
                calculate_amount_clamped(&self.config.sizing_config, session.remaining_volume());

            // Skip if amount is too small
            if amount < 0.001 {
                break;
            }

            // Get next wallet from strategy executor
            let wallet = match executor.next_wallet() {
                Some(w) => w,
                None => {
                    return Err("No more wallets available".to_string());
                }
            };

            let wallet_address = wallet.wallet.address.clone();

            // Determine if buy or sell based on wallet token balance
            let token_balance = wallet_token_balances
                .get(&wallet_address)
                .copied()
                .unwrap_or(0);
            let is_buy = token_balance == 0 || tx_id % 2 == 0;

            // Create transaction record
            let mut tx = VolumeTransaction::new(tx_id, wallet_address.clone(), is_buy, amount);

            // Insert pending swap to database
            let swap_db_id = insert_va_swap(
                &session.session_id,
                tx_id as i32,
                &wallet_address,
                is_buy,
                amount,
            )?;

            // Execute the swap
            let swap_result = if is_buy {
                tool_buy(wallet, &session.token_mint, amount, None).await
            } else {
                // For sells, use the token balance we have
                tool_sell(wallet, &session.token_mint, token_balance, None).await
            };

            match swap_result {
                Ok(result) => {
                    let token_amount = if is_buy {
                        result.output_amount as f64
                    } else {
                        result.input_amount as f64
                    };

                    tx.confirm(result.signature.clone(), token_amount);

                    // Update database
                    update_va_swap_result(
                        swap_db_id,
                        Some(&result.signature),
                        Some(token_amount),
                        "confirmed",
                        None,
                    )?;

                    // Update token balance tracking
                    if is_buy {
                        let current = wallet_token_balances
                            .entry(wallet_address.clone())
                            .or_insert(0);
                        *current += result.output_amount;
                    } else {
                        wallet_token_balances.insert(wallet_address.clone(), 0);
                    }

                    logger::info(
                        LogTag::Tools,
                        &format!(
                            "Volume tx {} confirmed: {} {:.4} SOL via {} sig={}",
                            tx_id,
                            if is_buy { "BUY" } else { "SELL" },
                            amount,
                            result.router_name,
                            &result.signature[..12]
                        ),
                    );
                }
                Err(e) => {
                    tx.fail(e.clone());

                    // Update database
                    update_va_swap_result(swap_db_id, None, None, "failed", Some(&e))?;

                    logger::warning(LogTag::Tools, &format!("Volume tx {} failed: {}", tx_id, e));
                }
            }

            // Add transaction to session
            session.add_transaction(tx);
            tx_id += 1;

            // Update session metrics in database periodically (every 5 transactions)
            if tx_id % 5 == 0 {
                update_va_session_metrics(
                    &session.session_id,
                    session.actual_volume_sol,
                    session.successful_buys as i32,
                    session.successful_sells as i32,
                    session.failed_count as i32,
                )?;
            }

            // Delay between transactions
            if session.remaining_volume() > 0.001 {
                let delay_ms = calculate_delay(&self.config.delay_config);
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        session.complete();
        self.status = ToolStatus::Completed;

        logger::info(
            LogTag::Tools,
            &format!(
                "Volume session {} completed: {:.4} SOL volume, {} buys, {} sells, {} failed ({:.1}% success)",
                session.session_id,
                session.actual_volume_sol,
                session.successful_buys,
                session.successful_sells,
                session.failed_count,
                session.success_rate()
            ),
        );

        Ok(())
    }

    /// Abort the current execution
    pub fn abort(&mut self) {
        self.abort_flag.store(true, Ordering::SeqCst);
        logger::info(LogTag::Tools, "Volume aggregator abort requested");
    }

    /// Get loaded wallet count
    pub fn wallet_count(&self) -> usize {
        self.strategy_executor
            .as_ref()
            .map(|e| e.wallet_count())
            .unwrap_or(0)
    }
}
