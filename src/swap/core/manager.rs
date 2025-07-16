use crate::swap::dex::{ GmgnSwap, JupiterSwap, RaydiumSwap };
use crate::swap::core::routes::RouteSelector;
use crate::swap::types::*;
use crate::rpc_manager::RpcManager;
use crate::trading::transaction_manager::TransactionManager;
use crate::types::TransactionType;
use anyhow::Result;
use base64::{ Engine as _, engine::general_purpose };
use solana_sdk::{
    signature::{ Keypair, Signature, Signer },
    transaction::{ Transaction, VersionedTransaction },
    message::VersionedMessage,
};
use std::sync::Arc;
use std::time::{ Duration, Instant };

pub struct SwapManager {
    config: SwapConfig,
    jupiter: Option<JupiterSwap>,
    raydium: Option<RaydiumSwap>,
    gmgn: Option<GmgnSwap>,
    route_selector: RouteSelector,
    rpc_manager: Arc<RpcManager>,
    transaction_manager: Arc<TransactionManager>,
}

impl SwapManager {
    pub fn new(
        config: SwapConfig,
        rpc_manager: Arc<RpcManager>,
        transaction_manager: Arc<TransactionManager>
    ) -> Self {
        let jupiter = if config.jupiter.enabled {
            Some(JupiterSwap::new(config.jupiter.clone()))
        } else {
            None
        };

        let raydium = if config.raydium.enabled {
            Some(RaydiumSwap::new(config.raydium.clone()))
        } else {
            None
        };

        let gmgn = if config.gmgn.enabled {
            Some(GmgnSwap::new(config.gmgn.clone()))
        } else {
            None
        };

        let route_selector = RouteSelector::new(config.clone());

        Self {
            config,
            jupiter,
            raydium,
            gmgn,
            route_selector,
            rpc_manager,
            transaction_manager,
        }
    }

    /// Execute a swap with automatic DEX selection and route optimization
    pub async fn execute_swap(
        &self,
        request: SwapRequest,
        wallet_keypair: &Keypair
    ) -> Result<SwapResult, SwapError> {
        if !self.config.enabled {
            return Err(SwapError::DexNotAvailable("Swap module is disabled".to_string()));
        }

        log::info!(
            "🔄 Starting swap: {} {} -> {} {}",
            request.amount,
            request.input_mint,
            "?",
            request.output_mint
        );

        let start_time = Instant::now();

        // Get quotes from all available DEXes
        let routes = self.get_all_quotes(&request).await?;

        if routes.is_empty() {
            return Err(SwapError::InvalidRoute("No routes found".to_string()));
        }

        // Filter routes by slippage
        let filtered_routes = self.route_selector.filter_by_slippage(routes);

        if filtered_routes.is_empty() {
            return Err(SwapError::SlippageTooHigh {
                expected: self.config.max_slippage,
                actual: 999.0, // Indicates all routes exceeded slippage
            });
        }

        // Select the best route
        let best_route = self.route_selector.select_best_route(filtered_routes)?;

        log::info!(
            "✅ Selected route: {} | Output: {} | Impact: {}%",
            best_route.dex,
            best_route.out_amount,
            best_route.price_impact_pct
        );

        // Execute the swap
        let result = self.execute_route(&best_route, &request, wallet_keypair).await?;

        let total_time = start_time.elapsed();
        log::info!(
            "🎯 Swap completed in {:.2}s: {} -> {} ({})",
            total_time.as_secs_f64(),
            result.input_amount,
            result.output_amount,
            result.dex_used
        );

        Ok(result)
    }

    /// Get quotes from all enabled DEXes
    async fn get_all_quotes(&self, request: &SwapRequest) -> Result<Vec<SwapRoute>, SwapError> {
        let mut routes = Vec::new();

        // Try Jupiter
        if let Some(ref jupiter) = self.jupiter {
            match jupiter.get_quote(request).await {
                Ok(route) => routes.push(route),
                Err(e) => log::warn!("Jupiter quote failed: {}", e),
            }
        }

        // Try Raydium
        if let Some(ref raydium) = self.raydium {
            match raydium.get_quote(request).await {
                Ok(route) => routes.push(route),
                Err(e) => log::warn!("Raydium quote failed: {}", e),
            }
        }

        // Try GMGN
        if let Some(ref gmgn) = self.gmgn {
            match gmgn.get_quote(request).await {
                Ok(route) => routes.push(route),
                Err(e) => log::warn!("GMGN quote failed: {}", e),
            }
        }

        log::info!("📊 Received {} quotes from DEXes", routes.len());

        Ok(routes)
    }

    /// Execute a specific route
    async fn execute_route(
        &self,
        route: &SwapRoute,
        request: &SwapRequest,
        wallet_keypair: &Keypair
    ) -> Result<SwapResult, SwapError> {
        let start_time = Instant::now();
        let user_public_key = wallet_keypair.pubkey().to_string();

        // Get the transaction from the appropriate DEX
        let swap_transaction = match route.dex {
            DexType::Jupiter => {
                let jupiter = self.jupiter
                    .as_ref()
                    .ok_or_else(||
                        SwapError::DexNotAvailable("Jupiter not available".to_string())
                    )?;
                jupiter.get_swap_transaction(route, &user_public_key).await?
            }
            DexType::Raydium => {
                let raydium = self.raydium
                    .as_ref()
                    .ok_or_else(||
                        SwapError::DexNotAvailable("Raydium not available".to_string())
                    )?;
                raydium.get_swap_transaction(route, &user_public_key).await?
            }
            DexType::Gmgn => {
                let gmgn = self.gmgn
                    .as_ref()
                    .ok_or_else(|| SwapError::DexNotAvailable("GMGN not available".to_string()))?;
                gmgn.get_swap_transaction(route, &user_public_key).await?
            }
        };

        // Decode and sign the transaction
        let mut transaction = self.decode_transaction(&swap_transaction.swap_transaction)?;

        // Get latest blockhash
        let blockhash = self.rpc_manager
            .get_latest_blockhash().await
            .map_err(|e| SwapError::TransactionFailed(format!("Failed to get blockhash: {}", e)))?;

        transaction
            .try_sign(&[wallet_keypair], blockhash)
            .map_err(|e|
                SwapError::TransactionFailed(format!("Failed to sign transaction: {}", e))
            )?;

        // Send the transaction
        let signature = self.send_transaction_with_retry(&transaction).await?;

        // Get current block height
        let block_height = self.rpc_manager.get_block_height().await.unwrap_or(0);

        // Parse amounts
        let input_amount = route.in_amount.parse().unwrap_or(0);
        let output_amount = route.out_amount.parse().unwrap_or(0);
        let price_impact = route.price_impact_pct.parse().unwrap_or(0.0);

        // Record the transaction
        let transaction_type = if request.input_mint == SOL_MINT {
            TransactionType::Sell
        } else {
            TransactionType::Buy
        };

        let amount_sol = if request.input_mint == SOL_MINT {
            (input_amount as f64) / 1_000_000_000.0 // Convert lamports to SOL
        } else {
            (output_amount as f64) / 1_000_000_000.0
        };

        let amount_tokens = if request.input_mint == SOL_MINT {
            output_amount as f64
        } else {
            input_amount as f64
        };

        let price = if amount_tokens > 0.0 { amount_sol / amount_tokens } else { 0.0 };

        // Estimate fee (5000 lamports base + additional for complex routes)
        let fee_lamports = 5000 + (route.route_plan.len() as u64) * 2000;
        let fee_sol = (fee_lamports as f64) / 1_000_000_000.0;

        if
            let Err(e) = self.transaction_manager.record_transaction(
                signature.to_string(),
                transaction_type,
                if request.input_mint == SOL_MINT {
                    request.output_mint.clone()
                } else {
                    request.input_mint.clone()
                },
                amount_sol,
                amount_tokens,
                price,
                block_height,
                fee_sol,
                None // position_id
            ).await
        {
            log::warn!("Failed to record transaction: {}", e);
        }

        Ok(SwapResult {
            success: true,
            signature: Some(signature.to_string()),
            dex_used: route.dex.to_string(),
            input_amount,
            output_amount,
            slippage: 0.0, // TODO: Calculate actual slippage
            fee: 0, // TODO: Calculate proper fee
            fee_lamports,
            price_impact,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            route: route.clone(),
            error: None,
            block_height: Some(block_height),
        })
    }

    /// Send transaction with retry logic
    async fn send_transaction_with_retry(
        &self,
        transaction: &Transaction
    ) -> Result<Signature, SwapError> {
        let mut last_error = None;

        for attempt in 1..=self.config.retry_attempts {
            log::debug!("Sending transaction attempt {}/{}", attempt, self.config.retry_attempts);

            let transaction_clone = transaction.clone();
            match self.rpc_manager.send_and_confirm_transaction(&transaction_clone).await {
                Ok(signature) => {
                    log::info!("✅ Transaction confirmed: {}", signature);
                    return Ok(signature);
                }
                Err(e) => {
                    log::warn!("❌ Transaction attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < self.config.retry_attempts {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Err(
            SwapError::TransactionFailed(
                format!(
                    "Failed after {} attempts: {}",
                    self.config.retry_attempts,
                    last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"))
                )
            )
        )
    }

    /// Decode base64 transaction - supports both legacy and versioned transactions
    fn decode_transaction(&self, transaction_data: &str) -> Result<Transaction, SwapError> {
        let transaction_bytes = general_purpose::STANDARD
            .decode(transaction_data)
            .map_err(|e| SwapError::SerializationError(format!("Base64 decode failed: {}", e)))?;

        // First try to deserialize as a legacy transaction
        if let Ok(transaction) = bincode::deserialize::<Transaction>(&transaction_bytes) {
            return Ok(transaction);
        }

        // If that fails, try to deserialize as a versioned transaction and convert to legacy
        match bincode::deserialize::<VersionedTransaction>(&transaction_bytes) {
            Ok(versioned_tx) => {
                // Convert versioned transaction to legacy transaction
                match versioned_tx.message {
                    VersionedMessage::Legacy(legacy_message) => {
                        Ok(Transaction {
                            signatures: versioned_tx.signatures,
                            message: legacy_message,
                        })
                    }
                    VersionedMessage::V0(_) => {
                        // For V0 messages, we need to resolve the lookup tables
                        // For now, return an error suggesting to use legacy transactions
                        Err(
                            SwapError::SerializationError(
                                "Versioned transactions with lookup tables not supported. Please use legacy transactions.".to_string()
                            )
                        )
                    }
                }
            }
            Err(e) =>
                Err(
                    SwapError::SerializationError(
                        format!("Failed to deserialize transaction (both legacy and versioned): {}", e)
                    )
                ),
        }
    }

    /// Get the best quote without executing
    pub async fn get_best_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        let routes = self.get_all_quotes(request).await?;

        if routes.is_empty() {
            return Err(SwapError::InvalidRoute("No routes found".to_string()));
        }

        let filtered_routes = self.route_selector.filter_by_slippage(routes);

        if filtered_routes.is_empty() {
            return Err(SwapError::SlippageTooHigh {
                expected: self.config.max_slippage,
                actual: 999.0,
            });
        }

        self.route_selector.select_best_route(filtered_routes)
    }

    /// Check if a specific DEX is available
    pub fn is_dex_available(&self, dex: &DexType) -> bool {
        match dex {
            DexType::Jupiter => self.jupiter.as_ref().map_or(false, |j| j.is_enabled()),
            DexType::Raydium => self.raydium.as_ref().map_or(false, |r| r.is_enabled()),
            DexType::Gmgn => self.gmgn.as_ref().map_or(false, |g| g.is_enabled()),
        }
    }

    /// Get swap configuration
    pub fn get_config(&self) -> &SwapConfig {
        &self.config
    }

    /// Update swap configuration
    pub fn update_config(&mut self, config: SwapConfig) {
        self.config = config.clone();

        // Update route selector with new config
        self.route_selector = RouteSelector::new(config);

        log::info!("🔧 Swap configuration updated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ Config, TransactionManagerConfig };
    use crate::database::Database;
    use crate::wallet::WalletTracker;

    fn create_test_swap_config() -> SwapConfig {
        SwapConfig {
            enabled: true,
            default_dex: "jupiter".to_string(),
            is_anti_mev: false,
            max_slippage: 0.01,
            timeout_seconds: 30,
            retry_attempts: 3,
            dex_preferences: vec!["jupiter".to_string(), "raydium".to_string(), "gmgn".to_string()],
            jupiter: JupiterConfig {
                enabled: true,
                base_url: "https://quote-api.jup.ag/v6".to_string(),
                timeout_seconds: 15,
                max_accounts: 64,
                only_direct_routes: false,
                as_legacy_transaction: false,
            },
            raydium: RaydiumConfig {
                enabled: true,
                base_url: "https://api.raydium.io/v2".to_string(),
                timeout_seconds: 15,
                pool_type: "all".to_string(),
            },
            gmgn: GmgnConfig {
                enabled: false, // Disabled for testing since we don't have API key
                base_url: "https://gmgn.ai/defi/quoterv1".to_string(),
                timeout_seconds: 15,
                api_key: "".to_string(),
                referral_account: "".to_string(),
                referral_fee_bps: 0,
            },
        }
    }

    #[tokio::test]
    async fn test_swap_manager_creation() {
        let config = create_test_swap_config();
        let rpc_manager = Arc::new(
            RpcManager::new("https://api.mainnet-beta.solana.com".to_string(), vec![])
        );
        let database = Arc::new(Database::new("test.db").unwrap());

        // Create a test config with a valid private key
        let test_keypair = Keypair::new();
        let mut test_config = Config::default();
        test_config.main_wallet_private = bs58::encode(&test_keypair.to_bytes()).into_string();

        let wallet_tracker = match WalletTracker::new(test_config.clone(), database.clone()) {
            Ok(tracker) => Arc::new(tracker),
            Err(_) => {
                // Skip test if wallet creation fails (expected in test environment)
                println!("Skipping test due to wallet creation failure in test environment");
                return;
            }
        };

        let transaction_manager = Arc::new(
            TransactionManager::new(
                TransactionManagerConfig {
                    cache_transactions: true,
                    cache_duration_hours: 24,
                    track_pnl: true,
                    auto_calculate_profits: true,
                },
                database,
                wallet_tracker
            )
        );

        let swap_manager = SwapManager::new(config, rpc_manager, transaction_manager);

        assert!(swap_manager.is_dex_available(&DexType::Jupiter));
        assert!(swap_manager.is_dex_available(&DexType::Raydium));
        assert!(!swap_manager.is_dex_available(&DexType::Gmgn)); // Disabled in test config
    }

    #[tokio::test]
    async fn test_get_quotes() {
        let config = create_test_swap_config();
        let rpc_manager = Arc::new(
            RpcManager::new("https://api.mainnet-beta.solana.com".to_string(), vec![])
        );
        let database = Arc::new(Database::new("test.db").unwrap());

        // Create a test config with a valid private key
        let test_keypair = Keypair::new();
        let mut test_config = Config::default();
        test_config.main_wallet_private = bs58::encode(&test_keypair.to_bytes()).into_string();

        let wallet_tracker = match WalletTracker::new(test_config.clone(), database.clone()) {
            Ok(tracker) => Arc::new(tracker),
            Err(_) => {
                // Skip test if wallet creation fails (expected in test environment)
                println!("Skipping test due to wallet creation failure in test environment");
                return;
            }
        };

        let transaction_manager = Arc::new(
            TransactionManager::new(
                TransactionManagerConfig {
                    cache_transactions: true,
                    cache_duration_hours: 24,
                    track_pnl: true,
                    auto_calculate_profits: true,
                },
                database,
                wallet_tracker
            )
        );

        let swap_manager = SwapManager::new(config, rpc_manager, transaction_manager);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 1_000_000, // 0.001 SOL
            slippage_bps: 50, // 0.5%
            user_public_key: "11111111111111111111111111111111".to_string(),
            dex_preference: None,
            is_anti_mev: false,
        };

        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                println!("Best quote found:");
                println!("  DEX: {}", route.dex);
                println!("  Input: {} {}", route.in_amount, route.input_mint);
                println!("  Output: {} {}", route.out_amount, route.output_mint);
                println!("  Price Impact: {}%", route.price_impact_pct);
                assert!(!route.out_amount.is_empty());
            }
            Err(e) => {
                println!("Quote request failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }
}
