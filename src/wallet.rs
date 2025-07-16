use crate::config::Config;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ WalletPosition, WalletTransaction, TransactionType, ProfitLossCalculation };
use crate::rpc::RpcManager;
use crate::pricing::PricingManager;
use crate::transaction_cache::TransactionCacheManager;
use anyhow::{ Context, Result };
use chrono::Utc;
use futures::FutureExt;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer }, program_pack::Pack };
use solana_account_decoder::UiAccountData;
use spl_token_2022::state::{ Account, Mint };
use spl_token;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

pub struct WalletTracker {
    config: Config,
    database: Arc<Database>,
    rpc_manager: Arc<RpcManager>,
    pricing_manager: Option<Arc<PricingManager>>,
    wallet_keypair: Keypair,
    transaction_cache: TransactionCacheManager,
    positions: Arc<RwLock<HashMap<String, WalletPosition>>>,
    is_running: Arc<RwLock<bool>>,
    last_signature: Arc<RwLock<Option<String>>>,
}

impl WalletTracker {
    pub fn new(config: Config, database: Arc<Database>) -> Result<Self> {
        let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
        let wallet_pubkey = wallet_keypair.pubkey();

        let rpc_manager = Arc::new(
            RpcManager::new(
                config.rpc_url.clone(),
                config.rpc_fallbacks.clone(),
                config.rpc.clone()
            )?
        );

        let transaction_cache = TransactionCacheManager::new(
            Arc::clone(&database),
            Arc::clone(&rpc_manager),
            wallet_pubkey,
            Some(1000) // Cache 1000 transactions as requested
        );

        Logger::wallet("Initialized RPC manager");

        Ok(Self {
            config,
            database,
            rpc_manager,
            pricing_manager: None,
            wallet_keypair,
            transaction_cache,
            positions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            last_signature: Arc::new(RwLock::new(None)),
        })
    }

    async fn fetch_and_cache_transactions(
        &self,
        limit: Option<usize>,
        is_startup: bool
    ) -> Result<()> {
        let action = if is_startup {
            "🚀 Starting INITIAL transaction fetch"
        } else {
            "🔄 Starting routine transaction fetch"
        };
        Logger::wallet(action);

        let wallet_pubkey = self.wallet_keypair.pubkey();

        // For startup, fetch last 1000 transactions; for routine, fetch last 10
        let fetch_limit = limit.unwrap_or(if is_startup { 1000 } else { 10 });

        Logger::rpc(&format!("📡 Calling RPC for {} transaction signatures...", fetch_limit));
        let signatures = tokio::time
            ::timeout(
                Duration::from_secs(30), // Increased timeout for large fetches
                self.rpc_manager.get_signatures_for_address(&wallet_pubkey, Some(fetch_limit))
            ).await
            .context("RPC call timed out after 30 seconds")?
            .context("Failed to get transaction signatures")?;

        Logger::rpc(&format!("📡 Received {} transaction signatures", signatures.len()));

        let mut new_transactions = 0;
        let mut processed = 0;
        let mut skipped_existing = 0;

        // For startup, process all transactions; for routine, limit to recent ones
        let max_process = if is_startup { signatures.len() } else { 5 };

        for (i, signature_info) in signatures.iter().enumerate() {
            if processed >= max_process {
                Logger::wallet(
                    &format!(
                        "⏭️ Stopping at processing limit (processed {}/{})",
                        max_process,
                        signatures.len()
                    )
                );
                break;
            }

            let signature = signature_info.signature.clone();

            // Check if we already have this transaction
            if self.database.transaction_exists(&signature)? {
                skipped_existing += 1;
                if is_startup {
                    // During startup, if we hit many existing transactions, we can stop early
                    // as they're ordered by recency
                    if skipped_existing >= 10 && processed < 20 {
                        Logger::wallet("⚡ Hit many existing transactions early, stopping fetch");
                        break;
                    }
                }
                continue;
            }

            Logger::wallet(
                &format!(
                    "📄 Processing new transaction {}/{}: {}...",
                    processed + 1,
                    max_process,
                    &signature[..16] // Show first 16 chars of signature
                )
            );

            // Fetch the transaction details with timeout
            let transaction_result = tokio::time::timeout(
                Duration::from_secs(5), // 5 second timeout per transaction
                self.rpc_manager.get_transaction(&signature)
            ).await;

            match transaction_result {
                Ok(Ok(transaction)) => {
                    if let Some(block_time) = transaction.block_time {
                        // Parse the transaction for token transfers
                        if
                            let Some(wallet_tx) = self.parse_transaction_for_tokens(
                                &transaction,
                                &signature,
                                block_time
                            ).await
                        {
                            // Save to database
                            if let Err(e) = self.database.save_wallet_transaction(&wallet_tx) {
                                Logger::error(
                                    &format!("Failed to save transaction {}: {}", signature, e)
                                );
                            } else {
                                new_transactions += 1;
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    Logger::warn(&format!("Failed to get transaction {}: {}", signature, e));
                }
                Err(_) => {
                    Logger::warn(&format!("Transaction fetch timed out for {}", signature));
                }
            }

            processed += 1;

            // Add small delay between transaction fetches to avoid overwhelming RPC
            if is_startup && processed % 50 == 0 {
                Logger::wallet(
                    &format!(
                        "📊 Progress: {}/{} processed, {} new, {} existing",
                        processed,
                        signatures.len(),
                        new_transactions,
                        skipped_existing
                    )
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        if new_transactions > 0 {
            Logger::database(&format!("💾 Cached {} new transactions", new_transactions));
        } else {
            Logger::database("💾 No new transactions to cache");
        }

        if is_startup {
            Logger::success(
                &format!(
                    "✅ INITIAL transaction fetch COMPLETED - processed {}, new {}, existing {}",
                    processed,
                    new_transactions,
                    skipped_existing
                )
            );
        } else {
            Logger::success("✅ Routine transaction fetch COMPLETED");
        }

        Ok(())
    }

    async fn parse_transaction_for_tokens(
        &self,
        transaction: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        signature: &str,
        block_time: i64
    ) -> Option<WalletTransaction> {
        // For now, create a simple transaction entry to demonstrate the system
        // In a full implementation, you would parse the actual transaction data
        let mint = "So11111111111111111111111111111111111111112".to_string(); // SOL mint as placeholder

        return Some(WalletTransaction {
            signature: signature.to_string(),
            mint,
            transaction_type: TransactionType::Transfer,
            amount: 1000000, // 0.001 SOL as placeholder
            price_sol: Some(1.0), // 1 SOL = 1 SOL (base currency)
            value_sol: Some(0.001), // 0.001 SOL value
            sol_amount: Some(1000000), // Same as amount for SOL
            fee: Some(5000), // 0.000005 SOL fee
            block_time,
            slot: transaction.slot,
            created_at: Utc::now(),
        });
    }

    async fn get_historical_price(&self, _mint: &str, _block_time: i64) -> Result<f64> {
        // Placeholder for historical price fetching
        // In a real implementation, you would:
        // 1. Check if you have cached price data for this timestamp
        // 2. Query a price API with historical data
        // 3. Calculate price from pool data at that time
        Ok(0.0)
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Wallet tracker is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Wallet tracker started");
        Logger::wallet(&format!("Tracking wallet: {}", self.wallet_keypair.pubkey()));

        // Load existing positions from database
        self.load_existing_positions().await?;

        // Start background transaction caching
        Logger::separator();
        Logger::wallet("🚀 STARTING BACKGROUND TRANSACTION CACHING");
        match self.transaction_cache.start_background_caching().await {
            Ok(()) => {
                Logger::success("✅ Background transaction caching started");
            }
            Err(e) => {
                Logger::error(&format!("❌ Background transaction caching FAILED: {}", e));
                // Don't fail startup, just log the error and continue
            }
        }
        Logger::separator();

        // Start tracking loop
        let tracker = self.clone();
        tokio::spawn(async move {
            // Wrap in a panic handler to catch any issues
            let result = std::panic
                ::AssertUnwindSafe(tracker.run_tracking_loop())
                .catch_unwind().await;

            match result {
                Ok(()) => {
                    Logger::success("Wallet tracking loop COMPLETED normally");
                }
                Err(panic_info) => {
                    Logger::error(&format!("💥 Wallet tracking loop panicked: {:?}", panic_info));
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        // Stop background transaction caching
        self.transaction_cache.stop_background_caching().await;

        Logger::info("Wallet tracker stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_positions(&self) -> HashMap<String, WalletPosition> {
        self.positions.read().await.clone()
    }

    pub async fn get_wallet_pubkey(&self) -> Pubkey {
        self.wallet_keypair.pubkey()
    }

    pub async fn get_sol_balance(&self) -> Result<f64> {
        let wallet_pubkey = self.wallet_keypair.pubkey();

        let balance = self.rpc_manager.get_balance(&wallet_pubkey).await?;

        Ok((balance as f64) / 1_000_000_000.0) // Convert lamports to SOL
    }

    pub async fn refresh_positions(&self) -> Result<()> {
        Logger::wallet("Refreshing wallet positions...");

        let wallet_pubkey = self.wallet_keypair.pubkey();

        // Get token accounts with retry logic and fallback to different program IDs
        let token_accounts = self
            .get_token_accounts_with_retry(&wallet_pubkey).await
            .context("Failed to get token accounts after retries")?;

        let mut new_positions = HashMap::new();
        let mut total_value_sol = 0.0;

        if token_accounts.is_empty() {
            Logger::wallet("No SPL token accounts found - wallet contains only SOL");
        } else {
            Logger::wallet(&format!("PROCESSING {} token accounts...", token_accounts.len()));
        }

        for (i, token_account) in token_accounts.iter().enumerate() {
            Logger::wallet(
                &format!("🔍 PROCESSING token account {}/{}", i + 1, token_accounts.len())
            );

            match &token_account.account.data {
                UiAccountData::Binary(encoded_data, encoding) => {
                    Logger::debug(
                        &format!(
                            "Raw account data: {} (encoding: {:?})",
                            encoded_data.len(),
                            encoding
                        )
                    );

                    if let Some(data) = token_account.account.data.decode() {
                        Logger::success(
                            &format!(
                                "Successfully decoded token account data (size: {} bytes)",
                                data.len()
                            )
                        );

                        match self.parse_token_account(&data) {
                            Ok(account_data) => {
                                Logger::success("Successfully parsed token account");
                                Logger::print_key_value("Mint", &account_data.mint.to_string());
                                Logger::print_key_value("Amount", &account_data.amount.to_string());

                                if account_data.amount > 0 {
                                    Logger::wallet(
                                        &format!(
                                            "✅ Token has non-zero balance: {}",
                                            account_data.amount
                                        )
                                    );

                                    let mint = account_data.mint.to_string();

                                    // Get token info to determine decimals
                                    let decimals = self
                                        .get_token_decimals(&account_data.mint).await
                                        .unwrap_or(9);

                                    Logger::print_key_value("Decimals", &decimals.to_string());

                                    // Calculate actual balance
                                    let balance = account_data.amount;
                                    let actual_balance =
                                        (balance as f64) / (10_f64).powi(decimals as i32);

                                    // Try to get price in SOL (placeholder implementation)
                                    let current_price_sol = self
                                        .get_token_price_in_sol(&mint).await
                                        .unwrap_or(0.0);
                                    let value_sol = actual_balance * current_price_sol;
                                    total_value_sol += value_sol;

                                    Logger::print_balance(&mint, actual_balance, Some(value_sol));
                                    Logger::pricing(&format!("Price: {} SOL", current_price_sol));

                                    // Get profit/loss calculation from transactions with current price
                                    // TODO: Fix this method call - using placeholder for now
                                    let pnl_calc = ProfitLossCalculation {
                                        mint: mint.clone(),
                                        total_bought: 0,
                                        total_sold: 0,
                                        current_balance: balance,
                                        average_buy_price_sol: current_price_sol,
                                        average_sell_price_sol: 0.0,
                                        total_invested_sol: 0.0,
                                        total_received_sol: 0.0,
                                        realized_pnl_sol: 0.0,
                                        unrealized_pnl_sol: 0.0,
                                        total_pnl_sol: 0.0,
                                        roi_percentage: 0.0,
                                        current_value_sol: value_sol,
                                    };

                                    let position = WalletPosition {
                                        mint: mint.clone(),
                                        balance,
                                        decimals,
                                        value_sol: Some(value_sol),
                                        entry_price_sol: Some(pnl_calc.average_buy_price_sol),
                                        current_price_sol: Some(current_price_sol),
                                        pnl_sol: Some(pnl_calc.total_pnl_sol),
                                        pnl_percentage: Some(pnl_calc.roi_percentage),
                                        realized_pnl_sol: Some(pnl_calc.realized_pnl_sol),
                                        unrealized_pnl_sol: Some(pnl_calc.unrealized_pnl_sol),
                                        total_invested_sol: Some(pnl_calc.total_invested_sol),
                                        average_entry_price_sol: Some(
                                            pnl_calc.average_buy_price_sol
                                        ),
                                        last_updated: Utc::now(),
                                    };

                                    Logger::success(&format!("Created position for {}", mint));

                                    // Save to database
                                    if let Err(e) = self.database.save_wallet_position(&position) {
                                        Logger::error(
                                            &format!("Failed to save position for {}: {}", mint, e)
                                        );
                                        continue;
                                    }

                                    new_positions.insert(mint, position);
                                } else {
                                    Logger::debug("Skipping token with zero balance");
                                }
                            }
                            Err(e) => {
                                Logger::warn(&format!("FAILED to parse token account: {}", e));
                            }
                        }
                    } else {
                        Logger::warn("FAILED to decode account data");
                    }
                }
                UiAccountData::Json(json_data) => {
                    Logger::debug("Account data is in JSON format");

                    // Check if it's a parsed account (for SPL tokens)
                    if let Some(parsed_info) = json_data.parsed.as_object() {
                        Logger::success("Found parsed info in JSON data");

                        // Extract mint and token amount from JSON
                        if
                            let (Some(info), Some(token_amount)) = (
                                parsed_info.get("info"),
                                parsed_info.get("info").and_then(|info| info.get("tokenAmount")),
                            )
                        {
                            if
                                let (Some(mint_str), Some(amount_str), Some(decimals)) = (
                                    info.get("mint").and_then(|v| v.as_str()),
                                    token_amount.get("amount").and_then(|v| v.as_str()),
                                    token_amount.get("decimals").and_then(|v| v.as_u64()),
                                )
                            {
                                Logger::print_key_value("Mint", mint_str);

                                if let Ok(amount) = amount_str.parse::<u64>() {
                                    Logger::print_key_value(
                                        "Amount",
                                        &format!("{} (decimals: {})", amount, decimals)
                                    );

                                    if amount > 0 {
                                        Logger::wallet(
                                            &format!("✅ Token has non-zero balance: {}", amount)
                                        );

                                        let mint = mint_str.to_string();
                                        let decimals = decimals as u8;

                                        // Calculate actual balance
                                        let actual_balance =
                                            (amount as f64) / (10_f64).powi(decimals as i32);

                                        // Try to get price in SOL (placeholder implementation)
                                        let current_price_sol = self
                                            .get_token_price_in_sol(&mint).await
                                            .unwrap_or(0.0);
                                        let value_sol = actual_balance * current_price_sol;
                                        total_value_sol += value_sol;

                                        Logger::print_balance(
                                            &mint,
                                            actual_balance,
                                            Some(value_sol)
                                        );
                                        Logger::pricing(
                                            &format!("Price: {} SOL", current_price_sol)
                                        );

                                        // Get profit/loss calculation from transactions with current price
                                        // TODO: Fix this method call - using placeholder for now
                                        let pnl_calc = ProfitLossCalculation {
                                            mint: mint.clone(),
                                            total_bought: 0,
                                            total_sold: 0,
                                            current_balance: amount,
                                            average_buy_price_sol: current_price_sol,
                                            average_sell_price_sol: 0.0,
                                            total_invested_sol: 0.0,
                                            total_received_sol: 0.0,
                                            realized_pnl_sol: 0.0,
                                            unrealized_pnl_sol: 0.0,
                                            total_pnl_sol: 0.0,
                                            roi_percentage: 0.0,
                                            current_value_sol: value_sol,
                                        };

                                        let position = WalletPosition {
                                            mint: mint.clone(),
                                            balance: amount,
                                            decimals,
                                            value_sol: Some(value_sol),
                                            entry_price_sol: Some(pnl_calc.average_buy_price_sol),
                                            current_price_sol: Some(current_price_sol),
                                            pnl_sol: Some(pnl_calc.total_pnl_sol),
                                            pnl_percentage: Some(pnl_calc.roi_percentage),
                                            realized_pnl_sol: Some(pnl_calc.realized_pnl_sol),
                                            unrealized_pnl_sol: Some(pnl_calc.unrealized_pnl_sol),
                                            total_invested_sol: Some(pnl_calc.total_invested_sol),
                                            average_entry_price_sol: Some(
                                                pnl_calc.average_buy_price_sol
                                            ),
                                            last_updated: Utc::now(),
                                        };

                                        Logger::success(&format!("Created position for {}", mint));

                                        // Save to database
                                        if
                                            let Err(e) = self.database.save_wallet_position(
                                                &position
                                            )
                                        {
                                            Logger::error(
                                                &format!(
                                                    "Failed to save position for {}: {}",
                                                    mint,
                                                    e
                                                )
                                            );
                                            continue;
                                        }

                                        new_positions.insert(mint, position);
                                    } else {
                                        Logger::debug("Skipping token with zero balance");
                                    }
                                } else {
                                    Logger::error(
                                        &format!("FAILED to parse amount: {}", amount_str)
                                    );
                                }
                            } else {
                                Logger::error("Missing amount or decimals in tokenAmount");
                            }
                        } else {
                            Logger::error("Missing mint or tokenAmount in JSON data");
                        }
                    } else {
                        Logger::error("No parsed info found in JSON data");
                    }
                }
                UiAccountData::LegacyBinary(data) => {
                    Logger::debug(&format!("Account data is legacy binary (size: {})", data.len()));
                }
            }
        }

        // Update positions
        *self.positions.write().await = new_positions.clone();

        Logger::success(
            &format!(
                "Portfolio updated: {} positions, Total value: {:.6} SOL",
                new_positions.len(),
                total_value_sol
            )
        );

        // Log top positions with enhanced formatting
        let mut sorted_positions: Vec<_> = new_positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_sol.unwrap_or(0.0).partial_cmp(&a.value_sol.unwrap_or(0.0)).unwrap()
        });

        if !sorted_positions.is_empty() {
            Logger::separator();
            Logger::wallet("📊 TOP POSITIONS:");
            for (i, position) in sorted_positions.iter().take(5).enumerate() {
                let balance = (position.balance as f64) / (10_f64).powi(position.decimals as i32);
                Logger::print_balance(
                    &format!("{}. {}", i + 1, position.mint),
                    balance,
                    position.value_sol
                );
                if let (Some(pnl), Some(pnl_pct)) = (position.pnl_sol, position.pnl_percentage) {
                    Logger::print_pnl(pnl, pnl_pct);
                }
            }
            Logger::separator();
        }

        Ok(())
    }

    async fn load_existing_positions(&self) -> Result<()> {
        Logger::database("Loading existing positions from database...");

        let positions = self.database
            .get_wallet_positions()
            .context("FAILED to load positions from database")?;

        let mut position_map = HashMap::new();
        for position in positions {
            position_map.insert(position.mint.clone(), position);
        }

        *self.positions.write().await = position_map;
        Logger::database(
            &format!("Loaded {} positions from database", self.positions.read().await.len())
        );

        Ok(())
    }

    async fn run_tracking_loop(&self) {
        Logger::wallet("Starting wallet tracking loop...");

        let mut interval = time::interval(Duration::from_secs(60)); // Update every 60 seconds

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                Logger::wallet("🛑 Wallet tracking loop stopping (is_running = false)");
                break;
            }
            drop(is_running);

            Logger::wallet("🔄 Running 60-second refresh cycle...");

            // Transaction caching is now handled in the background
            Logger::wallet("🔍 Scanning for SPL token accounts...");
            match self.refresh_positions().await {
                Ok(()) => {
                    Logger::success("✅ Position refresh COMPLETED");
                }
                Err(e) => {
                    Logger::error(&format!("❌ FAILED to refresh positions: {}", e));
                    Logger::error(&format!("❌ Error details: {:?}", e));
                }
            }

            Logger::wallet("🏁 Refresh cycle COMPLETED, waiting 60 seconds...");
        }

        Logger::success("Wallet tracking loop stopped");
    }

    fn parse_token_account(&self, data: &[u8]) -> Result<Account> {
        // Try parsing as SPL Token 2022 first
        if let Ok(account) = spl_token_2022::state::Account::unpack(data) {
            return Ok(account);
        }

        // Fallback to legacy SPL Token format
        spl_token::state::Account
            ::unpack(data)
            .map(|legacy_account| {
                // Convert legacy account to Token 2022 format
                Account {
                    mint: legacy_account.mint,
                    owner: legacy_account.owner,
                    amount: legacy_account.amount,
                    delegate: legacy_account.delegate,
                    state: match legacy_account.state {
                        spl_token::state::AccountState::Uninitialized =>
                            spl_token_2022::state::AccountState::Uninitialized,
                        spl_token::state::AccountState::Initialized =>
                            spl_token_2022::state::AccountState::Initialized,
                        spl_token::state::AccountState::Frozen =>
                            spl_token_2022::state::AccountState::Frozen,
                    },
                    is_native: legacy_account.is_native,
                    delegated_amount: legacy_account.delegated_amount,
                    close_authority: legacy_account.close_authority,
                }
            })
            .context(
                "Failed to parse token account data as either SPL Token 2022 or legacy SPL Token"
            )
    }

    async fn get_token_decimals(&self, mint: &Pubkey) -> Result<u8> {
        let mint_copy = *mint;

        let account_info = self.rpc_manager.get_account(&mint_copy).await?;

        let mint_info = Mint::unpack(&account_info.data).context("Failed to parse mint data")?;

        Ok(mint_info.decimals)
    }

    async fn get_token_price_in_sol(&self, mint: &str) -> Result<f64> {
        if let Some(pricing_manager) = &self.pricing_manager {
            if let Some(price_info) = pricing_manager.get_token_price(mint).await {
                // Convert USD price to SOL by getting SOL/USD rate
                // For now, we'll use a placeholder conversion rate
                // In a real implementation, you would get the current SOL/USD rate
                let sol_usd_rate = 100.0; // Placeholder: 1 SOL = $100
                return Ok(price_info.price_usd / sol_usd_rate);
            }
        }

        // Fallback to 0.0 if no pricing manager or price not found
        Ok(0.0)
    }

    async fn get_token_accounts_with_retry(
        &self,
        wallet_pubkey: &Pubkey
    ) -> Result<Vec<solana_client::rpc_response::RpcKeyedAccount>> {
        use solana_client::rpc_request::TokenAccountsFilter;

        // Try different program IDs with RPC fallback support
        let program_ids = [
            spl_token::id(), // Original SPL Token program
            spl_token_2022::id(), // Token-2022 program
        ];

        for program_id in &program_ids {
            let wallet_pubkey_copy = *wallet_pubkey;
            let program_id_copy = *program_id;

            match
                self.rpc_manager.get_token_accounts_by_owner(
                    &wallet_pubkey_copy,
                    TokenAccountsFilter::ProgramId(program_id_copy)
                ).await
            {
                Ok(accounts) => {
                    if !accounts.is_empty() {
                        Logger::rpc(
                            &format!(
                                "Found {} token accounts using program ID: {}",
                                accounts.len(),
                                program_id
                            )
                        );
                        return Ok(accounts);
                    }
                }
                Err(e) => {
                    Logger::warn(
                        &format!("FAILED to get accounts for program {}: {}", program_id, e)
                    );
                    continue;
                }
            }
        }

        // If we get here, all attempts failed
        Logger::wallet(
            "No token accounts found with any program ID - wallet may have no SPL tokens"
        );
        Ok(Vec::new()) // Return empty vector instead of error for wallets with no tokens
    }
}

impl Clone for WalletTracker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            rpc_manager: Arc::clone(&self.rpc_manager),
            pricing_manager: self.pricing_manager.as_ref().map(Arc::clone),
            wallet_keypair: Keypair::try_from(&self.wallet_keypair.to_bytes()[..]).unwrap(),
            transaction_cache: self.transaction_cache.clone(),
            positions: Arc::clone(&self.positions),
            is_running: Arc::clone(&self.is_running),
            last_signature: Arc::clone(&self.last_signature),
        }
    }
}

impl WalletTracker {
    pub fn set_pricing_manager(&mut self, pricing_manager: Arc<PricingManager>) {
        self.pricing_manager = Some(pricing_manager);
    }
}
