use crate::config::Config;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ WalletPosition, WalletTransaction, TransactionType, ProfitLossCalculation };
use crate::rpc::RpcManager;
use crate::pricing::PricingManager;
use crate::transaction_cache::TransactionCacheManager;
use crate::profit_calculator::ProfitLossCalculator;
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

pub struct EnhancedWalletTracker {
    config: Config,
    database: Arc<Database>,
    rpc_manager: Arc<RpcManager>,
    pricing_manager: Option<Arc<PricingManager>>,
    wallet_keypair: Keypair,
    transaction_cache: TransactionCacheManager,
    profit_calculator: ProfitLossCalculator,
    positions: Arc<RwLock<HashMap<String, WalletPosition>>>,
    is_running: Arc<RwLock<bool>>,
    cache_initialized: Arc<RwLock<bool>>,
}

impl EnhancedWalletTracker {
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

        let profit_calculator = ProfitLossCalculator::new(Arc::clone(&database));

        Logger::wallet("Enhanced Wallet Tracker initialized");
        Logger::wallet(&format!("Tracking wallet: {}", wallet_pubkey));

        Ok(Self {
            config,
            database,
            rpc_manager,
            pricing_manager: None,
            wallet_keypair,
            transaction_cache,
            profit_calculator,
            positions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            cache_initialized: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Enhanced wallet tracker is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("🚀 Enhanced Wallet Tracker started");

        // Start background transaction caching first
        Logger::wallet("🔧 Starting background transaction caching...");
        self.transaction_cache.start_background_caching().await?;

        // Mark cache as initialized
        *self.cache_initialized.write().await = true;

        // Load existing positions from database
        self.load_existing_positions().await?;

        // Start tracking loop
        let tracker = self.clone();
        tokio::spawn(async move {
            let result = std::panic
                ::AssertUnwindSafe(tracker.run_enhanced_tracking_loop())
                .catch_unwind().await;

            match result {
                Ok(()) => {
                    Logger::success("Enhanced wallet tracking loop completed normally");
                }
                Err(panic_info) => {
                    Logger::error(
                        &format!("💥 Enhanced wallet tracking loop panicked: {:?}", panic_info)
                    );
                }
            }
        });

        Ok(())
    }

    async fn run_enhanced_tracking_loop(&self) {
        Logger::wallet("🔄 Starting enhanced wallet tracking loop...");

        let mut interval = time::interval(Duration::from_secs(60)); // Update every 60 seconds

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                Logger::wallet("🛑 Enhanced wallet tracking loop stopping");
                break;
            }
            drop(is_running);

            Logger::wallet("🔄 Running enhanced 60-second refresh cycle...");

            // Refresh positions with accurate profit/loss calculation
            // Transaction caching is now handled in the background
            match self.refresh_positions_with_enhanced_pnl().await {
                Ok(()) => {
                    Logger::success("✅ Enhanced position refresh completed");
                }
                Err(e) => {
                    Logger::error(&format!("❌ Enhanced position refresh failed: {}", e));
                }
            }

            Logger::wallet("🏁 Enhanced refresh cycle completed, waiting 30 seconds...");
        }

        Logger::success("Enhanced wallet tracking loop stopped");
    }

    async fn refresh_positions_with_enhanced_pnl(&self) -> Result<()> {
        Logger::wallet("🔄 Refreshing positions with enhanced P&L calculation...");

        let wallet_pubkey = self.wallet_keypair.pubkey();

        // Get current token accounts
        let token_accounts = self
            .get_token_accounts_with_retry(&wallet_pubkey).await
            .context("Failed to get token accounts")?;

        let mut current_prices = HashMap::new();
        let mut new_positions = HashMap::new();
        let mut total_portfolio_value_sol = 0.0;

        Logger::wallet(&format!("🔍 Processing {} token accounts...", token_accounts.len()));

        for (i, token_account) in token_accounts.iter().enumerate() {
            Logger::wallet(
                &format!("🔍 Processing token account {}/{}", i + 1, token_accounts.len())
            );

            match &token_account.account.data {
                UiAccountData::Binary(encoded_data, encoding) => {
                    if let Some(data) = token_account.account.data.decode() {
                        match self.parse_token_account(&data) {
                            Ok(account_data) => {
                                if account_data.amount > 0 {
                                    let mint = account_data.mint.to_string();

                                    // Get token decimals and current price
                                    let decimals = self
                                        .get_token_decimals(&account_data.mint).await
                                        .unwrap_or(9);
                                    let current_price_sol = self
                                        .get_token_price_in_sol(&mint).await
                                        .unwrap_or(0.0);

                                    current_prices.insert(mint.clone(), current_price_sol);

                                    // Calculate enhanced P&L using the profit calculator
                                    let position = self.profit_calculator.update_position_with_pnl(
                                        &mint,
                                        account_data.amount,
                                        decimals,
                                        current_price_sol
                                    ).await?;

                                    total_portfolio_value_sol += position.value_sol.unwrap_or(0.0);

                                    Logger::print_balance(
                                        &mint,
                                        (account_data.amount as f64) /
                                            (10_f64).powi(decimals as i32),
                                        position.value_sol
                                    );

                                    if
                                        let (Some(pnl), Some(pnl_pct)) = (
                                            position.pnl_sol,
                                            position.pnl_percentage,
                                        )
                                    {
                                        Logger::print_pnl(pnl, pnl_pct);
                                    }

                                    // Save to database
                                    if let Err(e) = self.database.save_wallet_position(&position) {
                                        Logger::error(
                                            &format!("Failed to save position for {}: {}", mint, e)
                                        );
                                        continue;
                                    }

                                    new_positions.insert(mint, position);
                                }
                            }
                            Err(e) => {
                                Logger::warn(&format!("Failed to parse token account: {}", e));
                            }
                        }
                    }
                }
                UiAccountData::Json(json_data) => {
                    if let Some(parsed_info) = json_data.parsed.as_object() {
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
                                if let Ok(amount) = amount_str.parse::<u64>() {
                                    if amount > 0 {
                                        let mint = mint_str.to_string();
                                        let decimals = decimals as u8;
                                        let current_price_sol = self
                                            .get_token_price_in_sol(&mint).await
                                            .unwrap_or(0.0);

                                        current_prices.insert(mint.clone(), current_price_sol);

                                        // Calculate enhanced P&L
                                        let position =
                                            self.profit_calculator.update_position_with_pnl(
                                                &mint,
                                                amount,
                                                decimals,
                                                current_price_sol
                                            ).await?;

                                        total_portfolio_value_sol +=
                                            position.value_sol.unwrap_or(0.0);

                                        Logger::print_balance(
                                            &mint,
                                            (amount as f64) / (10_f64).powi(decimals as i32),
                                            position.value_sol
                                        );

                                        if
                                            let (Some(pnl), Some(pnl_pct)) = (
                                                position.pnl_sol,
                                                position.pnl_percentage,
                                            )
                                        {
                                            Logger::print_pnl(pnl, pnl_pct);
                                        }

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
                                    }
                                }
                            }
                        }
                    }
                }
                UiAccountData::LegacyBinary(_) => {
                    Logger::debug("Skipping legacy binary account data");
                }
            }
        }

        // Calculate portfolio-wide P&L
        Logger::wallet("📊 Calculating portfolio-wide P&L...");
        match self.profit_calculator.calculate_portfolio_pnl(&current_prices).await {
            Ok(portfolio_pnl) => {
                Logger::success(
                    &format!("📊 Portfolio P&L calculated for {} tokens", portfolio_pnl.len())
                );
            }
            Err(e) => {
                Logger::error(&format!("Failed to calculate portfolio P&L: {}", e));
            }
        }

        // Update positions
        *self.positions.write().await = new_positions.clone();

        Logger::success(
            &format!(
                "🎉 Enhanced portfolio updated: {} positions, Total value: {:.6} SOL",
                new_positions.len(),
                total_portfolio_value_sol
            )
        );

        Ok(())
    }

    async fn load_existing_positions(&self) -> Result<()> {
        Logger::database("Loading existing positions from database...");

        let positions = self.database
            .get_wallet_positions()
            .context("Failed to load positions from database")?;

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

    // Keep existing helper methods from the original WalletTracker
    fn parse_token_account(&self, data: &[u8]) -> Result<Account> {
        if let Ok(account) = spl_token_2022::state::Account::unpack(data) {
            return Ok(account);
        }

        spl_token::state::Account
            ::unpack(data)
            .map(|legacy_account| {
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
            .context("Failed to parse token account data")
    }

    async fn get_token_decimals(&self, mint: &Pubkey) -> Result<u8> {
        let account_info = self.rpc_manager.get_account(mint).await?;
        let mint_info = Mint::unpack(&account_info.data).context("Failed to parse mint data")?;
        Ok(mint_info.decimals)
    }

    async fn get_token_price_in_sol(&self, mint: &str) -> Result<f64> {
        if let Some(pricing_manager) = &self.pricing_manager {
            if let Some(price_info) = pricing_manager.get_token_price(mint).await {
                let sol_usd_rate = 100.0; // Placeholder conversion rate
                return Ok(price_info.price_usd / sol_usd_rate);
            }
        }
        Ok(0.0)
    }

    async fn get_token_accounts_with_retry(
        &self,
        wallet_pubkey: &Pubkey
    ) -> Result<Vec<solana_client::rpc_response::RpcKeyedAccount>> {
        use solana_client::rpc_request::TokenAccountsFilter;

        let program_ids = [spl_token::id(), spl_token_2022::id()];

        for program_id in &program_ids {
            match
                self.rpc_manager.get_token_accounts_by_owner(
                    wallet_pubkey,
                    TokenAccountsFilter::ProgramId(*program_id)
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
                        &format!("Failed to get accounts for program {}: {}", program_id, e)
                    );
                    continue;
                }
            }
        }

        Logger::wallet("No token accounts found - wallet may have no SPL tokens");
        Ok(Vec::new())
    }

    // Public interface methods
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        // Stop background transaction caching
        self.transaction_cache.stop_background_caching().await;

        Logger::info("Enhanced wallet tracker stopped");
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
        Ok((balance as f64) / 1_000_000_000.0)
    }

    pub fn set_pricing_manager(&mut self, pricing_manager: Arc<PricingManager>) {
        self.pricing_manager = Some(pricing_manager);
    }

    /// Get detailed transaction history for a specific token
    pub async fn get_token_transaction_history(
        &self,
        mint: &str
    ) -> Result<Vec<WalletTransaction>> {
        self.database.get_wallet_transactions_for_mint(mint)
    }

    /// Get comprehensive profit/loss calculation for a token
    pub async fn get_token_pnl(
        &self,
        mint: &str,
        current_price_sol: f64
    ) -> Result<ProfitLossCalculation> {
        self.profit_calculator.calculate_token_pnl(mint, current_price_sol).await
    }

    /// Force refresh of transaction cache
    pub async fn refresh_transaction_cache(&self) -> Result<usize> {
        self.transaction_cache.update_cache_with_new_transactions().await
    }
}

impl Clone for EnhancedWalletTracker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            rpc_manager: Arc::clone(&self.rpc_manager),
            pricing_manager: self.pricing_manager.as_ref().map(Arc::clone),
            wallet_keypair: Keypair::try_from(&self.wallet_keypair.to_bytes()[..]).unwrap(),
            transaction_cache: TransactionCacheManager::new(
                Arc::clone(&self.database),
                Arc::clone(&self.rpc_manager),
                self.wallet_keypair.pubkey(),
                Some(1000)
            ),
            profit_calculator: ProfitLossCalculator::new(Arc::clone(&self.database)),
            positions: Arc::clone(&self.positions),
            is_running: Arc::clone(&self.is_running),
            cache_initialized: Arc::clone(&self.cache_initialized),
        }
    }
}
