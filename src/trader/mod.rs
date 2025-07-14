use crate::core::{
    BotResult,
    BotError,
    TraderConfig,
    TradeSignal,
    SignalType,
    TokenOpportunity,
    TradeAnalysis,
    TradeResult,
    RiskAssessment,
    Portfolio,
};
use crate::wallet::WalletManager;
use solana_sdk::pubkey::Pubkey;
use chrono::{ Utc, Duration };
use std::collections::HashMap;

pub mod strategy;
pub mod execution;
pub mod analysis;

pub use strategy::*;
pub use execution::*;
pub use analysis::*;

/// Main trader manager for executing trades
#[derive(Debug)]
pub struct TraderManager {
    config: TraderConfig,
    strategy: TradingStrategy,
    executor: TradeExecutor,
    analyzer: TradeAnalyzer,
    active_signals: Vec<TradeSignal>,
}

impl TraderManager {
    /// Create a new trader manager
    pub fn new(bot_config: &crate::core::BotConfig) -> BotResult<Self> {
        let config = &bot_config.trader_config;
        let strategy = TradingStrategy::new(config);
        let executor = TradeExecutor::new(config);
        let analyzer = TradeAnalyzer::new();

        Ok(Self {
            config: config.clone(),
            strategy,
            executor,
            analyzer,
            active_signals: Vec::new(),
        })
    }

    /// Initialize the trader
    pub async fn initialize(&mut self) -> BotResult<()> {
        log::info!("💱 Initializing trader...");
        log::info!("📊 Entry amount: {} SOL", self.config.entry_amount_sol);
        log::info!("🎯 Max positions: {}", self.config.max_positions);
        log::info!("📈 DCA enabled: {}", self.config.dca_enabled);

        self.executor.initialize().await?;

        log::info!("✅ Trader initialized successfully");
        Ok(())
    }

    /// Analyze a token opportunity and generate trade signal
    pub async fn analyze_opportunity(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> BotResult<Option<TradeSignal>> {
        // Check if we already have a position in this token
        if portfolio.positions.iter().any(|p| p.token == opportunity.mint) {
            // Check for DCA opportunity
            if self.config.dca_enabled {
                return self.check_dca_opportunity(opportunity, portfolio).await;
            } else {
                return Ok(None);
            }
        }

        // Check if we have room for new positions
        if portfolio.positions.len() >= (self.config.max_positions as usize) {
            log::debug!("🚫 Max positions reached, skipping {}", opportunity.symbol);
            return Ok(None);
        }

        // Analyze the opportunity
        let analysis = self.analyzer.analyze_opportunity(opportunity, portfolio).await?;

        // Generate signal based on strategy
        let signal = self.strategy.generate_signal(opportunity, &analysis).await?;

        Ok(signal)
    }

    /// Check for DCA (Dollar Cost Averaging) opportunity
    async fn check_dca_opportunity(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> BotResult<Option<TradeSignal>> {
        if let Some(position) = portfolio.positions.iter().find(|p| p.token == opportunity.mint) {
            // Check if price has dropped enough for DCA
            let current_price = opportunity.metrics.price_usd;
            let avg_price = position.average_entry_price;

            let price_drop_percentage = ((avg_price - current_price) / avg_price) * 100.0;

            if price_drop_percentage >= self.config.dca_percentage {
                log::info!(
                    "📉 DCA opportunity for {}: {:.2}% drop",
                    opportunity.symbol,
                    price_drop_percentage
                );

                let analysis = TradeAnalysis {
                    entry_reason: format!(
                        "DCA opportunity: {:.2}% price drop",
                        price_drop_percentage
                    ),
                    technical_indicators: HashMap::new(),
                    fundamental_score: 0.7, // Moderate score for DCA
                    risk_assessment: RiskAssessment {
                        overall_risk: crate::core::RiskLevel::Medium,
                        liquidity_risk: crate::core::RiskLevel::Low,
                        volatility_risk: crate::core::RiskLevel::Medium,
                        concentration_risk: crate::core::RiskLevel::Medium,
                        smart_money_risk: crate::core::RiskLevel::Low,
                    },
                    expected_return: 20.0, // Conservative expectation for DCA
                    time_horizon: "medium".to_string(),
                };

                return Ok(
                    Some(TradeSignal {
                        token: opportunity.mint,
                        signal_type: SignalType::DCA,
                        strength: 0.6, // Moderate strength for DCA
                        recommended_amount: self.config.entry_amount_sol * 0.5, // Smaller DCA amount
                        max_slippage: self.config.max_slippage,
                        generated_at: Utc::now(),
                        valid_until: Utc::now() + Duration::minutes(30),
                        analysis_data: analysis,
                    })
                );
            }
        }

        Ok(None)
    }

    /// Execute a trade based on signal
    pub async fn execute_trade(
        &mut self,
        signal: &TradeSignal,
        wallet: &WalletManager
    ) -> BotResult<TradeResult> {
        log::info!(
            "💱 Executing {} trade for token: {}",
            match signal.signal_type {
                SignalType::Buy => "BUY",
                SignalType::Sell => "SELL",
                SignalType::DCA => "DCA",
                SignalType::Hold => "HOLD",
            },
            signal.token
        );

        // Validate signal is still valid
        if Utc::now() > signal.valid_until {
            return Err(BotError::Trading("Trade signal expired".to_string()));
        }

        // Check wallet balance
        if !wallet.has_sufficient_sol(signal.recommended_amount).await? {
            return Err(BotError::InsufficientFunds {
                needed: signal.recommended_amount,
                available: wallet.get_sol_balance().await?,
            });
        }

        // Execute the trade
        let result = match signal.signal_type {
            SignalType::Buy | SignalType::DCA => {
                self.executor.execute_buy(signal, wallet).await?
            }
            SignalType::Sell => { self.executor.execute_sell(signal, wallet).await? }
            SignalType::Hold => {
                return Err(BotError::Trading("Cannot execute HOLD signal".to_string()));
            }
        };

        // Add to active signals for tracking
        self.active_signals.push(signal.clone());

        log::info!("✅ Trade executed successfully: {}", result.transaction_id);
        Ok(result)
    }

    /// Check existing positions for sell opportunities
    pub async fn check_sell_opportunities(
        &self,
        portfolio: &Portfolio
    ) -> BotResult<Vec<TradeSignal>> {
        let mut sell_signals = Vec::new();

        for position in &portfolio.positions {
            // Check for take profit
            if position.unrealized_pnl_percentage >= self.config.take_profit_percentage {
                let signal = self.create_sell_signal(
                    &position.token,
                    "Take profit reached",
                    0.8, // High strength for take profit
                    position.total_amount as f64
                ).await?;

                sell_signals.push(signal);
            }

            // Check for stop loss (only if enabled and we're not in "never lose" mode)
            if
                self.config.stop_loss_enabled &&
                position.unrealized_pnl_percentage <= -self.config.stop_loss_percentage
            {
                // In "never lose" mode, we might implement a different strategy here
                // For now, we skip stop losses as per requirement
                log::warn!(
                    "⚠️ Position {} is at {}% loss, but stop loss disabled",
                    position.symbol,
                    position.unrealized_pnl_percentage
                );
            }
        }

        Ok(sell_signals)
    }

    /// Create a sell signal
    async fn create_sell_signal(
        &self,
        token: &Pubkey,
        reason: &str,
        strength: f64,
        amount: f64
    ) -> BotResult<TradeSignal> {
        Ok(TradeSignal {
            token: *token,
            signal_type: SignalType::Sell,
            strength,
            recommended_amount: amount,
            max_slippage: self.config.max_slippage,
            generated_at: Utc::now(),
            valid_until: Utc::now() + Duration::minutes(15),
            analysis_data: TradeAnalysis {
                entry_reason: reason.to_string(),
                technical_indicators: HashMap::new(),
                fundamental_score: 0.5,
                risk_assessment: RiskAssessment {
                    overall_risk: crate::core::RiskLevel::Low,
                    liquidity_risk: crate::core::RiskLevel::Low,
                    volatility_risk: crate::core::RiskLevel::Medium,
                    concentration_risk: crate::core::RiskLevel::Low,
                    smart_money_risk: crate::core::RiskLevel::Low,
                },
                expected_return: 0.0,
                time_horizon: "immediate".to_string(),
            },
        })
    }

    /// Clean up expired signals
    pub fn cleanup_expired_signals(&mut self) {
        let now = Utc::now();
        self.active_signals.retain(|signal| signal.valid_until > now);
    }

    /// Get current active signals
    pub fn get_active_signals(&self) -> &[TradeSignal] {
        &self.active_signals
    }

    /// Update trader configuration
    pub fn update_config(&mut self, config: TraderConfig) {
        self.config = config;
        self.strategy = TradingStrategy::new(&self.config);
        self.executor = TradeExecutor::new(&self.config);
    }
}
