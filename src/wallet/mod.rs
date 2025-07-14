use crate::core::{ BotResult, BotError, BotConfig, TokenBalance, WalletTransaction, RpcManager };
use solana_sdk::{
    pubkey::Pubkey,
    signature::{ Keypair, Signature },
    signer::Signer,
    transaction::Transaction,
};
use spl_token::state::Account as TokenAccount;
use std::collections::HashMap;
use std::str::FromStr;
use chrono::{ DateTime, Utc };

pub mod manager;
pub mod transactions;
pub mod balances;

pub use manager::*;
pub use transactions::*;
pub use balances::*;

/// Main wallet manager for the bot
#[derive(Debug)]
pub struct WalletManager {
    pub keypair: Keypair,
    pub public_key: Pubkey,
    config: BotConfig,
    rpc: RpcManager,
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
        })
    }

    /// Initialize the wallet manager
    pub async fn initialize(&mut self) -> BotResult<()> {
        log::info!("🔑 Initializing wallet manager...");
        log::info!("📍 Wallet address: {}", self.public_key);

        // Check wallet balance
        let balance = self.get_sol_balance().await?;
        log::info!("💰 SOL balance: {:.6} SOL", balance);

        if balance < 0.01 {
            log::warn!("⚠️ Low SOL balance, may not be able to perform trades");
        }

        // Verify RPC connection
        let health = self.rpc.health_check().await?;
        if !health {
            return Err(BotError::Rpc("RPC health check failed".to_string()));
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

    /// Get recent transactions
    pub async fn get_recent_transactions(&self) -> BotResult<Vec<WalletTransaction>> {
        let tx_manager = TransactionManager::new(&self.rpc);
        tx_manager.get_recent_transactions(&self.public_key, 50).await
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

    /// Get transaction history for a specific token
    pub async fn get_token_transaction_history(
        &self,
        token_mint: &Pubkey
    ) -> BotResult<Vec<WalletTransaction>> {
        let tx_manager = TransactionManager::new(&self.rpc);
        tx_manager.get_token_transactions(&self.public_key, token_mint).await
    }
}
