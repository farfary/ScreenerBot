use crate::{
    database::Database,
    logger::Logger,
    types::WalletPosition,
    rpc::RpcManager,
    pricing::PricingManager,
    profit_calculator::ProfitLossCalculator,
};
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use solana_account_decoder::UiAccountData;
use spl_token_2022::state::Account;
use spl_token;
use std::{ collections::HashMap, sync::Arc };

#[derive(Debug, Clone)]
pub struct TokenHolding {
    pub mint: String,
    pub balance: u64,
    pub decimals: u8,
    pub token_account: String,
}

#[derive(Clone)]
pub struct PositionManager {
    database: Arc<Database>,
    rpc_manager: Arc<RpcManager>,
    pricing_manager: Option<Arc<PricingManager>>,
    profit_calculator: ProfitLossCalculator,
}

impl PositionManager {
    pub fn new(database: Arc<Database>, rpc_manager: Arc<RpcManager>) -> Self {
        let profit_calculator = ProfitLossCalculator::new(Arc::clone(&database));

        Self {
            database,
            rpc_manager,
            pricing_manager: None,
            profit_calculator,
        }
    }

    pub fn set_pricing_manager(&mut self, pricing_manager: Arc<PricingManager>) {
        self.pricing_manager = Some(pricing_manager);
    }

    /// Get current token holdings from the blockchain
    pub async fn get_current_token_holdings(
        &self,
        wallet_pubkey: &Pubkey
    ) -> Result<Vec<TokenHolding>> {
        Logger::wallet("🔍 Scanning blockchain for current token holdings...");

        let token_accounts = self.get_token_accounts_with_retry(wallet_pubkey).await?;
        let mut holdings = Vec::new();

        if token_accounts.is_empty() {
            return Ok(holdings);
        }

        Logger::wallet(&format!("📊 Processing {} token accounts...", token_accounts.len()));

        for (i, token_account) in token_accounts.iter().enumerate() {
            Logger::wallet(&format!("🔍 Analyzing account {}/{}", i + 1, token_accounts.len()));

            match &token_account.account.data {
                UiAccountData::Binary(_encoded_data, _encoding) => {
                    if let Some(data) = token_account.account.data.decode() {
                        if let Ok(account_data) = self.parse_token_account(&data) {
                            if account_data.amount > 0 {
                                let decimals = self
                                    .get_token_decimals(&account_data.mint).await
                                    .unwrap_or(9);

                                let holding = TokenHolding {
                                    mint: account_data.mint.to_string(),
                                    balance: account_data.amount,
                                    decimals,
                                    token_account: token_account.pubkey.clone(),
                                };

                                let actual_balance =
                                    (account_data.amount as f64) / (10_f64).powi(decimals as i32);
                                Logger::wallet(
                                    &format!(
                                        "💎 Found: {} | Balance: {:.6} | Decimals: {}",
                                        &account_data.mint.to_string()[..8],
                                        actual_balance,
                                        decimals
                                    )
                                );

                                holdings.push(holding);
                            }
                        }
                    }
                }
                UiAccountData::Json(parsed_data) => {
                    if let Some(info) = parsed_data.parsed.get("info") {
                        if
                            let (Some(mint_str), Some(amount_str), Some(decimals)) = (
                                info.get("mint").and_then(|v| v.as_str()),
                                info
                                    .get("tokenAmount")
                                    .and_then(|v| v.get("amount"))
                                    .and_then(|v| v.as_str()),
                                info
                                    .get("tokenAmount")
                                    .and_then(|v| v.get("decimals"))
                                    .and_then(|v| v.as_u64()),
                            )
                        {
                            if let Ok(amount) = amount_str.parse::<u64>() {
                                if amount > 0 {
                                    let holding = TokenHolding {
                                        mint: mint_str.to_string(),
                                        balance: amount,
                                        decimals: decimals as u8,
                                        token_account: token_account.pubkey.clone(),
                                    };

                                    let actual_balance =
                                        (amount as f64) / (10_f64).powi(decimals as i32);
                                    Logger::wallet(
                                        &format!(
                                            "💎 Found: {} | Balance: {:.6} | Decimals: {}",
                                            &mint_str[..8],
                                            actual_balance,
                                            decimals
                                        )
                                    );

                                    holdings.push(holding);
                                }
                            }
                        }
                    }
                }
                _ => {
                    Logger::debug("Skipping account with unsupported data format");
                }
            }
        }

        Logger::success(&format!("✅ Found {} non-zero token holdings", holdings.len()));
        Ok(holdings)
    }

    /// Calculate positions with current prices and P&L
    pub async fn calculate_positions_with_pnl(
        &self,
        holdings: Vec<TokenHolding>
    ) -> Result<HashMap<String, WalletPosition>> {
        Logger::wallet("💰 Calculating positions with current prices and P&L...");

        let mut positions = HashMap::new();

        for holding in holdings {
            Logger::wallet(&format!("📊 Calculating P&L for {}", &holding.mint[..8]));

            // Get current price in SOL
            let current_price_sol = self.get_token_price_in_sol(&holding.mint).await.unwrap_or(0.0);

            // Calculate position with P&L using profit calculator
            let position = self.profit_calculator.update_position_with_pnl(
                &holding.mint,
                holding.balance,
                holding.decimals,
                current_price_sol
            ).await?;

            // Save to database
            if let Err(e) = self.database.save_wallet_position(&position) {
                Logger::error(&format!("Failed to save position for {}: {}", holding.mint, e));
                continue;
            }

            let actual_balance = (holding.balance as f64) / (10_f64).powi(holding.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);

            Logger::wallet(
                &format!(
                    "✅ {}: {:.6} tokens | {:.6} SOL | {}{}%",
                    &holding.mint[..8],
                    actual_balance,
                    value_sol,
                    if position.pnl_percentage.unwrap_or(0.0) >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    position.pnl_percentage.unwrap_or(0.0)
                )
            );

            positions.insert(holding.mint.clone(), position);
        }

        Logger::success(&format!("✅ Calculated {} positions with P&L", positions.len()));
        Ok(positions)
    }

    async fn get_token_accounts_with_retry(
        &self,
        wallet_pubkey: &Pubkey
    ) -> Result<Vec<solana_client::rpc_response::RpcKeyedAccount>> {
        use solana_client::rpc_request::TokenAccountsFilter;

        let program_ids = [
            spl_token::id(), // Original SPL Token program
            spl_token_2022::id(), // Token-2022 program
        ];

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
                        &format!("FAILED to get accounts for program {}: {}", program_id, e)
                    );
                    continue;
                }
            }
        }

        Logger::wallet("No token accounts found with any program ID");
        Ok(Vec::new())
    }

    fn parse_token_account(&self, data: &[u8]) -> Result<Account> {
        use solana_sdk::program_pack::Pack;

        // Try SPL Token 2022 first
        if let Ok(account) = spl_token_2022::state::Account::unpack(data) {
            return Ok(account);
        }

        // Fallback to original SPL Token
        if let Ok(account) = spl_token::state::Account::unpack(data) {
            // Convert to Token-2022 format manually
            let state = match account.state {
                spl_token::state::AccountState::Uninitialized =>
                    spl_token_2022::state::AccountState::Uninitialized,
                spl_token::state::AccountState::Initialized =>
                    spl_token_2022::state::AccountState::Initialized,
                spl_token::state::AccountState::Frozen =>
                    spl_token_2022::state::AccountState::Frozen,
            };

            return Ok(Account {
                mint: account.mint,
                owner: account.owner,
                amount: account.amount,
                delegate: account.delegate.into(),
                state,
                is_native: account.is_native.into(),
                delegated_amount: account.delegated_amount,
                close_authority: account.close_authority.into(),
            });
        }

        anyhow::bail!("Failed to parse token account data");
    }

    async fn get_token_decimals(&self, mint: &Pubkey) -> Result<u8> {
        match self.rpc_manager.get_account(mint).await {
            Ok(account_info) => {
                let data = &account_info.data;
                // Try to parse as mint account
                if data.len() >= 82 {
                    // Minimum size for mint account
                    // Decimals are at offset 44 for both SPL Token and Token-2022
                    return Ok(data[44]);
                }
                Ok(9) // Default fallback
            }
            _ => Ok(9), // Default fallback
        }
    }

    async fn get_token_price_in_sol(&self, mint: &str) -> Result<f64> {
        if let Some(ref pricing_manager) = self.pricing_manager {
            if let Some(price_info) = pricing_manager.get_token_price(mint).await {
                // Get SOL/USD rate to convert USD price to SOL price
                if
                    let Some(sol_price_info) = pricing_manager.get_token_price(
                        "So11111111111111111111111111111111111111112"
                    ).await
                {
                    let sol_usd_rate = sol_price_info.price_usd;
                    if sol_usd_rate > 0.0 {
                        return Ok(price_info.price_usd / sol_usd_rate);
                    }
                }
            }
        }

        // Fallback to 0.0 if no pricing manager or price not found
        Ok(0.0)
    }
}
