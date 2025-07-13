use crate::core::{ BotResult, BotError, TokenBalance, RpcManager };
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;
use chrono::Utc;
use std::collections::HashMap;

/// Manages wallet balance queries and updates
pub struct BalanceManager<'a> {
    rpc: &'a RpcManager,
}

impl<'a> BalanceManager<'a> {
    pub fn new(rpc: &'a RpcManager) -> Self {
        Self { rpc }
    }

    /// Get all token balances for a wallet
    pub async fn get_all_token_balances(&self, wallet: &Pubkey) -> BotResult<Vec<TokenBalance>> {
        log::debug!("📊 Fetching all token balances for wallet: {}", wallet);

        let mut balances = Vec::new();

        // Get SOL balance first
        let sol_balance = self.get_sol_balance(wallet).await?;
        balances.push(sol_balance);

        // Get all token accounts
        let token_accounts = self.get_token_accounts(wallet).await?;

        for account in token_accounts {
            if let Some(balance) = self.parse_token_account(account).await? {
                balances.push(balance);
            }
        }

        log::info!("💰 Found {} token balances", balances.len());
        Ok(balances)
    }

    /// Get SOL balance as TokenBalance struct
    async fn get_sol_balance(&self, wallet: &Pubkey) -> BotResult<TokenBalance> {
        let lamports = self.rpc.get_balance(wallet).await?;
        let sol_amount = (lamports as f64) / (crate::core::LAMPORTS_PER_SOL as f64);

        Ok(TokenBalance {
            mint: Pubkey::from_str(crate::core::WSOL_MINT).unwrap(),
            amount: lamports,
            decimals: 9,
            ui_amount: sol_amount,
            symbol: Some("SOL".to_string()),
            name: Some("Solana".to_string()),
            price_usd: None, // Will be filled by price service
            value_usd: None,
            last_updated: Utc::now(),
        })
    }

    /// Get all token accounts for a wallet
    async fn get_token_accounts(
        &self,
        wallet: &Pubkey
    ) -> BotResult<Vec<(Pubkey, solana_client::rpc_response::RpcKeyedAccount)>> {
        // Use a more direct approach to get token accounts
        let accounts = tokio::task
            ::spawn_blocking({
                let rpc_client = &self.rpc.client;
                let wallet = *wallet;
                move || {
                    rpc_client.get_token_accounts_by_owner(
                        &wallet,
                        solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id())
                    )
                }
            }).await
            .map_err(|e| BotError::Rpc(format!("Task failed: {}", e)))?
            .map_err(|e| BotError::Rpc(format!("Failed to get token accounts: {}", e)))?;

        Ok(
            accounts
                .into_iter()
                .map(|acc| (acc.pubkey, acc))
                .collect()
        )
    }

    /// Parse a token account into TokenBalance
    async fn parse_token_account(
        &self,
        (_pubkey, account): (Pubkey, solana_client::rpc_response::RpcKeyedAccount)
    ) -> BotResult<Option<TokenBalance>> {
        let account_data = account.account.data
            .decode()
            .ok_or_else(|| BotError::Parse("Failed to decode account data".to_string()))?;

        // Parse token account data
        if account_data.len() < TokenAccount::LEN {
            return Ok(None);
        }

        let token_account = TokenAccount::unpack(&account_data).map_err(|e|
            BotError::Parse(format!("Failed to unpack token account: {}", e))
        )?;

        // Skip empty accounts
        if token_account.amount == 0 {
            return Ok(None);
        }

        // Get token metadata
        let (symbol, name, decimals) = self
            .get_token_metadata(&token_account.mint).await
            .unwrap_or_else(|_| (None, None, 9)); // Default to 9 decimals if metadata fails

        let ui_amount = (token_account.amount as f64) / (10_f64).powi(decimals as i32);

        Ok(
            Some(TokenBalance {
                mint: token_account.mint,
                amount: token_account.amount,
                decimals,
                ui_amount,
                symbol,
                name,
                price_usd: None, // Will be filled by price service
                value_usd: None,
                last_updated: Utc::now(),
            })
        )
    }

    /// Get token metadata (symbol, name, decimals)
    async fn get_token_metadata(
        &self,
        mint: &Pubkey
    ) -> BotResult<(Option<String>, Option<String>, u8)> {
        // Try to get mint info
        let mint_info = tokio::task
            ::spawn_blocking({
                let rpc_client = &self.rpc.client;
                let mint = *mint;
                move || { rpc_client.get_account(&mint) }
            }).await
            .map_err(|e| BotError::Rpc(format!("Task failed: {}", e)))?
            .map_err(|e| BotError::Rpc(format!("Failed to get mint account: {}", e)))?;

        // Parse mint data to get decimals
        if mint_info.data.len() >= spl_token::state::Mint::LEN {
            if let Ok(mint_data) = spl_token::state::Mint::unpack(&mint_info.data) {
                // For now, just return decimals. Symbol and name would need metadata program
                return Ok((None, None, mint_data.decimals));
            }
        }

        // Default fallback
        Ok((None, None, 9))
    }

    /// Get specific token balance
    pub async fn get_token_balance(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey
    ) -> BotResult<Option<TokenBalance>> {
        let token_accounts = self.get_token_accounts(wallet).await?;

        for account in token_accounts {
            if let Some(balance) = self.parse_token_account(account).await? {
                if balance.mint == *mint {
                    return Ok(Some(balance));
                }
            }
        }

        Ok(None)
    }
}

use std::str::FromStr;
