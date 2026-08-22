//! Price calculator module
//!
//! This module handles the core price calculation logic:
//! - Decodes pool account data using program-specific decoders
//! - Calculates token prices from pool reserves (SOL-based pricing only)
//! - Handles price triangulation for indirect pairs
//! - Updates price cache and history
//!
//! Solana-owned: dispatches on the concrete `ProgramKind` and operates on
//! `Pubkey`-keyed account bundles fetched over RPC. The shared, chain-neutral
//! `PoolDescriptor` (`crate::pools::types`) is converted to/from Solana
//! addresses only at this boundary — `crate::pools` itself never sees a
//! `Pubkey`.

use super::decoders;
use super::fetcher::{AccountData, PoolAccountBundle};
use super::reserve_accounts::reserve_pubkeys;
use super::types::ProgramKind;
use crate::pools::cache;
use crate::pools::types::{PoolDescriptor, PriceResult};

use crate::chains::solana::constants::SOL_MINT;
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};
use crate::tokens::database::get_global_database;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::{mpsc, Notify};

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
    /// Metrics
    operations: Arc<std::sync::atomic::AtomicU64>,
    errors: Arc<std::sync::atomic::AtomicU64>,
    prices_calculated: Arc<std::sync::atomic::AtomicU64>,
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
            operations: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            prices_calculated: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get metrics for this calculator instance
    pub fn get_metrics(&self) -> (u64, u64, u64) {
        (
            self.operations.load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
            self.prices_calculated
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Get sender for sending calculation requests
    pub fn get_sender(&self) -> mpsc::UnboundedSender<CalculatorMessage> {
        self.calculator_tx.clone()
    }

    /// Start calculator background task
    pub async fn start_calculator_task(&self, shutdown: Arc<Notify>) {
        logger::info(LogTag::PoolCalculator, "Starting price calculator task");

        let _pool_directory = self.pool_directory.clone();
        let sol_reference_price = self.sol_reference_price.clone();

        // Clone metrics for tracking in background task
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);
        let prices_calculated = Arc::clone(&self.prices_calculated);

        // Take the receiver from the Arc<RwLock>
        let mut calculator_rx = {
            let mut rx_lock = self.calculator_rx.write().unwrap();
            rx_lock.take().expect("Calculator receiver already taken")
        };

        tokio::spawn(async move {
            logger::info(LogTag::PoolCalculator, "Price calculator task started");

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        logger::info(LogTag::PoolCalculator, "Price calculator task shutting down");
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
                                let token_mint = if pool_descriptor.base_mint.address() != SOL_MINT {
                                    pool_descriptor.base_mint.address().to_owned()
                                } else {
                                    pool_descriptor.quote_mint.address().to_owned()
                                };

                                record_safe(Event::info(
                                    EventCategory::Pool,
                                    Some("price_calculation_started".to_owned()),
                                    Some(token_mint.clone()),
                                    Some(pool_id.to_string()),
                                    serde_json::json!({
                                        "pool_id": pool_id.to_string(),
                                        "program_kind": pool_descriptor.program_kind.as_str(),
                                        "base_mint": pool_descriptor.base_mint.address(),
                                        "quote_mint": pool_descriptor.quote_mint.address()
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
                                    // Track metrics
                                    operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    prices_calculated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                    // Update cache with calculated price
                                    cache::update_price(price_result.clone());

                                    if let Some(db) = get_global_database() {
                                        if let Err(e) = db.mark_pool_price_calculated(
                                            &price_result.mint,
                                            &price_result.pool_address,
                                        ) {
                                            logger::warning(
                                                LogTag::PoolCalculator,
                                                &format!(
                                                    "Failed to persist pool price timestamp for mint={} pool={} error={}",
                                                    price_result.mint,
                                                    price_result.pool_address,
                                                    e
                                                ),
                                            );
                                        }
                                    } else {
                                        logger::warning(
                                            LogTag::PoolCalculator,
                                            &format!(
                                                "Token database unavailable; skipping pool price timestamp persist for mint={} pool={}",
                                                price_result.mint,
                                                price_result.pool_address
                                            ),
                                        );
                                    }

                                    record_safe(Event::info(
                                        EventCategory::Pool,
                                        Some("price_calculation_success".to_owned()),
                                        Some(token_mint.clone()),
                                        Some(pool_id.to_string()),
                                        serde_json::json!({
                                            "pool_id": pool_id.to_string(),
                                            "token_mint": token_mint,
                                            "price_sol": price_result.price_sol,
                                            "sol_reserves": price_result.sol_reserves,
                                            "token_reserves": price_result.token_reserves,
                                            "duration_ms": calculation_duration.as_millis(),
                                            "program_kind": pool_descriptor.program_kind.as_str()
                                        })
                                    )).await;

                                    logger::debug(
                                        LogTag::PoolCalculator,
                                        &format!(
                                            "Calculated price for token {} in pool {}: {} SOL",
                                            price_result.mint,
                                            pool_id,
                                            price_result.price_sol
                                        ),
                                    );
                                } else if let Some(error) = result.error {
                                    // Track error
                                    errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                    record_safe(Event::error(
                                        EventCategory::Pool,
                                        Some("price_calculation_failed".to_owned()),
                                        Some(token_mint.clone()),
                                        Some(pool_id.to_string()),
                                        serde_json::json!({
                                            "pool_id": pool_id.to_string(),
                                            "token_mint": token_mint,
                                            "error": error,
                                            "duration_ms": calculation_duration.as_millis(),
                                            "program_kind": pool_descriptor.program_kind.as_str(),
                                            "account_count": account_bundle.accounts.len()
                                        })
                                    )).await;

                                    logger::warning(
                                        LogTag::PoolCalculator,
                                        &format!(
                                            "Failed to calculate price for token {} in pool {}: {}",
                                            token_mint,
                                            pool_id,
                                            error
                                        ),
                                    );

                                    // Blacklist the pool so canonical selection skips it next cycle
                                    // (e.g. migrated PumpFunLegacy bonding curves with zero reserves)
                                    let pool_id_str = pool_id.to_string();
                                    let token_mint_clone = token_mint.clone();
                                    let program_kind_str = pool_descriptor.program_kind.as_str().to_owned();
                                    tokio::spawn(async move {
                                        if let Err(e) = crate::pools::database::add_pool_to_blacklist(
                                            crate::chains::ChainId::Solana,
                                            &pool_id_str,
                                            "decoder_failed",
                                            Some(&token_mint_clone),
                                            Some(&program_kind_str),
                                        ).await {
                                            crate::logger::warning(
                                                crate::logger::LogTag::PoolCalculator,
                                                &format!("Failed to blacklist pool {}: {}", pool_id_str, e),
                                            );
                                        }
                                    });
                                }
                            }

                            Some(CalculatorMessage::Shutdown) => {
                                logger::info(LogTag::PoolCalculator, "Calculator received shutdown signal");
                                break;
                            }

                            None => {
                                logger::info(LogTag::PoolCalculator, "Calculator channel closed");
                                break;
                            }
                        }
                    }
                }
            }

            logger::info(LogTag::PoolCalculator, "Price calculator task completed");
        });
    }

    /// Calculate price for a pool (static version for task)
    async fn calculate_pool_price_static(
        pool_id: Pubkey,
        pool_descriptor: &PoolDescriptor,
        account_bundle: &PoolAccountBundle,
        _sol_reference_price: &Arc<RwLock<f64>>,
    ) -> PoolCalculationResult {
        // Determine which token we're calculating price for (the non-SOL token).
        // Discovery stage already ensures one side is SOL, so this always succeeds
        // and needs nothing but a string compare — no address parsing required.
        let (target_mint, _sol_is_base): (&str, bool) =
            if pool_descriptor.base_mint.address() == SOL_MINT {
                (pool_descriptor.quote_mint.address(), true)
            } else {
                (pool_descriptor.base_mint.address(), false)
            };

        {
            logger::debug(
                LogTag::PoolCalculator,
                &format!(
                    "Calculating price for pool {} ({}) - {}/{} (token: {})",
                    pool_id,
                    pool_descriptor.program_kind.as_str(),
                    pool_descriptor.base_mint.address(),
                    pool_descriptor.quote_mint.address(),
                    target_mint
                ),
            );
        }

        // Reserve accounts are chain-neutral on `PoolDescriptor`; convert to
        // `Pubkey` once per calculation (not per account lookup) to check bundle
        // completeness. This is all-or-error: a malformed reserve address must
        // reject the calculation, never shrink the required set and price a
        // partial pool.
        let reserve_pubkeys: Vec<Pubkey> = match reserve_pubkeys(pool_descriptor) {
            Ok(pubkeys) => pubkeys,
            Err(err) => {
                record_safe(Event::error(
                    EventCategory::Pool,
                    Some("invalid_reserve_accounts".to_owned()),
                    Some(target_mint.to_owned()),
                    Some(pool_id.to_string()),
                    serde_json::json!({
                        "pool_id": pool_id.to_string(),
                        "program_kind": pool_descriptor.program_kind.as_str(),
                        "target_mint": target_mint,
                        "required_accounts": pool_descriptor.reserve_accounts.len(),
                        "error": err.to_string()
                    }),
                ))
                .await;

                return PoolCalculationResult {
                    pool_id,
                    price_result: None,
                    error: Some(format!("Invalid reserve accounts: {err}")),
                };
            }
        };

        // Check if bundle has required accounts
        if !account_bundle.is_complete(&reserve_pubkeys) {
            record_safe(Event::warn(
                EventCategory::Pool,
                Some("incomplete_account_bundle".to_owned()),
                Some(target_mint.to_owned()),
                Some(pool_id.to_string()),
                serde_json::json!({
                    "pool_id": pool_id.to_string(),
                    "program_kind": pool_descriptor.program_kind.as_str(),
                    "target_mint": target_mint,
                    "required_accounts": pool_descriptor.reserve_accounts.len(),
                    "available_accounts": account_bundle.accounts.len(),
                    "error": "Missing required pool accounts"
                }),
            ))
            .await;

            return PoolCalculationResult {
                pool_id,
                price_result: None,
                error: Some("Incomplete account bundle".to_owned()),
            };
        }

        // Convert account bundle to format expected by decoders
        let accounts_map = Self::convert_bundle_to_accounts_map(account_bundle);

        // Use decoder to calculate price. Route on the real Solana ProgramKind,
        // recovered from the descriptor's chain-neutral protocol identity at
        // this boundary.
        let program_kind = ProgramKind::from_protocol_id(&pool_descriptor.program_kind);
        let sol_mint_str = SOL_MINT.to_string();

        let decoded_result =
            decoders::decode_pool(program_kind, &accounts_map, target_mint, &sol_mint_str);

        match decoded_result {
            Some(mut price_result) => {
                // Ensure we're returning price for the target token (not SOL)
                price_result.mint = target_mint.to_owned();
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
                record_safe(Event::error(
                    EventCategory::Pool,
                    Some("decoder_failed".to_owned()),
                    Some(target_mint.to_owned()),
                    Some(pool_id.to_string()),
                    serde_json::json!({
                        "pool_id": pool_id.to_string(),
                        "program_kind": pool_descriptor.program_kind.as_str(),
                        "target_mint": target_mint,
                        "account_count": accounts_map.len(),
                        "required_accounts": pool_descriptor.reserve_accounts.len(),
                        "error": "Decoder failed to parse pool data"
                    }),
                ))
                .await;

                PoolCalculationResult {
                    pool_id,
                    price_result: None,
                    error: Some("Decoder failed to parse pool data".to_owned()),
                }
            }
        }
    }

    /// Convert PoolAccountBundle to the format expected by decoders
    fn convert_bundle_to_accounts_map(bundle: &PoolAccountBundle) -> HashMap<String, AccountData> {
        bundle
            .accounts
            .iter()
            .map(|(pubkey, account_data)| (pubkey.to_string(), account_data.clone()))
            .collect()
    }

    /// Calculate confidence score based on liquidity and data freshness
    fn calculate_confidence(price_result: &PriceResult, bundle: &PoolAccountBundle) -> f32 {
        let mut confidence = 1.0f32;

        // Check for invalid reserves first
        if !price_result.sol_reserves.is_finite() {
            confidence = 0.0;
        }

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
        confidence.clamp(0.0, 1.0)
    }

    /// Public interface: Request calculation for a pool
    pub fn request_calculation(
        &self,
        pool_id: Pubkey,
        pool_descriptor: PoolDescriptor,
        account_bundle: PoolAccountBundle,
    ) -> Result<(), String> {
        let message = CalculatorMessage::CalculatePool {
            pool_id,
            pool_descriptor,
            account_bundle,
        };

        self.calculator_tx
            .send(message)
            .map_err(|e| format!("Failed to send calculation request: {e}"))?;

        Ok(())
    }

    /// Get the canonical pool used for pricing a given token mint (highest-quality pool)
    pub fn get_canonical_pool(&self, mint: &str) -> Option<PoolDescriptor> {
        if mint == SOL_MINT {
            return None;
        }

        let candidates: Vec<PoolDescriptor> = {
            let directory = self.pool_directory.read().ok()?;
            directory
                .values()
                .filter(|pool| {
                    pool.base_mint.address() == mint || pool.quote_mint.address() == mint
                })
                .cloned()
                .collect()
        };

        if candidates.is_empty() {
            return None;
        }

        // Select highest liquidity pool for this mint
        candidates.into_iter().max_by(|a, b| {
            a.liquidity_usd
                .partial_cmp(&b.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{AccountId, AssetId, ChainId, PoolId};
    use crate::pools::types::ProtocolId;

    fn descriptor(base: &str, quote: &str, liquidity_usd: f64) -> PoolDescriptor {
        PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, format!("pool-{base}-{quote}")).unwrap(),
            program_kind: ProtocolId::new("RAYDIUM CPMM"),
            base_mint: AssetId::new(ChainId::Solana, base).unwrap(),
            quote_mint: AssetId::new(ChainId::Solana, quote).unwrap(),
            reserve_accounts: vec![
                AccountId::new(ChainId::Solana, "vault-a").unwrap(),
                AccountId::new(ChainId::Solana, "vault-b").unwrap(),
            ],
            liquidity_usd,
            volume_h24_usd: 0.0,
            last_updated: Instant::now(),
        }
    }

    #[test]
    fn get_canonical_pool_picks_highest_liquidity_by_string_mint() {
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let calculator = PriceCalculator::new(directory.clone());

        let low = descriptor("TokenA", SOL_MINT, 100.0);
        let high = descriptor("TokenA", SOL_MINT, 500.0);
        {
            let mut guard = directory.write().unwrap();
            guard.insert(Pubkey::new_unique(), low);
            guard.insert(Pubkey::new_unique(), high.clone());
        }

        let canonical = calculator
            .get_canonical_pool("TokenA")
            .expect("canonical pool");
        assert_eq!(canonical.pool_id, high.pool_id);
    }

    #[test]
    fn get_canonical_pool_rejects_the_sol_mint_itself() {
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let calculator = PriceCalculator::new(directory);
        assert!(calculator.get_canonical_pool(SOL_MINT).is_none());
    }

    #[tokio::test]
    async fn calculate_pool_price_rejects_malformed_reserve_without_partial_decode() {
        // record_safe() reads config; point at a nonexistent path so it
        // initializes from defaults (ignore Err: another test may have
        // already set the process-wide CONFIG OnceLock).
        let _ = crate::config::load_config_from_path("/nonexistent/screenerbot-test-config.toml");

        let mut descriptor = descriptor("TokenA", SOL_MINT, 100.0);
        descriptor.reserve_accounts =
            vec![AccountId::new(ChainId::Solana, "not-a-valid-pubkey").unwrap()];

        let pool_id = Pubkey::new_unique();
        let bundle = PoolAccountBundle::new(pool_id);
        let sol_reference_price = Arc::new(RwLock::new(100.0));

        let result = PriceCalculator::calculate_pool_price_static(
            pool_id,
            &descriptor,
            &bundle,
            &sol_reference_price,
        )
        .await;

        assert!(
            result.price_result.is_none(),
            "a malformed required reserve must never yield a partial price"
        );
        let error = result
            .error
            .expect("malformed reserve must report an error");
        assert!(!error.is_empty());
    }
}
