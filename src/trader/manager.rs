use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::config::TraderConfig;
use crate::trader::database::TraderDatabase;
use crate::trader::position::Position;
use crate::trader::strategy::TradingStrategy;
use crate::trader::types::*;
use crate::types::TokenInfo;
use crate::marketdata::TokenData;
use crate::swap::SwapManager;
use crate::marketdata::MarketData;
use crate::discovery::Discovery;
use crate::pool::PoolModule;

pub struct TraderManager {
    config: TraderConfig,
    database: Arc<TraderDatabase>,
    strategy: Arc<RwLock<TradingStrategy>>,
    swap_manager: Arc<SwapManager>,
    market_data: Arc<MarketData>,
    discovery: Arc<Discovery>,
    pool_module: Arc<PoolModule>,
    positions: Arc<RwLock<HashMap<String, Position>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<TraderStats>>,
}

impl TraderManager {
    pub fn new(
        config: TraderConfig,
        swap_manager: Arc<SwapManager>,
        market_data: Arc<MarketData>,
        discovery: Arc<Discovery>,
        pool_module: Arc<PoolModule>
    ) -> Result<Self> {
        let database = Arc::new(TraderDatabase::new(&config.database_path)?);
        let strategy = Arc::new(RwLock::new(TradingStrategy::new(config.clone())));
        let positions = Arc::new(RwLock::new(HashMap::new()));
        let running = Arc::new(RwLock::new(false));
        let stats = Arc::new(
            RwLock::new(TraderStats {
                total_trades: 0,
                successful_trades: 0,
                failed_trades: 0,
                total_invested_sol: 0.0,
                total_realized_pnl_sol: 0.0,
                total_unrealized_pnl_sol: 0.0,
                win_rate: 0.0,
                average_trade_size_sol: 0.0,
                largest_win_sol: 0.0,
                largest_loss_sol: 0.0,
                active_positions: 0,
                closed_positions: 0,
            })
        );

        Ok(Self {
            config,
            database,
            strategy,
            swap_manager,
            market_data,
            discovery,
            pool_module,
            positions,
            running,
            stats,
        })
    }

    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        println!("🎯 Trader module starting...");

        // Load existing positions from database
        self.load_existing_positions().await?;

        // Update stats
        self.update_stats().await?;

        // Start background tasks
        self.start_price_monitoring().await;
        self.start_position_monitoring().await;
        self.start_discovery_monitoring().await;

        println!("🎯 Trader module started successfully");
        Ok(())
    }

    pub async fn stop(&self) {
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        println!("🎯 Trader module stopped");
    }

    async fn load_existing_positions(&self) -> Result<()> {
        let active_positions = self.database.get_active_positions()?;
        let mut positions = self.positions.write().await;

        for (id, summary) in active_positions {
            let mut position = Position::from_summary(id, summary);
            position.dca_levels = self.database.get_dca_levels(id)?;
            positions.insert(position.token_address.clone(), position);
        }

        println!("📊 Loaded {} active positions", positions.len());
        Ok(())
    }

    async fn start_price_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let positions = Arc::clone(&self.positions);
        let running = Arc::clone(&self.running);
        let market_data = Arc::clone(&self.market_data);
        let pool_module = Arc::clone(&self.pool_module);

        tokio::spawn(async move {
            // Check price every 5 seconds as requested
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut last_top_tokens_update = std::time::Instant::now();
            let mut top_tokens: Vec<TokenData> = Vec::new();

            println!("🎯 Starting price monitoring for top 20 tokens (5 sec intervals)");

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                // Update top 20 tokens every 60 seconds to avoid overloading
                if
                    last_top_tokens_update.elapsed() > Duration::from_secs(60) ||
                    top_tokens.is_empty()
                {
                    match market_data.get_top_tokens_by_volume(20).await {
                        Ok(tokens) => {
                            top_tokens = tokens;
                            println!("📊 Updated top 20 tokens list ({} tokens)", top_tokens.len());
                            last_top_tokens_update = std::time::Instant::now();
                        }
                        Err(e) => {
                            println!("❌ Error getting top tokens: {}", e);
                            continue;
                        }
                    }
                }

                // Monitor prices for top tokens using pool module
                for token in &top_tokens {
                    // Get previous price from strategy for comparison
                    let previous_price = strategy.read().await.get_current_price(&token.mint);

                    // Get real-time price from pool module
                    match pool_module.get_real_time_price(&token.mint).await {
                        Ok(Some(pool_price)) => {
                            // Update strategy with new price from pool
                            strategy.write().await.update_price(&token.mint, pool_price);

                            // Calculate price change if we have a previous price
                            if let Some(prev_price) = previous_price {
                                if (pool_price - prev_price).abs() > prev_price * 0.001 {
                                    // Show changes > 0.1%
                                    let change_percent =
                                        ((pool_price - prev_price) / prev_price) * 100.0;
                                    let change_indicator = if change_percent > 0.0 {
                                        "📈"
                                    } else {
                                        "📉"
                                    };
                                    println!(
                                        "{} {} ({}): ${:.8} → ${:.8} ({:+.2}%)",
                                        change_indicator,
                                        token.symbol,
                                        &token.mint[..8],
                                        prev_price,
                                        pool_price,
                                        change_percent
                                    );
                                }
                            } else {
                                // First time getting price for this token
                                println!(
                                    "🎯 {} ({}): Initial Price=${:.8} | Volume=${:.2}K",
                                    token.symbol,
                                    &token.mint[..8],
                                    pool_price,
                                    token.volume_24h / 1000.0
                                );
                            }
                        }
                        Ok(None) => {
                            // Fallback to market data price if pool price not available
                            if token.price_usd > 0.0 {
                                let previous_price = strategy
                                    .read().await
                                    .get_current_price(&token.mint);
                                strategy.write().await.update_price(&token.mint, token.price_usd);

                                if previous_price.is_none() {
                                    println!(
                                        "⚠️  {} ({}): Using Market Price=${:.8} (Pool N/A)",
                                        token.symbol,
                                        &token.mint[..8],
                                        token.price_usd
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Error getting pool price for {}: {}", token.symbol, e);
                            // Fallback to market data price
                            if token.price_usd > 0.0 {
                                strategy.write().await.update_price(&token.mint, token.price_usd);
                            }
                        }
                    }
                }

                // Print monitoring summary every 30 seconds
                if last_top_tokens_update.elapsed() > Duration::from_secs(30) {
                    let mut pool_prices = 0;
                    let mut market_prices = 0;

                    for token in &top_tokens {
                        match pool_module.get_real_time_price(&token.mint).await {
                            Ok(Some(_)) => {
                                pool_prices += 1;
                            }
                            _ => {
                                market_prices += 1;
                            }
                        }
                    }

                    println!(
                        "📊 Monitoring Summary: {} tokens | {} pool prices | {} market fallbacks",
                        top_tokens.len(),
                        pool_prices,
                        market_prices
                    );
                    last_top_tokens_update = std::time::Instant::now();
                }

                // Update prices for active positions using pool module
                let positions_clone = {
                    let positions_read = positions.read().await;
                    positions_read.clone()
                };

                for (token_address, _position) in &positions_clone {
                    match pool_module.get_real_time_price(token_address).await {
                        Ok(Some(pool_price)) => {
                            // Update strategy with new price from pool
                            strategy.write().await.update_price(token_address, pool_price);

                            // Update position price
                            {
                                let mut positions_write = positions.write().await;
                                if let Some(pos) = positions_write.get_mut(token_address) {
                                    pos.update_price(pool_price);
                                }
                            }
                        }
                        Ok(None) => {
                            // Fallback to market data if pool price not available
                            if
                                let Ok(Some(token_data)) =
                                    market_data.get_token_data(token_address).await
                            {
                                let current_price = token_data.price_usd;
                                strategy.write().await.update_price(token_address, current_price);

                                let mut positions_write = positions.write().await;
                                if let Some(pos) = positions_write.get_mut(token_address) {
                                    pos.update_price(current_price);
                                }
                            }
                        }
                        Err(e) => {
                            println!(
                                "❌ Error getting pool price for position {}: {}",
                                token_address,
                                e
                            );
                        }
                    }
                }

                if !positions_clone.is_empty() {
                    println!("📊 Updated prices for {} positions", positions_clone.len());
                }
            }
        });
    }

    async fn start_position_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let positions = Arc::clone(&self.positions);
        let running = Arc::clone(&self.running);
        let database = Arc::clone(&self.database);
        let trader_manager = self.clone_for_async();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.position_check_interval_seconds)
            );

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let positions_clone = {
                    let positions_read = positions.read().await;
                    positions_read.clone()
                };

                for (_token_address, position) in positions_clone {
                    let current_price = position.current_price;

                    // Analyze position for signals
                    let signals = strategy.read().await.analyze_position(&position, current_price);

                    for signal in signals {
                        // Record signal in database
                        if let Err(e) = database.record_signal(&signal) {
                            eprintln!("❌ Failed to record signal: {}", e);
                        }

                        // Execute trade based on signal
                        if let Err(e) = trader_manager.execute_trade_signal(&signal).await {
                            eprintln!("❌ Failed to execute trade: {}", e);
                        }
                    }
                }
            }
        });
    }

    async fn start_discovery_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let discovery = Arc::clone(&self.discovery);
        let market_data = Arc::clone(&self.market_data);
        let running = Arc::clone(&self.running);
        let trader_manager = self.clone_for_async();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.price_check_interval_seconds)
            );

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                // Get recent discovered tokens from database
                let discovery_db = discovery.get_database();
                if let Ok(recent_tokens) = discovery_db.get_recent_tokens(1) {
                    for discovered_token in recent_tokens {
                        // Get token data from market data
                        if
                            let Ok(Some(token_data)) = market_data.get_token_data(
                                &discovered_token.mint
                            ).await
                        {
                            let current_price = token_data.price_usd;

                            if current_price > 0.0 {
                                // Convert TokenData to TokenInfo
                                let token_info = Self::token_data_to_token_info(&token_data);

                                // Update strategy with new price
                                strategy
                                    .write().await
                                    .update_price(&discovered_token.mint, current_price);

                                // Check for buy signals
                                if
                                    let Some(signal) = strategy
                                        .read().await
                                        .analyze_token(&token_info, current_price)
                                {
                                    println!(
                                        "📡 New buy signal: {} at ${:.6}",
                                        token_info.symbol,
                                        current_price
                                    );

                                    // Execute buy trade
                                    if
                                        let Err(e) = trader_manager.execute_trade_signal(
                                            &signal
                                        ).await
                                    {
                                        eprintln!("❌ Failed to execute buy trade: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn execute_trade_signal(&self, signal: &TradeSignal) -> Result<()> {
        let position = {
            let positions = self.positions.read().await;
            positions.get(&signal.token_address).cloned()
        };

        match signal.signal_type {
            TradeSignalType::Buy => {
                self.execute_buy_trade(signal, position).await?;
            }
            TradeSignalType::Sell => {
                if let Some(pos) = position {
                    self.execute_sell_trade(signal, pos).await?;
                }
            }
            TradeSignalType::DCA => {
                if let Some(pos) = position {
                    self.execute_dca_trade(signal, pos).await?;
                }
            }
            TradeSignalType::StopLoss => {
                if let Some(pos) = position {
                    self.execute_stop_loss_trade(signal, pos).await?;
                }
            }
        }

        Ok(())
    }

    async fn execute_buy_trade(
        &self,
        signal: &TradeSignal,
        existing_position: Option<Position>
    ) -> Result<()> {
        let trade_size = self.strategy
            .read().await
            .calculate_trade_size(signal, existing_position.as_ref());

        println!(
            "🟢 Executing BUY: {} - ${:.6} (${:.4} SOL)",
            signal.token_address,
            signal.current_price,
            trade_size
        );

        // Execute swap (dry run or real based on config)
        let trade_result = if self.config.dry_run {
            self.simulate_buy_trade(signal, trade_size).await
        } else {
            self.execute_real_buy_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Buy trade executed successfully");
                self.process_buy_trade_result(signal, result, existing_position).await?;
            }
            Err(e) => {
                eprintln!("❌ Buy trade failed: {}", e);
                let failed_result = TradeResult {
                    success: false,
                    transaction_hash: None,
                    amount_sol: trade_size,
                    amount_tokens: 0.0,
                    price_per_token: signal.current_price,
                    fees: 0.0,
                    slippage: 0.0,
                    timestamp: Utc::now(),
                    error: Some(e.to_string()),
                };
                self.process_buy_trade_result(signal, failed_result, existing_position).await?;
            }
        }

        Ok(())
    }

    async fn execute_sell_trade(&self, signal: &TradeSignal, position: Position) -> Result<()> {
        let trade_size = position.total_tokens;

        println!(
            "🔴 Executing SELL: {} - ${:.6} ({:.4} tokens)",
            signal.token_address,
            signal.current_price,
            trade_size
        );

        let trade_result = if self.config.dry_run {
            self.simulate_sell_trade(signal, trade_size).await
        } else {
            self.execute_real_sell_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Sell trade executed successfully");
                self.process_sell_trade_result(signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ Sell trade failed: {}", e);
                let failed_result = TradeResult {
                    success: false,
                    transaction_hash: None,
                    amount_sol: trade_size * signal.current_price,
                    amount_tokens: trade_size,
                    price_per_token: signal.current_price,
                    fees: 0.0,
                    slippage: 0.0,
                    timestamp: Utc::now(),
                    error: Some(e.to_string()),
                };
                self.process_sell_trade_result(signal, failed_result, position).await?;
            }
        }

        Ok(())
    }

    async fn execute_dca_trade(&self, signal: &TradeSignal, position: Position) -> Result<()> {
        let trade_size = self.strategy.read().await.calculate_trade_size(signal, Some(&position));

        println!(
            "🟡 Executing DCA: {} - ${:.6} (${:.4} SOL, Level {})",
            signal.token_address,
            signal.current_price,
            trade_size,
            position.dca_level + 1
        );

        let trade_result = if self.config.dry_run {
            self.simulate_buy_trade(signal, trade_size).await
        } else {
            self.execute_real_buy_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ DCA trade executed successfully");
                self.process_dca_trade_result(signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ DCA trade failed: {}", e);
            }
        }

        Ok(())
    }

    async fn execute_stop_loss_trade(
        &self,
        signal: &TradeSignal,
        position: Position
    ) -> Result<()> {
        let trade_size = position.total_tokens;

        println!(
            "🛑 Executing STOP LOSS: {} - ${:.6} ({:.4} tokens)",
            signal.token_address,
            signal.current_price,
            trade_size
        );

        let trade_result = if self.config.dry_run {
            self.simulate_sell_trade(signal, trade_size).await
        } else {
            self.execute_real_sell_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Stop loss executed successfully");
                self.process_stop_loss_trade_result(signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ Stop loss failed: {}", e);
            }
        }

        Ok(())
    }

    async fn simulate_buy_trade(
        &self,
        signal: &TradeSignal,
        amount_sol: f64
    ) -> Result<TradeResult> {
        // Simulate slippage and fees
        let slippage = 0.01; // 1% slippage
        let fees = amount_sol * 0.0025; // 0.25% fees
        let effective_amount = amount_sol - fees;
        let effective_price = signal.current_price * (1.0 + slippage);
        let tokens_received = effective_amount / effective_price;

        Ok(TradeResult {
            success: true,
            transaction_hash: Some(format!("SIMULATED_BUY_{}", Utc::now().timestamp())),
            amount_sol,
            amount_tokens: tokens_received,
            price_per_token: effective_price,
            fees,
            slippage,
            timestamp: Utc::now(),
            error: None,
        })
    }

    async fn simulate_sell_trade(
        &self,
        signal: &TradeSignal,
        amount_tokens: f64
    ) -> Result<TradeResult> {
        // Simulate slippage and fees
        let slippage = 0.01; // 1% slippage
        let effective_price = signal.current_price * (1.0 - slippage);
        let sol_received = amount_tokens * effective_price;
        let fees = sol_received * 0.0025; // 0.25% fees
        let net_sol = sol_received - fees;

        Ok(TradeResult {
            success: true,
            transaction_hash: Some(format!("SIMULATED_SELL_{}", Utc::now().timestamp())),
            amount_sol: net_sol,
            amount_tokens,
            price_per_token: effective_price,
            fees,
            slippage,
            timestamp: Utc::now(),
            error: None,
        })
    }

    async fn execute_real_buy_trade(
        &self,
        _signal: &TradeSignal,
        _amount_sol: f64
    ) -> Result<TradeResult> {
        // Use swap manager to execute real trade
        // This is a placeholder - implement actual swap logic
        Err(anyhow::anyhow!("Real trading not implemented yet"))
    }

    async fn execute_real_sell_trade(
        &self,
        _signal: &TradeSignal,
        _amount_tokens: f64
    ) -> Result<TradeResult> {
        // Use swap manager to execute real trade
        // This is a placeholder - implement actual swap logic
        Err(anyhow::anyhow!("Real trading not implemented yet"))
    }

    async fn process_buy_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        existing_position: Option<Position>
    ) -> Result<()> {
        let mut position = existing_position.unwrap_or_else(|| {
            let mut pos = Position::new(signal.token_address.clone(), "Unknown".to_string());
            pos.initialize_dca_levels(&self.config);
            pos
        });

        if result.success {
            position.add_buy_trade(result.amount_sol, result.amount_tokens, result.price_per_token);
        }

        // Save position to database
        let position_id = position.save_to_database(&self.database)?;
        position.id = Some(position_id);

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "BUY", &result)?;

        // Update positions map
        {
            let mut positions = self.positions.write().await;
            positions.insert(signal.token_address.clone(), position);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_sell_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_sell_trade(
                result.amount_sol,
                result.amount_tokens,
                result.price_per_token
            );
            position.status = PositionStatus::Closed;
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "SELL", &result)?;

        // Remove from active positions if closed
        if matches!(position.status, PositionStatus::Closed) {
            let mut positions = self.positions.write().await;
            positions.remove(&signal.token_address);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_dca_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_buy_trade(result.amount_sol, result.amount_tokens, result.price_per_token);

            // Mark DCA level as executed
            let next_level = position.dca_level + 1;
            position.execute_dca_level(next_level, result.price_per_token)?;

            // Update DCA level in database
            if let Some(position_id) = position.id {
                self.database.update_dca_level(position_id, next_level, result.price_per_token)?;
            }
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "DCA", &result)?;

        // Update positions map
        {
            let mut positions = self.positions.write().await;
            positions.insert(signal.token_address.clone(), position);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_stop_loss_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_sell_trade(
                result.amount_sol,
                result.amount_tokens,
                result.price_per_token
            );
            position.status = PositionStatus::StopLoss;
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "STOP_LOSS", &result)?;

        // Remove from active positions
        {
            let mut positions = self.positions.write().await;
            positions.remove(&signal.token_address);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn update_stats(&self) -> Result<()> {
        let db_stats = self.database.get_trader_stats()?;
        let mut stats = self.stats.write().await;
        *stats = db_stats;
        Ok(())
    }

    pub async fn get_stats(&self) -> TraderStats {
        self.stats.read().await.clone()
    }

    pub async fn get_positions(&self) -> Vec<PositionSummary> {
        let positions = self.positions.read().await;
        positions
            .values()
            .map(|p| p.to_summary())
            .collect()
    }

    pub async fn get_position(&self, token_address: &str) -> Option<PositionSummary> {
        let positions = self.positions.read().await;
        positions.get(token_address).map(|p| p.to_summary())
    }

    // Helper function to convert TokenData to TokenInfo
    fn token_data_to_token_info(token_data: &TokenData) -> TokenInfo {
        TokenInfo {
            mint: token_data.mint.clone(),
            symbol: token_data.symbol.clone(),
            name: token_data.name.clone(),
            decimals: token_data.decimals,
            supply: token_data.total_supply as u64,
            market_cap: Some(token_data.market_cap),
            price: Some(token_data.price_usd),
            volume_24h: Some(token_data.volume_24h),
            liquidity: Some(token_data.liquidity_usd),
            pool_address: token_data.top_pool_address.clone(),
            discovered_at: token_data.last_updated,
            last_updated: token_data.last_updated,
            is_active: true,
        }
    }

    fn clone_for_async(&self) -> TraderManagerAsync {
        TraderManagerAsync {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            strategy: Arc::clone(&self.strategy),
            swap_manager: Arc::clone(&self.swap_manager),
            market_data: Arc::clone(&self.market_data),
            discovery: Arc::clone(&self.discovery),
            pool_module: Arc::clone(&self.pool_module),
            positions: Arc::clone(&self.positions),
            running: Arc::clone(&self.running),
            stats: Arc::clone(&self.stats),
        }
    }
}

// Helper struct for async operations
#[derive(Clone)]
struct TraderManagerAsync {
    config: TraderConfig,
    database: Arc<TraderDatabase>,
    strategy: Arc<RwLock<TradingStrategy>>,
    swap_manager: Arc<SwapManager>,
    market_data: Arc<MarketData>,
    discovery: Arc<Discovery>,
    pool_module: Arc<PoolModule>,
    positions: Arc<RwLock<HashMap<String, Position>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<TraderStats>>,
}

impl TraderManagerAsync {
    async fn execute_trade_signal(&self, signal: &TradeSignal) -> Result<()> {
        // Create a temporary TraderManager with the same data
        let temp_manager = TraderManager {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            strategy: Arc::clone(&self.strategy),
            swap_manager: Arc::clone(&self.swap_manager),
            market_data: Arc::clone(&self.market_data),
            discovery: Arc::clone(&self.discovery),
            pool_module: Arc::clone(&self.pool_module),
            positions: Arc::clone(&self.positions),
            running: Arc::clone(&self.running),
            stats: Arc::clone(&self.stats),
        };

        temp_manager.execute_trade_signal(signal).await
    }
}
