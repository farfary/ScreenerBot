use crate::core::{ BotResult, BotError, TokenBalance, RpcManager };
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;
use solana_program::program_pack::Pack;
use chrono::Utc;
use std::str::FromStr;

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
        log::info!("🔍 Getting token accounts for wallet: {}", wallet);

        // Use SPL Token program ID
        let spl_token_program = spl_token::id();

        match self.rpc.get_token_accounts_by_owner(wallet, &spl_token_program).await {
            Ok(accounts) => {
                log::info!("✅ Found {} token accounts", accounts.len());
                Ok(accounts)
            }
            Err(e) => {
                log::warn!("⚠️ Failed to get token accounts: {}, using empty list", e);
                Ok(Vec::new())
            }
        }
    }

    /// Parse a token account into TokenBalance
    async fn parse_token_account(
        &self,
        (_pubkey, account): (Pubkey, solana_client::rpc_response::RpcKeyedAccount)
    ) -> BotResult<Option<TokenBalance>> {
        // Handle different account data formats
        let account_data = match &account.account.data {
            solana_account_decoder::UiAccountData::Binary(data, encoding) => {
                match encoding {
                    solana_account_decoder::UiAccountEncoding::Base64 => {
                        use base64::prelude::*;
                        BASE64_STANDARD.decode(data).map_err(|e| {
                            log::debug!("Base64 decode error: {}", e);
                            BotError::Parse(format!("Failed to decode base64 account data: {}", e))
                        })?
                    }
                    solana_account_decoder::UiAccountEncoding::Base58 => {
                        bs58::decode(data).into_vec().map_err(|e| {
                            log::debug!("Base58 decode error: {}", e);
                            BotError::Parse(format!("Failed to decode base58 account data: {}", e))
                        })?
                    }
                    _ => {
                        log::debug!("Unsupported account data encoding: {:?}", encoding);
                        return Err(BotError::Parse(format!("Unsupported account data encoding: {:?}", encoding)));
                    }
                }
            }
            solana_account_decoder::UiAccountData::Json(_) => {
                log::debug!("Received JSON account data format, skipping token account parsing");
                return Ok(None);
            }
            solana_account_decoder::UiAccountData::LegacyBinary(data) => {
                use base64::prelude::*;
                BASE64_STANDARD.decode(data).map_err(|e| {
                    log::debug!("Legacy binary decode error: {}", e);
                    BotError::Parse(format!("Failed to decode legacy binary account data: {}", e))
                })?
            }
        };

        // Parse token account data
        if account_data.len() < TokenAccount::LEN {
            log::debug!(
                "🔍 Account data too short: {} bytes, expected at least {}",
                account_data.len(),
                TokenAccount::LEN
            );
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
        _mint: &Pubkey
    ) -> BotResult<(Option<String>, Option<String>, u8)> {
        // Simplified implementation for compilation
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
