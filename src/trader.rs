use crate::config::TraderConfig;
use crate::database::Database;
use crate::discovery::Discovery;
use crate::logger::Logger;
use crate::types::{ TradeSignal, SignalType, TokenInfo, WalletPosition };
use crate::wallet::WalletTracker;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Trader {
    config: TraderConfig,
    database: Arc<Database>,
    discovery: Arc<Discovery>,
    wallet_tracker: Arc<WalletTracker>,
    active_signals: Arc<RwLock<HashMap<String, TradeSignal>>>,
    is_running: Arc<RwLock<bool>>,
}

impl Trader {
    pub fn new(
        config: TraderConfig,
        database: Arc<Database>,
        discovery: Arc<Discovery>,
        wallet_tracker: Arc<WalletTracker>
    ) -> Self {
        Self {
            config,
            database,
            discovery,
            wallet_tracker,
            active_signals: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            Logger::warn("Trading module is disabled in config (safety feature)");
            return Ok(());
        }

        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Trader is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Trading module started");
        Logger::trader("⚠️  TRADING IS ENABLED - Real trades may be executed!");

        // Start trading analysis loop
        let trader = self.clone();
        tokio::spawn(async move {
            trader.run_analysis_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::info("Trading module stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_active_signals(&self) -> HashMap<String, TradeSignal> {
        self.active_signals.read().await.clone()
    }

    pub async fn analyze_market(&self) -> Result<Vec<TradeSignal>> {
        Logger::trader("Analyzing market for trading opportunities...");

        let mut signals = Vec::new();

        // Get discovered tokens
        let discovered_tokens = self.discovery.get_cached_tokens().await;

        // Get current wallet positions
        let positions = self.wallet_tracker.get_positions().await;

        // Analyze discovered tokens for entry opportunities
        for (_mint, token) in &discovered_tokens {
            if let Some(signal) = self.analyze_token_for_entry(token).await {
                if signal.confidence >= self.config.min_confidence_score {
                    signals.push(signal);
                }
            }
        }

        // Analyze existing positions for exit opportunities
        for (mint, position) in &positions {
            if let Some(token) = discovered_tokens.get(mint) {
                if let Some(signal) = self.analyze_position_for_exit(token, position).await {
                    if signal.confidence >= self.config.min_confidence_score {
                        signals.push(signal);
                    }
                }
            }
        }

        Logger::trader(&format!("Generated {} trading signals", signals.len()));

        // Update active signals
        let mut active_signals = self.active_signals.write().await;
        active_signals.clear();
        for signal in &signals {
            active_signals.insert(signal.token_mint.clone(), signal.clone());
        }

        Ok(signals)
    }

    async fn analyze_token_for_entry(&self, token: &TokenInfo) -> Option<TradeSignal> {
        // Simple analysis logic - you would implement more sophisticated analysis
        let mut confidence = 0.0;
        let mut reasons = Vec::new();

        // Check liquidity
        if let Some(liquidity) = token.liquidity {
            if liquidity > 50000.0 {
                confidence += 0.2;
                reasons.push("Good liquidity");
            }
        }

        // Check volume
        if let Some(volume) = token.volume_24h {
            if volume > 100000.0 {
                confidence += 0.3;
                reasons.push("High volume");
            }
        }

        // Check if it's a newly discovered token (potential early entry)
        let hours_since_discovery = chrono::Utc
            ::now()
            .signed_duration_since(token.discovered_at)
            .num_hours();

        if hours_since_discovery < 24 {
            confidence += 0.2;
            reasons.push("Recently discovered");
        }

        // Price movement analysis (placeholder)
        if let Some(price) = token.price {
            if price > 0.0 {
                confidence += 0.1;
                reasons.push("Has price data");
            }
        }

        if confidence >= 0.5 {
            Some(TradeSignal {
                token_mint: token.mint.clone(),
                signal_type: SignalType::Buy,
                confidence,
                price: token.price.unwrap_or(0.0),
                volume: token.volume_24h.unwrap_or(0.0),
                timestamp: chrono::Utc::now(),
                reason: reasons.join(", "),
            })
        } else {
            None
        }
    }

    async fn analyze_position_for_exit(
        &self,
        token: &TokenInfo,
        position: &WalletPosition
    ) -> Option<TradeSignal> {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();
        let mut signal_type = SignalType::Hold;

        // Check for profit taking
        if let Some(pnl_pct) = position.pnl_percentage {
            if pnl_pct >= self.config.min_profit_percentage {
                confidence += 0.8;
                signal_type = SignalType::Sell;
                reasons.push(format!("Take profit: {:.1}%", pnl_pct));
            } else if pnl_pct <= self.config.max_loss_percentage {
                confidence += 0.9;
                signal_type = SignalType::Sell;
                reasons.push(format!("Stop loss: {:.1}%", pnl_pct));
            }
        }

        // Check volume decline (might indicate loss of interest)
        if let Some(volume) = token.volume_24h {
            if volume < 10000.0 {
                confidence += 0.3;
                signal_type = SignalType::Sell;
                reasons.push("Low volume".to_string());
            }
        }

        // Check liquidity decline
        if let Some(liquidity) = token.liquidity {
            if liquidity < 20000.0 {
                confidence += 0.4;
                signal_type = SignalType::Sell;
                reasons.push("Low liquidity".to_string());
            }
        }

        if confidence >= 0.5 && matches!(signal_type, SignalType::Sell) {
            Some(TradeSignal {
                token_mint: token.mint.clone(),
                signal_type,
                confidence,
                price: token.price.unwrap_or(0.0),
                volume: token.volume_24h.unwrap_or(0.0),
                timestamp: chrono::Utc::now(),
                reason: reasons.join(", "),
            })
        } else {
            None
        }
    }

    async fn run_analysis_loop(&self) {
        use tokio::time::{ interval, Duration };

        Logger::trader("Starting trading analysis loop...");

        let mut interval = interval(Duration::from_secs(60)); // Analyze every minute

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            match self.analyze_market().await {
                Ok(signals) => {
                    if !signals.is_empty() {
                        Logger::trader(
                            &format!("Analysis complete - {} signals generated", signals.len())
                        );

                        // Log high-confidence signals
                        for signal in signals.iter().filter(|s| s.confidence > 0.7) {
                            match signal.signal_type {
                                SignalType::Buy => {
                                    Logger::trader(
                                        &format!(
                                            "🟢 BUY SIGNAL: {} (Confidence: {:.1}%) - {}",
                                            signal.token_mint,
                                            signal.confidence * 100.0,
                                            signal.reason
                                        )
                                    );
                                }
                                SignalType::Sell => {
                                    Logger::trader(
                                        &format!(
                                            "🔴 SELL SIGNAL: {} (Confidence: {:.1}%) - {}",
                                            signal.token_mint,
                                            signal.confidence * 100.0,
                                            signal.reason
                                        )
                                    );
                                }
                                SignalType::Hold => {
                                    Logger::trader(
                                        &format!(
                                            "🟡 HOLD: {} (Confidence: {:.1}%) - {}",
                                            signal.token_mint,
                                            signal.confidence * 100.0,
                                            signal.reason
                                        )
                                    );
                                }
                            }
                        }

                        // Note: Actual trade execution would go here
                        // For safety, we're not implementing actual trades in this demo
                        Logger::trader("⚠️  Trade execution is not implemented for safety");
                    }
                }
                Err(e) => {
                    Logger::error(&format!("Market analysis failed: {}", e));
                }
            }
        }

        Logger::trader("Trading analysis loop stopped");
    }

    // Placeholder for future implementation
    pub async fn execute_trade(&self, signal: &TradeSignal) -> Result<()> {
        Logger::trader(
            &format!(
                "Would execute {:?} trade for {} with confidence {:.1}%",
                signal.signal_type,
                signal.token_mint,
                signal.confidence * 100.0
            )
        );

        // TODO: Implement actual trade execution
        // This would involve:
        // 1. Creating and sending transactions
        // 2. Calculating optimal amounts
        // 3. Managing slippage
        // 4. Error handling and retries

        Ok(())
    }
}

impl Clone for Trader {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            discovery: Arc::clone(&self.discovery),
            wallet_tracker: Arc::clone(&self.wallet_tracker),
            active_signals: Arc::clone(&self.active_signals),
            is_running: Arc::clone(&self.is_running),
        }
    }
}
