use crate::config::Config;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::WalletPosition;
use crate::rpc_manager::RpcManager;
use anyhow::{ Context, Result };
use chrono::Utc;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer }, program_pack::Pack };
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
    wallet_keypair: Keypair,
    positions: Arc<RwLock<HashMap<String, WalletPosition>>>,
    is_running: Arc<RwLock<bool>>,
}

impl WalletTracker {
    pub fn new(config: Config, database: Arc<Database>) -> Result<Self> {
        let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);

        let rpc_manager = Arc::new(
            RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone())
        );

        Logger::wallet(
            &format!("Initialized RPC manager with {} endpoints", rpc_manager.get_client_count())
        );

        Ok(Self {
            config,
            database,
            rpc_manager,
            wallet_keypair,
            positions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        })
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

        // Start tracking loop
        let tracker = self.clone();
        tokio::spawn(async move {
            tracker.run_tracking_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
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

        let balance = self.rpc_manager.execute_with_fallback(move |client| {
            client.get_balance(&wallet_pubkey).context("Failed to get SOL balance")
        }).await?;

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
        let mut total_value_usd = 0.0;

        if token_accounts.is_empty() {
            Logger::wallet("No SPL token accounts found - wallet contains only SOL");
        } else {
            Logger::wallet(&format!("Processing {} token accounts...", token_accounts.len()));
        }

        for token_account in token_accounts {
            if let Some(data) = token_account.account.data.decode() {
                if let Ok(account_data) = self.parse_token_account(&data) {
                    if account_data.amount > 0 {
                        let mint = account_data.mint.to_string();

                        // Get token info to determine decimals
                        let decimals = self
                            .get_token_decimals(&account_data.mint).await
                            .unwrap_or(9);

                        // Calculate actual balance
                        let balance = account_data.amount;
                        let actual_balance = (balance as f64) / (10_f64).powi(decimals as i32);

                        // Try to get price (placeholder implementation)
                        let current_price = self.get_token_price(&mint).await.unwrap_or(0.0);
                        let value_usd = actual_balance * current_price;
                        total_value_usd += value_usd;

                        // Get existing position for PnL calculation
                        let existing_position = self.positions.read().await.get(&mint).cloned();
                        let (entry_price, pnl, pnl_percentage) = if
                            let Some(existing) = existing_position
                        {
                            let entry = existing.entry_price.unwrap_or(current_price);
                            let pnl_val = (current_price - entry) * actual_balance;
                            let pnl_pct = if entry > 0.0 {
                                ((current_price - entry) / entry) * 100.0
                            } else {
                                0.0
                            };
                            (Some(entry), Some(pnl_val), Some(pnl_pct))
                        } else {
                            (Some(current_price), Some(0.0), Some(0.0))
                        };

                        let position = WalletPosition {
                            mint: mint.clone(),
                            balance,
                            decimals,
                            value_usd: Some(value_usd),
                            entry_price,
                            current_price: Some(current_price),
                            pnl,
                            pnl_percentage,
                            last_updated: Utc::now(),
                        };

                        // Save to database
                        if let Err(e) = self.database.save_wallet_position(&position) {
                            Logger::error(&format!("Failed to save position for {}: {}", mint, e));
                            continue;
                        }

                        new_positions.insert(mint, position);
                    }
                }
            }
        }

        // Update positions
        *self.positions.write().await = new_positions.clone();

        Logger::wallet(
            &format!(
                "Portfolio updated: {} positions, Total value: ${:.2}",
                new_positions.len(),
                total_value_usd
            )
        );

        // Log top positions
        let mut sorted_positions: Vec<_> = new_positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_usd.unwrap_or(0.0).partial_cmp(&a.value_usd.unwrap_or(0.0)).unwrap()
        });

        for (i, position) in sorted_positions.iter().take(5).enumerate() {
            let balance = (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            Logger::wallet(
                &format!(
                    "  {}. {} - Balance: {:.4}, Value: ${:.2}, PnL: {:.2}%",
                    i + 1,
                    position.mint,
                    balance,
                    position.value_usd.unwrap_or(0.0),
                    position.pnl_percentage.unwrap_or(0.0)
                )
            );
        }

        Ok(())
    }

    async fn load_existing_positions(&self) -> Result<()> {
        Logger::wallet("Loading existing positions from database...");

        let positions = self.database
            .get_wallet_positions()
            .context("Failed to load positions from database")?;

        let mut position_map = HashMap::new();
        for position in positions {
            position_map.insert(position.mint.clone(), position);
        }

        *self.positions.write().await = position_map;
        Logger::wallet(
            &format!("Loaded {} positions from database", self.positions.read().await.len())
        );

        Ok(())
    }

    async fn run_tracking_loop(&self) {
        Logger::wallet("Starting wallet tracking loop...");

        let mut interval = time::interval(
            Duration::from_secs(self.config.general.update_interval_seconds)
        );

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            if let Err(e) = self.refresh_positions().await {
                Logger::error(&format!("Failed to refresh positions: {}", e));
            }
        }

        Logger::wallet("Wallet tracking loop stopped");
    }

    fn parse_token_account(&self, data: &[u8]) -> Result<Account> {
        Account::unpack(data).context("Failed to parse token account data")
    }

    async fn get_token_decimals(&self, mint: &Pubkey) -> Result<u8> {
        let mint_copy = *mint;

        let account_info = self.rpc_manager.execute_with_fallback(move |client| {
            client.get_account(&mint_copy).context("Failed to get mint account")
        }).await?;

        let mint_info = Mint::unpack(&account_info.data).context("Failed to parse mint data")?;

        Ok(mint_info.decimals)
    }

    async fn get_token_price(&self, _mint: &str) -> Result<f64> {
        // Placeholder implementation
        // In a real implementation, you would:
        // 1. Query Jupiter API for price
        // 2. Check if the token has a known price source
        // 3. Calculate based on liquidity pools
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
                self.rpc_manager.execute_with_fallback(move |client| {
                    client
                        .get_token_accounts_by_owner(
                            &wallet_pubkey_copy,
                            TokenAccountsFilter::ProgramId(program_id_copy)
                        )
                        .context("Failed to get token accounts")
                }).await
            {
                Ok(accounts) => {
                    if !accounts.is_empty() {
                        Logger::wallet(
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
                    Logger::wallet(
                        &format!("Failed to get accounts for program {}: {}", program_id, e)
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
            wallet_keypair: Keypair::try_from(&self.wallet_keypair.to_bytes()[..]).unwrap(),
            positions: Arc::clone(&self.positions),
            is_running: Arc::clone(&self.is_running),
        }
    }
}
