use crate::core::{ BotResult, BotError, BotConfig, TokenBalance, WalletTransaction, RpcManager };
use crate::cache::CacheManager;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{ Keypair, Signature },
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

pub mod manager;
pub mod transactions;
pub mod balances;
pub mod display;

// Export public modules and structs
pub use transactions::*;
pub use balances::*;
pub use display::WalletStatusDisplay;

/// Main wallet manager for the bot
#[derive(Debug)]
pub struct WalletManager {
    pub keypair: Keypair,
    pub public_key: Pubkey,
    config: BotConfig,
    rpc: RpcManager,
    cache: Option<CacheManager>,
}

impl WalletManager {
    /// Create a new wallet manager
    pub fn new(config: &BotConfig) -> BotResult<Self> {
        // Parse the private key from config
        let keypair = Keypair::from_base58_string(&config.main_wallet_private);
        let public_key = keypair.pubkey();

        // Verify the public key matches config
        let expected_public = Pubkey::from_str(&config.main_wallet_public).map_err(|e|
            BotError::Wallet(format!("Invalid public key: {}", e))
        )?;

        if public_key != expected_public {
            return Err(
                BotError::Wallet("Private key doesn't match configured public key".to_string())
            );
        }

        let rpc = RpcManager::new(crate::core::constants::DEFAULT_RPC_URL)?;

        Ok(Self {
            keypair,
            public_key,
            config: config.clone(),
            rpc,
            cache: None,
        })
    }

    /// Create a new wallet manager with cache
    pub fn with_cache(config: &BotConfig, cache: CacheManager) -> BotResult<Self> {
        let mut wallet = Self::new(config)?;
        wallet.cache = Some(cache);
        Ok(wallet)
    }

    /// Initialize the wallet manager
    pub async fn initialize(&mut self) -> BotResult<()> {
        log::info!("🔑 Initializing wallet manager...");
        log::info!("📍 Wallet address: {}", self.public_key);

        // Check wallet balance
        let balance = self.get_sol_balance().await?;
        log::info!("💰 SOL balance: {:.6} SOL", balance);

        // Get all token balances for initial assessment
        let balances = self.get_all_balances().await?;
        let token_count = balances.len().saturating_sub(1); // Exclude SOL

        // Display startup status
        WalletStatusDisplay::display_startup_status(&self.public_key, balance, token_count);

        // Balance warnings
        if balance < 0.01 {
            log::warn!("⚠️ Low SOL balance, may not be able to perform trades");
        }

        // Verify RPC connection
        if let Err(e) = self.rpc.health_check().await {
            return Err(BotError::Rpc(format!("RPC health check failed: {}", e)));
        }

        log::info!("✅ Wallet manager initialized successfully");
        Ok(())
    }

    /// Get SOL balance
    pub async fn get_sol_balance(&self) -> BotResult<f64> {
        let lamports = self.rpc.get_balance(&self.public_key).await?;
        Ok((lamports as f64) / (crate::core::LAMPORTS_PER_SOL as f64))
    }

    /// Get all token balances
    pub async fn get_all_balances(&self) -> BotResult<Vec<TokenBalance>> {
        let balance_manager = BalanceManager::new(&self.rpc);
        balance_manager.get_all_token_balances(&self.public_key).await
    }

    /// Get recent transactions with caching support
    pub async fn get_recent_transactions(&self) -> BotResult<Vec<WalletTransaction>> {
        let tx_manager = if let Some(cache) = &self.cache {
            TransactionManager::with_cache(&self.rpc, cache)
        } else {
            TransactionManager::new(&self.rpc)
        };

        tx_manager.get_recent_transactions(&self.public_key, 50).await
    }

    /// Get transaction history for a specific token with caching
    pub async fn get_token_transaction_history(
        &self,
        token_mint: &Pubkey
    ) -> BotResult<Vec<WalletTransaction>> {
        let tx_manager = if let Some(cache) = &self.cache {
            TransactionManager::with_cache(&self.rpc, cache)
        } else {
            TransactionManager::new(&self.rpc)
        };

        tx_manager.get_token_transactions(&self.public_key, token_mint).await
    }

    /// Display comprehensive wallet status
    pub async fn display_wallet_status(
        &self,
        positions: &[crate::core::Position],
        portfolio_health: &crate::core::PortfolioHealth
    ) -> BotResult<()> {
        log::info!("📊 Generating comprehensive wallet status...");

        // Get current balances
        let balances = self.get_all_balances().await?;

        // Get recent transactions
        let recent_transactions = self.get_recent_transactions().await?;

        // Create display formatter
        let display = WalletStatusDisplay::new().with_colors(true).with_transaction_summary(true);

        // Display comprehensive wallet status
        display.display_wallet_status(
            &self.public_key,
            &balances,
            positions,
            portfolio_health,
            &recent_transactions
        );

        Ok(())
    }

    /// Display quick wallet status for regular updates
    pub async fn display_quick_status(
        &self,
        portfolio_health: &crate::core::PortfolioHealth
    ) -> BotResult<()> {
        let balances = self.get_all_balances().await?;
        let display = WalletStatusDisplay::new();
        display.display_quick_status(&balances, portfolio_health);
        Ok(())
    }

    /// Sign and send a transaction
    pub async fn send_transaction(&self, mut transaction: Transaction) -> BotResult<Signature> {
        // Sign the transaction
        transaction.sign(&[&self.keypair], self.rpc.client.get_latest_blockhash()?);

        // Send the transaction
        self.rpc.send_transaction(&transaction).await
    }

    /// Get wallet address
    pub fn get_address(&self) -> &Pubkey {
        &self.public_key
    }

    /// Get keypair for signing
    pub fn get_keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Check if wallet has sufficient SOL for a trade
    pub async fn has_sufficient_sol(&self, amount_needed: f64) -> BotResult<bool> {
        let current_balance = self.get_sol_balance().await?;
        Ok(current_balance >= amount_needed + 0.001) // Leave some for fees
    }

    /// Force refresh transaction cache
    pub async fn refresh_transaction_cache(&self) -> BotResult<()> {
        if let Some(cache) = &self.cache {
            log::info!("🔄 Refreshing transaction cache...");

            // Get fresh transactions from RPC (bypassing cache)
            let tx_manager = TransactionManager::new(&self.rpc);
            let fresh_transactions = tx_manager.get_recent_transactions(
                &self.public_key,
                100
            ).await?;

            // Cache the fresh transactions
            for transaction in fresh_transactions {
                cache.cache_transaction(&self.public_key, &transaction).await?;
            }

            log::info!("✅ Transaction cache refreshed");
        }
        Ok(())
    }

    /// Get cache statistics if cache is available
    pub async fn get_cache_stats(&self) -> Option<crate::cache::CacheStats> {
        if let Some(cache) = &self.cache { cache.get_cache_stats().await.ok() } else { None }
    }
}
