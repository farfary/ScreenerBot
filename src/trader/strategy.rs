use anyhow::Result;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

use crate::config::TraderConfig;
use crate::trader::types::*;
use crate::trader::position::Position;
use crate::types::TokenInfo;

#[derive(Debug)]
pub struct TradingStrategy {
    config: TraderConfig,
    price_history: HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    last_signals: HashMap<String, DateTime<Utc>>,
}

impl TradingStrategy {
    pub fn new(config: TraderConfig) -> Self {
        Self {
            config,
            price_history: HashMap::new(),
            last_signals: HashMap::new(),
        }
    }

    pub fn update_price(&mut self, token_address: &str, price: f64) {
        let entry = self.price_history.entry(token_address.to_string()).or_insert_with(Vec::new);
        entry.push((Utc::now(), price));

        // Keep only last 100 price points to avoid memory issues
        if entry.len() > 100 {
            entry.drain(0..entry.len() - 100);
        }
    }

    pub fn analyze_token(&self, token_info: &TokenInfo, current_price: f64) -> Option<TradeSignal> {
        let token_address = &token_info.mint;

        // Check if we have enough price history
        let price_history = self.price_history.get(token_address)?;
        if price_history.len() < 2 {
            return None;
        }

        // Check cooldown period to avoid too frequent signals
        if let Some(last_signal_time) = self.last_signals.get(token_address) {
            let cooldown_duration = chrono::Duration::seconds(300); // 5 minutes cooldown
            if Utc::now() - *last_signal_time < cooldown_duration {
                return None;
            }
        }

        // Get the price from 10 minutes ago (or earliest available)
        let lookback_time = Utc::now() - chrono::Duration::minutes(10);
        let reference_price = self
            .get_price_at_time(token_address, lookback_time)
            .unwrap_or_else(|| price_history.first().unwrap().1);

        // Calculate price change percentage
        let price_change_percent = ((current_price - reference_price) / reference_price) * 100.0;

        // Check for buy signal (price dropped by trigger percent)
        if price_change_percent <= self.config.buy_trigger_percent {
            return Some(TradeSignal {
                token_address: token_address.clone(),
                signal_type: TradeSignalType::Buy,
                current_price,
                trigger_price: reference_price,
                timestamp: Utc::now(),
                volume_24h: token_info.volume_24h.unwrap_or(0.0),
                liquidity: token_info.liquidity.unwrap_or(0.0),
            });
        }

        None
    }

    pub fn analyze_position(&self, position: &Position, current_price: f64) -> Vec<TradeSignal> {
        let mut signals = Vec::new();

        if !matches!(position.status, PositionStatus::Active) {
            return signals;
        }

        // Check for stop loss
        if position.should_stop_loss(self.config.stop_loss_percent) {
            signals.push(TradeSignal {
                token_address: position.token_address.clone(),
                signal_type: TradeSignalType::StopLoss,
                current_price,
                trigger_price: position.average_buy_price *
                (1.0 + self.config.stop_loss_percent / 100.0),
                timestamp: Utc::now(),
                volume_24h: 0.0,
                liquidity: 0.0,
            });
        }

        // Check for take profit
        if position.should_take_profit(self.config.sell_trigger_percent) {
            signals.push(TradeSignal {
                token_address: position.token_address.clone(),
                signal_type: TradeSignalType::Sell,
                current_price,
                trigger_price: position.average_buy_price *
                (1.0 + self.config.sell_trigger_percent / 100.0),
                timestamp: Utc::now(),
                volume_24h: 0.0,
                liquidity: 0.0,
            });
        }

        // Check for DCA opportunities  
        if self.config.dca_enabled && position.should_dca(&self.config) {
            signals.push(TradeSignal {
                token_address: position.token_address.clone(),
                signal_type: TradeSignalType::DCA,
                current_price,
                trigger_price: position.average_buy_price * 
                (1.0 + self.config.dca_min_loss_percent / 100.0),
                timestamp: Utc::now(),
                volume_24h: 0.0,
                liquidity: 0.0,
            });
        }

        signals
    }

    pub fn should_buy(&self, token_info: &TokenInfo, current_price: f64) -> bool {
        // Basic filters
        if let Some(liquidity) = token_info.liquidity {
            if liquidity < 10000.0 {
                return false;
            }
        } else {
            return false;
        }

        if let Some(volume_24h) = token_info.volume_24h {
            if volume_24h < 50000.0 {
                return false;
            }
        } else {
            return false;
        }

        // Check if we have a buy signal
        self.analyze_token(token_info, current_price).is_some()
    }

    pub fn calculate_trade_size(&self, signal: &TradeSignal, position: Option<&Position>) -> f64 {
        match signal.signal_type {
            TradeSignalType::Buy => self.config.trade_size_sol,
            TradeSignalType::DCA => {
                if let Some(pos) = position {
                    pos.get_dca_amount_sol(&self.config)
                } else {
                    self.config.trade_size_sol
                }
            }
            TradeSignalType::Sell | TradeSignalType::StopLoss => {
                // Sell all tokens
                if let Some(pos) = position {
                    pos.total_tokens * signal.current_price
                } else {
                    0.0
                }
            }
        }
    }

    pub fn get_price_at_time(
        &self,
        token_address: &str,
        target_time: DateTime<Utc>
    ) -> Option<f64> {
        let price_history = self.price_history.get(token_address)?;

        // Find the price entry closest to the target time
        let mut closest_entry = None;
        let mut closest_diff = chrono::Duration::seconds(86400); // 24 hours as max diff

        for (timestamp, price) in price_history {
            let diff = if *timestamp > target_time {
                *timestamp - target_time
            } else {
                target_time - *timestamp
            };

            if diff < closest_diff {
                closest_diff = diff;
                closest_entry = Some(*price);
            }
        }

        closest_entry
    }

    pub fn get_current_price(&self, token_address: &str) -> Option<f64> {
        self.price_history
            .get(token_address)
            .and_then(|history| history.last())
            .map(|(_, price)| *price)
    }

    pub fn record_signal(&mut self, token_address: &str) {
        self.last_signals.insert(token_address.to_string(), Utc::now());
    }

    pub fn validate_trade_conditions(
        &self,
        token_info: &TokenInfo,
        signal: &TradeSignal
    ) -> Result<()> {
        // Check minimum liquidity
        if let Some(liquidity) = token_info.liquidity {
            if liquidity < 10000.0 {
                return Err(anyhow::anyhow!("Insufficient liquidity: {}", liquidity));
            }
        } else {
            return Err(anyhow::anyhow!("No liquidity data available"));
        }

        // Check minimum volume
        if let Some(volume_24h) = token_info.volume_24h {
            if volume_24h < 50000.0 {
                return Err(anyhow::anyhow!("Insufficient volume: {}", volume_24h));
            }
        } else {
            return Err(anyhow::anyhow!("No volume data available"));
        }

        // Check price sanity
        if signal.current_price <= 0.0 {
            return Err(anyhow::anyhow!("Invalid price: {}", signal.current_price));
        }

        // Check if market cap is within limits
        if let Some(market_cap) = token_info.market_cap {
            if market_cap < 10000.0 {
                return Err(anyhow::anyhow!("Market cap too low: {}", market_cap));
            }
            if market_cap > 1000000.0 {
                return Err(anyhow::anyhow!("Market cap too high: {}", market_cap));
            }
        }

        Ok(())
    }

    pub fn get_risk_assessment(
        &self,
        token_info: &TokenInfo,
        position: Option<&Position>
    ) -> RiskLevel {
        let mut risk_score = 0;

        // Liquidity risk
        if let Some(liquidity) = token_info.liquidity {
            if liquidity < 50000.0 {
                risk_score += 2;
            } else if liquidity < 100000.0 {
                risk_score += 1;
            }
        } else {
            risk_score += 3; // No liquidity data is very risky
        }

        // Volume risk
        if let Some(volume_24h) = token_info.volume_24h {
            if volume_24h < 100000.0 {
                risk_score += 2;
            } else if volume_24h < 500000.0 {
                risk_score += 1;
            }
        } else {
            risk_score += 2; // No volume data is risky
        }

        // Position risk
        if let Some(pos) = position {
            if pos.dca_count >= 2 {
                risk_score += 1;
            }
            if pos.unrealized_pnl_percent < -30.0 {
                risk_score += 2;
            }
        }

        match risk_score {
            0..=1 => RiskLevel::Low,
            2..=3 => RiskLevel::Medium,
            4..=5 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}
