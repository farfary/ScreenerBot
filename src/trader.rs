use crate::prelude::*;
use crate::trades::get_token_trades;
use crate::shutdown;
use std::collections::VecDeque;
use serde::{ Deserialize, Serialize };
use rayon::prelude::*;

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

/// supervisor that starts both position monitoring and token discovery tasks
pub fn start_trader_loop() {
    println!("🚀 [Screener] Trader loop started!");

    // ── Start fast position monitoring task ──────────────────────────────────────────────────────
    task::spawn(async move {
        use std::panic::AssertUnwindSafe;

        loop {
            if shutdown::is_shutdown_requested() {
                break;
            }

            // run the position monitoring logic and trap panics
            let run = AssertUnwindSafe(position_monitor_loop()).catch_unwind().await;

            match run {
                Ok(_) => {
                    break;
                } // exited via SHUTDOWN
                Err(e) => {
                    eprintln!("💥 [PANIC] Position monitor crashed: {:?}", e);
                    eprintln!("🔄 [RESTART] Restarting position monitor in 3 seconds...");
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
    });

    // ── Start price monitoring task for discovered tokens ──────────────────────────────────────────────────────
    task::spawn(async move {
        use std::panic::AssertUnwindSafe;

        loop {
            if shutdown::is_shutdown_requested() {
                break;
            }

            // run the price monitoring logic and trap panics
            let run = AssertUnwindSafe(price_monitoring_loop()).catch_unwind().await;

            match run {
                Ok(_) => {
                    break;
                } // exited via SHUTDOWN
                Err(e) => {
                    eprintln!("💥 [PANIC] Price monitoring crashed: {:?}", e);
                    eprintln!("🔄 [RESTART] Restarting price monitoring in 3 seconds...");
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
    });

    // ── Start token discovery task ──────────────────────────────────────────────────────
    task::spawn(async move {
        use std::panic::AssertUnwindSafe;

        loop {
            if shutdown::is_shutdown_requested() {
                break;
            }

            // run the token discovery logic and trap panics
            let run = AssertUnwindSafe(token_discovery_loop()).catch_unwind().await;

            match run {
                Ok(_) => {
                    break;
                } // exited via SHUTDOWN
                Err(e) => {
                    eprintln!("💥 [PANIC] Token discovery crashed: {:?}", e);
                    eprintln!("🔄 [RESTART] Restarting token discovery in 5 seconds...");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    // ── positions print task ──────────────────────────────────────────────────────
    task::spawn(async move {
        let mut counter = 0;
        let mut consecutive_failures = 0;

        loop {
            if shutdown::is_shutdown_requested() {
                println!("🔄 [PRINT TASK] Shutdown signal received, stopping print task");
                break;
            }

            // Add error handling and timeout for print_summary
            let print_result = tokio::time::timeout(
                Duration::from_secs(30), // 30 second timeout
                print_summary()
            ).await;

            match print_result {
                Ok(_) => {
                    consecutive_failures = 0; // Reset failure counter on success

                    // Print performance report every 10 cycles (roughly every 100 seconds)
                    counter += 1;
                    if counter % 10 == 0 {
                        let perf_result = tokio::time::timeout(
                            Duration::from_secs(15),
                            print_performance_report()
                        ).await;

                        if let Err(e) = perf_result {
                            eprintln!("⚠️ [PRINT TASK] Performance report timed out: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!(
                        "💥 [PRINT TASK] Summary print failed/timed out (failure #{}: {:?}",
                        consecutive_failures,
                        e
                    );

                    // If we have too many consecutive failures, increase the sleep time
                    if consecutive_failures >= 3 {
                        eprintln!(
                            "🚨 [PRINT TASK] Too many consecutive failures, extending sleep to 30 seconds"
                        );
                        sleep(Duration::from_secs(30)).await;
                        consecutive_failures = 0; // Reset after extended sleep
                        continue;
                    }
                }
            }

            // Add heartbeat log every 60 cycles (10 minutes) to confirm task is alive
            if counter % 60 == 0 {
                println!("💓 [PRINT TASK] Heartbeat - task running normally (cycle {})", counter);
            }

            sleep(Duration::from_secs(POSITIONS_PRINT_TIME)).await;
        }

        println!("⏹️ [PRINT TASK] Print summary task stopped");
    });
}

/// Fast position monitoring task - checks open positions with dual frequency
/// - Positions in profit >2%: every 5 seconds
/// - All positions: every 30 seconds
async fn position_monitor_loop() {
    use std::time::Instant;
    println!("🔥 [POSITION MONITOR] Started dual-frequency position monitoring task");
    println!("📊 [POSITION MONITOR] Profitable positions (>2%): every {}s", POSITIONS_FREQUENT_CHECK_TIME_SEC);
    println!("📊 [POSITION MONITOR] All positions: every {}s", POSITIONS_CHECK_TIME_SEC);

    /* ── wait for TOKENS ──────────────────────────────── */
    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }
        if !TOKENS.read().await.is_empty() {
            break;
        }
        println!("⏳ [POSITION MONITOR] Waiting for TOKENS to be loaded …");
        sleep(Duration::from_secs(1)).await;
    }
    println!("✅ [POSITION MONITOR] TOKENS loaded! Starting position monitoring.");

    /* ── local state for position monitoring ──────────────────────────────────── */
    let mut notified_profit_bucket: HashMap<String, i32> = HashMap::new();
    let mut sell_failures: HashMap<String, u8> = HashMap::new(); // mint -> fails
    let mut cycle_counter = 0u64; // Track cycles for dual-frequency checking
    let normal_check_interval = POSITIONS_CHECK_TIME_SEC / POSITIONS_FREQUENT_CHECK_TIME_SEC; // 30/5 = 6

    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }

        cycle_counter += 1;
        let is_full_check_cycle = cycle_counter % normal_check_interval == 0;

        // Get all position mints first
        let all_position_mints: Vec<String> = {
            let pos = OPEN_POSITIONS.read().await;
            pos.keys().cloned().collect()
        };

        if all_position_mints.is_empty() {
            if is_full_check_cycle {
                println!("📭 [POSITION MONITOR] No open positions found");
            }
            tokio::time::sleep(Duration::from_secs(POSITIONS_FREQUENT_CHECK_TIME_SEC)).await;
            continue;
        }

        // For frequent checks, get current prices to filter profitable positions
        let position_mints = if is_full_check_cycle {
            // Full check: all positions
            println!(
                "🔄 [POSITION MONITOR] FULL CHECK - Monitoring {} positions (cycle {})",
                all_position_mints.len(),
                cycle_counter
            );
            all_position_mints
        } else {
            // Frequent check: only profitable positions (>2%)
            let positions = OPEN_POSITIONS.read().await;
            let mut profitable_mints = Vec::new();

            // Use fast pricing for positions (they should have known pools)
            let prices = tokio::task
                ::spawn_blocking({
                    let mints = all_position_mints.clone();
                    move || batch_prices_for_positions(&crate::configs::RPC, &mints)
                }).await
                .unwrap_or_default();

            for (mint, position) in positions.iter() {
                if let Some(&current_price) = prices.get(mint) {
                    if current_price > 0.0 {
                        let current_value = current_price * position.token_amount;
                        let profit_sol = current_value - position.sol_spent;
                        let profit_pct = if position.sol_spent > 0.0 {
                            (profit_sol / position.sol_spent) * 100.0
                        } else {
                            0.0
                        };

                        // Only include positions with >2% profit for frequent checking
                        if profit_pct > 2.0 {
                            profitable_mints.push(mint.clone());
                        }
                    }
                }
            }

            if !profitable_mints.is_empty() {
                println!(
                    "📊 [POSITION MONITOR] PROFITABLE CHECK - Monitoring {}/{} profitable positions (>2%) (cycle {})",
                    profitable_mints.len(),
                    all_position_mints.len(),
                    cycle_counter
                );
            }

            profitable_mints
        };

        if position_mints.is_empty() {
            tokio::time::sleep(Duration::from_secs(POSITIONS_FREQUENT_CHECK_TIME_SEC)).await;
            continue;
        }

        // Add open positions to trades monitoring
        {
            let tokens = TOKENS.read().await;
            let tokens_to_monitor: Vec<&Token> = tokens
                .iter()
                .filter(|token| position_mints.contains(&token.mint))
                .collect();

            if !tokens_to_monitor.is_empty() {
                add_tokens_to_monitor(&tokens_to_monitor).await;
                crate::ohlcv::add_tokens_to_ohlcv_monitor(&tokens_to_monitor).await;

                // Add as priority tokens since these are open positions
                for token in &tokens_to_monitor {
                    crate::ohlcv::add_priority_token(&token.mint).await;
                }
            }
        }

        /* ── BATCH PRICE FETCHING for positions only ─────────── */
        println!("🔄 [POSITION MONITOR] Checking {} open positions...", position_mints.len());
        let cycle_start = Instant::now();

        let prices = tokio::task
            ::spawn_blocking({
                let mints = position_mints.clone();
                move || batch_prices_for_positions(&crate::configs::RPC, &mints)
            }).await
            .unwrap_or_else(|e| {
                eprintln!("❌ [POSITION MONITOR] Fast position pricing panicked: {}", e);
                HashMap::new()
            });

        let successful_prices = prices.len();
        let failed_prices = position_mints.len() - successful_prices;

        if successful_prices > 0 {
            println!(
                "✅ [POSITION MONITOR] Price cycle completed in {} ms - Success: {}/{} - Failed: {}",
                cycle_start.elapsed().as_millis(),
                successful_prices,
                position_mints.len(),
                failed_prices
            );
        } else {
            eprintln!(
                "❌ [POSITION MONITOR] No prices fetched successfully, falling back to individual fetches"
            );
        }

        // Print position status every 5 iterations (roughly every 10 seconds)
        static mut LOOP_COUNTER: u32 = 0;
        unsafe {
            LOOP_COUNTER += 1;
            if LOOP_COUNTER % 5 == 0 {
                println!("💹 [POSITION MONITOR] Monitoring {} positions", position_mints.len());
            }
        }

        /* ── iterate position mints and process with fetched prices ──────── */
        for mint in &position_mints {
            if shutdown::is_shutdown_requested() {
                return;
            }

            // Get price from batch results or fallback to individual fetch
            let current_price = if let Some(&price) = prices.get(mint) {
                price
            } else {
                // Fallback to individual fetch for failed batches
                let symbol = TOKENS.read().await
                    .iter()
                    .find(|t| t.mint == *mint)
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

                        // Check if this token has an open position
                        let has_open_position = {
                            let positions = OPEN_POSITIONS.read().await;
                            positions.contains_key(mint.as_str())
                        };

                        if has_open_position {
                            println!("⚠️ [POSITION] {} has open position but price fetch failed - using last known price", mint);
                            println!(
                                "📊 [POSITION] Token may be rugged/problematic but we continue monitoring the open position"
                            );
                            // For open positions, use the last cached price if available
                            let cached_price = {
                                PRICE_CACHE.read()
                                    .unwrap()
                                    .get(mint)
                                    .map(|&(_, p)| p)
                                    .unwrap_or(0.0)
                            };
                            if cached_price > 0.0 {
                                cached_price
                            } else {
                                println!("⚠️ [POSITION] No cached price for {} - cannot monitor position", mint);
                                continue;
                            }
                        } else {
                            // Only blacklist tokens that DON'T have open positions
                            if
                                e.to_string().contains("no valid pools") ||
                                e.to_string().contains("Unsupported program id") ||
                                e.to_string().contains("is not an SPL-Token mint") ||
                                e.to_string().contains("AccountNotFound") ||
                                e.to_string().contains("base reserve is zero")
                            {
                                println!("⚠️ Blacklisting mint (no open position): {}", mint);
                                crate::configs::add_to_blacklist(&mint).await;
                            }
                            continue;
                        }
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
                if let Some(t) = tokens.iter().find(|t| t.mint == *mint) {
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

            let _now = Instant::now();

            // -- Check if this token has an open position
            let open_positions = OPEN_POSITIONS.read().await;
            let has_position = open_positions.contains_key(mint.as_str());
            drop(open_positions);

            // Skip if no position for this token (position monitor only handles existing positions)
            if !has_position {
                continue;
            }

            /* ---------- DCA & trailing stop ---------- */
            let pos_opt = {
                let guard = OPEN_POSITIONS.read().await; // read-lock
                guard.get(mint.as_str()).cloned() // clone the Position, no &refs
            };

            // ───────── POSITION MANAGEMENT (using strategy.rs) ───────────────────────────────
            if let Some(mut pos) = pos_opt {
                // Get strategy decision for this position
                let action = evaluate_position(&token, &pos, current_price).await;

                match action {
                    PositionAction::DCA { sol_amount } => {
                        let lamports = (sol_amount * 1_000_000_000.0) as u64;
                        // Use consistent profit calculation for logging
                        let current_value = current_price * pos.token_amount;
                        let profit_sol = current_value - pos.sol_spent;
                        let drop_pct = if pos.sol_spent > 0.0 {
                            (profit_sol / pos.sol_spent) * 100.0
                        } else {
                            0.0
                        };

                        // Store original position before DCA
                        let _original_position = pos.clone();

                        match shutdown::safe_buy_gmgn_with_amounts(&mint, lamports, &symbol).await {
                            Ok((tx, actual_tokens_added)) => {
                                pos.token_amount += actual_tokens_added;
                                pos.sol_spent += sol_amount + TRANSACTION_FEE_SOL;
                                pos.dca_count += 1;
                                pos.entry_price = pos.sol_spent / pos.token_amount;
                                pos.last_dca_price = current_price;
                                pos.last_dca_time = Utc::now();

                                OPEN_POSITIONS.write().await.insert(mint.clone(), pos.clone());
                                save_open().await;

                                println!(
                                    "🟢 DCA #{:02} {} @ {:.9} (∆{:.2}%) | Tokens: +{:.9} | {tx}",
                                    pos.dca_count,
                                    symbol,
                                    current_price,
                                    drop_pct,
                                    actual_tokens_added
                                );
                            }
                            Err(e) => {
                                println!("❌ DCA failed: {}", e);
                            }
                        }
                    }

                    PositionAction::Sell { reason } => {
                        // Check if sell for this mint is permanently blacklisted
                        {
                            let set = SKIPPED_SELLS.lock().await;
                            if set.contains(mint.as_str()) {
                                println!("⛔️ [SKIPPED_SELLS] Not selling {} because it's blacklisted after 10 fails.", mint);
                                OPEN_POSITIONS.write().await.remove(mint.as_str());
                                notified_profit_bucket.remove(mint.as_str());
                                continue;
                            }
                        }

                        match shutdown::safe_sell_all_gmgn(&mint, current_price, &symbol).await {
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
                                    if reason.contains("stop_loss") {
                                        "⛔️ [STOP LOSS]"
                                    } else {
                                        "🔴"
                                    },
                                    symbol,
                                    profit_pct,
                                    reason
                                );

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

                                // Remove position
                                OPEN_POSITIONS.write().await.remove(mint.as_str());
                                notified_profit_bucket.remove(mint.as_str());
                                save_open().await;
                            }
                            Err(e) => {
                                let fails = sell_failures.entry(mint.clone()).or_default();
                                *fails += 1;
                                println!("❌ Sell failed for {} (fail {}/10): {e}", mint, *fails);
                                if *fails >= 10 {
                                    add_skipped_sell(mint.as_str()).await;
                                    println!("⛔️ [SKIPPED_SELLS] Added {} to skipped sells after 10 fails.", mint);
                                    OPEN_POSITIONS.write().await.remove(mint.as_str());
                                    notified_profit_bucket.remove(mint.as_str());
                                    save_open().await;
                                }
                            }
                        }
                        continue;
                    }

                    PositionAction::Hold => {
                        // Just holding - check for peak updates and profit notifications
                    }
                }

                /* ——— Peak update & milestone notifications (moved to strategy) ——— */
                if should_update_peak(&pos, current_price) {
                    if let Some(p) = OPEN_POSITIONS.write().await.get_mut(mint.as_str()) {
                        p.peak_price = current_price;
                    }

                    let bucket = get_profit_bucket(&pos, current_price);
                    if bucket > *notified_profit_bucket.get(mint.as_str()).unwrap_or(&-1) {
                        notified_profit_bucket.insert(mint.clone(), bucket);
                        let current_value = current_price * pos.token_amount;
                        let profit_sol = current_value - pos.sol_spent;
                        let profit_now = if pos.sol_spent > 0.0 {
                            (profit_sol / pos.sol_spent) * 100.0
                        } else {
                            0.0
                        };
                        println!(
                            "📈 {} new peak {:.2}% (price {:.9})",
                            symbol,
                            profit_now,
                            current_price
                        );
                    }
                }
            }
        } // end for mint

        // Use frequent check interval for all cycles
        sleep(Duration::from_secs(POSITIONS_FREQUENT_CHECK_TIME_SEC)).await;
    }
}

/// Price monitoring task - monitors price changes for discovered tokens (not in positions)
async fn price_monitoring_loop() {
    use std::time::Instant;
    println!("🔥 [PRICE MONITOR] Started price monitoring task for discovered tokens");
    println!("📊 [PRICE MONITOR] Price updates: every {}s", PRICE_MONITORING_CHECK_TIME_SEC);
    println!("🔍 [PRICE MONITOR] Monitors tokens without open positions for entry opportunities");

    /* ── wait for TOKENS ──────────────────────────────── */
    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }
        if !TOKENS.read().await.is_empty() {
            break;
        }
        println!("⏳ [PRICE MONITOR] Waiting for TOKENS to be loaded …");
        sleep(Duration::from_secs(1)).await;
    }
    println!("✅ [PRICE MONITOR] TOKENS loaded! Starting price monitoring.");

    let mut cycle_counter = 0;

    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }

        let cycle_start = Instant::now();
        cycle_counter += 1;

        // Get tokens that are discovered but not in open positions
        let tokens_to_monitor: Vec<String> = {
            let tokens = TOKENS.read().await;
            let open_positions = OPEN_POSITIONS.read().await;
            let mut filtered_tokens = Vec::new();

            for token in tokens.iter() {
                // Only monitor tokens that are NOT in open positions
                if !open_positions.contains_key(&token.mint) {
                    filtered_tokens.push(token.mint.clone());
                }
            }

            filtered_tokens
        };

        if !tokens_to_monitor.is_empty() {
            println!(
                "💰 [PRICE MONITOR] Cycle #{} - Monitoring {} tokens for price changes...",
                cycle_counter,
                tokens_to_monitor.len()
            );

            // Get current prices for monitoring tokens
            let prices = tokio::task
                ::spawn_blocking({
                    let mints = tokens_to_monitor.clone();
                    move || {
                        println!("💰 [PRICE MONITOR] Getting prices for {} tokens", mints.len());

                        // Use the same separated pricing strategy as token discovery
                        crate::pool_price::batch_prices_for_discovery_separated(
                            &crate::configs::RPC,
                            &mints,
                            false // No debug for price monitoring by default
                        )
                    }
                }).await
                .unwrap_or_else(|e| {
                    eprintln!("❌ [PRICE MONITOR] Batch pricing failed: {:?}", e);
                    HashMap::new()
                });

            let mut price_updates = 0;
            let mut successful_prices = 0;

            for (mint, price) in prices {
                if price > 0.0 {
                    successful_prices += 1;

                    // Find token symbol for better logging
                    let symbol = {
                        let tokens = TOKENS.read().await;
                        tokens
                            .iter()
                            .find(|t| t.mint == mint)
                            .map(|t| t.symbol.clone())
                            .unwrap_or_else(|| mint[..8].to_string())
                    };

                    // Log price update (only for tokens we're actively tracking)
                    if cycle_counter % 10 == 0 {
                        // Every 10th cycle (reduce spam)
                        println!("💰 [PRICE UPDATE] {} price: ${:.9}", symbol, price);
                    }
                    price_updates += 1;
                }
            }

            if cycle_counter % 5 == 0 {
                // Every 5th cycle
                println!(
                    "📊 [PRICE MONITOR] Cycle #{} completed: {}/{} prices updated ({:.1}% success) in {:.2}s",
                    cycle_counter,
                    successful_prices,
                    tokens_to_monitor.len(),
                    if tokens_to_monitor.len() > 0 {
                        ((successful_prices as f64) / (tokens_to_monitor.len() as f64)) * 100.0
                    } else {
                        0.0
                    },
                    cycle_start.elapsed().as_secs_f64()
                );
            }
        } else {
            if cycle_counter % 20 == 0 {
                // Every 20th cycle when no tokens to monitor
                println!(
                    "💰 [PRICE MONITOR] No tokens to monitor (all tokens have open positions or list is empty)"
                );
            }
        }

        sleep(Duration::from_secs(PRICE_MONITORING_CHECK_TIME_SEC)).await;
    }
}

/// Token discovery task - scans TOKENS list for new buy opportunities
async fn token_discovery_loop() {
    use std::time::Instant;
    println!("🔥 [TOKEN DISCOVERY] Started token discovery task using TOKENS list only");

    /* ── wait for TOKENS ──────────────────────────────── */
    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }
        if !TOKENS.read().await.is_empty() {
            break;
        }
        println!("⏳ [TOKEN DISCOVERY] Waiting for TOKENS to be loaded …");
        sleep(Duration::from_secs(1)).await;
    }
    println!("✅ [TOKEN DISCOVERY] TOKENS loaded! Starting token discovery monitoring.");

    let mut cycle_counter = 0;

    loop {
        if shutdown::is_shutdown_requested() {
            return;
        }

        let cycle_start = Instant::now();
        let debug_discovery = crate::configs::ARGS.iter().any(|a| a == "--debug-token-discovery");

        // Get all available tokens from TOKENS list for discovery
        let tokens_to_check: Vec<String> = {
            let tokens = TOKENS.read().await;
            let open_positions = OPEN_POSITIONS.read().await;
            let mut filtered_tokens = Vec::new();

            if debug_discovery {
                println!("🔍 [DEBUG-DISCOVERY] Total tokens in TOKENS list: {}", tokens.len());
                println!("🔍 [DEBUG-DISCOVERY] Open positions count: {}", open_positions.len());
            }

            for token in tokens.iter() {
                // Skip tokens that already have open positions
                if !open_positions.contains_key(&token.mint) {
                    // Check cooldown before adding to check list
                    let (can_enter, minutes_since) = can_enter_token_position(&token.mint).await;
                    if can_enter {
                        filtered_tokens.push(token.mint.clone());
                        if debug_discovery {
                            println!(
                                "🔍 [DEBUG-DISCOVERY] Added {} ({}) to check list",
                                token.symbol,
                                token.mint
                            );
                        }
                    } else if debug_discovery {
                        println!(
                            "🔍 [DEBUG-DISCOVERY] Skipped {} ({}) - in cooldown ({} min ago)",
                            token.symbol,
                            token.mint,
                            minutes_since
                        );
                    }
                } else if debug_discovery {
                    println!(
                        "🔍 [DEBUG-DISCOVERY] Skipped {} ({}) - has open position",
                        token.symbol,
                        token.mint
                    );
                }
            }

            if debug_discovery {
                println!(
                    "🔍 [DEBUG-DISCOVERY] Filtered tokens to check: {}",
                    filtered_tokens.len()
                );
            }

            filtered_tokens
        };

        if !tokens_to_check.is_empty() {
            cycle_counter += 1;

            println!(
                "🔍 [TOKEN DISCOVERY] Cycle #{} - Checking {} available tokens for entry opportunities...",
                cycle_counter,
                tokens_to_check.len()
            );

            if debug_discovery {
                println!(
                    "🔍 [DEBUG-DISCOVERY] Cycle #{} tokens to check: {:?}",
                    cycle_counter,
                    tokens_to_check.iter().take(10).collect::<Vec<_>>()
                );
                if tokens_to_check.len() > 10 {
                    println!(
                        "🔍 [DEBUG-DISCOVERY] ... and {} more tokens",
                        tokens_to_check.len() - 10
                    );
                }
            }

            // Check if we can open more positions
            let can_open_more = {
                let open_positions = OPEN_POSITIONS.read().await;
                open_positions.len() < MAX_OPEN_POSITIONS
            };

            if debug_discovery {
                let open_count = {
                    let open_positions = OPEN_POSITIONS.read().await;
                    open_positions.len()
                };
                println!(
                    "🔍 [DEBUG-DISCOVERY] Can open more positions: {} (current: {}, max: {})",
                    can_open_more,
                    open_count,
                    MAX_OPEN_POSITIONS
                );
            }

            if can_open_more {
                // Check tokens for entry opportunities using separated pricing strategy
                let debug_prices = crate::configs::ARGS.iter().any(|a| a == "--debug-prices");

                if debug_discovery {
                    println!(
                        "🔍 [DEBUG-DISCOVERY] Starting batch pricing for {} tokens",
                        tokens_to_check.len()
                    );
                }

                let prices = tokio::task
                    ::spawn_blocking({
                        let mints = tokens_to_check.clone();
                        move || {
                            if debug_prices {
                                println!(
                                    "🔍 [DEBUG-PRICES] Starting token discovery pricing for {} tokens",
                                    mints.len()
                                );
                            }

                            // Use separated pricing strategy for token discovery
                            crate::pool_price::batch_prices_for_discovery_separated(
                                &crate::configs::RPC,
                                &mints,
                                debug_prices
                            )
                        }
                    }).await
                    .unwrap_or_else(|e| {
                        eprintln!("❌ [TOKEN DISCOVERY] Batch pricing failed: {:?}", e);
                        HashMap::new()
                    });

                if debug_discovery {
                    println!(
                        "🔍 [DEBUG-DISCOVERY] Received {} prices from batch pricing",
                        prices.len()
                    );
                    if prices.is_empty() {
                        println!(
                            "🔍 [DEBUG-DISCOVERY] No prices received, skipping token evaluation"
                        );
                    }
                }

                for (mint, current_price) in prices.iter() {
                    if shutdown::is_shutdown_requested() {
                        return;
                    }

                    if debug_discovery {
                        println!(
                            "🔍 [DEBUG-DISCOVERY] Evaluating token {} at price {:.9}",
                            mint,
                            current_price
                        );
                    }

                    // Double-check position doesn't exist before buying
                    {
                        let open_positions = OPEN_POSITIONS.read().await;
                        if open_positions.contains_key(mint) {
                            if debug_discovery {
                                println!("🔍 [DEBUG-DISCOVERY] Skipping {} - position created during processing", mint);
                            }
                            continue;
                        }
                    }

                    // Check cooldown again before buying
                    let (can_enter, minutes_since) = can_enter_token_position(mint).await;
                    if !can_enter {
                        if debug_discovery {
                            println!(
                                "🔍 [DEBUG-DISCOVERY] Skipping {} - in cooldown ({} min ago)",
                                mint,
                                minutes_since
                            );
                        } else {
                            println!(
                                "⏰ [TOKEN DISCOVERY] {} in cooldown ({} min ago), skipping",
                                mint,
                                minutes_since
                            );
                        }
                        continue;
                    }

                    // Process token entry logic
                    let token_info = {
                        let tokens = TOKENS.read().await;
                        tokens
                            .iter()
                            .find(|t| t.mint == *mint)
                            .cloned()
                    };

                    if let Some(token) = token_info {
                        if debug_discovery {
                            println!(
                                "🔍 [DEBUG-DISCOVERY] Found token info for {}: {} ({})",
                                mint,
                                token.symbol,
                                token.name
                            );
                            println!(
                                "🔍 [DEBUG-DISCOVERY] Token liquidity: {:.2} SOL",
                                token.liquidity.base + token.liquidity.quote
                            );
                        }

                        // Check if we should buy this token
                        let trades = get_token_trades(mint).await;
                        let ohlcv = crate::ohlcv::get_token_ohlcv_dataframe(mint).await;

                        if debug_discovery {
                            println!(
                                "🔍 [DEBUG-DISCOVERY] Trades data available: {}",
                                trades.is_some()
                            );
                            println!(
                                "🔍 [DEBUG-DISCOVERY] OHLCV data available: {}",
                                ohlcv.is_some()
                            );
                        }

                        if
                            should_buy(
                                &token,
                                true, // Enable buying for token discovery
                                *current_price,
                                trades.as_ref(),
                                ohlcv.as_ref()
                            ).await
                        {
                            let symbol = if token.symbol.is_empty() {
                                "UNKNOWN"
                            } else {
                                &token.symbol
                            };
                            let name = if token.name.is_empty() { "UNKNOWN" } else { &token.name };

                            // Calculate dynamic trade size
                            let liquidity_sol = token.liquidity.base + token.liquidity.quote;
                            let dynamic_trade_size = calculate_trade_size_sol(liquidity_sol);

                            if debug_discovery {
                                println!("🔍 [DEBUG-DISCOVERY] BUY DECISION: {} passed should_buy() check", symbol);
                                println!(
                                    "🔍 [DEBUG-DISCOVERY] Calculated trade size: {:.4} SOL (liquidity: {:.2} SOL)",
                                    dynamic_trade_size,
                                    liquidity_sol
                                );
                            }

                            println!(
                                "🎯 [TOKEN DISCOVERY] ENTRY BUY {}: price={:.9} size={:.4}SOL",
                                symbol,
                                *current_price,
                                dynamic_trade_size
                            );

                            // Final check before executing buy
                            {
                                let open_positions = OPEN_POSITIONS.read().await;
                                if open_positions.contains_key(mint) {
                                    if debug_discovery {
                                        println!("🔍 [DEBUG-DISCOVERY] Aborting buy for {} - position created during final check", mint);
                                    } else {
                                        println!("🚫 [TOKEN DISCOVERY] {} position created during processing, aborting buy", mint);
                                    }
                                    continue;
                                }
                            }

                            let lamports = (dynamic_trade_size * 1_000_000_000.0) as u64;

                            if debug_discovery {
                                println!(
                                    "🔍 [DEBUG-DISCOVERY] Executing buy for {} with {} lamports",
                                    mint,
                                    lamports
                                );
                            }

                            match
                                shutdown::safe_buy_gmgn_with_amounts(mint, lamports, symbol).await
                            {
                                Ok((tx, actual_tokens_received)) => {
                                    if debug_discovery {
                                        println!(
                                            "🔍 [DEBUG-DISCOVERY] Buy successful for {}: tx={}, tokens_received={:.9}",
                                            mint,
                                            tx,
                                            actual_tokens_received
                                        );
                                    }
                                    println!("✅ [TOKEN DISCOVERY] BUY success: {tx}");

                                    // Check if position was created during buy execution
                                    let mut should_create_position = true;
                                    {
                                        let open_positions = OPEN_POSITIONS.read().await;
                                        if open_positions.contains_key(mint) {
                                            if debug_discovery {
                                                println!("🔍 [DEBUG-DISCOVERY] Position already exists for {} after buy, might be a DCA", mint);
                                            } else {
                                                println!("⚠️ [TOKEN DISCOVERY] Position already exists for {}, this might be a DCA", mint);
                                            }
                                            should_create_position = false;
                                        }
                                    }

                                    if should_create_position {
                                        if debug_discovery {
                                            println!("🔍 [DEBUG-DISCOVERY] Creating new position for {}", mint);
                                        }
                                        // Create position with ACTUAL tokens received
                                        let new_position = Position {
                                            entry_price: *current_price,
                                            peak_price: *current_price,
                                            dca_count: 0,
                                            token_amount: actual_tokens_received,
                                            sol_spent: dynamic_trade_size + TRANSACTION_FEE_SOL,
                                            sol_received: 0.0,
                                            open_time: Utc::now(),
                                            close_time: None,
                                            last_dca_price: *current_price,
                                            last_dca_time: Utc::now(),
                                        };

                                        OPEN_POSITIONS.write().await.insert(
                                            mint.clone(),
                                            new_position
                                        );
                                        save_open().await;

                                        if debug_discovery {
                                            println!("🔍 [DEBUG-DISCOVERY] Position created and saved for {}", mint);
                                        }
                                    }

                                    // Break to avoid opening too many positions at once
                                    if debug_discovery {
                                        println!(
                                            "🔍 [DEBUG-DISCOVERY] Breaking token discovery loop after successful buy"
                                        );
                                    }
                                    break;
                                }
                                Err(e) => {
                                    if debug_discovery {
                                        println!(
                                            "🔍 [DEBUG-DISCOVERY] Buy failed for {}: {}",
                                            mint,
                                            e
                                        );
                                    } else {
                                        println!("❌ [TOKEN DISCOVERY] BUY failed: {}", e);
                                    }
                                }
                            }
                        } else if debug_discovery {
                            println!("🔍 [DEBUG-DISCOVERY] Token {} did not pass should_buy() check", mint);
                        }
                    } else if debug_discovery {
                        println!("🔍 [DEBUG-DISCOVERY] No token info found for mint {}", mint);
                    }
                }
            } else {
                if debug_discovery {
                    let open_count = {
                        let open_positions = OPEN_POSITIONS.read().await;
                        open_positions.len()
                    };
                    println!(
                        "� [DEBUG-DISCOVERY] Maximum positions reached: {}/{}",
                        open_count,
                        MAX_OPEN_POSITIONS
                    );
                } else {
                    println!(
                        "�📊 [TOKEN DISCOVERY] Maximum positions reached, skipping new entries"
                    );
                }
            }
        } else if debug_discovery {
            println!("🔍 [DEBUG-DISCOVERY] No tokens to check in this cycle");
        }

        let cycle_duration = cycle_start.elapsed();
        if debug_discovery {
            println!(
                "🔍 [DEBUG-DISCOVERY] Cycle #{} completed in {:.2}s - checked {} tokens",
                cycle_counter,
                cycle_duration.as_secs_f64(),
                tokens_to_check.len()
            );
        } else {
            println!(
                "⏱️ [TOKEN DISCOVERY] Cycle #{} completed in {:.2}s",
                cycle_counter,
                cycle_duration.as_secs_f64()
            );
        }

        // Use TOKEN_DISCOVERY_CHECK_TIME_SEC for consistent timing
        sleep(Duration::from_secs(TOKEN_DISCOVERY_CHECK_TIME_SEC)).await;
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

    // ✅ store in CLOSED_POSITIONS
    {
        let mut closed = CLOSED_POSITIONS.write().await;

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
            last_dca_time: open_time, // Use open_time for closed positions
        });

        // Keep only the most recent 100 positions (by close_time)
        if closed.len() > 100 {
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

    // Record the exit in performance tracking
    let is_rug = profit_pct < -80.0; // Consider >80% loss as potential rug
    let exit_reason = if profit_pct > 0.0 { "profit_taking" } else { "stop_loss" };
    let _ = record_trade_exit(mint, sell_price, sol_received, exit_reason, dca_count, is_rug).await;
}
