use anyhow::Result;
use tokio::time::{ Duration, Instant };

pub mod config;
pub mod constants;
pub mod error;
pub mod rpc;
pub mod types;

// Import actual implementations from other modules
use crate::wallet::WalletManager;
use crate::cache::CacheManager;
use crate::screener::ScreenerManager;
use crate::trader::TraderManager;
use crate::portfolio::PortfolioManager;

/// Re-export important types
pub use types::*;
pub use config::*;
pub use error::*;
pub use rpc::RpcManager;
pub use constants::*;

/// Core bot runtime that manages all components
#[derive(Debug)]
pub struct BotRuntime {
    pub config: BotConfig,
    pub wallet_manager: WalletManager,
    pub cache: CacheManager,
    pub rpc_client: RpcManager,
    pub screener: ScreenerManager,
    pub trader: TraderManager,
    pub portfolio: PortfolioManager,
    pub is_running: bool,
    pub last_update: Instant,
}

impl BotRuntime {
    /// Initialize the bot with configuration
    pub async fn new(config_path: &str) -> Result<Self> {
        let config = BotConfig::load(config_path)?;

        let rpc_client = RpcManager::new(crate::core::constants::DEFAULT_RPC_URL)?;
        let cache = CacheManager::new(&config)?;

        // Create wallet manager with cache support
        let wallet_manager = WalletManager::with_cache(&config, cache.clone())?;

        let screener = ScreenerManager::new(&config)?;
        let trader = TraderManager::new(&config)?;
        let portfolio = PortfolioManager::new(&config)?;

        Ok(Self {
            config,
            wallet_manager,
            cache,
            rpc_client,
            screener,
            trader,
            portfolio,
            is_running: false,
            last_update: Instant::now(),
        })
    }

    /// Start the bot main loop
    pub async fn start(&mut self) -> Result<()> {
        log::info!("🚀 Starting ScreenerBot...");

        self.is_running = true;
        self.initialize_components().await?;

        // Main bot loop
        while self.is_running {
            if let Err(e) = self.run_cycle().await {
                log::error!("Bot cycle error: {}", e);
                // Continue running unless it's a critical error
                if self.is_critical_error(&e) {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(60)).await; // Default 1 minute loop delay
        }

        log::info!("🛑 ScreenerBot stopped");
        Ok(())
    }

    /// Stop the bot
    pub fn stop(&mut self) {
        log::info!("Stopping bot...");
        self.is_running = false;
    }

    /// Initialize all bot components
    async fn initialize_components(&mut self) -> Result<()> {
        log::info!("🔧 Initializing bot components...");

        // Initialize wallet and load current holdings
        self.wallet_manager.initialize().await?;

        // Initialize cache system
        self.cache.initialize().await?;

        // Initialize screener sources
        self.screener.initialize().await?;

        // Initialize trader
        self.trader.initialize().await?;

        // Load and update portfolio
        self.portfolio.initialize(&self.wallet_manager, &self.cache).await?;

        log::info!("✅ All components initialized successfully");
        Ok(())
    }

    /// Run one complete bot cycle
    async fn run_cycle(&mut self) -> Result<()> {
        log::info!("🔄 Starting bot cycle...");

        // 1. Update wallet and portfolio
        self.update_portfolio().await?;

        // 2. Run screener to find new opportunities
        let opportunities = self.screener.scan_opportunities().await?;

        // 3. Analyze and filter opportunities
        let filtered_opportunities = self.analyze_opportunities(opportunities).await?;

        // 4. Execute trades based on strategy
        if !filtered_opportunities.is_empty() {
            self.execute_trades(filtered_opportunities).await?;
        }

        // 5. Update portfolio after trades
        self.update_portfolio().await?;

        // 6. Print portfolio status
        self.print_portfolio_status().await?;

        self.last_update = Instant::now();
        log::info!("✅ Bot cycle completed");

        Ok(())
    }

    /// Update portfolio from on-chain data
    async fn update_portfolio(&mut self) -> Result<()> {
        log::debug!("📊 Updating portfolio...");

        // Get latest wallet balances
        let balances = self.wallet_manager.get_all_balances().await?;

        // Get recent transactions
        let recent_txs = self.wallet_manager.get_recent_transactions().await?;

        // Update portfolio with new data
        self.portfolio.update(balances, &recent_txs, &self.cache).await?;

        Ok(())
    }

    /// Analyze opportunities from screener
    async fn analyze_opportunities(
        &self,
        opportunities: Vec<TokenOpportunity>
    ) -> Result<Vec<TradeSignal>> {
        log::debug!("🔍 Analyzing {} opportunities...", opportunities.len());

        let mut signals = Vec::new();

        for opportunity in opportunities {
            if
                let Some(signal) = self.trader.analyze_opportunity(
                    &opportunity,
                    &self.portfolio.current_portfolio
                ).await?
            {
                signals.push(signal);
            }
        }

        log::info!("📈 Found {} valid trade signals", signals.len());
        Ok(signals)
    }

    /// Execute trades based on signals
    async fn execute_trades(&mut self, signals: Vec<TradeSignal>) -> Result<()> {
        log::info!("💱 Executing {} trades...", signals.len());

        for signal in signals {
            match self.trader.execute_trade(&signal, &self.wallet_manager).await {
                Ok(result) => {
                    log::info!("✅ Trade executed: {:?}", result);
                    // Cache the trade result
                    self.cache.store_trade_result(&result).await?;
                }
                Err(e) => {
                    log::error!("❌ Trade failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Print current portfolio status to console
    async fn print_portfolio_status(&self) -> Result<()> {
        self.portfolio.print_status().await?;
        Ok(())
    }

    /// Check if error is critical enough to stop the bot
    fn is_critical_error(&self, error: &anyhow::Error) -> bool {
        // Define critical errors that should stop the bot
        let error_str = error.to_string().to_lowercase();
        error_str.contains("wallet access") ||
            error_str.contains("private key") ||
            error_str.contains("insufficient funds")
    }
}
