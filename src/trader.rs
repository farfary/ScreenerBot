#![allow(warnings)]
use crate::prelude::*;

use std::collections::HashMap;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use std::sync::atomic::Ordering;
use std::time::Instant;
use std::collections::{ VecDeque };
use tokio::task;
use futures::FutureExt;
use std::collections::HashSet;
use std::fs::{ OpenOptions, File };
use std::io::{ BufRead, BufReader, Write };
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

// Constants
pub const TRADE_SIZE_SOL: f64 = 0.002; // amount of SOL to spend on each buy
pub const MAX_OPEN_POSITIONS: usize = 50; // allow up to 50 open positions
pub const MAX_DCA_COUNT: u8 = 2; // max 3 DCA per position
pub const TRANSACTION_FEE_SOL: f64 = 0.00003;
pub const POSITIONS_CHECK_TIME: u64 = 15; // 10 seconds
pub const PRICE_HISTORY_CAP: usize = 60; // 5 min @ 5 s/loop





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
            sleep(Duration::from_secs(15)).await;
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
    let mut price_histories: HashMap<String, VecDeque<f64>> = HashMap::new();
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

            /* ── symbol string ────────────────────────── */
            let symbol = TOKENS.read().await
                .iter()
                .find(|t| t.mint == mint)
                .map(|t| t.symbol.clone())
                .unwrap_or_else(|| mint.chars().take(4).collect());

            let hist = price_histories.entry(mint.clone()).or_insert_with(VecDeque::new);
            hist.push_back(current_price);
            if hist.len() > PRICE_HISTORY_CAP {
                hist.pop_front();
            }

            let now = Instant::now();

            // -- Check open position state for this token
            let open_positions = OPEN_POSITIONS.read().await;
            let open = open_positions.contains_key(&mint);
            let can_open_more = open_positions.len() < MAX_OPEN_POSITIONS;
            drop(open_positions);

            // -------------- SMART ENTRY BUY: Only on Extreme/Abnormal Drops (No Comeback Buys) --------------
            const DROP_LOOKBACK: usize = 32; // Candles to look back
            const MIN_TOTAL_DROP: f64 = 15.0; // Require total drop at least -15%
            const MIN_DROP_SCALE: f64 = 2.5; // Last drop must be >2.5x avg drop
            const MIN_DROP_ABS: f64 = 8.0; // Or last drop at least -8%
            const MIN_BOUNCE: f64 = 0.5; // Optional: Require at least +0.5% bounce (set 0 for instant)

            let mut buy_signal = false;

            if hist.len() > DROP_LOOKBACK + 2 {
                let window: Vec<f64> = hist.iter().rev().take(DROP_LOOKBACK).cloned().collect();
                let high = window.iter().cloned().fold(f64::MIN, f64::max);
                let last = *window.first().unwrap();
                let prev = *hist.get(hist.len() - 2).unwrap();
                let two_back = *hist.get(hist.len() - 3).unwrap();

                let total_drop = if high > 0.0 { ((last - high) / high) * 100.0 } else { 0.0 };
                let curr_drop = ((last - prev) / prev) * 100.0;
                let prev_drop = ((prev - two_back) / two_back) * 100.0;

                let mut drops = Vec::new();
                for i in 1..=DROP_LOOKBACK {
                    let now = hist[hist.len() - i];
                    let prev = hist[hist.len() - i - 1];
                    let drop = ((now - prev) / prev) * 100.0;
                    if drop < 0.0 {
                        drops.push(drop.abs());
                    }
                }
                let avg_drop = if !drops.is_empty() {
                    drops.iter().sum::<f64>() / (drops.len() as f64)
                } else {
                    0.0
                };

                if
                    total_drop <= -MIN_TOTAL_DROP &&
                    (curr_drop <= -MIN_DROP_ABS ||
                        (curr_drop < -0.5 && curr_drop.abs() > MIN_DROP_SCALE * avg_drop)) &&
                    prev_drop < 0.0 &&
                    !open &&
                    can_open_more
                {
                    // Rug-prevention: skip ultra dumps
                    if curr_drop > -60.0 {
                        let bounced = ((last - prev) / prev) * 100.0 > MIN_BOUNCE;
                        if bounced || MIN_BOUNCE <= 0.0 {
                            buy_signal = true;
                            println!(
                                "🟢 SMART BUY SIGNAL {}: total_drop={:.2}%, last_drop={:.2}% (avg={:.2}%) (window={})",
                                symbol,
                                total_drop,
                                curr_drop,
                                avg_drop,
                                DROP_LOOKBACK
                            );
                        }
                    } else {
                        println!("⚠️ [SKIP] {}: Dump >60% in 1 candle, likely rug, skip!", symbol);
                    }
                }
            }

            if buy_signal && !open && can_open_more {
                println!(
                    "🚀 ENTRY BUY {}: [scalping drop] histlen={} price={:.9}",
                    symbol,
                    hist.len(),
                    current_price
                );
                let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                    println!("✅ BUY success: {tx}");
                    let bought = TRADE_SIZE_SOL / current_price;
                    OPEN_POSITIONS.write().await.insert(mint.clone(), Position {
                        entry_price: current_price,
                        peak_price: current_price,
                        dca_count: 1,
                        token_amount: bought,
                        sol_spent: TRADE_SIZE_SOL + TRANSACTION_FEE_SOL,
                        sol_received: 0.0,
                        open_time: Utc::now(),
                        close_time: None,
                        last_dca_price: current_price,
                    });
                }
            }

            /* ---------- DCA & trailing stop ---------- */
            let pos_opt = {
                let guard = OPEN_POSITIONS.read().await; // read-lock
                guard.get(&mint).cloned() // clone the Position, no &refs
            };

            // ───────── HARD-CODED PARAMS (configure at top of file) ───────────────
            const DCA_BASE_TRIGGER_PCT: f64 = -12.0; // first DCA at -12 %
            const DCA_STEP_TRIGGER_PCT: f64 = -5.0; // every extra DCA needs another -5 %
            const DCA_MIN_COOLDOWN_MIN: i64 = 3; // min minutes between DCAs
            const DCA_SIZE_FACTOR: f64 = 0.25; // each DCA adds 25 % more size

            // ───────── DCA + TRAILING (single block) ───────────────────────────────
            if let Some(mut pos) = pos_opt {
                let now = Utc::now();
                let elapsed = now - pos.open_time;
                let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

                // override 3rd DCA (pos.dca_count == 2) to require ≥70% drop
                let next_trig = if pos.dca_count == 2 {
                    -80.0
                } else {
                    DCA_BASE_TRIGGER_PCT + (pos.dca_count as f64) * DCA_STEP_TRIGGER_PCT
                };

                let cooldown_ok =
                    elapsed.num_minutes() >= DCA_MIN_COOLDOWN_MIN * ((pos.dca_count as i64) + 1);

                /* ——— DCA ladder ——— */
                if
                    pos.dca_count < MAX_DCA_COUNT &&
                    current_price < pos.last_dca_price &&
                    drop_pct <= next_trig &&
                    cooldown_ok
                {
                    let sol_size =
                        TRADE_SIZE_SOL * (1.0 + (pos.dca_count as f64) * DCA_SIZE_FACTOR);
                    let lamports = (sol_size * 1_000_000_000.0) as u64;

                    if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                        let added = sol_size / current_price;
                        pos.token_amount += added;
                        pos.sol_spent += sol_size + TRANSACTION_FEE_SOL;
                        pos.dca_count += 1;
                        pos.entry_price = pos.sol_spent / pos.token_amount;
                        pos.last_dca_price = current_price;

                        OPEN_POSITIONS.write().await.insert(mint.clone(), pos.clone());

                        println!(
                            "🟢 DCA #{:02} {} @ {:.9} (∆{:.2}% / trigger {:.2}%) | {tx}",
                            pos.dca_count,
                            symbol,
                            current_price,
                            drop_pct,
                            next_trig
                        );
                    }
                }

                /* ——— peak update & milestone log ——— */
                if current_price > pos.peak_price {
                    if let Some(p) = OPEN_POSITIONS.write().await.get_mut(&mint) {
                        p.peak_price = current_price;
                    }
                    let profit_now = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
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

                // ────── SMART TRAILING-STOP + FAST PROFIT CAPTURE (for 10–30% gains) ──────
                // logic sell 1
                /* ——— dynamic trailing stop ——— */
                /*──────────────── Smart trailing-stop 5 % → 1000 % ────────────────*/
                const TP_START_PCT: f64 = 4.0; // enable at +5 %
                const TP_MIN_DROP: f64 = 0.0; // lock breakeven at +5 %
                const TP_MID_PCT: f64 = 25.0; // reach −2 % trail by +25 %
                const TP_MID_DROP: f64 = -1.0;

                const TP_MAX_PCT: f64 = 1000.0; // theoretic upper bound
                const TP_MAX_DROP: f64 = -40.0; // never wider than −40 %

                #[inline]
                fn calc_trail(profit_pct: f64) -> f64 {
                    if profit_pct < TP_START_PCT {
                        return f64::NEG_INFINITY; // trailing not active yet
                    }
                    if profit_pct <= TP_MID_PCT {
                        // linear easing  0 → −2 %
                        let t = (profit_pct - TP_START_PCT) / (TP_MID_PCT - TP_START_PCT);
                        return TP_MIN_DROP + t * (TP_MID_DROP - TP_MIN_DROP);
                    }
                    // quadratic easing  −2 %  →  −40 %
                    let norm = ((profit_pct - TP_MID_PCT) / (TP_MAX_PCT - TP_MID_PCT)).min(1.0);
                    let widen = norm * norm; // slow at first, faster later
                    let drop = TP_MID_DROP + widen * (TP_MAX_DROP - TP_MID_DROP);
                    drop.max(TP_MAX_DROP) // clamp
                }

                /*──────── Replace your old trailing-stop block with this ────────*/
                let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                let trail = calc_trail(profit_pct);

                // ───────── STOP-LOSS: Sell if loss > 50% ─────────
                const STOP_LOSS_PCT: f64 = -50.0;
                const EARLY_RUG_SL: f64 = -20.0; // -20% early SL
                let held_secs = (Utc::now() - pos.open_time).num_seconds();
                let is_early = held_secs < 900; // 15 min

                let stop_loss = if is_early { EARLY_RUG_SL } else { STOP_LOSS_PCT };

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

                // STOP-LOSS SELL
                if profit_pct <= stop_loss {
                    match sell_all_gmgn(&mint, current_price).await {
                        Ok(tx) => {
                            println!(
                                "⛔️ [STOP LOSS] SELL {symbol} at {profit_pct:.2}% (entry: {entry:.9}, now: {current_price:.9}) | {tx}",
                                symbol = symbol,
                                profit_pct = profit_pct,
                                entry = pos.entry_price,
                                current_price = current_price,
                                tx = tx
                            );
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
                            OPEN_POSITIONS.write().await.remove(&mint);
                            notified_profit_bucket.remove(&mint);
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
                            }
                        }
                    }
                    continue; // always continue, do not try trailing-stop if stop-loss triggers
                }

                // TRAILING-STOP SELL
                if trail > f64::NEG_INFINITY && drop_from_peak <= trail {
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
                            println!(
                                "🔴 SELL {symbol} +{:.2}% | peak {:.2}% | trail {:.2}%  |  {tx}",
                                profit_pct,
                                drop_from_peak.abs(),
                                trail
                            );
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
                            OPEN_POSITIONS.write().await.remove(&mint);
                            notified_profit_bucket.remove(&mint);
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
                            }
                        }
                    }
                    // continue to next mint (no matter success/fail)
                    continue;
                }
            }
        } // end for mint

        sleep(Duration::from_secs(POSITIONS_CHECK_TIME)).await;
    }
}


/// returns `Some(rsi)` if `values` has `period+1` points, otherwise `None`
fn rsi(values: &VecDeque<f64>, period: usize) -> Option<f64> {
    if values.len() <= period {
        return None;
    }
    let mut gain = 0.0;
    let mut loss = 0.0;
    for i in values.len() - period..values.len() - 1 {
        let diff = values[i + 1] - values[i];
        if diff >= 0.0 {
            gain += diff;
        } else {
            loss += -diff;
        }
    }
    if loss == 0.0 {
        return Some(100.0);
    }
    let rs = gain / loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

#[inline]
fn pct_change(old: f64, new_: f64) -> f64 {
    ((new_ - old) / old) * 100.0
}

fn ema(series: &VecDeque<f64>, period: usize) -> Option<f64> {
    if series.len() < period {
        return None;
    }
    let k = 2.0 / ((period as f64) + 1.0);
    let mut e = series[series.len() - period];
    for i in series.len() - period + 1..series.len() {
        e = series[i] * k + e * (1.0 - k);
    }
    Some(e)
}

fn atr_pct(hist: &VecDeque<f64>, period: usize) -> Option<f64> {
    if hist.len() < period + 1 {
        return None;
    }
    let mut sum = 0.0;
    for i in hist.len() - period + 1..hist.len() {
        let pct = ((hist[i] - hist[i - 1]).abs() / hist[i - 1]) * 100.0;
        sum += pct;
    }
    Some(sum / (period as f64)) // average % true range
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
    let profit_sol = sol_received - sol_spent - TRANSACTION_FEE_SOL;
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


