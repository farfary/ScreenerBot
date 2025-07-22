use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::positions::Position;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Advanced profit calculation system with auto-learning and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitCalculationConfig {
    /// Dynamic stop loss percentage (learned from historical performance)
    pub stop_loss_percent: f64,
    /// Dynamic profit target percentage
    pub profit_target_percent: f64,
    /// Time decay parameters
    pub time_decay_start_secs: f64,
    pub max_hold_time_secs: f64,
    /// Performance metrics for optimization
    pub win_rate: f64,
    pub avg_profit_percent: f64,
    pub avg_loss_percent: f64,
    /// Number of trades analyzed for these settings
    pub trades_analyzed: usize,
    /// Last optimization timestamp
    pub last_optimization: DateTime<Utc>,
}

impl Default for ProfitCalculationConfig {
    fn default() -> Self {
        Self {
            stop_loss_percent: -30.0, // More conservative initial stop loss
            profit_target_percent: 25.0, // More realistic profit target
            time_decay_start_secs: 180.0, // Start decay after 3 minutes
            max_hold_time_secs: 3600.0, // Max 1 hour hold
            win_rate: 0.0,
            avg_profit_percent: 0.0,
            avg_loss_percent: 0.0,
            trades_analyzed: 0,
            last_optimization: Utc::now(),
        }
    }
}

/// Accurate P&L calculation with decimal handling
#[derive(Debug, Clone)]
pub struct AccuratePnL {
    pub pnl_sol: f64,
    pub pnl_percent: f64,
    pub effective_entry_price: f64,
    pub effective_current_price: f64,
    pub total_fees_paid: f64,
    pub calculation_method: String,
}

/// Trade performance data for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradePerformance {
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub hold_duration_secs: f64,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub max_profit_percent: f64,
    pub max_loss_percent: f64,
    pub final_pnl_percent: Option<f64>,
    pub was_profitable: Option<bool>,
    pub exit_reason: String,
}

/// Auto-learning profit calculation system
pub struct ProfitCalculationSystem {
    config: ProfitCalculationConfig,
    performance_history: Vec<TradePerformance>,
    cache_file: String,
}

impl ProfitCalculationSystem {
    pub fn new() -> Self {
        let cache_file = "profit_calculation_cache.json".to_string();
        let (config, performance_history) = Self::load_cache(&cache_file);

        Self {
            config,
            performance_history,
            cache_file,
        }
    }

    /// Load configuration and performance history from cache
    fn load_cache(cache_file: &str) -> (ProfitCalculationConfig, Vec<TradePerformance>) {
        if Path::new(cache_file).exists() {
            match fs::read_to_string(cache_file) {
                Ok(content) => {
                    match
                        serde_json::from_str::<(ProfitCalculationConfig, Vec<TradePerformance>)>(
                            &content
                        )
                    {
                        Ok((config, history)) => {
                            log(
                                LogTag::System,
                                "CONFIG",
                                &format!(
                                    "Loaded profit calculation cache with {} trades",
                                    history.len()
                                )
                            );
                            return (config, history);
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "WARN",
                                &format!("Failed to parse profit calculation cache: {}", e)
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Failed to read profit calculation cache: {}", e)
                    );
                }
            }
        }

        log(LogTag::System, "CONFIG", "Using default profit calculation settings");
        (ProfitCalculationConfig::default(), Vec::new())
    }

    /// Save configuration and performance history to cache
    pub fn save_cache(&self) {
        let data = (&self.config, &self.performance_history);
        match serde_json::to_string_pretty(&data) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.cache_file, content) {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to save profit calculation cache: {}", e)
                    );
                } else {
                    log(LogTag::System, "CACHE", "Saved profit calculation cache");
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to serialize profit calculation cache: {}", e)
                );
            }
        }
    }

    /// Calculate accurate P&L using all available data sources
    pub fn calculate_accurate_pnl(
        &self,
        position: &Position,
        current_price: f64,
        token_decimals: Option<u8>
    ) -> AccuratePnL {
        // Method 1: Use actual transaction data when available (most accurate)
        if
            let (Some(token_amount), Some(effective_entry_price)) = (
                position.token_amount,
                position.effective_entry_price,
            )
        {
            if let Some(decimals) = token_decimals {
                return self.calculate_from_transaction_data(
                    position,
                    current_price,
                    token_amount,
                    effective_entry_price,
                    decimals
                );
            }
        }

        // Method 2: Use effective entry price if available
        if let Some(effective_entry_price) = position.effective_entry_price {
            return self.calculate_from_effective_price(
                position,
                current_price,
                effective_entry_price
            );
        }

        // Method 3: Fallback to original entry price
        self.calculate_from_original_price(position, current_price)
    }

    /// Most accurate calculation using actual transaction data
    fn calculate_from_transaction_data(
        &self,
        position: &Position,
        current_price: f64,
        token_amount: u64,
        effective_entry_price: f64,
        decimals: u8
    ) -> AccuratePnL {
        // Convert raw token amount to UI amount
        let ui_token_amount = (token_amount as f64) / (10_f64).powi(decimals as i32);

        // Calculate actual entry cost (what we paid in SOL)
        let actual_entry_cost = position.entry_size_sol;

        // Calculate current value of tokens
        let current_token_value = ui_token_amount * current_price;

        // Estimate total fees (buy fee already paid, sell fee estimated)
        let estimated_total_fees = self.estimate_total_fees(actual_entry_cost);

        // Net P&L calculation
        let pnl_sol = current_token_value - actual_entry_cost - estimated_total_fees;
        let pnl_percent = (pnl_sol / actual_entry_cost) * 100.0;

        AccuratePnL {
            pnl_sol,
            pnl_percent,
            effective_entry_price,
            effective_current_price: current_price,
            total_fees_paid: estimated_total_fees,
            calculation_method: "transaction_data".to_string(),
        }
    }

    /// Calculate using effective entry price
    fn calculate_from_effective_price(
        &self,
        position: &Position,
        current_price: f64,
        effective_entry_price: f64
    ) -> AccuratePnL {
        let price_change_percent =
            ((current_price - effective_entry_price) / effective_entry_price) * 100.0;
        let estimated_fees = self.estimate_total_fees(position.entry_size_sol);
        let fee_percent = (estimated_fees / position.entry_size_sol) * 100.0;

        let pnl_percent = price_change_percent - fee_percent;
        let pnl_sol = (pnl_percent / 100.0) * position.entry_size_sol;

        AccuratePnL {
            pnl_sol,
            pnl_percent,
            effective_entry_price,
            effective_current_price: current_price,
            total_fees_paid: estimated_fees,
            calculation_method: "effective_price".to_string(),
        }
    }

    /// Fallback calculation using original entry price
    fn calculate_from_original_price(
        &self,
        position: &Position,
        current_price: f64
    ) -> AccuratePnL {
        let price_change_percent =
            ((current_price - position.entry_price) / position.entry_price) * 100.0;
        let estimated_fees = self.estimate_total_fees(position.entry_size_sol);
        let fee_percent = (estimated_fees / position.entry_size_sol) * 100.0;

        let pnl_percent = price_change_percent - fee_percent;
        let pnl_sol = (pnl_percent / 100.0) * position.entry_size_sol;

        AccuratePnL {
            pnl_sol,
            pnl_percent,
            effective_entry_price: position.entry_price,
            effective_current_price: current_price,
            total_fees_paid: estimated_fees,
            calculation_method: "original_price".to_string(),
        }
    }

    /// Estimate total fees based on trade size (more accurate than fixed fee)
    fn estimate_total_fees(&self, trade_size_sol: f64) -> f64 {
        // Base fees: network fees + DEX fees + slippage
        let base_fee = 0.000005; // Network fee (5k lamports)
        let dex_fee_percent = 0.0025; // 0.25% DEX fee
        let slippage_estimate = 0.005; // 0.5% estimated slippage

        // Calculate percentage-based fees
        let percentage_fees = trade_size_sol * (dex_fee_percent + slippage_estimate);

        // Total fees for buy + sell
        (base_fee + percentage_fees) * 2.0
    }

    /// Smart sell decision with auto-learning
    pub fn should_sell_smart(
        &mut self,
        position: &Position,
        current_price: f64,
        now: DateTime<Utc>,
        token_decimals: Option<u8>
    ) -> (f64, String) {
        let time_held_secs = (now - position.entry_time).num_seconds() as f64;

        // Calculate accurate P&L
        let accurate_pnl = self.calculate_accurate_pnl(position, current_price, token_decimals);

        // Get dynamic configuration
        let config = &self.config;

        // Decision factors with explanations
        let mut urgency = 0.0;
        let mut reasons = Vec::new();

        // 1. Stop Loss Protection (adaptive)
        if accurate_pnl.pnl_percent <= config.stop_loss_percent {
            urgency = 1.0;
            reasons.push(
                format!(
                    "Stop loss triggered: {:.2}% <= {:.2}%",
                    accurate_pnl.pnl_percent,
                    config.stop_loss_percent
                )
            );
        } else if
            // 2. Profit Target Achievement
            accurate_pnl.pnl_percent >= config.profit_target_percent
        {
            urgency = 0.85;
            reasons.push(
                format!(
                    "Profit target reached: {:.2}% >= {:.2}%",
                    accurate_pnl.pnl_percent,
                    config.profit_target_percent
                )
            );
        }

        // 4. Time-based urgency (adaptive to market conditions)
        if time_held_secs > config.time_decay_start_secs {
            let time_factor = self.calculate_time_urgency(time_held_secs, accurate_pnl.pnl_percent);
            if time_factor > urgency {
                urgency = time_factor;
                reasons.push(
                    format!("Time decay factor: {:.2} after {:.0}s", time_factor, time_held_secs)
                );
            }
        }

        // 5. Market recovery potential (learned behavior)
        if accurate_pnl.pnl_percent < 0.0 && accurate_pnl.pnl_percent > -15.0 {
            let recovery_factor = self.estimate_recovery_probability(
                position,
                accurate_pnl.pnl_percent,
                time_held_secs
            );
            if recovery_factor > 0.5 {
                urgency *= 0.7; // Reduce urgency if recovery is likely
                reasons.push(format!("Recovery potential detected: {:.2}", recovery_factor));
            }
        }

        // 6. Prevent early exits (minimum hold time with exceptions)
        if time_held_secs < 180.0 && accurate_pnl.pnl_percent > -20.0 {
            urgency *= 0.3; // Significantly reduce urgency for early exits
            reasons.push("Early exit protection active".to_string());
        }

        urgency = urgency.max(0.0).min(1.0);
        let reason_str = if reasons.is_empty() {
            "No sell signals".to_string()
        } else {
            reasons.join("; ")
        };

        (urgency, reason_str)
    }

    /// Calculate time-based urgency with market awareness
    fn calculate_time_urgency(&self, time_held_secs: f64, pnl_percent: f64) -> f64 {
        let config = &self.config;

        if time_held_secs < config.time_decay_start_secs {
            return 0.0;
        }

        let excess_time = time_held_secs - config.time_decay_start_secs;
        let max_excess = config.max_hold_time_secs - config.time_decay_start_secs;
        let time_ratio = (excess_time / max_excess).min(1.0);

        // Adjust urgency based on P&L
        let pnl_adjustment = if pnl_percent > 0.0 {
            1.0 // Normal urgency for profitable positions
        } else if pnl_percent > -10.0 {
            0.7 // Reduced urgency for small losses
        } else if pnl_percent > -25.0 {
            1.2 // Increased urgency for medium losses
        } else {
            1.5 // High urgency for large losses
        };

        (time_ratio * 0.6 * pnl_adjustment).min(1.0)
    }

    /// Estimate probability of price recovery based on historical data
    fn estimate_recovery_probability(
        &self,
        position: &Position,
        current_pnl_percent: f64,
        time_held_secs: f64
    ) -> f64 {
        if self.performance_history.is_empty() {
            return 0.3; // Default moderate recovery chance
        }

        // Find similar historical scenarios
        let similar_trades: Vec<&TradePerformance> = self.performance_history
            .iter()
            .filter(|trade| {
                // Similar loss range
                let loss_similar =
                    trade.max_loss_percent >= current_pnl_percent - 5.0 &&
                    trade.max_loss_percent <= current_pnl_percent + 5.0;

                // Similar time range
                let time_similar =
                    trade.hold_duration_secs >= time_held_secs - 300.0 &&
                    trade.hold_duration_secs <= time_held_secs + 300.0;

                loss_similar && time_similar
            })
            .collect();

        if similar_trades.is_empty() {
            return 0.3;
        }

        // Calculate recovery rate
        let recovery_count = similar_trades
            .iter()
            .filter(|trade| {
                if let Some(final_pnl) = trade.final_pnl_percent {
                    final_pnl > current_pnl_percent + 5.0 // Recovered by at least 5%
                } else {
                    false
                }
            })
            .count();

        (recovery_count as f64) / (similar_trades.len() as f64)
    }

    /// Record trade performance for learning
    pub fn record_trade_performance(&mut self, position: &Position, exit_reason: String) {
        let performance = TradePerformance {
            entry_time: position.entry_time,
            exit_time: position.exit_time,
            hold_duration_secs: if let Some(exit_time) = position.exit_time {
                (exit_time - position.entry_time).num_seconds() as f64
            } else {
                (Utc::now() - position.entry_time).num_seconds() as f64
            },
            entry_price: position.effective_entry_price.unwrap_or(position.entry_price),
            exit_price: position.exit_price,
            max_profit_percent: ((position.price_highest - position.entry_price) /
                position.entry_price) *
            100.0,
            max_loss_percent: ((position.price_lowest - position.entry_price) /
                position.entry_price) *
            100.0,
            final_pnl_percent: position.pnl_percent,
            was_profitable: position.pnl_sol.map(|pnl| pnl > 0.0),
            exit_reason,
        };

        self.performance_history.push(performance);

        // Optimize settings every 10 trades
        if self.performance_history.len() % 10 == 0 {
            self.optimize_settings();
        }

        self.save_cache();
    }

    /// Auto-optimize settings based on performance history
    fn optimize_settings(&mut self) {
        if self.performance_history.len() < 10 {
            return;
        }

        let recent_trades: Vec<&TradePerformance> = self.performance_history
            .iter()
            .rev()
            .take(50) // Use last 50 trades for optimization
            .collect();

        // Calculate performance metrics
        let profitable_trades = recent_trades
            .iter()
            .filter(|t| t.was_profitable.unwrap_or(false))
            .count();

        let win_rate = (profitable_trades as f64) / (recent_trades.len() as f64);

        let avg_profit: f64 =
            recent_trades
                .iter()
                .filter_map(|t| t.final_pnl_percent)
                .filter(|&pnl| pnl > 0.0)
                .sum::<f64>() / (profitable_trades.max(1) as f64);

        let avg_loss: f64 =
            recent_trades
                .iter()
                .filter_map(|t| t.final_pnl_percent)
                .filter(|&pnl| pnl < 0.0)
                .sum::<f64>() / ((recent_trades.len() - profitable_trades).max(1) as f64);

        // Optimize stop loss
        if win_rate < 0.4 && avg_loss < -20.0 {
            // Tighten stop loss if losing too much
            self.config.stop_loss_percent = (self.config.stop_loss_percent * 0.8).max(-50.0);
            log(
                LogTag::System,
                "OPTIMIZE",
                &format!("Tightened stop loss to {:.1}%", self.config.stop_loss_percent)
            );
        } else if win_rate > 0.6 && avg_loss > -15.0 {
            // Loosen stop loss if performance is good
            self.config.stop_loss_percent = (self.config.stop_loss_percent * 1.1).min(-15.0);
            log(
                LogTag::System,
                "OPTIMIZE",
                &format!("Loosened stop loss to {:.1}%", self.config.stop_loss_percent)
            );
        }

        // Optimize profit target
        if avg_profit > 30.0 && win_rate > 0.5 {
            // Increase profit target if achieving good profits
            self.config.profit_target_percent = (self.config.profit_target_percent * 1.1).min(50.0);
            log(
                LogTag::System,
                "OPTIMIZE",
                &format!("Increased profit target to {:.1}%", self.config.profit_target_percent)
            );
        } else if avg_profit < 15.0 {
            // Lower profit target if struggling to achieve profits
            self.config.profit_target_percent = (self.config.profit_target_percent * 0.9).max(15.0);
            log(
                LogTag::System,
                "OPTIMIZE",
                &format!("Reduced profit target to {:.1}%", self.config.profit_target_percent)
            );
        }

        // Update metrics
        self.config.win_rate = win_rate;
        self.config.avg_profit_percent = avg_profit;
        self.config.avg_loss_percent = avg_loss;
        self.config.trades_analyzed = recent_trades.len();
        self.config.last_optimization = Utc::now();

        log(
            LogTag::System,
            "OPTIMIZE",
            &format!(
                "Settings optimized - Win Rate: {:.1}%, Avg Profit: {:.1}%, Avg Loss: {:.1}%",
                win_rate * 100.0,
                avg_profit,
                avg_loss
            )
        );
    }

    /// Get current configuration
    pub fn get_config(&self) -> &ProfitCalculationConfig {
        &self.config
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (f64, f64, f64, usize) {
        (
            self.config.win_rate,
            self.config.avg_profit_percent,
            self.config.avg_loss_percent,
            self.config.trades_analyzed,
        )
    }
}

/// Global instance of the profit calculation system
use once_cell::sync::Lazy;
use std::sync::{ Arc, Mutex };

pub static PROFIT_SYSTEM: Lazy<Arc<Mutex<ProfitCalculationSystem>>> = Lazy::new(|| {
    Arc::new(Mutex::new(ProfitCalculationSystem::new()))
});
