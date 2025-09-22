/// Price calculator module
///
/// This module handles the core price calculation logic:
/// - Decodes pool account data using program-specific decoders
/// - Calculates token prices from pool reserves (SOL-based pricing only)
/// - Handles price triangulation for indirect pairs
/// - Updates price cache and history

use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_token_decimals_sync, decimals::SOL_DECIMALS };
use crate::events::{ record_safe, Event, EventCategory, Severity };
use super::cache;
use super::decoders;
use super::fetcher::{ AccountData, PoolAccountBundle };
use super::types::{ PriceResult, ProgramKind, PoolDescriptor, SOL_MINT };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{ Arc, RwLock };
use std::time::Instant;
use tokio::sync::{ mpsc, Notify };

/// Message types for calculator communication
#[derive(Debug, Clone)]
pub enum CalculatorMessage {
    /// Request to calculate price for a pool bundle
    CalculatePool {
        pool_id: Pubkey,
        pool_descriptor: PoolDescriptor,
        account_bundle: PoolAccountBundle,
    },
    /// Signal shutdown
    Shutdown,
}

/// Pool calculation result
#[derive(Debug, Clone)]
pub struct PoolCalculationResult {
    pub pool_id: Pubkey,
    pub price_result: Option<PriceResult>,
    pub error: Option<String>,
}

/// Price calculation engine
pub struct PriceCalculator {
    /// Pool directory for metadata
    pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    /// Channel for receiving calculation requests
    calculator_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<CalculatorMessage>>>>,
    /// Channel sender for sending calculation requests
    calculator_tx: mpsc::UnboundedSender<CalculatorMessage>,
    /// SOL price reference (assuming 1 SOL = X USD, but we only use SOL prices)
    sol_reference_price: Arc<RwLock<f64>>,
}

impl PriceCalculator {
    /// Create new price calculator
    pub fn new(pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>) -> Self {
        let (calculator_tx, calculator_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory,
            calculator_rx: Arc::new(RwLock::new(Some(calculator_rx))),
            calculator_tx,
            sol_reference_price: Arc::new(RwLock::new(100.0)),
        }
    }

    /// Get sender for sending calculation requests
    pub fn get_sender(&self) -> mpsc::UnboundedSender<CalculatorMessage> {
        self.calculator_tx.clone()
    }

    /// Start calculator background task
    pub async fn start_calculator_task(&self, shutdown: Arc<Notify>) {
        if is_debug_pool_calculator_enabled() {
            log(LogTag::PoolCalculator, "INFO", "Starting price calculator task");
        }

        let pool_directory = self.pool_directory.clone();
        let sol_reference_price = self.sol_reference_price.clone();

        // Take the receiver from the Arc<RwLock>
        let mut calculator_rx = {
            let mut rx_lock = self.calculator_rx.write().unwrap();
            rx_lock.take().expect("Calculator receiver already taken")
        };

        tokio::spawn(async move {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "INFO", "Price calculator task started");
            }

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        if is_debug_pool_calculator_enabled() {
                            log(LogTag::PoolCalculator, "INFO", "Price calculator task shutting down");
                        }
                        break;
                    }

                    message = calculator_rx.recv() => {
                        match message {
                            Some(CalculatorMessage::CalculatePool { 
                                pool_id, 
                                pool_descriptor, 
                                account_bundle 
                            }) => {
                                let calculation_start = Instant::now();
                                let token_mint = if pool_descriptor.base_mint.to_string() != SOL_MINT { 
                                    pool_descriptor.base_mint.to_string()
                                } else { 
                                    pool_descriptor.quote_mint.to_string()
                                };
                                
                                record_safe(Event::info(
                                    EventCategory::Pool,
                                    Some("price_calculation_started".to_string()),
                                    Some(token_mint.clone()),
                                    Some(pool_id.to_string()),
                                    serde_json::json!({
                                        "pool_id": pool_id.to_string(),
                                        "program_kind": format!("{:?}", pool_descriptor.program_kind),
                                        "base_mint": pool_descriptor.base_mint.to_string(),
                                        "quote_mint": pool_descriptor.quote_mint.to_string()
                                    })
                                )).await;
                                
                                let result = Self::calculate_pool_price_static(
                                    pool_id,
                                    &pool_descriptor,
                                    &account_bundle,
                                    &sol_reference_price,
                                ).await;

                                let calculation_duration = calculation_start.elapsed();

                                if let Some(price_result) = result.price_result {
                                    // Update cache with calculated price
                                    cache::update_price(price_result.clone());

                                    record_safe(Event::info(
                                        EventCategory::Pool,
                                        Some("price_calculation_success".to_string()),
                                        Some(token_mint.clone()),
                                        Some(pool_id.to_string()),
                                        serde_json::json!({
                                            "pool_id": pool_id.to_string(),
                                            "token_mint": token_mint,
                                            "price_sol": price_result.price_sol,
                                            "sol_reserves": price_result.sol_reserves,
                                            "token_reserves": price_result.token_reserves,
                                            "duration_ms": calculation_duration.as_millis(),
                                            "program_kind": format!("{:?}", pool_descriptor.program_kind)
                                        })
                                    )).await;

                                    if is_debug_pool_calculator_enabled() {
                                        log(
                                            LogTag::PoolCalculator,
                                            "SUCCESS",
                                            &format!(
                                                "Calculated price for token {} in pool {}: {} SOL",
                                                price_result.mint,
                                                pool_id,
                                                price_result.price_sol
                                            )
                                        );
                                    }
                                } else if let Some(error) = result.error {
                                    record_safe(Event::error(
                                        EventCategory::Pool,
                                        Some("price_calculation_failed".to_string()),
                                        Some(token_mint.clone()),
                                        Some(pool_id.to_string()),
                                        serde_json::json!({
                                            "pool_id": pool_id.to_string(),
                                            "token_mint": token_mint,
                                            "error": error,
                                            "duration_ms": calculation_duration.as_millis(),
                                            "program_kind": format!("{:?}", pool_descriptor.program_kind),
                                            "account_count": account_bundle.accounts.len()
                                        })
                                    )).await;
                                    
                                    log(
                                        LogTag::PoolCalculator,
                                        "WARN",
                                        &format!("Failed to calculate price for token {} in pool {}: {}", 
                                            token_mint,
                                            pool_id, 
                                            error)
                                    );
                                }
                            }

                            Some(CalculatorMessage::Shutdown) => {
                                if is_debug_pool_calculator_enabled() {
                                    log(LogTag::PoolCalculator, "INFO", "Calculator received shutdown signal");
                                }
                                break;
                            }

                            None => {
                                if is_debug_pool_calculator_enabled() {
                                    log(LogTag::PoolCalculator, "INFO", "Calculator channel closed");
                                }
                                break;
                            }
                        }
                    }
                }
            }

            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "INFO", "Price calculator task completed");
            }
        });
    }

    /// Calculate price for a pool (static version for task)
    async fn calculate_pool_price_static(
        pool_id: Pubkey,
        pool_descriptor: &PoolDescriptor,
        account_bundle: &PoolAccountBundle,
        sol_reference_price: &Arc<RwLock<f64>>
    ) -> PoolCalculationResult {
        if is_debug_pool_calculator_enabled() {
            let target_token = if pool_descriptor.base_mint.to_string() != SOL_MINT {
                pool_descriptor.base_mint
            } else {
                pool_descriptor.quote_mint
            };
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "Calculating price for pool {} ({}) - {}/{} (token: {})",
                    pool_id,
                    pool_descriptor.program_kind.display_name(),
                    pool_descriptor.base_mint,
                    pool_descriptor.quote_mint,
                    target_token
                )
            );
        }

        // Check if bundle has required accounts
        if !account_bundle.is_complete(&pool_descriptor.reserve_accounts) {
            let target_mint = if pool_descriptor.base_mint.to_string() != SOL_MINT {
                pool_descriptor.base_mint.to_string()
            } else {
                pool_descriptor.quote_mint.to_string()
            };

            record_safe(
                Event::warn(
                    EventCategory::Pool,
                    Some("incomplete_account_bundle".to_string()),
                    Some(target_mint.clone()),
                    Some(pool_id.to_string()),
                    serde_json::json!({
                    "pool_id": pool_id.to_string(),
                    "program_kind": format!("{:?}", pool_descriptor.program_kind),
                    "target_mint": target_mint,
                    "required_accounts": pool_descriptor.reserve_accounts.len(),
                    "available_accounts": account_bundle.accounts.len(),
                    "error": "Missing required pool accounts"
                })
                )
            ).await;

            return PoolCalculationResult {
                pool_id,
                price_result: None,
                error: Some("Incomplete account bundle".to_string()),
            };
        }

        // Convert account bundle to format expected by decoders
        let accounts_map = Self::convert_bundle_to_accounts_map(account_bundle);

        // Determine which token we're calculating price for (the non-SOL token)
        // Note: Discovery stage already ensures one side is SOL, so this should always succeed
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).unwrap();
        let (target_mint, _sol_is_base) = if pool_descriptor.base_mint == sol_mint_pubkey {
            (pool_descriptor.quote_mint, true)
        } else {
            // quote_mint must be SOL since discovery filters non-SOL pairs
            (pool_descriptor.base_mint, false)
        };

        // Use decoder to calculate price
        // Pass the target token as base_mint and SOL as quote_mint for consistent decoding
        let target_mint_str = target_mint.to_string();
        let sol_mint_str = SOL_MINT.to_string();

        let decoded_result = decoders::decode_pool(
            pool_descriptor.program_kind,
            &accounts_map,
            &target_mint_str,
            &sol_mint_str
        );

        match decoded_result {
            Some(mut price_result) => {
                // Ensure we're returning price for the target token (not SOL)
                price_result.mint = target_mint.to_string();
                price_result.pool_address = pool_id.to_string();
                price_result.slot = account_bundle.slot;

                // Calculate confidence based on liquidity and freshness
                let confidence = Self::calculate_confidence(&price_result, account_bundle);
                price_result.confidence = confidence;

                PoolCalculationResult {
                    pool_id,
                    price_result: Some(price_result),
                    error: None,
                }
            }
            None => {
                record_safe(
                    Event::error(
                        EventCategory::Pool,
                        Some("decoder_failed".to_string()),
                        Some(target_mint.to_string()),
                        Some(pool_id.to_string()),
                        serde_json::json!({
                        "pool_id": pool_id.to_string(),
                        "program_kind": format!("{:?}", pool_descriptor.program_kind),
                        "target_mint": target_mint.to_string(),
                        "account_count": accounts_map.len(),
                        "required_accounts": pool_descriptor.reserve_accounts.len(),
                        "error": "Decoder failed to parse pool data"
                    })
                    )
                ).await;

                PoolCalculationResult {
                    pool_id,
                    price_result: None,
                    error: Some("Decoder failed to parse pool data".to_string()),
                }
            }
        }
    }

    /// Convert PoolAccountBundle to the format expected by decoders
    fn convert_bundle_to_accounts_map(bundle: &PoolAccountBundle) -> HashMap<String, AccountData> {
        bundle.accounts
            .iter()
            .map(|(pubkey, account_data)| (pubkey.to_string(), account_data.clone()))
            .collect()
    }

    /// Calculate confidence score based on liquidity and data freshness
    fn calculate_confidence(price_result: &PriceResult, bundle: &PoolAccountBundle) -> f32 {
        let mut confidence = 1.0f32;

        // Reduce confidence based on age
        let age_seconds = bundle.last_updated.elapsed().as_secs();
        if age_seconds > 10 {
            confidence *= 0.9; // 10% reduction for data older than 10 seconds
        }
        if age_seconds > 30 {
            confidence *= 0.8; // Additional 20% reduction for data older than 30 seconds
        }

        // Reduce confidence for very low liquidity (less than 1 SOL)
        if price_result.sol_reserves < 1.0 {
            confidence *= 0.5;
        } else if price_result.sol_reserves < 10.0 {
            confidence *= 0.8;
        }

        // Ensure confidence is between 0.0 and 1.0
        confidence.max(0.0).min(1.0)
    }

    /// Public interface: Request calculation for a pool
    pub fn request_calculation(
        &self,
        pool_id: Pubkey,
        pool_descriptor: PoolDescriptor,
        account_bundle: PoolAccountBundle
    ) -> Result<(), String> {
        let message = CalculatorMessage::CalculatePool {
            pool_id,
            pool_descriptor,
            account_bundle,
        };

        self.calculator_tx
            .send(message)
            .map_err(|e| format!("Failed to send calculation request: {}", e))?;

        Ok(())
    }

    /// Calculate price from pool account data (synchronous version for direct use)
    pub fn calculate_price_sync(
        &self,
        pool_accounts: &HashMap<String, AccountData>,
        program_kind: ProgramKind,
        base_mint: &str,
        quote_mint: &str,
        pool_id: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "Calculating price for token {} in pool {} using {} decoder",
                    base_mint,
                    pool_id,
                    program_kind.display_name()
                )
            );
        }

        // Use decoder to parse and calculate
        let mut price_result = decoders::decode_pool(
            program_kind,
            pool_accounts,
            base_mint,
            quote_mint
        )?;

        // Set pool address
        price_result.pool_address = pool_id.to_string();

        // Calculate confidence (simplified for sync version)
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let mut confidence = 1.0f32;
        if age_seconds > 10 {
            confidence *= 0.9;
        }
        if price_result.sol_reserves < 1.0 {
            confidence *= 0.5;
        }
        price_result.confidence = confidence.max(0.0).min(1.0);

        Some(price_result)
    }

    /// Update price in cache
    pub fn update_price(&self, price: PriceResult) {
        cache::update_price(price);
    }

    /// Get calculation statistics
    pub fn get_calculation_stats(&self) -> CalculationStats {
        // For now, return basic stats
        // In a full implementation, we would track detailed metrics
        CalculationStats {
            total_calculations: 0,
            successful_calculations: 0,
            failed_calculations: 0,
            average_confidence: 0.0,
        }
    }

    /// Update SOL reference price (for future USD calculations if needed)
    pub fn update_sol_reference_price(&self, sol_price_usd: f64) {
        let mut reference = self.sol_reference_price.write().unwrap();
        *reference = sol_price_usd;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Updated SOL reference price to ${:.2}", sol_price_usd)
            );
        }
    }
}

/// Calculation statistics
#[derive(Debug, Clone)]
pub struct CalculationStats {
    pub total_calculations: u64,
    pub successful_calculations: u64,
    pub failed_calculations: u64,
    pub average_confidence: f32,
}
