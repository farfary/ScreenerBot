use crate::prelude::*;
use crate::price_validation::{
    get_trading_price,
    get_realtime_price_change,
    has_sufficient_price_history,
};

/// Dynamic drop detection with real-time pool price integration
pub struct DropDetector {
    pub detection_threshold: f64,
    pub confirmation_window_seconds: u64,
    pub min_volume_for_detection: f64,
}

impl Default for DropDetector {
    fn default() -> Self {
        Self {
            detection_threshold: -3.0, // Start detecting at -3% drops
            confirmation_window_seconds: 120, // 2 minutes confirmation
            min_volume_for_detection: 1000.0, // Minimum $1000 volume
        }
    }
}

impl DropDetector {
    /// Detect fast drops using real-time pool prices (seconds response)
    pub fn detect_fast_drop(&self, token: &Token) -> Option<DropSignal> {
        // Use real-time pool price for immediate detection
        if let Some(current_price) = get_trading_price(&token.mint) {
            // Check for immediate price changes using our price history
            if has_sufficient_price_history(&token.mint, 2) {
                // 2 minutes of data
                if let Some(change_2m) = get_realtime_price_change(&token.mint, 2) {
                    if change_2m <= self.detection_threshold {
                        return Some(DropSignal {
                            token_mint: token.mint.clone(),
                            drop_percentage: change_2m,
                            detection_source: DropSource::RealTimePool,
                            confidence: self.calculate_fast_drop_confidence(token, change_2m),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Analyze historical drops using dataframe/trades data (2+ minutes old data)
    pub fn analyze_historical_drop(
        &self,
        token: &Token,
        dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
    ) -> Option<DropSignal> {
        // Use dataframe for deeper analysis of confirmed drops
        if let Some(df) = dataframe {
            let change_5m = token.price_change.m5;
            if change_5m <= self.detection_threshold {
                return Some(DropSignal {
                    token_mint: token.mint.clone(),
                    drop_percentage: change_5m,
                    detection_source: DropSource::Historical,
                    confidence: self.calculate_historical_drop_confidence(token, change_5m, df),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
        None
    }

    /// Calculate drop opportunity score based on multiple factors
    pub fn calculate_drop_opportunity_score(
        &self,
        signal: &DropSignal,
        token: &Token,
        liquidity_sol: f64
    ) -> f64 {
        let mut score = 0.0;

        // Base score from drop percentage (deeper drops = higher opportunity)
        let drop_magnitude = signal.drop_percentage.abs();
        score += (drop_magnitude / 10.0).min(1.0) * 0.3; // Max 0.3 from drop size

        // Liquidity factor (higher liquidity = safer)
        let liquidity_factor = (liquidity_sol / 1000.0).min(1.0);
        score += liquidity_factor * 0.2; // Max 0.2 from liquidity

        // Volume factor (ensure actual trading activity)
        let volume_factor = (token.volume.h24 / 10000.0).min(1.0);
        score += volume_factor * 0.2; // Max 0.2 from volume

        // Detection speed bonus (faster detection = better entry)
        let speed_bonus = match signal.detection_source {
            DropSource::RealTimePool => 0.2, // Fastest detection
            DropSource::Historical => 0.1, // Slower but more confirmed
        };
        score += speed_bonus;

        // Confidence factor
        score += signal.confidence * 0.1; // Max 0.1 from confidence

        score.min(1.0)
    }

    fn calculate_fast_drop_confidence(&self, token: &Token, drop_pct: f64) -> f64 {
        let mut confidence = 0.5f64; // Base confidence for real-time detection

        // Higher confidence for moderate drops (not extreme)
        if drop_pct >= -15.0 && drop_pct <= -5.0 {
            confidence += 0.3;
        }

        // Volume confirmation
        if token.volume.h1 > 5000.0 {
            confidence += 0.2;
        }

        confidence.min(1.0)
    }

    fn calculate_historical_drop_confidence(
        &self,
        token: &Token,
        drop_pct: f64,
        _dataframe: &crate::ohlcv::TokenOhlcvCache
    ) -> f64 {
        let mut confidence = 0.7f64; // Higher base confidence for historical data

        // Confidence adjustments based on drop characteristics
        if drop_pct >= -20.0 && drop_pct <= -3.0 {
            confidence += 0.2; // Healthy dip range
        }

        // Liquidity confidence
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        if liquidity_sol > 100.0 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct DropSignal {
    pub token_mint: String,
    pub drop_percentage: f64,
    pub detection_source: DropSource,
    pub confidence: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum DropSource {
    RealTimePool, // Detected from real-time pool price changes
    Historical, // Detected from dataframe/API data
}

impl DropSignal {
    pub fn is_fresh(&self, max_age_seconds: u64) -> bool {
        let age = chrono::Utc::now() - self.timestamp;
        age.num_seconds() < (max_age_seconds as i64)
    }

    pub fn is_actionable(&self, min_confidence: f64) -> bool {
        self.confidence >= min_confidence && self.is_fresh(300) // 5 minutes max age
    }
}
