use anyhow::Result;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
pub mod monitor;

use std::sync::Arc;

use crate::marketdata::{ MarketDatabase };
use crate::marketdata::database::{ TokenBlacklist, RugDetectionEvent, LiquidityHistory };

pub use monitor::{ RugDetectionMonitor, MonitoringStats };

/// Configuration for rug detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugDetectionConfig {
    pub enabled: bool,
    pub max_liquidity_drop_percent: f64, // 80.0
    pub min_peak_liquidity: f64, // 50000.0
    pub critical_liquidity_threshold: f64, // 1000.0
    pub detection_window_hours: i64, // 24
    pub auto_blacklist: bool, // true
    pub volume_anomaly_threshold: f64, // 0.1 (10% of normal)
    pub reserve_imbalance_threshold: f64, // 90.0 (90% drain)
}

impl Default for RugDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_liquidity_drop_percent: 80.0,
            min_peak_liquidity: 50000.0,
            critical_liquidity_threshold: 1000.0,
            detection_window_hours: 24,
            auto_blacklist: true,
            volume_anomaly_threshold: 0.1,
            reserve_imbalance_threshold: 90.0,
        }
    }
}

/// Rug detection result
#[derive(Debug, Clone)]
pub struct RugDetectionResult {
    pub is_rug: bool,
    pub confidence: f64, // 0.0 to 1.0
    pub reasons: Vec<String>,
    pub recommended_action: RugAction,
}

/// Recommended action after rug detection
#[derive(Debug, Clone)]
pub enum RugAction {
    Continue,
    Monitor,
    Blacklist,
    SellImmediately,
}

/// Core rug detection engine
pub struct RugDetectionEngine {
    database: Arc<MarketDatabase>,
    config: RugDetectionConfig,
}

impl RugDetectionEngine {
    /// Create new rug detection engine
    pub fn new(database: Arc<MarketDatabase>, config: RugDetectionConfig) -> Self {
        Self { database, config }
    }

    /// Analyze token for rug pull indicators
    pub async fn analyze_token(
        &self,
        token_address: &str,
        current_liquidity: f64
    ) -> Result<RugDetectionResult> {
        if !self.config.enabled {
            return Ok(RugDetectionResult {
                is_rug: false,
                confidence: 0.0,
                reasons: vec!["Rug detection disabled".to_string()],
                recommended_action: RugAction::Continue,
            });
        }

        let mut is_rug = false;
        let mut confidence = 0.0;
        let mut reasons = Vec::new();

        // Check if already blacklisted
        if self.database.is_blacklisted(token_address)? {
            return Ok(RugDetectionResult {
                is_rug: true,
                confidence: 1.0,
                reasons: vec!["Token is already blacklisted".to_string()],
                recommended_action: RugAction::Blacklist,
            });
        }

        // 1. Liquidity cliff detection
        if
            let Some(cliff_result) = self.detect_liquidity_cliff(
                token_address,
                current_liquidity
            ).await?
        {
            is_rug = true;
            confidence += cliff_result.confidence;
            reasons.extend(cliff_result.reasons);
        }

        // 2. Dead pool detection
        if let Some(dead_result) = self.detect_dead_pool(token_address, current_liquidity).await? {
            is_rug = true;
            confidence += dead_result.confidence;
            reasons.extend(dead_result.reasons);
        }

        // 3. Volume anomaly detection
        if let Some(volume_result) = self.detect_volume_anomaly(token_address).await? {
            confidence += volume_result.confidence;
            reasons.extend(volume_result.reasons);
            if volume_result.confidence > 0.5 {
                is_rug = true;
            }
        }

        // Cap confidence at 1.0
        confidence = confidence.min(1.0);

        // Determine action
        let recommended_action = if is_rug && confidence > 0.8 {
            RugAction::SellImmediately
        } else if is_rug || confidence > 0.6 {
            RugAction::Blacklist
        } else if confidence > 0.3 {
            RugAction::Monitor
        } else {
            RugAction::Continue
        };

        Ok(RugDetectionResult {
            is_rug,
            confidence,
            reasons,
            recommended_action,
        })
    }

    /// Detect liquidity cliff (sudden massive drop)
    async fn detect_liquidity_cliff(
        &self,
        token_address: &str,
        current_liquidity: f64
    ) -> Result<Option<RugDetectionResult>> {
        // Get peak liquidity in detection window
        let peak_liquidity = self.database
            .get_peak_liquidity(token_address, self.config.detection_window_hours)?
            .unwrap_or(current_liquidity);

        // Check if peak was significant enough to matter
        if peak_liquidity < self.config.min_peak_liquidity {
            return Ok(None);
        }

        // Calculate drop percentage
        let drop_percent = ((peak_liquidity - current_liquidity) / peak_liquidity) * 100.0;

        if drop_percent >= self.config.max_liquidity_drop_percent {
            let confidence = if drop_percent >= 95.0 {
                1.0
            } else if drop_percent >= 90.0 {
                0.9
            } else {
                0.7
            };

            // Record the event
            let event = RugDetectionEvent {
                id: 0, // Will be set by database
                token_address: token_address.to_string(),
                event_type: "liquidity_cliff".to_string(),
                before_value: peak_liquidity,
                after_value: current_liquidity,
                percentage_change: drop_percent,
                detected_at: Utc::now(),
            };
            self.database.record_rug_event(&event)?;

            return Ok(
                Some(RugDetectionResult {
                    is_rug: true,
                    confidence,
                    reasons: vec![
                        format!(
                            "Liquidity cliff detected: {:.1}% drop from ${:.2} to ${:.2}",
                            drop_percent,
                            peak_liquidity,
                            current_liquidity
                        )
                    ],
                    recommended_action: RugAction::SellImmediately,
                })
            );
        }

        Ok(None)
    }

    /// Detect dead pool (near-zero liquidity after significant activity)
    async fn detect_dead_pool(
        &self,
        token_address: &str,
        current_liquidity: f64
    ) -> Result<Option<RugDetectionResult>> {
        // Check if current liquidity is critically low
        if current_liquidity > self.config.critical_liquidity_threshold {
            return Ok(None);
        }

        // Get historical liquidity to see if it was ever significant
        let history = self.database.get_liquidity_history(
            token_address,
            self.config.detection_window_hours * 2
        )?;

        if history.is_empty() {
            return Ok(None);
        }

        // Find maximum historical liquidity
        let max_historical = history
            .iter()
            .map(|h| h.liquidity_sol)
            .fold(0.0, f64::max);

        // If it was never significant, not a rug
        if max_historical < self.config.min_peak_liquidity {
            return Ok(None);
        }

        let confidence = if current_liquidity < 100.0 {
            0.9
        } else if current_liquidity < 500.0 {
            0.7
        } else {
            0.5
        };

        // Record the event
        let event = RugDetectionEvent {
            id: 0,
            token_address: token_address.to_string(),
            event_type: "dead_pool".to_string(),
            before_value: max_historical,
            after_value: current_liquidity,
            percentage_change: ((max_historical - current_liquidity) / max_historical) * 100.0,
            detected_at: Utc::now(),
        };
        self.database.record_rug_event(&event)?;

        Ok(
            Some(RugDetectionResult {
                is_rug: true,
                confidence,
                reasons: vec![
                    format!(
                        "Dead pool detected: liquidity fell from ${:.2} to ${:.2}",
                        max_historical,
                        current_liquidity
                    )
                ],
                recommended_action: RugAction::Blacklist,
            })
        )
    }

    /// Detect volume anomaly (trading stopped despite previous activity)
    async fn detect_volume_anomaly(
        &self,
        token_address: &str
    ) -> Result<Option<RugDetectionResult>> {
        // Get recent rug events to see if volume anomaly was already detected
        let recent_events = self.database.get_rug_events(token_address, 1)?; // Last 1 hour

        let volume_anomaly_exists = recent_events.iter().any(|e| e.event_type == "volume_anomaly");

        if volume_anomaly_exists {
            return Ok(None); // Already detected recently
        }

        // This would require volume data from market data module
        // For now, return a placeholder
        Ok(None)
    }

    /// Auto-blacklist token if rug is detected
    pub async fn auto_blacklist_if_rug(
        &self,
        token_address: &str,
        result: &RugDetectionResult
    ) -> Result<()> {
        if !self.config.auto_blacklist {
            return Ok(());
        }

        if result.is_rug && result.confidence > 0.7 {
            let peak_liquidity = self.database.get_peak_liquidity(
                token_address,
                self.config.detection_window_hours
            )?;

            // Get current token data for final liquidity
            let token_data = self.database.get_token(token_address)?;
            let final_liquidity = token_data.map(|t| t.liquidity_sol);

            let drop_percentage = if
                let (Some(peak), Some(final_liq)) = (peak_liquidity, final_liquidity)
            {
                Some(((peak - final_liq) / peak) * 100.0)
            } else {
                None
            };

            let blacklist_entry = TokenBlacklist {
                token_address: token_address.to_string(),
                reason: "rug_detected".to_string(),
                blacklisted_at: Utc::now(),
                peak_liquidity,
                final_liquidity,
                drop_percentage,
            };

            self.database.add_to_blacklist(&blacklist_entry)?;
        }

        Ok(())
    }

    /// Get rug detection statistics
    pub async fn get_stats(&self) -> Result<RugDetectionStats> {
        let _database = &self.database;

        // This would require additional database queries
        // For now, return placeholder stats
        Ok(RugDetectionStats {
            total_events_detected: 0,
            tokens_blacklisted: 0,
            liquidity_cliffs_detected: 0,
            dead_pools_detected: 0,
            volume_anomalies_detected: 0,
            last_detection_run: Utc::now(),
        })
    }
}

/// Rug detection statistics
#[derive(Debug, Clone)]
pub struct RugDetectionStats {
    pub total_events_detected: u64,
    pub tokens_blacklisted: u64,
    pub liquidity_cliffs_detected: u64,
    pub dead_pools_detected: u64,
    pub volume_anomalies_detected: u64,
    pub last_detection_run: DateTime<Utc>,
}
