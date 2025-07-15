// Trading module - Organized and standard implementation
// This module handles all trading operations including position management,
// profit strategies, and transaction execution

pub mod position_manager;
pub mod transaction_manager;
pub mod profit_strategy;
pub mod trade_executor;
pub mod risk_manager;

pub use position_manager::PositionManager;
pub use transaction_manager::TransactionManager;
pub use profit_strategy::ProfitStrategy as ProfitStrategyManager;
pub use trade_executor::TradeExecutor;
pub use risk_manager::RiskManager;

use crate::config::{ TraderConfig, TradingConfig };
use crate::database::Database;
use crate::discovery::Discovery;
use crate::logger::Logger;
use crate::types::{ TradeSignal, TradingPosition, PortfolioMetrics };
use crate::wallet::WalletTracker;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Main trading orchestrator that coordinates all trading operations
pub struct TradingEngine {
    pub config: TraderConfig,
    pub trading_config: TradingConfig,
    pub position_manager: Arc<PositionManager>,
    pub transaction_manager: Arc<TransactionManager>,
    pub profit_strategy: Arc<ProfitStrategyManager>,
    pub trade_executor: Arc<TradeExecutor>,
    pub risk_manager: Arc<RiskManager>,
    is_running: Arc<RwLock<bool>>,
}

impl TradingEngine {
    pub fn new(
        config: TraderConfig,
        trading_config: TradingConfig,
        database: Arc<Database>,
        discovery: Arc<Discovery>,
        wallet_tracker: Arc<WalletTracker>
    ) -> Self {
        let position_manager = Arc::new(
            PositionManager::new(config.clone(), Arc::clone(&database), Arc::clone(&wallet_tracker))
        );

        let transaction_manager = Arc::new(
            TransactionManager::new(
                trading_config.transaction_manager.clone(),
                Arc::clone(&database),
                Arc::clone(&wallet_tracker)
            )
        );

        let profit_strategy = Arc::new(
            ProfitStrategyManager::new(config.time_based_profit.clone())
        );

        let trade_executor = Arc::new(
            TradeExecutor::new(
                config.clone(),
                trading_config.clone(),
                Arc::clone(&wallet_tracker),
                Arc::clone(&transaction_manager)
            )
        );

        let risk_manager = Arc::new(
            RiskManager::new(config.clone(), Arc::clone(&position_manager))
        );

        Self {
            config,
            trading_config,
            position_manager,
            transaction_manager,
            profit_strategy,
            trade_executor,
            risk_manager,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            Logger::warn("Trading engine is disabled in config (safety feature)");
            return Ok(());
        }

        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Trading engine is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Trading engine started");
        Logger::trader("🚀 Advanced Trading System Online - Real trades enabled!");

        // Start all submodules
        self.position_manager.start().await?;
        self.transaction_manager.start().await?;

        // Start main trading loop
        let engine = self.clone();
        tokio::spawn(async move {
            engine.run_trading_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;

        self.position_manager.stop().await;
        self.transaction_manager.stop().await;

        Logger::info("Trading engine stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_portfolio_metrics(&self) -> Result<PortfolioMetrics> {
        self.position_manager.get_portfolio_metrics().await
    }

    pub async fn get_open_positions(&self) -> Result<Vec<TradingPosition>> {
        self.position_manager.get_open_positions().await
    }

    async fn run_trading_loop(&self) {
        use tokio::time::{ interval, Duration };

        Logger::trader("Starting advanced trading loop...");
        let mut interval = interval(Duration::from_secs(self.config.position_check_interval_secs));

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            if let Err(e) = self.process_trading_cycle().await {
                Logger::error(&format!("Trading cycle error: {}", e));
            }
        }

        Logger::trader("Trading loop stopped");
    }

    async fn process_trading_cycle(&self) -> Result<()> {
        // 1. Update existing positions
        self.position_manager.update_positions().await?;

        // 2. Check for profit opportunities
        let positions = self.position_manager.get_open_positions().await?;
        for position in &positions {
            if let Some(action) = self.profit_strategy.evaluate_position(position).await? {
                match action {
                    crate::types::SignalType::Sell => {
                        Logger::trader(
                            &format!(
                                "💰 Closing profitable position: {} (+{:.2}%)",
                                position.token_mint,
                                position.pnl_percentage
                            )
                        );
                        self.trade_executor.close_position(position).await?;
                    }
                    _ => {}
                }
            }
        }

        // 3. Check for new opportunities (only if we have room for more positions)
        let open_count = positions.len() as u32;
        if open_count < self.config.max_open_positions {
            // Analyze market for new entries
            // This would integrate with your discovery system
        }

        Ok(())
    }
}

impl Clone for TradingEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            trading_config: self.trading_config.clone(),
            position_manager: Arc::clone(&self.position_manager),
            transaction_manager: Arc::clone(&self.transaction_manager),
            profit_strategy: Arc::clone(&self.profit_strategy),
            trade_executor: Arc::clone(&self.trade_executor),
            risk_manager: Arc::clone(&self.risk_manager),
            is_running: Arc::clone(&self.is_running),
        }
    }
}
