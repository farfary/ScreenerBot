/// Pool price calculator task
/// Runs as background task monitoring pool data and automatically calculating prices

use crate::pools::types::{ PriceResult, PoolInfo };
use crate::pools::decoders::{ DecoderFactory, PoolDecodedResult };
use crate::pools::cache::{ PoolCache, AccountData };
use crate::pools::tokens::PoolToken;
use crate::pools::analyzer::TokenAvailability;
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };

/// Pool price calculator task service
pub struct PoolCalculatorTask {
    decoder_factory: DecoderFactory,
    cache: Arc<PoolCache>,
    /// Task running status
    is_running: Arc<RwLock<bool>>,
    /// Last calculation times per token
    last_calculated: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl PoolCalculatorTask {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            decoder_factory: DecoderFactory::new(),
            cache,
            is_running: Arc::new(RwLock::new(false)),
            last_calculated: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the calculator background task
    pub async fn start_task(&self, cache: Arc<PoolCache>) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            log(LogTag::Pool, "CALCULATOR_TASK_RUNNING", "Calculator task already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        log(LogTag::Pool, "CALCULATOR_TASK_START", "🧮 Starting price calculator task");

        // Clone necessary data for the background task
        let decoder_factory = self.decoder_factory.clone();
        let is_running = self.is_running.clone();
        let last_calculated = self.last_calculated.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                match
                    Self::calculate_prices_for_available_tokens(
                        &decoder_factory,
                        &cache,
                        &last_calculated
                    ).await
                {
                    Ok(calculated_count) => {
                        if calculated_count > 0 {
                            log(
                                LogTag::Pool,
                                "CALCULATOR_TASK_CYCLE",
                                &format!("✅ Calculated {} prices", calculated_count)
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "CALCULATOR_TASK_ERROR",
                            &format!("❌ Calculator task error: {}", e)
                        );
                    }
                }

                // Sleep between calculation cycles
                sleep(Duration::from_secs(5)).await;
            }

            log(LogTag::Pool, "CALCULATOR_TASK_STOP", "🛑 Calculator task stopped");
        });
    }

    /// Stop the calculator task
    pub async fn stop_task(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }

    /// Calculate prices for all available tokens with ready pool data
    async fn calculate_prices_for_available_tokens(
        decoder_factory: &DecoderFactory,
        cache: &Arc<PoolCache>,
        last_calculated: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>
    ) -> Result<usize, String> {
        let calculable_tokens = cache.get_all_token_availability().await;
        let mut calculated_count = 0;

        for (token_mint, availability) in calculable_tokens {
            if !availability.calculable || availability.best_pool.is_none() {
                continue;
            }

            // Check if we need to calculate (every 30 seconds)
            if !Self::should_calculate_price(&token_mint, last_calculated).await {
                continue;
            }

            let best_pool = availability.best_pool.as_ref().unwrap();

            // Prepare pool data from cache
            let prepared_data = match
                Self::prepare_pool_data_from_cache(
                    &best_pool.pool_address,
                    &best_pool.pool_program_id,
                    &availability.reserve_accounts,
                    cache
                ).await
            {
                Ok(data) => data,
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "CALCULATOR_PREPARE_ERROR",
                        &format!("Failed to prepare data for {}: {}", &token_mint[..8], e)
                    );
                    continue;
                }
            };

            // Get decoder and calculate price
            if let Some(decoder) = decoder_factory.get_decoder(&best_pool.pool_program_id) {
                match decoder.decode_pool_data(&prepared_data) {
                    Ok(decoded_result) => {
                        match
                            Self::calculate_price_from_decoded_result(&decoded_result, &token_mint)
                        {
                            Ok(Some(price_result)) => {
                                // Store price in cache
                                cache.store_price(&token_mint, price_result).await;
                                calculated_count += 1;

                                // Update last calculated time
                                {
                                    let mut last_calc = last_calculated.write().await;
                                    last_calc.insert(token_mint.clone(), Utc::now());
                                }

                                log(
                                    LogTag::Pool,
                                    "CALCULATOR_PRICE_SUCCESS",
                                    &format!("💰 Calculated price for token {}", &token_mint[..8])
                                );
                            }
                            Ok(None) => {
                                log(
                                    LogTag::Pool,
                                    "CALCULATOR_NO_PRICE",
                                    &format!("No valid price for token {}", &token_mint[..8])
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Pool,
                                    "CALCULATOR_CALC_ERROR",
                                    &format!(
                                        "Price calculation error for {}: {}",
                                        &token_mint[..8],
                                        e
                                    )
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "CALCULATOR_DECODE_ERROR",
                            &format!("Pool decode error for {}: {}", &token_mint[..8], e)
                        );
                    }
                }
            }
        }

        Ok(calculated_count)
    }

    /// Check if we should calculate price for a token (time-based)
    async fn should_calculate_price(
        token_mint: &str,
        last_calculated: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>
    ) -> bool {
        let last_calc_map = last_calculated.read().await;
        match last_calc_map.get(token_mint) {
            Some(last_time) => {
                let now = Utc::now();
                let duration = now.signed_duration_since(*last_time);
                duration.num_seconds() > 30 // Calculate every 30 seconds
            }
            None => true, // Never calculated
        }
    }

    /// Prepare pool data from cache
    async fn prepare_pool_data_from_cache(
        pool_address: &str,
        program_id: &str,
        reserve_addresses: &[String],
        cache: &Arc<PoolCache>
    ) -> Result<PreparedPoolData, String> {
        // Get pool account data
        let pool_account = cache
            .get_account(pool_address).await
            .ok_or_else(|| format!("Pool account data not available: {}", pool_address))?;

        if !pool_account.exists {
            return Err(format!("Pool account does not exist: {}", pool_address));
        }

        if pool_account.is_expired() {
            return Err(format!("Pool account data expired: {}", pool_address));
        }

        let mut prepared_data = PreparedPoolData::new(
            pool_address.to_string(),
            program_id.to_string(),
            pool_account.data.clone()
        );

        // Add reserve account data
        for reserve_address in reserve_addresses {
            if let Some(reserve_account) = cache.get_account(reserve_address).await {
                if reserve_account.exists && !reserve_account.is_expired() {
                    prepared_data.add_reserve_account(
                        reserve_address.clone(),
                        reserve_account.data.clone()
                    );
                }
            }
        }

        Ok(prepared_data)
    }

    /// Calculate price from decoded pool result
    fn calculate_price_from_decoded_result(
        decoded: &PoolDecodedResult,
        token_mint: &str
    ) -> Result<Option<PriceResult>, String> {
        // Find which token in the pair matches the requested token
        let (token_reserves, sol_reserves, token_decimals, sol_decimals) = if
            decoded.token_a_mint == token_mint
        {
            // Token A is the target token, Token B should be SOL
            if decoded.token_b_mint == crate::pools::constants::SOL_MINT {
                (
                    decoded.token_a_reserve,
                    decoded.token_b_reserve,
                    decoded.token_a_decimals,
                    decoded.token_b_decimals,
                )
            } else {
                return Err(format!("Pool does not contain SOL pair for token {}", token_mint));
            }
        } else if decoded.token_b_mint == token_mint {
            // Token B is the target token, Token A should be SOL
            if decoded.token_a_mint == crate::pools::constants::SOL_MINT {
                (
                    decoded.token_b_reserve,
                    decoded.token_a_reserve,
                    decoded.token_b_decimals,
                    decoded.token_a_decimals,
                )
            } else {
                return Err(format!("Pool does not contain SOL pair for token {}", token_mint));
            }
        } else {
            return Err(format!("Token {} not found in pool", token_mint));
        };

        if token_reserves == 0 || sol_reserves == 0 {
            return Ok(None);
        }

        // Convert raw reserves to decimal values
        let token_reserve_decimal = (token_reserves as f64) / (10_f64).powi(token_decimals as i32);
        let sol_reserve_decimal = (sol_reserves as f64) / (10_f64).powi(sol_decimals as i32);

        if token_reserve_decimal > 0.0 && sol_reserve_decimal > 0.0 {
            let price_sol = sol_reserve_decimal / token_reserve_decimal;

            let result = PriceResult::new(
                token_mint.to_string(),
                price_sol,
                sol_reserve_decimal,
                token_reserve_decimal,
                decoded.pool_address.clone(),
                decoded.program_id.clone()
            );

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Check if calculator task is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get calculator statistics
    pub async fn get_calculator_stats(&self) -> CalculatorStats {
        let last_calc_map = self.last_calculated.read().await;
        let total_calculated = last_calc_map.len();
        let is_running = *self.is_running.read().await;

        CalculatorStats {
            total_calculated,
            is_running,
            updated_at: Utc::now(),
        }
    }
}

/// Prepared pool data for decoders
/// Contains all necessary account data for pool decoding
#[derive(Debug, Clone)]
pub struct PreparedPoolData {
    /// Pool address
    pub pool_address: String,
    /// Pool program ID
    pub program_id: String,
    /// Pool account data
    pub pool_account_data: Vec<u8>,
    /// Reserve account data (vault accounts)
    pub reserve_accounts_data: HashMap<String, Vec<u8>>,
}

impl PreparedPoolData {
    pub fn new(pool_address: String, program_id: String, pool_account_data: Vec<u8>) -> Self {
        Self {
            pool_address,
            program_id,
            pool_account_data,
            reserve_accounts_data: HashMap::new(),
        }
    }

    /// Add reserve account data
    pub fn add_reserve_account(&mut self, address: String, data: Vec<u8>) {
        self.reserve_accounts_data.insert(address, data);
    }
}

/// Calculator statistics
#[derive(Debug, Clone)]
pub struct CalculatorStats {
    pub total_calculated: usize,
    pub is_running: bool,
    pub updated_at: DateTime<Utc>,
}
