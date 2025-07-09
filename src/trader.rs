#![allow(warnings)]
use crate::prelude::*;
use crate::web_server::MARKET_DATAFRAMES;
use std::collections::VecDeque;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use rayon::prelude::*;
use anyhow::anyhow;

// GeckoTerminal API response structures
#[derive(Debug, Deserialize)]
struct GeckoTerminalResponse {
    data: GeckoTerminalData,
    meta: GeckoTerminalMeta,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalData {
    attributes: GeckoTerminalAttributes,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalAttributes {
    ohlcv_list: Vec<[f64; 6]>, // [timestamp, open, high, low, close, volume]
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalMeta {
    base: TokenInfo,
    quote: TokenInfo,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    address: String,
    name: String,
    symbol: String,
}

// Cache structure for storing OHLCV data on disk with multiple timeframes
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedOHLCV {
    pool_address: String,
    timestamp_cached: u64,
    base_token: String,
    quote_token: String,
    // Store data for all timeframes
    minute_data: Vec<[f64; 6]>,
    hour_data: Vec<[f64; 6]>,
    day_data: Vec<[f64; 6]>,
}

// Constants for caching
const CACHE_DIR: &str = ".ohlcv_cache";
const CACHE_DURATION_HOURS: u64 = 6; // Cache data for 6 hours
const MAX_OHLCV_LIMIT: usize = 1000; // Maximum data points to fetch
const DEFAULT_OHLCV_LIMIT: usize = 200; // Default amount of historical data

// Constants for pre-trade data validation
const MIN_MINUTE_DATA_POINTS: usize = 50; // Minimum minute data for trading
const MIN_HOUR_DATA_POINTS: usize = 24; // Minimum hour data for trading
const MIN_DAY_DATA_POINTS: usize = 7; // Minimum day data for trading
const MIN_LEGACY_DATA_POINTS: usize = 32; // Minimum legacy price data for trading

// Timeframe enum for different OHLCV intervals
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timeframe {
    Minute,
    Hour,
    Day,
}

impl Timeframe {
    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::Minute => "minute",
            Timeframe::Hour => "hour",
            Timeframe::Day => "day",
        }
    }

    pub fn aggregate_value(&self) -> u32 {
        match self {
            Timeframe::Minute => 1, // 1 minute
            Timeframe::Hour => 1, // 1 hour
            Timeframe::Day => 1, // 1 day
        }
    }
}

// OHLCV data for a specific timeframe
#[derive(Debug, Clone)]
pub struct TimeframeData {
    pub timestamps: VecDeque<u64>,
    pub opens: VecDeque<f64>,
    pub highs: VecDeque<f64>,
    pub lows: VecDeque<f64>,
    pub closes: VecDeque<f64>,
    pub volumes: VecDeque<f64>,
}

impl TimeframeData {
    pub fn new() -> Self {
        Self {
            timestamps: VecDeque::new(),
            opens: VecDeque::new(),
            highs: VecDeque::new(),
            lows: VecDeque::new(),
            closes: VecDeque::new(),
            volumes: VecDeque::new(),
        }
    }

    pub fn add_ohlcv(
        &mut self,
        timestamp: u64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64
    ) {
        self.timestamps.push_back(timestamp);
        self.opens.push_back(open);
        self.highs.push_back(high);
        self.lows.push_back(low);
        self.closes.push_back(close);
        self.volumes.push_back(volume);

        // Keep reasonable limits for each timeframe
        let max_size = 1000; // Configurable based on needs
        if self.timestamps.len() > max_size {
            self.timestamps.pop_front();
            self.opens.pop_front();
            self.highs.pop_front();
            self.lows.pop_front();
            self.closes.pop_front();
            self.volumes.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }

    pub fn latest_close(&self) -> Option<f64> {
        self.closes.back().copied()
    }
}

// Market data structure with multiple timeframes
#[derive(Debug, Clone)]
pub struct MarketDataFrame {
    pub pool_address: String,
    pub base_token: String,
    pub quote_token: String,
    pub minute_data: TimeframeData,
    pub hour_data: TimeframeData,
    pub day_data: TimeframeData,
    pub last_updated: u64,
    // Legacy fields for backward compatibility
    pub prices: VecDeque<f64>,
    pub volumes: VecDeque<f64>,
    pub timestamps: VecDeque<u64>,
    pub highs: VecDeque<f64>,
    pub lows: VecDeque<f64>,
    pub opens: VecDeque<f64>,
    pub closes: VecDeque<f64>,
}

impl MarketDataFrame {
    pub fn new() -> Self {
        Self {
            pool_address: String::new(),
            base_token: String::new(),
            quote_token: String::new(),
            minute_data: TimeframeData::new(),
            hour_data: TimeframeData::new(),
            day_data: TimeframeData::new(),
            last_updated: 0,
            // Legacy fields for backward compatibility
            prices: VecDeque::new(),
            volumes: VecDeque::new(),
            timestamps: VecDeque::new(),
            highs: VecDeque::new(),
            lows: VecDeque::new(),
            opens: VecDeque::new(),
            closes: VecDeque::new(),
        }
    }

    pub fn new_with_pool_info(
        pool_address: String,
        base_token: String,
        quote_token: String
    ) -> Self {
        Self {
            pool_address,
            base_token,
            quote_token,
            minute_data: TimeframeData::new(),
            hour_data: TimeframeData::new(),
            day_data: TimeframeData::new(),
            last_updated: 0,
            prices: VecDeque::new(),
            volumes: VecDeque::new(),
            timestamps: VecDeque::new(),
            highs: VecDeque::new(),
            lows: VecDeque::new(),
            opens: VecDeque::new(),
            closes: VecDeque::new(),
        }
    }

    // Get timeframe data by enum
    pub fn get_timeframe_data(&self, timeframe: Timeframe) -> &TimeframeData {
        match timeframe {
            Timeframe::Minute => &self.minute_data,
            Timeframe::Hour => &self.hour_data,
            Timeframe::Day => &self.day_data,
        }
    }

    pub fn get_timeframe_data_mut(&mut self, timeframe: Timeframe) -> &mut TimeframeData {
        match timeframe {
            Timeframe::Minute => &mut self.minute_data,
            Timeframe::Hour => &mut self.hour_data,
            Timeframe::Day => &mut self.day_data,
        }
    }

    // Load OHLCV data from GeckoTerminal API with caching
    pub async fn load_historical_data(
        &mut self,
        pool_address: &str,
        mint: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Try to load from cache first
        if let Ok(cached_data) = load_cached_ohlcv(pool_address).await {
            if is_cache_valid(&cached_data) {
                println!("📋 [CACHE] Using cached OHLCV data for pool {}", pool_address);
                self.load_from_cached_data(&cached_data);
                return Ok(());
            }
        }

        // Fetch fresh data from GeckoTerminal API
        println!("🌐 [GECKO] Fetching fresh OHLCV data for pool {}", pool_address);

        // Fetch all timeframes
        let minute_data = fetch_gecko_ohlcv(
            pool_address,
            Timeframe::Minute,
            DEFAULT_OHLCV_LIMIT
        ).await?;
        let hour_data = fetch_gecko_ohlcv(
            pool_address,
            Timeframe::Hour,
            DEFAULT_OHLCV_LIMIT
        ).await?;
        let day_data = fetch_gecko_ohlcv(pool_address, Timeframe::Day, 30).await?; // 30 days should be enough

        // Load data into timeframes
        self.load_timeframe_data(Timeframe::Minute, &minute_data.data.attributes.ohlcv_list);
        self.load_timeframe_data(Timeframe::Hour, &hour_data.data.attributes.ohlcv_list);
        self.load_timeframe_data(Timeframe::Day, &day_data.data.attributes.ohlcv_list);

        // Update pool info
        self.pool_address = pool_address.to_string();
        self.base_token = minute_data.meta.base.symbol;
        self.quote_token = minute_data.meta.quote.symbol;
        self.last_updated = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Cache the data with all timeframes
        let cache_data = CachedOHLCV {
            pool_address: pool_address.to_string(),
            timestamp_cached: self.last_updated,
            base_token: self.base_token.clone(),
            quote_token: self.quote_token.clone(),
            minute_data: minute_data.data.attributes.ohlcv_list,
            hour_data: hour_data.data.attributes.ohlcv_list,
            day_data: day_data.data.attributes.ohlcv_list,
        };
        save_cached_ohlcv(&cache_data).await?;

        // Update legacy price data with minute data for backward compatibility
        self.update_legacy_data();

        Ok(())
    }

    fn load_timeframe_data(&mut self, timeframe: Timeframe, ohlcv_list: &[[f64; 6]]) {
        let timeframe_data = self.get_timeframe_data_mut(timeframe);

        for ohlcv in ohlcv_list {
            let [timestamp, open, high, low, close, volume] = *ohlcv;
            timeframe_data.add_ohlcv(timestamp as u64, open, high, low, close, volume);
        }
    }

    fn load_from_cached_data(&mut self, cached: &CachedOHLCV) {
        self.pool_address = cached.pool_address.clone();
        self.base_token = cached.base_token.clone();
        self.quote_token = cached.quote_token.clone();
        self.last_updated = cached.timestamp_cached;

        // Load cached data for all timeframes
        self.load_timeframe_data(Timeframe::Minute, &cached.minute_data);
        self.load_timeframe_data(Timeframe::Hour, &cached.hour_data);
        self.load_timeframe_data(Timeframe::Day, &cached.day_data);
        self.update_legacy_data();
    }

    // Update legacy fields for backward compatibility
    pub fn update_legacy_data(&mut self) {
        // Use minute data for legacy compatibility
        self.prices.clear();
        self.volumes.clear();
        self.timestamps.clear();
        self.opens.clear();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();

        for i in 0..self.minute_data.len() {
            if
                let (
                    Some(&timestamp),
                    Some(&open),
                    Some(&high),
                    Some(&low),
                    Some(&close),
                    Some(&volume),
                ) = (
                    self.minute_data.timestamps.get(i),
                    self.minute_data.opens.get(i),
                    self.minute_data.highs.get(i),
                    self.minute_data.lows.get(i),
                    self.minute_data.closes.get(i),
                    self.minute_data.volumes.get(i),
                )
            {
                self.timestamps.push_back(timestamp);
                self.opens.push_back(open);
                self.highs.push_back(high);
                self.lows.push_back(low);
                self.closes.push_back(close);
                self.volumes.push_back(volume);
                self.prices.push_back(close); // Use close as price for legacy compatibility
            }
        }
    }

    pub fn add_price_data(&mut self, price: f64, volume: f64, timestamp: u64) {
        self.prices.push_back(price);
        self.volumes.push_back(volume);
        self.timestamps.push_back(timestamp);
        // For simplicity, use current price as OHLC for now
        self.opens.push_back(price);
        self.highs.push_back(price);
        self.lows.push_back(price);
        self.closes.push_back(price);

        // Keep only last N entries
        const MAX_SIZE: usize = 100;
        if self.prices.len() > MAX_SIZE {
            self.prices.pop_front();
            self.volumes.pop_front();
            self.timestamps.pop_front();
            self.highs.pop_front();
            self.lows.pop_front();
            self.opens.pop_front();
            self.closes.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    pub fn latest_price(&self) -> Option<f64> {
        self.prices.back().copied()
    }

    pub fn price_history(&self) -> &VecDeque<f64> {
        &self.prices
    }

    // Check if dataframe has sufficient data for reliable trading decisions
    pub fn has_sufficient_data_for_trading(&self) -> (bool, String) {
        let minute_count = self.minute_data.len();
        let hour_count = self.hour_data.len();
        let day_count = self.day_data.len();
        let legacy_count = self.prices.len();

        // Check each timeframe
        let minute_ok = minute_count >= MIN_MINUTE_DATA_POINTS;
        let hour_ok = hour_count >= MIN_HOUR_DATA_POINTS;
        let day_ok = day_count >= MIN_DAY_DATA_POINTS;
        let legacy_ok = legacy_count >= MIN_LEGACY_DATA_POINTS;

        // Require at least minute data and legacy data to be sufficient
        let has_minimum = minute_ok && legacy_ok;

        // Create detailed status message
        let status = format!(
            "Data status: Minute({}/{}){} Hour({}/{}){} Day({}/{}){} Legacy({}/{}){}",
            minute_count,
            MIN_MINUTE_DATA_POINTS,
            if minute_ok {
                "✅"
            } else {
                "❌"
            },
            hour_count,
            MIN_HOUR_DATA_POINTS,
            if hour_ok {
                "✅"
            } else {
                "❌"
            },
            day_count,
            MIN_DAY_DATA_POINTS,
            if day_ok {
                "✅"
            } else {
                "❌"
            },
            legacy_count,
            MIN_LEGACY_DATA_POINTS,
            if legacy_ok {
                "✅"
            } else {
                "❌"
            }
        );

        (has_minimum, status)
    }

    // Check if we need to load more historical data
    pub fn needs_historical_data_loading(&self) -> bool {
        self.minute_data.len() < MIN_MINUTE_DATA_POINTS ||
            self.hour_data.len() < MIN_HOUR_DATA_POINTS ||
            self.day_data.len() < MIN_DAY_DATA_POINTS
    }
}

/// supervisor that restarts the trader loop on *any* panic
pub fn start_trader_loop() {
    println!("🚀 [Screener] Trader loop started!");

    // ── supervisor task ──────────────────────────────────────────────────────
    task::spawn(async move {
        use std::panic::AssertUnwindSafe;

        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }

            // run the heavy async logic and trap panics
            let run = AssertUnwindSafe(trader_main_loop()).catch_unwind().await;

            match run {
                Ok(_) => {
                    break;
                } // exited via SHUTDOWN
                Err(e) => {
                    eprintln!("❌ Trader loop panicked: {e:?} — restarting in 1 s");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    task::spawn(async {
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }
            print_open_positions().await;
            sleep(Duration::from_secs(POSITIONS_PRINT_TIME)).await;
        }
    });
}

async fn trader_main_loop() {
    use std::time::Instant;
    println!("🔥 Entered MAIN TRADER LOOP TASK");

    /* ── wait for TOKENS ──────────────────────────────── */
    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            return;
        }
        if !TOKENS.read().await.is_empty() {
            break;
        }
        println!("⏳ Waiting for TOKENS to be loaded …");
        sleep(Duration::from_secs(1)).await;
    }
    println!("✅ TOKENS loaded! Proceeding with trader loop.");

    /* ── local state ──────────────────────────────────── */
    let mut notified_profit_bucket: HashMap<String, i32> = HashMap::new();
    let mut sell_failures: HashMap<String, u8> = HashMap::new(); // mint -> fails

    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            return;
        }

        /* ── build mint list ──────────────────────────── */
        let mut all_mints: Vec<String> = {
            let t = TOKENS.read().await;
            t.iter()
                .map(|tok| tok.mint.clone())
                .collect()
        };

        // Add open positions to mint list
        let open_position_mints: Vec<String> = {
            let pos = OPEN_POSITIONS.read().await;
            pos.keys().cloned().collect()
        };

        for mint in &open_position_mints {
            if !all_mints.contains(mint) {
                all_mints.push(mint.clone());
            }
        }

        // Remove blacklisted mints
        let filtered_mints: Vec<String> = {
            let blacklist = BLACKLIST.read().await;
            all_mints
                .into_iter()
                .filter(|mint| !blacklist.contains(mint))
                .collect()
        };

        if filtered_mints.is_empty() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Check if trading is blocked due to pending transactions
        if TransactionManager::is_trading_blocked().await {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        /* ── BATCH PRICE FETCHING (saves RPC costs!) ─────────── */
        println!("🔄 [BATCH] Starting price update cycle for {} tokens...", filtered_mints.len());
        let cycle_start = Instant::now();

        let prices = tokio::task
            ::spawn_blocking({
                let mints = filtered_mints.clone();
                move || batch_prices_from_pools(&crate::configs::RPC, &mints)
            }).await
            .unwrap_or_else(|e| {
                eprintln!("❌ Batch price fetch panicked: {}", e);
                HashMap::new()
            });

        let successful_prices = prices.len();
        let failed_prices = filtered_mints.len() - successful_prices;

        if successful_prices > 0 {
            println!(
                "✅ [BATCH] Price cycle completed in {} ms - Success: {}/{} - Failed: {}",
                cycle_start.elapsed().as_millis(),
                successful_prices,
                filtered_mints.len(),
                failed_prices
            );
        } else {
            eprintln!(
                "❌ [BATCH] No prices fetched successfully, falling back to individual fetches"
            );
        }

        /* ── iterate mints and process with fetched prices ──────── */
        for mint in filtered_mints {
            if SHUTDOWN.load(Ordering::SeqCst) {
                return;
            }

            // Get price from batch results or fallback to individual fetch
            let current_price = if let Some(&price) = prices.get(&mint) {
                price
            } else {
                // Fallback to individual fetch for failed batches
                let symbol = TOKENS.read().await
                    .iter()
                    .find(|t| t.mint == mint)
                    .map(|t| t.symbol.clone())
                    .unwrap_or_else(|| mint.chars().take(4).collect());

                match
                    tokio::task::spawn_blocking({
                        let m = mint.clone();
                        move || price_from_biggest_pool(&crate::configs::RPC, &m)
                    }).await
                {
                    Ok(Ok(p)) if p > 0.0 => {
                        println!("🔄 [FALLBACK] Individual fetch for {}: {:.12} SOL", symbol, p);
                        p
                    }
                    Ok(Err(e)) => {
                        eprintln!("❌ [FALLBACK] Price error for {}: {}", symbol, e);
                        if
                            e.to_string().contains("no valid pools") ||
                            e.to_string().contains("Unsupported program id") ||
                            e.to_string().contains("is not an SPL-Token mint") ||
                            e.to_string().contains("AccountNotFound") ||
                            e.to_string().contains("base reserve is zero")
                        {
                            println!("⚠️ Blacklisting mint: {}", mint);
                            crate::configs::add_to_blacklist(&mint).await;
                        }
                        continue;
                    }
                    _ => {
                        eprintln!("❌ [FALLBACK] Failed to fetch price for {}", mint);
                        continue;
                    }
                }
            };

            /* ── symbol string & token lookup ────────────────────────── */
            let (symbol, token) = {
                let tokens = TOKENS.read().await;
                if let Some(t) = tokens.iter().find(|t| t.mint == mint) {
                    (t.symbol.clone(), t.clone())
                } else {
                    // Fallback if token not found in TOKENS list
                    let symbol = mint.chars().take(4).collect();
                    (
                        symbol,
                        Token {
                            mint: mint.clone(),
                            symbol: mint.chars().take(4).collect(),
                            name: "Unknown".to_string(),
                            balance: "0".to_string(),
                            ata_pubkey: "".to_string(),
                            program_id: "".to_string(),
                            dex_id: "".to_string(),
                            url: "".to_string(),
                            pair_address: "".to_string(),
                            labels: Vec::new(),
                            quote_address: "".to_string(),
                            quote_name: "".to_string(),
                            quote_symbol: "".to_string(),
                            price_native: "0".to_string(),
                            price_usd: "0".to_string(),
                            last_price_usd: "0".to_string(),
                            volume_usd: "0".to_string(),
                            fdv_usd: "0".to_string(),
                            image_url: "".to_string(),
                            txns: Txns {
                                m5: TxnCount { buys: 0, sells: 0 },
                                h1: TxnCount { buys: 0, sells: 0 },
                                h6: TxnCount { buys: 0, sells: 0 },
                                h24: TxnCount { buys: 0, sells: 0 },
                            },
                            volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                            price_change: PriceChange { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                            liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                            pair_created_at: 0,
                            rug_check: RugCheckData::default(),
                        },
                    )
                }
            };

            // Get or create dataframe for this token
            let mut dataframe = {
                let dataframes = MARKET_DATAFRAMES.read().await;
                dataframes.get(&mint).cloned().unwrap_or_else(MarketDataFrame::new)
            };

            // Try to load historical data if we don't have enough data
            if dataframe.minute_data.len() < 50 {
                // Try to find pool address for this mint and load historical data
                if let Some(pool_address) = get_pool_address_for_mint(&mint).await {
                    if let Err(e) = dataframe.load_historical_data(&pool_address, &mint).await {
                        eprintln!("⚠️ Failed to load historical data for {}: {}", mint, e);
                    }
                }
            }

            let current_timestamp = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Estimate volume from token data (convert to numerical value)
            let estimated_volume = token.volume.h1; // Use hourly volume as proxy
            dataframe.add_price_data(current_price, estimated_volume, current_timestamp);

            // Update the global dataframes storage
            {
                let mut dataframes = MARKET_DATAFRAMES.write().await;
                dataframes.insert(mint.clone(), dataframe.clone());
            }

            let now = Instant::now();

            // -- Check open position state for this token
            let open_positions = OPEN_POSITIONS.read().await;
            let open = open_positions.contains_key(&mint);
            let can_open_more = open_positions.len() < MAX_OPEN_POSITIONS;
            drop(open_positions);

            // Check if we have sufficient data for trading decisions
            let (has_sufficient_data, data_status) = dataframe.has_sufficient_data_for_trading();

            if !has_sufficient_data {
                println!("⚠️ [DATA] Insufficient data for trading {}: {}", symbol, data_status);
                continue; // Skip this token until we have sufficient data
            }

            // Check if we should buy
            let buy_signal = should_buy(&dataframe, &token, !open && can_open_more, current_price);

            if buy_signal {
                println!(
                    "🚀 ENTRY BUY {}: [scalping drop] histlen={} price={:.9}",
                    symbol,
                    dataframe.len(),
                    current_price
                );
                let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;

                // Create position before transaction
                let bought = TRADE_SIZE_SOL / current_price;
                let new_position = Position {
                    entry_price: current_price,
                    peak_price: current_price,
                    dca_count: 1,
                    token_amount: bought,
                    sol_spent: TRADE_SIZE_SOL + TRANSACTION_FEE_SOL,
                    sol_received: 0.0,
                    open_time: Utc::now(),
                    close_time: None,
                    last_dca_price: current_price,
                };

                match buy_gmgn(&mint, lamports).await {
                    Ok(tx) => {
                        println!("✅ BUY success: {tx}");

                        // Create pending transaction record
                        let pending_tx = PendingTransaction::new(
                            tx.clone(),
                            TransactionType::Buy,
                            mint.clone(),
                            symbol.clone(),
                            TRADE_SIZE_SOL,
                            bought,
                            current_price,
                            None // No position data needed for new buy
                        );

                        // Add to pending transactions
                        if
                            let Err(e) =
                                TransactionManager::add_pending_transaction(pending_tx).await
                        {
                            println!("❌ Failed to add pending transaction: {}", e);
                        }

                        // Add position to open positions
                        OPEN_POSITIONS.write().await.insert(mint.clone(), new_position);
                        save_open().await;
                    }
                    Err(e) => {
                        println!("❌ BUY failed: {}", e);
                    }
                }
            }

            /* ---------- DCA & trailing stop ---------- */
            let pos_opt = {
                let guard = OPEN_POSITIONS.read().await; // read-lock
                guard.get(&mint).cloned() // clone the Position, no &refs
            };

            // ───────── DCA + TRAILING (single block) ───────────────────────────────
            if let Some(mut pos) = pos_opt {
                // Check if we have sufficient data for trading decisions
                let (has_sufficient_data, data_status) =
                    dataframe.has_sufficient_data_for_trading();

                if !has_sufficient_data {
                    println!(
                        "⚠️ [DATA] Insufficient data for DCA/sell decision {}: {}",
                        symbol,
                        data_status
                    );
                    continue; // Skip this token until we have sufficient data
                }

                // Check if we should DCA
                let dca_signal = should_dca(&dataframe, &token, &pos, current_price);

                if dca_signal {
                    let sol_size =
                        TRADE_SIZE_SOL * (1.0 + (pos.dca_count as f64) * DCA_SIZE_FACTOR);
                    let lamports = (sol_size * 1_000_000_000.0) as u64;
                    // Use consistent profit calculation for DCA trigger
                    let current_value = current_price * pos.token_amount;
                    let profit_sol = current_value - pos.sol_spent;
                    let drop_pct = if pos.sol_spent > 0.0 {
                        (profit_sol / pos.sol_spent) * 100.0
                    } else {
                        0.0
                    };

                    // Store original position before DCA
                    let original_position = pos.clone();

                    match buy_gmgn(&mint, lamports).await {
                        Ok(tx) => {
                            let added = sol_size / current_price;
                            pos.token_amount += added;
                            pos.sol_spent += sol_size + TRANSACTION_FEE_SOL;
                            pos.dca_count += 1;
                            pos.entry_price = pos.sol_spent / pos.token_amount;
                            pos.last_dca_price = current_price;

                            // Create pending transaction record
                            let pending_tx = PendingTransaction::new(
                                tx.clone(),
                                TransactionType::DCA,
                                mint.clone(),
                                symbol.clone(),
                                sol_size,
                                added,
                                current_price,
                                Some(create_position_data(&original_position)) // Store original position for recovery
                            );

                            // Add to pending transactions
                            if
                                let Err(e) =
                                    TransactionManager::add_pending_transaction(pending_tx).await
                            {
                                println!("❌ Failed to add pending DCA transaction: {}", e);
                            }

                            OPEN_POSITIONS.write().await.insert(mint.clone(), pos.clone());
                            save_open().await;

                            println!(
                                "🟢 DCA #{:02} {} @ {:.9} (∆{:.2}%) | {tx}",
                                pos.dca_count,
                                symbol,
                                current_price,
                                drop_pct
                            );
                        }
                        Err(e) => {
                            println!("❌ DCA failed: {}", e);
                        }
                    }
                }

                /* ——— peak update & milestone log ——— */
                if current_price > pos.peak_price {
                    if let Some(p) = OPEN_POSITIONS.write().await.get_mut(&mint) {
                        p.peak_price = current_price;
                    }
                    // Use consistent profit calculation method
                    let current_value = current_price * pos.token_amount;
                    let profit_sol = current_value - pos.sol_spent;
                    let profit_now = if pos.sol_spent > 0.0 {
                        (profit_sol / pos.sol_spent) * 100.0
                    } else {
                        0.0
                    };
                    let bucket = (profit_now / 2.0).floor() as i32; // announce every +2 %

                    if bucket > *notified_profit_bucket.get(&mint).unwrap_or(&-1) {
                        notified_profit_bucket.insert(mint.clone(), bucket);
                        println!(
                            "📈 {} new peak {:.2}% (price {:.9})",
                            symbol,
                            profit_now,
                            current_price
                        );
                    }
                }

                // Check if we should sell
                let (should_sell_signal, sell_reason) = should_sell(
                    &dataframe,
                    &token,
                    &pos,
                    current_price
                );

                if should_sell_signal {
                    // Check if sell for this mint is permanently blacklisted
                    {
                        let set = SKIPPED_SELLS.lock().await;

                        if set.contains(&mint) {
                            println!("⛔️ [SKIPPED_SELLS] Not selling {} because it's blacklisted after 10 fails.", mint);
                            OPEN_POSITIONS.write().await.remove(&mint);
                            notified_profit_bucket.remove(&mint);
                            continue;
                        }
                    }

                    match sell_all_gmgn(&mint, current_price).await {
                        Ok(tx) => {
                            // Use consistent profit calculation method
                            let current_value = current_price * pos.token_amount;
                            let profit_sol = current_value - pos.sol_spent;
                            let profit_pct = if pos.sol_spent > 0.0 {
                                (profit_sol / pos.sol_spent) * 100.0
                            } else {
                                0.0
                            };
                            let drop_from_peak =
                                ((current_price - pos.peak_price) / pos.peak_price) * 100.0;

                            println!(
                                "{} SELL {} at {:.2}% | {} | {tx}",
                                if sell_reason == "stop_loss" {
                                    "⛔️ [STOP LOSS]"
                                } else {
                                    "🔴"
                                },
                                symbol,
                                profit_pct,
                                sell_reason
                            );

                            // Create pending transaction record
                            let pending_tx = PendingTransaction::new(
                                tx.clone(),
                                TransactionType::Sell,
                                mint.clone(),
                                symbol.clone(),
                                profit_sol,
                                pos.token_amount,
                                current_price,
                                Some(create_position_data(&pos)) // Store position for recovery
                            );

                            // Add to pending transactions
                            if
                                let Err(e) =
                                    TransactionManager::add_pending_transaction(pending_tx).await
                            {
                                println!("❌ Failed to add pending sell transaction: {}", e);
                            }

                            // Process sell
                            sell_token(
                                &symbol,
                                &mint,
                                current_price,
                                pos.entry_price,
                                pos.peak_price,
                                drop_from_peak,
                                pos.sol_spent,
                                pos.token_amount,
                                pos.dca_count,
                                pos.last_dca_price,
                                pos.open_time
                            ).await;

                            // Remove position (will be restored if transaction fails)
                            OPEN_POSITIONS.write().await.remove(&mint);
                            notified_profit_bucket.remove(&mint);
                            save_open().await;
                        }
                        Err(e) => {
                            let fails = sell_failures.entry(mint.clone()).or_default();
                            *fails += 1;
                            println!("❌ Sell failed for {} (fail {}/10): {e}", mint, *fails);
                            if *fails >= 10 {
                                add_skipped_sell(&mint);
                                println!("⛔️ [SKIPPED_SELLS] Added {} to skipped sells after 10 fails.", mint);
                                OPEN_POSITIONS.write().await.remove(&mint);
                                notified_profit_bucket.remove(&mint);
                                save_open().await;
                            }
                        }
                    }
                    continue;
                }
            }
        } // end for mint

        // Print transaction status every 5 iterations (roughly every 10 seconds)
        static mut LOOP_COUNTER: u32 = 0;
        unsafe {
            LOOP_COUNTER += 1;
            if LOOP_COUNTER % 5 == 0 {
                let tx_summary = TransactionManager::get_transaction_summary().await;
                println!("{}", tx_summary);
            }
        }

        sleep(Duration::from_secs(POSITIONS_CHECK_TIME)).await;
    }
}

// ── utils.rs (or wherever you keep helpers) ──────────────────────────────────
pub async fn sell_token(
    symbol: &str,
    mint: &str,
    sell_price: f64,
    entry: f64,
    peak: f64,
    drop_pct: f64,
    sol_spent: f64,
    token_amount: f64,
    dca_count: u8,
    last_dca_price: f64,
    open_time: DateTime<Utc>
) {
    let close_time = Utc::now();
    let sol_received = token_amount * sell_price - TRANSACTION_FEE_SOL;
    let profit_sol = sol_received - sol_spent; // Don't double deduct transaction fees
    let profit_pct = (profit_sol / sol_spent) * 100.0;

    println!("\n🔴 [SELL] Close position with trailing stop");
    println!("   • Token           : {} ({})", symbol, mint);
    println!("   • Entry Price     : {:.9} SOL", entry);
    println!("   • Peak Price      : {:.9} SOL", peak);
    println!("   • Sell Price      : {:.9} SOL", sell_price);
    println!("   • Tokens Sold     : {:.9}", token_amount);
    println!("   • SOL Spent       : {:.9} SOL", sol_spent);
    println!("   • SOL Received    : {:.9} SOL", sol_received);
    println!("   • Profit (SOL)    : {:.9} SOL", profit_sol);
    println!("   • Profit Percent  : {:.2}%", profit_pct);
    println!("   • Drop From Peak  : {:.2}%", drop_pct);
    println!("   • DCA Count       : {}", dca_count);
    println!("   • Last DCA Price  : {:.9} SOL", last_dca_price);
    println!("   • Open Time       : {}", open_time);
    println!("   • Close Time      : {}", close_time);
    println!("💰 [Screener] Executed SELL {}\n", symbol);

    // ✅ store in RECENT_CLOSED_POSITIONS
    {
        let mut closed = RECENT_CLOSED_POSITIONS.write().await;

        closed.insert(mint.to_string(), Position {
            entry_price: entry,
            peak_price: peak,
            dca_count,
            token_amount,
            sol_spent,
            sol_received,
            open_time,
            close_time: Some(close_time),
            last_dca_price,
        });

        // Keep only the most recent 10 positions (by close_time)
        if closed.len() > 10 {
            // Remove the oldest by close_time
            if
                let Some((oldest_mint, _)) = closed
                    .iter()
                    .min_by_key(|(_, pos)| pos.close_time)
                    .map(|(mint, _)| (mint.clone(), ()))
            {
                closed.remove(&oldest_mint);
            }
        }
    }
}

// GeckoTerminal API functions
async fn fetch_gecko_ohlcv(
    pool_address: &str,
    timeframe: Timeframe,
    limit: usize
) -> Result<GeckoTerminalResponse, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?limit={}&currency=token&include_empty_intervals=false&token=base",
        pool_address,
        timeframe.as_str(),
        limit.min(MAX_OHLCV_LIMIT)
    );

    println!("🌐 [GECKO] Fetching {} data from: {}", timeframe.as_str(), url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("accept", "application/json")
        .header("User-Agent", "ScreenerBot/1.0")
        .send().await?;

    if !response.status().is_success() {
        return Err(format!("GeckoTerminal API error: {}", response.status()).into());
    }

    let gecko_response: GeckoTerminalResponse = response.json().await?;

    println!(
        "✅ [GECKO] Fetched {} {} candles for pool {}",
        gecko_response.data.attributes.ohlcv_list.len(),
        timeframe.as_str(),
        pool_address
    );

    Ok(gecko_response)
}

// Caching functions
async fn load_cached_ohlcv(pool_address: &str) -> Result<CachedOHLCV, Box<dyn std::error::Error>> {
    let cache_file = format!("{}/{}.json", CACHE_DIR, pool_address);

    if !Path::new(&cache_file).exists() {
        return Err("Cache file not found".into());
    }

    let cache_content = fs::read_to_string(&cache_file)?;
    let cached_data: CachedOHLCV = serde_json::from_str(&cache_content)?;

    Ok(cached_data)
}

async fn save_cached_ohlcv(cache_data: &CachedOHLCV) -> Result<(), Box<dyn std::error::Error>> {
    // Create cache directory if it doesn't exist
    fs::create_dir_all(CACHE_DIR)?;

    let cache_file = format!("{}/{}.json", CACHE_DIR, cache_data.pool_address);
    let cache_content = serde_json::to_string_pretty(cache_data)?;

    fs::write(&cache_file, cache_content)?;

    // Clean old cache files
    clean_old_cache_files().await?;

    Ok(())
}

fn is_cache_valid(cached_data: &CachedOHLCV) -> bool {
    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache_age_hours = (now - cached_data.timestamp_cached) / 3600;
    cache_age_hours < CACHE_DURATION_HOURS
}

async fn clean_old_cache_files() -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(CACHE_DIR).exists() {
        return Ok(());
    }

    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let entries = fs::read_dir(CACHE_DIR)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        let file_age_hours = (now - duration.as_secs()) / 3600;

                        if file_age_hours > CACHE_DURATION_HOURS {
                            if let Err(e) = fs::remove_file(&path) {
                                eprintln!("⚠️ Failed to remove old cache file {:?}: {}", path, e);
                            } else {
                                println!("🗑️ Removed old cache file: {:?}", path);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// Helper function to get pool address from mint
async fn get_pool_address_for_mint(mint: &str) -> Option<String> {
    // Use existing pool finding logic from pool_price.rs
    use crate::pool_price::POOL_CACHE;
    use crate::helpers::fetch_solana_pairs;
    use crate::pools::decoder::decode_any_pool;

    // First check cache
    {
        let cache = POOL_CACHE.read();
        if let Some(pool_pk) = cache.get(mint) {
            return Some(pool_pk.to_string());
        }
    }

    // If not in cache, try to find biggest pool
    match
        tokio::task::spawn_blocking({
            let mint = mint.to_string();
            move || {
                let rpc = &crate::configs::RPC;
                fetch_solana_pairs(&mint).and_then(|pools| {
                    pools
                        .par_iter()
                        .filter_map(|pk| {
                            decode_any_pool(rpc, pk)
                                .ok()
                                .map(|(b, q, _, _)| (*pk, (b as u128) + (q as u128)))
                        })
                        .max_by_key(|&(_, liq)| liq)
                        .map(|(pk, _)| pk)
                        .ok_or_else(|| anyhow::anyhow!("no valid pools for {}", mint))
                })
            }
        }).await
    {
        Ok(Ok(pool_pk)) => {
            // Cache the result
            {
                POOL_CACHE.write().insert(mint.to_string(), pool_pk);
            }
            Some(pool_pk.to_string())
        }
        _ => None,
    }
}
