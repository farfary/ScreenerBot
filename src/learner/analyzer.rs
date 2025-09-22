//! learner/analyzer.rs
//!
//! Pattern analysis and feature extraction engine.
//!
//! This module handles:
//! * Feature extraction from trade records
//! * Pattern recognition and similarity matching
//! * Historical context analysis
//! * Success probability calculations
//!
//! The analyzer builds rich feature vectors that capture:
//! * Drop patterns and velocity
//! * Market context and liquidity
//! * ATH proximity and risk factors
//! * Temporal patterns and seasonality
//! * Historical token behavior

use crate::learner::types::*;
use crate::learner::database::LearningDatabase;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_learning_enabled;
use chrono::{ DateTime, Utc, Timelike, Datelike };
use std::collections::HashMap;
use std::f64::consts::PI;

/// Pattern analysis engine
pub struct PatternAnalyzer {
    // Cache for token statistics to avoid repeated database queries
    token_stats_cache: tokio::sync::RwLock<HashMap<String, TokenStatistics>>,
    cache_ttl: std::time::Duration,
    last_cache_cleanup: tokio::sync::RwLock<std::time::Instant>,
}

#[derive(Debug, Clone)]
struct TokenStatistics {
    trade_count: usize,
    avg_profit: f64,
    success_rate: f64,
    avg_hold_duration: f64,
    avg_peak_time: f64,
    volatility_score: f64,
    last_updated: std::time::Instant,
}

impl PatternAnalyzer {
    /// Create new pattern analyzer
    pub fn new() -> Self {
        Self {
            token_stats_cache: tokio::sync::RwLock::new(HashMap::new()),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
            last_cache_cleanup: tokio::sync::RwLock::new(std::time::Instant::now()),
        }
    }

    /// Extract feature vector from trade record
    pub async fn extract_features(
        &self,
        trade: &TradeRecord,
        database: &LearningDatabase
    ) -> Result<FeatureVector, String> {
        let mut features = FeatureVector::new(trade.id);

        // Extract drop pattern features
        self.extract_drop_features(trade, &mut features).await?;

        // Extract market context features
        self.extract_market_context_features(trade, &mut features).await?;

        // Extract ATH proximity features
        self.extract_ath_features(trade, &mut features).await?;

        // Extract temporal features
        self.extract_temporal_features(trade, &mut features).await?;

        // Extract historical features
        self.extract_historical_features(trade, database, &mut features).await?;

        // Generate labels for training
        self.generate_labels(trade, &mut features).await?;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Extracted features for trade {}: {} features, success_label: {:?}",
                    trade.id,
                    FeatureVector::FEATURE_COUNT,
                    features.success_label
                )
            );
        }

        Ok(features)
    }

    /// Extract drop pattern features
    async fn extract_drop_features(
        &self,
        trade: &TradeRecord,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Normalize drop percentages by typical volatility for the timeframe
        features.drop_10s_norm = self.normalize_drop(trade.drop_10s_pct, 10.0);
        features.drop_30s_norm = self.normalize_drop(trade.drop_30s_pct, 30.0);
        features.drop_60s_norm = self.normalize_drop(trade.drop_60s_pct, 60.0);
        features.drop_120s_norm = self.normalize_drop(trade.drop_120s_pct, 120.0);
        features.drop_320s_norm = self.normalize_drop(trade.drop_320s_pct, 320.0);

        // Calculate drop velocity (percentage per minute)
        if let Some(drop_30s) = trade.drop_30s_pct {
            features.drop_velocity_30s = (drop_30s / 0.5).abs().min(100.0) / 100.0; // Normalize to 0-1
        }

        // Calculate drop acceleration (change in velocity)
        if let (Some(drop_30s), Some(drop_60s)) = (trade.drop_30s_pct, trade.drop_60s_pct) {
            let vel_early = (drop_30s / 0.5).abs();
            let vel_total = (drop_60s / 1.0).abs();
            features.drop_acceleration = ((vel_early - vel_total) / 50.0).clamp(-1.0, 1.0);
        }

        Ok(())
    }

    /// Extract market context features
    async fn extract_market_context_features(
        &self,
        trade: &TradeRecord,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Liquidity tier (0-1 scale)
        if let Some(liquidity) = trade.liquidity_at_entry {
            features.liquidity_tier = self.liquidity_to_tier(liquidity);
        }

        // Transaction activity score
        let tx_5m = trade.tx_activity_5m.unwrap_or(0) as f64;
        let tx_1h = trade.tx_activity_1h.unwrap_or(0) as f64;
        features.tx_activity_score = self.tx_activity_to_score(tx_5m, tx_1h);

        // Security score normalized
        if let Some(security_score) = trade.security_score {
            features.security_score_norm = (security_score as f64) / 100.0;
        }

        // Holder count (log scale)
        if let Some(holder_count) = trade.holder_count {
            features.holder_count_log = (holder_count as f64).ln().max(0.0) / 20.0; // Normalize to ~0-1
        }

        // Market cap tier (estimated from liquidity and price)
        if
            let (Some(liquidity), Some(sol_reserves)) = (
                trade.liquidity_at_entry,
                trade.sol_reserves_at_entry,
            )
        {
            features.market_cap_tier = self.estimate_market_cap_tier(
                liquidity,
                sol_reserves,
                trade.entry_price
            );
        }

        Ok(())
    }

    /// Extract ATH proximity features
    async fn extract_ath_features(
        &self,
        trade: &TradeRecord,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Convert distance from ATH to proximity (closer = higher value)
        features.ath_prox_15m = trade.ath_dist_15m_pct
            .map(|dist| 1.0 - (dist / 100.0).min(1.0))
            .unwrap_or(0.0);

        features.ath_prox_1h = trade.ath_dist_1h_pct
            .map(|dist| 1.0 - (dist / 100.0).min(1.0))
            .unwrap_or(0.0);

        features.ath_prox_6h = trade.ath_dist_6h_pct
            .map(|dist| 1.0 - (dist / 100.0).min(1.0))
            .unwrap_or(0.0);

        // Combined ATH risk score (higher when very close to any ATH)
        let proximities = [features.ath_prox_15m, features.ath_prox_1h, features.ath_prox_6h];
        let max_proximity = proximities.iter().cloned().fold(0.0f64, f64::max);
        let avg_proximity = proximities.iter().sum::<f64>() / 3.0;

        // Risk increases exponentially as we get very close to ATH
        features.ath_risk_score = (max_proximity * 0.7 + avg_proximity * 0.3).powf(2.0);

        Ok(())
    }

    /// Extract temporal features
    async fn extract_temporal_features(
        &self,
        trade: &TradeRecord,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Encode hour of day as sine/cosine for cyclical nature
        let hour_radians = ((trade.hour_of_day as f64) * 2.0 * PI) / 24.0;
        features.hour_sin = hour_radians.sin();
        features.hour_cos = hour_radians.cos();

        // Encode day of week as sine/cosine
        let day_radians = ((trade.day_of_week as f64) * 2.0 * PI) / 7.0;
        features.day_sin = day_radians.sin();
        features.day_cos = day_radians.cos();

        Ok(())
    }

    /// Extract historical features about the token
    async fn extract_historical_features(
        &self,
        trade: &TradeRecord,
        database: &LearningDatabase,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Check if this is a re-entry
        features.re_entry_flag = if trade.was_re_entry { 1.0 } else { 0.0 };

        // Get token statistics (cached)
        let token_stats = self.get_token_statistics(&trade.mint, database).await?;

        features.token_trade_count = (token_stats.trade_count as f64).ln().max(0.0) / 10.0; // Log scale
        features.avg_hold_duration = (token_stats.avg_hold_duration / 3600.0).min(24.0) / 24.0; // Hours, max 24

        // Count recent exits for this token (last 24 hours)
        let recent_trades = database.get_trades_for_mint(&trade.mint).await?;
        let recent_exits = recent_trades
            .iter()
            .filter(|t| (trade.entry_time - t.exit_time).num_hours() <= 24)
            .count();
        features.recent_exit_count = (recent_exits as f64).min(10.0) / 10.0;

        Ok(())
    }

    /// Generate training labels
    async fn generate_labels(
        &self,
        trade: &TradeRecord,
        features: &mut FeatureVector
    ) -> Result<(), String> {
        // Success label: profitable exit
        features.success_label = Some(if trade.pnl_pct > 0.0 { 1.0 } else { 0.0 });

        // Quick success label: >25% profit in <20 minutes
        let quick_profit = trade.pnl_pct > 25.0 && trade.hold_duration_sec < 1200; // 20 minutes
        features.quick_success_label = Some(if quick_profit { 1.0 } else { 0.0 });

        // Risk label: >18% drawdown in first 8 minutes
        let high_early_risk =
            trade.max_down_pct.abs() > 18.0 &&
            trade.dd_reached_sec.map(|t| t < 480).unwrap_or(false); // 8 minutes
        features.risk_label = Some(if high_early_risk { 1.0 } else { 0.0 });

        // Peak time label: normalized time to reach peak
        if let Some(peak_time) = trade.peak_reached_sec {
            features.peak_time_label = Some(((peak_time as f64) / 3600.0).min(1.0)); // Normalize to hours, max 1
        }

        Ok(())
    }

    /// Get cached token statistics
    async fn get_token_statistics(
        &self,
        mint: &str,
        database: &LearningDatabase
    ) -> Result<TokenStatistics, String> {
        // Cleanup cache if needed
        self.cleanup_cache_if_needed().await;

        // Check cache first
        {
            let cache = self.token_stats_cache.read().await;
            if let Some(stats) = cache.get(mint) {
                if stats.last_updated.elapsed() < self.cache_ttl {
                    return Ok(stats.clone());
                }
            }
        }

        // Load from database
        let trades = database.get_trades_for_mint(mint).await?;
        let stats = self.calculate_token_statistics(&trades);

        // Update cache
        {
            let mut cache = self.token_stats_cache.write().await;
            cache.insert(mint.to_string(), stats.clone());
        }

        Ok(stats)
    }

    /// Calculate token statistics from trades
    fn calculate_token_statistics(&self, trades: &[TradeRecord]) -> TokenStatistics {
        if trades.is_empty() {
            return TokenStatistics {
                trade_count: 0,
                avg_profit: 0.0,
                success_rate: 0.0,
                avg_hold_duration: 0.0,
                avg_peak_time: 0.0,
                volatility_score: 0.0,
                last_updated: std::time::Instant::now(),
            };
        }

        let profitable_trades = trades
            .iter()
            .filter(|t| t.pnl_pct > 0.0)
            .count();
        let success_rate = (profitable_trades as f64) / (trades.len() as f64);

        let avg_profit =
            trades
                .iter()
                .map(|t| t.pnl_pct)
                .sum::<f64>() / (trades.len() as f64);
        let avg_hold_duration =
            trades
                .iter()
                .map(|t| t.hold_duration_sec as f64)
                .sum::<f64>() / (trades.len() as f64);

        let avg_peak_time =
            trades
                .iter()
                .filter_map(|t| t.peak_reached_sec)
                .map(|t| t as f64)
                .sum::<f64>() / (trades.len() as f64);

        // Calculate volatility as average of max swings
        let volatility_score =
            trades
                .iter()
                .map(|t| (t.max_up_pct - t.max_down_pct).abs())
                .sum::<f64>() / (trades.len() as f64);

        TokenStatistics {
            trade_count: trades.len(),
            avg_profit,
            success_rate,
            avg_hold_duration,
            avg_peak_time,
            volatility_score,
            last_updated: std::time::Instant::now(),
        }
    }

    /// Clean up old cache entries
    async fn cleanup_cache_if_needed(&self) {
        let mut last_cleanup = self.last_cache_cleanup.write().await;
        if last_cleanup.elapsed() > std::time::Duration::from_secs(600) {
            // 10 minutes
            let mut cache = self.token_stats_cache.write().await;
            let cutoff = std::time::Instant::now() - self.cache_ttl;
            cache.retain(|_, stats| stats.last_updated > cutoff);
            *last_cleanup = std::time::Instant::now();
        }
    }

    /// Normalize drop percentage by timeframe
    fn normalize_drop(&self, drop_pct: Option<f64>, timeframe_sec: f64) -> f64 {
        if let Some(drop) = drop_pct {
            // Expected volatility increases with sqrt of time
            let expected_volatility = 2.0 * (timeframe_sec / 60.0).sqrt(); // Base 2% per minute
            let normalized = drop.abs() / expected_volatility;
            normalized.min(2.0) / 2.0 // Cap at 2x expected, normalize to 0-1
        } else {
            0.0
        }
    }

    /// Convert liquidity to tier (0-1)
    fn liquidity_to_tier(&self, liquidity: f64) -> f64 {
        // Tiers: <1K=0.0, 1K-10K=0.2, 10K-100K=0.4, 100K-1M=0.6, 1M-10M=0.8, >10M=1.0
        if liquidity < 1000.0 {
            0.0
        } else if liquidity < 10000.0 {
            0.2
        } else if liquidity < 100000.0 {
            0.4
        } else if liquidity < 1000000.0 {
            0.6
        } else if liquidity < 10000000.0 {
            0.8
        } else {
            1.0
        }
    }

    /// Convert transaction activity to score (0-1)
    fn tx_activity_to_score(&self, tx_5m: f64, tx_1h: f64) -> f64 {
        // Combine 5-minute and 1-hour activity with different weights
        let score_5m = (tx_5m / 20.0).min(1.0); // Max score at 20 tx/5min
        let score_1h = (tx_1h / 200.0).min(1.0); // Max score at 200 tx/hour

        // Weight recent activity more heavily
        (score_5m * 0.7 + score_1h * 0.3).min(1.0)
    }

    /// Estimate market cap tier from liquidity and reserves
    fn estimate_market_cap_tier(&self, liquidity: f64, sol_reserves: f64, price: f64) -> f64 {
        // Rough estimate: market_cap ≈ price * total_supply
        // We can estimate total_supply from liquidity structure
        if sol_reserves > 0.0 && price > 0.0 {
            let estimated_token_supply = liquidity / (2.0 * price); // Rough estimate
            let estimated_market_cap = estimated_token_supply * price;

            // Tiers: <100K=0.1, 100K-1M=0.3, 1M-10M=0.5, 10M-100M=0.7, >100M=0.9
            if estimated_market_cap < 100000.0 {
                0.1
            } else if estimated_market_cap < 1000000.0 {
                0.3
            } else if estimated_market_cap < 10000000.0 {
                0.5
            } else if estimated_market_cap < 100000000.0 {
                0.7
            } else {
                0.9
            }
        } else {
            0.0
        }
    }

    /// Calculate price volatility from price series
    fn calculate_price_volatility(&self, prices: &[f64]) -> f64 {
        if prices.len() < 2 {
            return 0.0;
        }

        // Calculate percentage changes
        let mut changes = Vec::new();
        for i in 1..prices.len() {
            if prices[i - 1] > 0.0 {
                let change = ((prices[i] - prices[i - 1]) / prices[i - 1]).abs() * 100.0;
                changes.push(change);
            }
        }

        if changes.is_empty() {
            return 0.0;
        }

        // Return average absolute percentage change as volatility measure
        changes.iter().sum::<f64>() / (changes.len() as f64)
    }

    /// Calculate multi-timeframe drops from pools price history
    /// Returns (drop_10s, drop_30s, drop_60s, drop_120s, drop_320s) in percentages
    fn calculate_multi_timeframe_drops(
        &self,
        price_history: &[crate::pools::PriceResult],
        current_price: f64
    ) -> (f64, f64, f64, f64, f64) {
        use chrono::Utc;

        let now = Utc::now();
        let windows_sec = [10, 30, 60, 120, 320];
        let mut drops = [0.0; 5];

        for (i, &window_sec) in windows_sec.iter().enumerate() {
            // Find highest price in this time window
            let mut window_high = current_price;

            for price_result in price_history.iter().rev() {
                let price_time = price_result.get_utc_timestamp();
                let age_seconds = (now - price_time).num_seconds();

                if age_seconds <= (window_sec as i64) {
                    if price_result.price_sol > window_high {
                        window_high = price_result.price_sol;
                    }
                } else {
                    break; // Prices are ordered, so we can stop here
                }
            }

            // Calculate drop percentage
            if window_high > 0.0 && window_high >= current_price {
                drops[i] = ((window_high - current_price) / window_high) * 100.0;
            }
        }

        (drops[0], drops[1], drops[2], drops[3], drops[4])
    }

    /// Calculate ATH distances from pools price history
    /// Returns (ath_15m, ath_1h, ath_6h) distances as percentages
    fn calculate_ath_distances(
        &self,
        price_history: &[crate::pools::PriceResult],
        current_price: f64
    ) -> (f64, f64, f64) {
        use chrono::Utc;

        let now = Utc::now();
        let windows_sec = [15 * 60, 60 * 60, 6 * 60 * 60]; // 15m, 1h, 6h
        let mut ath_distances = [0.0; 3];

        for (i, &window_sec) in windows_sec.iter().enumerate() {
            let mut window_high = current_price;

            for price_result in price_history.iter().rev() {
                let price_time = price_result.get_utc_timestamp();
                let age_seconds = (now - price_time).num_seconds();

                if age_seconds <= (window_sec as i64) {
                    if price_result.price_sol > window_high {
                        window_high = price_result.price_sol;
                    }
                } else {
                    break;
                }
            }

            // Calculate distance from ATH as percentage
            if window_high > 0.0 && window_high >= current_price {
                ath_distances[i] = ((window_high - current_price) / window_high) * 100.0;
            }
        }

        (ath_distances[0], ath_distances[1], ath_distances[2])
    }
    /// Find similar tokens based on trading patterns
    pub async fn find_similar_tokens(
        &self,
        target_mint: &str,
        database: &LearningDatabase,
        limit: usize
    ) -> Result<Vec<SimilarToken>, String> {
        // Get target token's features
        let target_trades = database.get_trades_for_mint(target_mint).await?;
        if target_trades.is_empty() {
            return Ok(Vec::new());
        }

        let target_stats = self.calculate_token_statistics(&target_trades);

        // Get all unique mints from database for comparison
        let all_mints = database.get_all_unique_mints().await.unwrap_or_default();
        if all_mints.len() < 2 {
            return Ok(Vec::new()); // Need at least 2 tokens for comparison
        }

        let mut similar_tokens = Vec::new();

        // Calculate similarity for each token
        for mint in all_mints {
            if mint == target_mint {
                continue; // Skip self
            }

            let trades = database.get_trades_for_mint(&mint).await.unwrap_or_default();
            if trades.is_empty() {
                continue;
            }

            let stats = self.calculate_token_statistics(&trades);
            let similarity = self.calculate_token_similarity(&target_stats, &stats);

            if similarity > 0.1 {
                // Only include reasonably similar tokens
                similar_tokens.push(SimilarToken {
                    mint: mint.clone(),
                    symbol: "UNKNOWN".to_string(), // Will need to be filled from database
                    similarity_score: similarity,
                    trade_count: trades.len(),
                    avg_profit: stats.avg_profit,
                    success_rate: stats.success_rate,
                    last_trade: chrono::Utc::now(), // Will be set correctly later
                });
            }
        }

        // Sort by similarity score descending
        similar_tokens.sort_by(|a, b|
            b.similarity_score.partial_cmp(&a.similarity_score).unwrap_or(std::cmp::Ordering::Equal)
        );

        // Limit results
        similar_tokens.truncate(limit);

        Ok(similar_tokens)
    }

    /// Calculate similarity between two token statistics
    fn calculate_token_similarity(
        &self,
        stats1: &TokenStatistics,
        stats2: &TokenStatistics
    ) -> f64 {
        let mut similarity_sum = 0.0;
        let mut weight_sum = 0.0;

        // Compare success rates (high weight)
        let success_rate_diff = (stats1.success_rate - stats2.success_rate).abs();
        let success_rate_sim = 1.0 - success_rate_diff.min(1.0);
        similarity_sum += success_rate_sim * 0.3;
        weight_sum += 0.3;

        // Compare average profit (high weight)
        let profit_diff = (stats1.avg_profit - stats2.avg_profit).abs() / 100.0; // Normalize to 0-1 range
        let profit_sim = 1.0 - profit_diff.min(1.0);
        similarity_sum += profit_sim * 0.25;
        weight_sum += 0.25;

        // Compare duration patterns (medium weight)
        let duration_diff = (stats1.avg_hold_duration - stats2.avg_hold_duration).abs() / 3600.0; // Hours
        let duration_sim = 1.0 - (duration_diff / 24.0).min(1.0); // Normalize to 24h max
        similarity_sum += duration_sim * 0.2;
        weight_sum += 0.2;

        // Compare volatility patterns (medium weight)
        let vol_diff = (stats1.volatility_score - stats2.volatility_score).abs() / 100.0;
        let vol_sim = 1.0 - vol_diff.min(1.0);
        similarity_sum += vol_sim * 0.15;
        weight_sum += 0.15;

        // Compare drawdown patterns (low weight)
        // Use volatility score as proxy for drawdown difference
        let dd_diff = (stats1.volatility_score - stats2.volatility_score).abs() / 100.0;
        let dd_sim = 1.0 - dd_diff.min(1.0);
        similarity_sum += dd_sim * 0.1;
        weight_sum += 0.1;

        if weight_sum > 0.0 {
            similarity_sum / weight_sum
        } else {
            0.0
        }
    }

    /// Calculate pattern similarity score between two trades
    pub fn calculate_pattern_similarity(&self, trade1: &TradeRecord, trade2: &TradeRecord) -> f64 {
        let mut similarity = 0.0;
        let mut weight_sum = 0.0;

        // Compare drop patterns
        let drop_patterns1 = [
            trade1.drop_10s_pct.unwrap_or(0.0),
            trade1.drop_30s_pct.unwrap_or(0.0),
            trade1.drop_60s_pct.unwrap_or(0.0),
            trade1.drop_120s_pct.unwrap_or(0.0),
            trade1.drop_320s_pct.unwrap_or(0.0),
        ];

        let drop_patterns2 = [
            trade2.drop_10s_pct.unwrap_or(0.0),
            trade2.drop_30s_pct.unwrap_or(0.0),
            trade2.drop_60s_pct.unwrap_or(0.0),
            trade2.drop_120s_pct.unwrap_or(0.0),
            trade2.drop_320s_pct.unwrap_or(0.0),
        ];

        // Calculate pattern correlation
        for (d1, d2) in drop_patterns1.iter().zip(drop_patterns2.iter()) {
            let diff = (d1 - d2).abs();
            let pattern_sim = 1.0 - (diff / 50.0).min(1.0); // Normalize by max expected difference
            similarity += pattern_sim * 0.4; // 40% weight for drop patterns
            weight_sum += 0.4;
        }

        // Compare hold durations
        let duration_diff = (trade1.hold_duration_sec - trade2.hold_duration_sec).abs() as f64;
        let duration_sim = 1.0 - (duration_diff / 3600.0).min(1.0); // Normalize by 1 hour
        similarity += duration_sim * 0.2; // 20% weight
        weight_sum += 0.2;

        // Compare outcomes
        let pnl_diff = (trade1.pnl_pct - trade2.pnl_pct).abs();
        let pnl_sim = 1.0 - (pnl_diff / 100.0).min(1.0); // Normalize by 100%
        similarity += pnl_sim * 0.2; // 20% weight
        weight_sum += 0.2;

        // Compare market context
        if let (Some(liq1), Some(liq2)) = (trade1.liquidity_at_entry, trade2.liquidity_at_entry) {
            let liq_ratio = liq1.min(liq2) / liq1.max(liq2);
            similarity += liq_ratio * 0.2; // 20% weight
            weight_sum += 0.2;
        }

        if weight_sum > 0.0 {
            similarity / weight_sum
        } else {
            0.0
        }
    }

    /// Extract features for entry decision (used by integration.rs)
    pub async fn extract_features_for_entry(
        &self,
        mint: &str,
        current_price: f64,
        drop_percent: f64,
        ath_proximity: f64
    ) -> Result<FeatureVector, String> {
        use chrono::Timelike;
        use std::f64::consts::PI;
        use crate::pools::{ get_price_history, get_price_history_stats };
        use crate::tokens::security::get_security_analyzer;

        let now = chrono::Utc::now();
        let hour = now.hour() as f64;
        let day = now.weekday().num_days_from_sunday() as f64;

        // Get real price history from pools system
        let price_history = get_price_history(mint);

        // Calculate multi-timeframe drops from real pools data
        let (drop_10s_pct, drop_30s_pct, drop_60s_pct, drop_120s_pct, drop_320s_pct) =
            self.calculate_multi_timeframe_drops(&price_history, current_price);

        // Calculate real ATH distances from pools data
        let (ath_dist_15m_pct, ath_dist_1h_pct, ath_dist_6h_pct) = self.calculate_ath_distances(
            &price_history,
            current_price
        );

        // Calculate actual market context features from real data
        let (liquidity_tier, market_cap_tier, tx_activity_score) = if
            let Some(latest_price) = price_history.last()
        {
            let liquidity = latest_price.sol_reserves;
            let liquidity_tier = self.liquidity_to_tier(liquidity);

            // Estimate market cap tier from reserves and price
            let market_cap_tier = self.estimate_market_cap_tier(
                liquidity,
                latest_price.sol_reserves,
                current_price
            );

            // Simple activity score based on price volatility in last 10 points
            let tx_activity_score = if price_history.len() >= 10 {
                let recent_prices: Vec<f64> = price_history
                    .iter()
                    .rev()
                    .take(10)
                    .map(|p| p.price_sol)
                    .collect();

                let volatility = self.calculate_price_volatility(&recent_prices);
                (volatility / 50.0).min(1.0) // Normalize volatility to 0-1
            } else {
                0.3 // Low activity if insufficient data
            };

            (liquidity_tier, market_cap_tier, tx_activity_score)
        } else {
            // Fallback values if no price history available
            (0.2, 0.2, 0.1)
        };

        // Get real security data if available - simplified for new security system
        let (security_score_norm, holder_count_log) = {
            // TODO: Reimplement with new async security API
            // For now, use neutral values
            (0.5, 0.1)
        };

        // Create feature vector with real market data
        let features = FeatureVector {
            trade_id: 0, // Not associated with a trade yet

            // Real drop pattern features from pools price history
            drop_10s_norm: (drop_10s_pct / 100.0).min(1.0),
            drop_30s_norm: (drop_30s_pct / 100.0).min(1.0),
            drop_60s_norm: (drop_60s_pct / 100.0).min(1.0),
            drop_120s_norm: (drop_120s_pct / 100.0).min(1.0),
            drop_320s_norm: (drop_320s_pct / 100.0).min(1.0),
            drop_velocity_30s: if drop_30s_pct > 0.0 {
                drop_30s_pct / 30.0
            } else {
                0.0
            },
            drop_acceleration: {
                // Calculate acceleration from drop differences
                let vel_30s = drop_30s_pct / 0.5; // 30s velocity per minute
                let vel_60s = drop_60s_pct / 1.0; // 60s velocity per minute
                ((vel_30s - vel_60s) / 50.0).clamp(-1.0, 1.0)
            },

            // Real market context data from pools
            liquidity_tier,
            tx_activity_score,
            security_score_norm,
            holder_count_log,
            market_cap_tier,

            // Real ATH proximity from pools data
            ath_prox_15m: 1.0 - (ath_dist_15m_pct / 100.0).min(1.0),
            ath_prox_1h: 1.0 - (ath_dist_1h_pct / 100.0).min(1.0),
            ath_prox_6h: 1.0 - (ath_dist_6h_pct / 100.0).min(1.0),
            ath_risk_score: {
                let proximities = [
                    1.0 - (ath_dist_15m_pct / 100.0).min(1.0),
                    1.0 - (ath_dist_1h_pct / 100.0).min(1.0),
                    1.0 - (ath_dist_6h_pct / 100.0).min(1.0),
                ];
                let max_proximity = proximities.iter().cloned().fold(0.0f64, f64::max);
                let avg_proximity = proximities.iter().sum::<f64>() / 3.0;
                (max_proximity * 0.7 + avg_proximity * 0.3).powf(2.0)
            },

            // Temporal features
            hour_sin: ((hour * 2.0 * PI) / 24.0).sin(),
            hour_cos: ((hour * 2.0 * PI) / 24.0).cos(),
            day_sin: ((day * 2.0 * PI) / 7.0).sin(),
            day_cos: ((day * 2.0 * PI) / 7.0).cos(),

            // Historical features (defaults for now, can be enhanced later)
            re_entry_flag: 0.0,
            token_trade_count: 0.0,
            recent_exit_count: 0.0,
            avg_hold_duration: 60.0,

            // Labels (not set for prediction)
            success_label: None,
            quick_success_label: None,
            risk_label: None,
            peak_time_label: None,

            created_at: chrono::Utc::now(),
        };

        if crate::global::is_debug_learning_enabled() {
            crate::logger::log(
                crate::logger::LogTag::Learning,
                "DEBUG",
                &format!(
                    "Real features for {}: drops(10s:{:.1}% 30s:{:.1}% 60s:{:.1}% 120s:{:.1}% 320s:{:.1}%) ath_dist(15m:{:.1}% 1h:{:.1}% 6h:{:.1}%) liquidity:{:.3} tx_activity:{:.3}",
                    mint,
                    drop_10s_pct,
                    drop_30s_pct,
                    drop_60s_pct,
                    drop_120s_pct,
                    drop_320s_pct,
                    ath_dist_15m_pct,
                    ath_dist_1h_pct,
                    ath_dist_6h_pct,
                    liquidity_tier,
                    tx_activity_score
                )
            );
        }

        Ok(features)
    }

    /// Extract features for exit decision (used by integration.rs)
    pub async fn extract_features_for_exit(
        &self,
        mint: &str,
        current_price: f64,
        entry_price: f64,
        position_duration_mins: u32
    ) -> Result<FeatureVector, String> {
        use chrono::Timelike;
        use std::f64::consts::PI;

        let now = chrono::Utc::now();
        let hour = now.hour() as f64;
        let day = now.weekday().num_days_from_sunday() as f64;
        let current_profit = ((current_price - entry_price) / entry_price) * 100.0;

        // Create feature vector for exit prediction
        let features = FeatureVector {
            trade_id: 0, // Not associated with a trade yet

            // Drop pattern features (use entry assumptions)
            drop_10s_norm: 0.3,
            drop_30s_norm: 0.3,
            drop_60s_norm: 0.3,
            drop_120s_norm: 0.3,
            drop_320s_norm: 0.3,
            drop_velocity_30s: 1.0,
            drop_acceleration: 0.0,

            // Market context (reasonable defaults)
            liquidity_tier: 0.5,
            tx_activity_score: 0.5,
            security_score_norm: 0.8,
            holder_count_log: 8.0,
            market_cap_tier: 0.5,

            // ATH proximity (assume still good)
            ath_prox_15m: 0.8,
            ath_prox_1h: 0.8,
            ath_prox_6h: 0.8,
            ath_risk_score: 0.2,

            // Temporal features
            hour_sin: ((hour * 2.0 * PI) / 24.0).sin(),
            hour_cos: ((hour * 2.0 * PI) / 24.0).cos(),
            day_sin: ((day * 2.0 * PI) / 7.0).sin(),
            day_cos: ((day * 2.0 * PI) / 7.0).cos(),

            // Historical features
            re_entry_flag: 0.0,
            token_trade_count: 1.0, // At least one trade (current)
            recent_exit_count: 0.0,
            avg_hold_duration: position_duration_mins as f64,

            // Labels (not set for prediction)
            success_label: None,
            quick_success_label: None,
            risk_label: None,
            peak_time_label: None,

            created_at: chrono::Utc::now(),
        };

        Ok(features)
    }

    /// Find matching trading patterns for given drop sequence
    pub async fn find_matching_patterns(
        &self,
        database: &LearningDatabase,
        mint: &str,
        drop_10s: f64,
        drop_30s: f64,
        drop_60s: f64,
        drop_120s: f64,
        drop_320s: f64,
        confidence_threshold: f64
    ) -> Result<Vec<TradingPattern>, String> {
        // Get historical trades with similar drop patterns
        let trades = database.get_trades_for_mint(mint).await?;

        let mut pattern_matches = Vec::new();
        let target_drops = [drop_10s, drop_30s, drop_60s, drop_120s, drop_320s];

        // Analyze each historical trade for pattern similarity
        for trade in trades.iter() {
            // Skip phantom or forced exits
            if trade.phantom_exit || trade.forced_exit {
                continue;
            }

            let trade_drops = [
                trade.drop_10s_pct.unwrap_or(0.0),
                trade.drop_30s_pct.unwrap_or(0.0),
                trade.drop_60s_pct.unwrap_or(0.0),
                trade.drop_120s_pct.unwrap_or(0.0),
                trade.drop_320s_pct.unwrap_or(0.0),
            ];

            // Calculate pattern similarity
            let similarity = self.calculate_drop_pattern_similarity(&target_drops, &trade_drops);

            if similarity >= confidence_threshold {
                // Determine pattern type based on drop characteristics
                let pattern_type = self.classify_drop_pattern(&trade_drops);

                // Create trading pattern from this match
                let pattern = TradingPattern {
                    pattern_id: format!("{}_{}", trade.mint, trade.id),
                    pattern_type,
                    confidence: similarity,
                    drop_sequence: trade_drops.to_vec(),
                    duration_range: (
                        trade.hold_duration_sec - 300, // +/- 5 min
                        trade.hold_duration_sec + 300,
                    ),
                    success_rate: if trade.pnl_pct > 0.0 {
                        1.0
                    } else {
                        0.0
                    },
                    avg_profit: trade.pnl_pct,
                    avg_duration: trade.hold_duration_sec,
                    sample_count: 1,
                    liquidity_min: trade.liquidity_at_entry,
                    tx_activity_min: trade.tx_activity_1h,
                    ath_distance_max: trade.ath_dist_15m_pct.map(|a| a.abs()),
                    last_updated: Utc::now(),
                };

                pattern_matches.push(pattern);
            }
        }

        // Sort by confidence (highest first)
        pattern_matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "pattern_search",
                &format!(
                    "Found {} matching patterns for mint {} with similarity >= {}",
                    pattern_matches.len(),
                    mint,
                    confidence_threshold
                )
            );
        }

        Ok(pattern_matches)
    }

    /// Calculate similarity between two drop pattern sequences
    fn calculate_drop_pattern_similarity(&self, pattern1: &[f64; 5], pattern2: &[f64; 5]) -> f64 {
        let mut similarity_sum = 0.0;
        let weights = [0.15, 0.2, 0.25, 0.25, 0.15]; // Weight middle timeframes more

        for (i, (d1, d2)) in pattern1.iter().zip(pattern2.iter()).enumerate() {
            // Calculate normalized difference (smaller difference = higher similarity)
            let max_drop = d1.abs().max(d2.abs()).max(1.0); // Avoid division by zero
            let diff = (d1 - d2).abs() / max_drop;
            let point_similarity = (1.0 - diff.min(1.0)).max(0.0);

            similarity_sum += point_similarity * weights[i];
        }

        similarity_sum
    }

    /// Classify drop pattern into pattern type
    fn classify_drop_pattern(&self, drops: &[f64; 5]) -> PatternType {
        let [drop_10s, drop_30s, drop_60s, drop_120s, drop_320s] = drops;

        // Sharp drop: Most drop happens in first timeframes
        if drop_10s.abs() > drop_30s.abs() * 0.7 && drop_30s.abs() > drop_60s.abs() * 0.7 {
            return PatternType::SharpDrop;
        }

        // Gradual drop: Drop accelerates over time
        if drop_320s.abs() > drop_120s.abs() && drop_120s.abs() > drop_60s.abs() {
            return PatternType::GradualDrop;
        }

        // Double bottom: Drop, recovery, then another drop
        if
            drop_60s.abs() > drop_30s.abs() &&
            drop_120s.abs() > drop_60s.abs() &&
            drop_320s.abs() < drop_120s.abs()
        {
            return PatternType::DoubleBottom;
        }

        // Breakout: Large drop then recovery
        if drop_120s.abs() > 15.0 && drop_320s.abs() < drop_120s.abs() * 0.5 {
            return PatternType::Breakout;
        }

        // Momentum: Consistent small drops
        if drops.iter().all(|&d| d.abs() < 10.0 && d < 0.0) {
            return PatternType::Momentum;
        }

        // Default to reversal for large drops
        if drop_320s.abs() > 20.0 {
            PatternType::Reversal
        } else {
            PatternType::SharpDrop
        }
    }
}
