//! Volume Aggregator Executor
//!
//! Handles the execution of volume generation sessions.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use tokio::time::{sleep, Duration};

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::rpc::get_rpc_client;
use crate::swaps::router::{Quote, QuoteRequest, SwapMode};
use crate::swaps::registry::get_registry;
use crate::tools::ToolStatus;
use crate::wallets::{self, WalletRole, WalletWithKey};

use super::types::{VolumeConfig, VolumeSession, VolumeTransaction};

/// Minimum SOL balance required per wallet for gas fees
const MIN_WALLET_BALANCE_SOL: f64 = 0.01;

/// Volume Aggregator for generating trading volume
pub struct VolumeAggregator {
    /// Configuration for this session
    config: VolumeConfig,
    /// Available wallets for execution (address, keypair)
    wallets: Vec<WalletWithKey>,
    /// Current execution status
    status: ToolStatus,
    /// Flag to abort execution
    abort_flag: Arc<AtomicBool>,
}

impl VolumeAggregator {
    /// Create a new volume aggregator with the given configuration
    pub fn new(config: VolumeConfig) -> Self {
        Self {
            config,
            wallets: Vec::new(),
            status: ToolStatus::Ready,
            abort_flag: Arc::new(AtomicBool::new(false)),
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

        // Load all wallets with keys
        let all_wallets = wallets::get_wallets_with_keys().await?;

        // Filter to only secondary wallets
        let mut secondary_wallets: Vec<WalletWithKey> = all_wallets
            .into_iter()
            .filter(|w| w.wallet.role == WalletRole::Secondary)
            .collect();

        if secondary_wallets.is_empty() {
            return Err(
                "No secondary wallets available. Create secondary wallets first.".to_string(),
            );
        }

        // Validate minimum wallet count
        if secondary_wallets.len() < 2 {
            return Err(format!(
                "At least 2 secondary wallets required for volume generation. Found: {}",
                secondary_wallets.len()
            ));
        }

        // Limit to requested number of wallets
        if secondary_wallets.len() > self.config.num_wallets {
            secondary_wallets.truncate(self.config.num_wallets);
        }

        // Check wallet balances
        let rpc_client = get_rpc_client();
        let mut valid_wallets = Vec::new();
        
        for wallet in secondary_wallets {
            match rpc_client.get_sol_balance(&wallet.wallet.address).await {
                Ok(balance) => {
                    if balance >= MIN_WALLET_BALANCE_SOL {
                        valid_wallets.push(wallet);
                    } else {
                        logger::warning(
                            LogTag::Tools,
                            &format!(
                                "Wallet {} has insufficient balance: {} SOL (min: {} SOL)",
                                wallet.wallet.address, balance, MIN_WALLET_BALANCE_SOL
                            ),
                        );
                    }
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Tools,
                        &format!(
                            "Failed to check balance for wallet {}: {}",
                            wallet.wallet.address, e
                        ),
                    );
                }
            }
        }

        if valid_wallets.len() < 2 {
            return Err(format!(
                "At least 2 wallets with sufficient balance required. Found: {} valid wallets",
                valid_wallets.len()
            ));
        }

        logger::info(
            LogTag::Tools,
            &format!(
                "Loaded {} secondary wallets with sufficient balance for volume aggregation",
                valid_wallets.len()
            ),
        );

        self.wallets = valid_wallets;
        self.status = ToolStatus::Ready;

        Ok(())
    }

    /// Execute the volume generation session
    pub async fn execute(&mut self) -> Result<VolumeSession, String> {
        if self.wallets.is_empty() {
            return Err("No wallets available. Call prepare() first.".to_string());
        }

        self.status = ToolStatus::Running;
        self.abort_flag.store(false, Ordering::SeqCst);

        let mut session = VolumeSession::new(&self.config.token_mint);

        logger::info(
            LogTag::Tools,
            &format!(
                "Starting volume generation session {} for token {}",
                session.session_id, self.config.token_mint
            ),
        );

        let mut remaining_volume = self.config.total_volume_sol;
        let mut tx_id = 0;
        let mut rng = StdRng::from_entropy();
        let mut wallet_idx = 0;

        // Track token holdings per wallet for sells
        let mut wallet_token_balances: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        while remaining_volume > 0.0 {
            // Check abort flag
            if self.abort_flag.load(Ordering::SeqCst) {
                logger::warning(
                    LogTag::Tools,
                    &format!("Volume session {} aborted by user", session.session_id),
                );
                self.status = ToolStatus::Aborted;
                session.fail("Aborted by user".to_string());
                return Ok(session);
            }

            // Calculate amount for this transaction
            let amount = if self.config.randomize_amounts {
                rng.gen_range(self.config.min_amount_sol..=self.config.max_amount_sol)
            } else {
                self.config.min_amount_sol
            };

            // Clamp to remaining volume
            let amount = amount.min(remaining_volume);

            // Skip if amount is too small
            if amount < 0.001 {
                break;
            }

            // Get wallet for this transaction (round-robin)
            let wallet = &self.wallets[wallet_idx % self.wallets.len()];
            wallet_idx += 1;

            // Determine if buy or sell based on wallet token balance
            // First transaction for each wallet should be a buy
            let wallet_address = wallet.wallet.address.clone();
            let token_balance = wallet_token_balances.get(&wallet_address).copied().unwrap_or(0);
            let is_buy = token_balance == 0 || tx_id % 2 == 0;

            // Create transaction record
            let mut tx = VolumeTransaction::new(tx_id, wallet_address.clone(), is_buy, amount);

            // Execute the swap
            match self.execute_swap(wallet, is_buy, amount).await {
                Ok((signature, token_amount)) => {
                    tx.confirm(signature.clone(), token_amount);
                    
                    // Update token balance tracking
                    if is_buy {
                        let current = wallet_token_balances.entry(wallet_address).or_insert(0);
                        *current += token_amount as u64;
                    } else {
                        let current = wallet_token_balances.entry(wallet_address).or_insert(0);
                        *current = current.saturating_sub(token_amount as u64);
                    }
                    
                    logger::info(
                        LogTag::Tools,
                        &format!(
                            "Volume tx {} confirmed: {} {} SOL, sig={}",
                            tx_id,
                            if is_buy { "BUY" } else { "SELL" },
                            amount,
                            signature
                        ),
                    );
                }
                Err(e) => {
                    tx.fail(e.clone());
                    logger::warning(
                        LogTag::Tools,
                        &format!("Volume tx {} failed: {}", tx_id, e),
                    );
                }
            }

            // Only count successful transactions toward volume
            if tx.status == super::types::TransactionStatus::Confirmed {
                remaining_volume -= amount;
            }

            session.add_transaction(tx);
            tx_id += 1;

            // Delay between transactions
            if remaining_volume > 0.0 {
                sleep(Duration::from_millis(self.config.delay_between_ms)).await;
            }
        }

        session.complete();
        self.status = ToolStatus::Completed;

        logger::info(
            LogTag::Tools,
            &format!(
                "Volume session {} completed: {} SOL volume, {} buys, {} sells, {} failed",
                session.session_id,
                session.total_volume_sol,
                session.successful_buys,
                session.successful_sells,
                session.failed_count
            ),
        );

        Ok(session)
    }

    /// Abort the current execution
    pub fn abort(&mut self) {
        self.abort_flag.store(true, Ordering::SeqCst);
        logger::info(LogTag::Tools, "Volume aggregator abort requested");
    }

    /// Execute a single swap transaction using the wallet's keypair
    async fn execute_swap(
        &self,
        wallet: &WalletWithKey,
        is_buy: bool,
        amount_sol: f64,
    ) -> Result<(String, f64), String> {
        let token_mint = self.config.token_mint.to_string();
        let wallet_address = wallet.wallet.address.clone();
        
        // Determine input/output based on buy/sell
        let (input_mint, output_mint, input_amount) = if is_buy {
            // Buy: SOL -> Token
            let lamports = (amount_sol * 1_000_000_000.0) as u64;
            (SOL_MINT.to_string(), token_mint.clone(), lamports)
        } else {
            // Sell: Token -> SOL
            // For sells, we need to get the token balance and sell that amount
            // The amount_sol here is what we expect to receive
            // We'll estimate based on the amount
            let estimated_tokens = (amount_sol * 1_000_000_000.0) as u64; // Rough estimate
            (token_mint.clone(), SOL_MINT.to_string(), estimated_tokens)
        };

        // Get slippage from config
        let slippage_pct = with_config(|cfg| cfg.swaps.slippage.quote_default_pct);

        // Create quote request
        let quote_request = QuoteRequest {
            input_mint: input_mint.clone(),
            output_mint: output_mint.clone(),
            input_amount,
            wallet_address: wallet_address.clone(),
            slippage_pct,
            swap_mode: SwapMode::ExactIn,
        };

        // Get quote from registry (uses best available router)
        let registry = get_registry();
        let enabled = registry.enabled_routers();
        
        if enabled.is_empty() {
            return Err("No swap routers enabled".to_string());
        }

        // Get quote from first enabled router (Jupiter preferred)
        let router = &enabled[0];
        let quote = router
            .get_quote(&quote_request)
            .await
            .map_err(|e| format!("Failed to get quote: {}", e))?;

        logger::debug(
            LogTag::Tools,
            &format!(
                "Got quote for {} {}: {} -> {} (impact: {:.2}%)",
                if is_buy { "BUY" } else { "SELL" },
                token_mint,
                quote.input_amount,
                quote.output_amount,
                quote.price_impact_pct
            ),
        );

        // Execute the swap with custom signing
        let signature = self
            .execute_swap_with_keypair(&quote, &wallet.keypair)
            .await?;

        // Calculate token amount received/sent
        let token_amount = if is_buy {
            quote.output_amount as f64
        } else {
            quote.input_amount as f64
        };

        Ok((signature, token_amount))
    }

    /// Execute a swap transaction signed with a specific keypair
    async fn execute_swap_with_keypair(
        &self,
        quote: &Quote,
        keypair: &Keypair,
    ) -> Result<String, String> {
        // Deserialize quote response from execution_data
        let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| format!("Quote deserialization failed: {}", e))?;

        // Build swap request for Jupiter API
        let swap_req = serde_json::json!({
            "userPublicKey": keypair.pubkey().to_string(),
            "quoteResponse": quote_response,
            "dynamicComputeUnitLimit": true,
            "prioritizationFeeLamports": with_config(|cfg| cfg.swaps.jupiter.default_priority_fee),
        });

        // Call Jupiter swap endpoint
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.jup.ag/swap/v1/swap")
            .header("x-api-key", "YOUR_JUPITER_API_KEY")
            .header("Content-Type", "application/json")
            .json(&swap_req)
            .send()
            .await
            .map_err(|e| format!("Jupiter swap request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown".to_string());
            return Err(format!("Jupiter swap failed ({}): {}", status, error_text));
        }

        #[derive(serde::Deserialize)]
        struct JupiterSwapResponse {
            #[serde(rename = "swapTransaction")]
            swap_transaction: String,
        }

        let swap_response: JupiterSwapResponse = response
            .json()
            .await
            .map_err(|e| format!("Jupiter swap response parse failed: {}", e))?;

        // Sign and send using the provided keypair
        let rpc_client = get_rpc_client();
        let signature = rpc_client
            .sign_send_and_confirm_with_keypair(&swap_response.swap_transaction, keypair)
            .await
            .map_err(|e| format!("Transaction failed: {}", e))?;

        Ok(signature)
    }
}
