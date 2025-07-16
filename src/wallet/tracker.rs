use crate::{
    config::Config,
    database::Database,
    logger::Logger,
    types::WalletPosition,
    rpc::RpcManager,
    pricing::PricingManager,
};
use super::{ positions::PositionManager, portfolio::PortfolioAnalyzer, display::ConsoleDisplay };
use anyhow::{ Context, Result };
use futures::FutureExt;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer } };
use std::{ collections::HashMap, sync::Arc, time::Duration };
use tokio::{ sync::RwLock, time };

pub struct WalletTracker {
    config: Config,
    database: Arc<Database>,
    rpc_manager: Arc<RpcManager>,
    pricing_manager: Option<Arc<PricingManager>>,
    wallet_keypair: Keypair,
    positions: Arc<RwLock<HashMap<String, WalletPosition>>>,
    is_running: Arc<RwLock<bool>>,
    last_signature: Arc<RwLock<Option<String>>>,
    position_manager: PositionManager,
    portfolio_analyzer: PortfolioAnalyzer,
    console_display: ConsoleDisplay,
}

impl WalletTracker {
    pub fn new(config: Config, database: Arc<Database>) -> Result<Self> {
        let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);

        let rpc_manager = Arc::new(
            RpcManager::new(
                config.rpc_url.clone(),
                config.rpc_fallbacks.clone(),
                config.rpc.clone()
            )?
        );

        Logger::wallet("Initialized RPC manager");

        let position_manager = PositionManager::new(
            Arc::clone(&database),
            Arc::clone(&rpc_manager)
        );
        let portfolio_analyzer = PortfolioAnalyzer::new(Arc::clone(&database));
        let console_display = ConsoleDisplay::new();

        Ok(Self {
            config,
            database,
            rpc_manager,
            pricing_manager: None,
            wallet_keypair,
            positions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            last_signature: Arc::new(RwLock::new(None)),
            position_manager,
            portfolio_analyzer,
            console_display,
        })
    }

    pub fn set_pricing_manager(&mut self, pricing_manager: Arc<PricingManager>) {
        self.pricing_manager = Some(Arc::clone(&pricing_manager));
        self.position_manager.set_pricing_manager(pricing_manager);
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Wallet tracker is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Wallet tracker started");
        Logger::wallet(&format!("Tracking wallet: {}", self.wallet_keypair.pubkey()));

        // Load existing positions from database
        self.load_existing_positions().await?;

        // Start tracking loop
        let tracker = self.clone();
        tokio::spawn(async move {
            let result = std::panic
                ::AssertUnwindSafe(tracker.run_tracking_loop())
                .catch_unwind().await;

            match result {
                Ok(()) => {
                    Logger::success("Wallet tracking loop COMPLETED normally");
                }
                Err(panic_info) => {
                    Logger::error(&format!("💥 Wallet tracking loop panicked: {:?}", panic_info));
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::info("Wallet tracker stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_positions(&self) -> HashMap<String, WalletPosition> {
        self.positions.read().await.clone()
    }

    pub async fn get_wallet_pubkey(&self) -> Pubkey {
        self.wallet_keypair.pubkey()
    }

    pub async fn get_sol_balance(&self) -> Result<f64> {
        let wallet_pubkey = self.wallet_keypair.pubkey();
        let balance = self.rpc_manager.get_balance(&wallet_pubkey).await?;
        Ok((balance as f64) / 1_000_000_000.0) // Convert lamports to SOL
    }

    pub async fn refresh_positions(&self) -> Result<()> {
        Logger::wallet("🔄 Refreshing current token positions...");

        // Get current token holdings from blockchain
        let current_holdings = self.position_manager.get_current_token_holdings(
            &self.wallet_keypair.pubkey()
        ).await?;

        if current_holdings.is_empty() {
            Logger::wallet("📝 No SPL token holdings found - wallet contains only SOL");
            *self.positions.write().await = HashMap::new();
            return Ok(());
        }

        Logger::wallet(&format!("💎 Found {} token holdings", current_holdings.len()));

        // Calculate positions with P&L
        let positions = self.position_manager.calculate_positions_with_pnl(current_holdings).await?;

        // Update positions in memory
        *self.positions.write().await = positions.clone();

        // Display current positions in console
        self.console_display.show_current_positions(&positions).await?;

        // Get and display portfolio summary
        let portfolio_summary = self.portfolio_analyzer.calculate_portfolio_summary(
            &positions,
            self.get_sol_balance().await?
        ).await?;

        self.console_display.show_portfolio_summary(&portfolio_summary).await?;

        Logger::success("✅ Position refresh completed");
        Ok(())
    }

    async fn load_existing_positions(&self) -> Result<()> {
        Logger::database("Loading existing positions from database...");
        let positions = self.database
            .get_wallet_positions()
            .context("FAILED to load positions from database")?;

        let mut position_map = HashMap::new();
        for position in positions {
            position_map.insert(position.mint.clone(), position);
        }

        *self.positions.write().await = position_map;
        Logger::database(
            &format!("Loaded {} positions from database", self.positions.read().await.len())
        );
        Ok(())
    }

    async fn run_tracking_loop(&self) {
        Logger::wallet("🚀 Starting enhanced wallet tracking loop...");
        let mut interval = time::interval(Duration::from_secs(30)); // Update every 30 seconds
        let mut summary_counter = 0;

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                Logger::wallet("🛑 Wallet tracking loop stopping (is_running = false)");
                break;
            }
            drop(is_running);

            // Refresh positions and display current holdings
            match self.refresh_positions().await {
                Ok(()) => {
                    summary_counter += 1;

                    // Every 5 cycles (2.5 minutes), show detailed analytics
                    if summary_counter >= 5 {
                        summary_counter = 0;
                        self.show_detailed_analytics().await;
                    }
                }
                Err(e) => {
                    Logger::error(&format!("❌ FAILED to refresh positions: {}", e));
                }
            }

            Logger::wallet("⏱️  Next update in 30 seconds...");
        }

        Logger::wallet("🏁 Wallet tracking loop ended");
    }

    async fn show_detailed_analytics(&self) {
        Logger::separator();
        Logger::wallet("📊 DETAILED PORTFOLIO ANALYTICS");
        Logger::separator();

        let positions = self.get_positions().await;

        // Show performance metrics
        if let Ok(performance) = self.portfolio_analyzer.get_performance_metrics(&positions).await {
            let _ = self.console_display.show_performance_metrics(&performance).await;
        }

        // Show top and worst performers
        if let Ok(top_positions) = self.portfolio_analyzer.get_top_positions(&positions, 5).await {
            let _ = self.console_display.show_top_positions(&top_positions).await;
        }

        if
            let Ok(worst_positions) = self.portfolio_analyzer.get_worst_positions(
                &positions,
                3
            ).await
        {
            let _ = self.console_display.show_worst_positions(&worst_positions).await;
        }

        Logger::separator();
    }
}

impl Clone for WalletTracker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            rpc_manager: Arc::clone(&self.rpc_manager),
            pricing_manager: self.pricing_manager.as_ref().map(Arc::clone),
            wallet_keypair: Keypair::try_from(&self.wallet_keypair.to_bytes()[..]).unwrap(),
            positions: Arc::clone(&self.positions),
            is_running: Arc::clone(&self.is_running),
            last_signature: Arc::clone(&self.last_signature),
            position_manager: self.position_manager.clone(),
            portfolio_analyzer: self.portfolio_analyzer.clone(),
            console_display: self.console_display.clone(),
        }
    }
}
