// #![allow(warnings)]

use crate::dexscreener::TOKENS;
use std::collections::HashMap;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use crate::swap_gmgn::*;
use crate::pool_price::*;
use crate::configs::BLACKLIST;
use crate::persistence::*;
use std::sync::atomic::Ordering;
use std::time::Instant;
use crate::utilitis::{ *, batch_prices_from_pools };
use crate::configs::RPC;
use std::collections::{ VecDeque };
use tokio::task;
use futures::FutureExt;

// Constants
const TRADE_SIZE_SOL: f64 = 0.005; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 50; // allow up to 50 open positions
const MAX_DCA_COUNT: u8 = 2; // max 3 DCA per position
const TRANSACTION_FEE_SOL: f64 = 0.00003;
pub const POSITIONS_CHECK_TIME: u64 = 8; // 10 seconds

const PRICE_HISTORY_CAP: usize = 60; // 5 min @ 5 s/loop

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

    let mut drop_streaks: HashMap<String, usize> = HashMap::new();
    let mut last_drop_scales: HashMap<String, Vec<f64>> = HashMap::new();
    let mut last_up_scales: HashMap<String, Vec<f64>> = HashMap::new();
    let mut drop_required: HashMap<String, usize> = HashMap::new();

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
                move || batch_prices_from_pools(&RPC, &mints)
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
                        move || price_from_biggest_pool(&RPC, &m)
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

            /* ---------- ENTRY: streak drop buy ---------- */

            // Local streak/drop state for all tokens (at top with other state, outside the loop):
            // let mut drop_streaks: HashMap<String, usize> = HashMap::new();
            // let mut last_drop_scales: HashMap<String, Vec<f64>> = HashMap::new();
            // let mut last_up_scales: HashMap<String, Vec<f64>> = HashMap::new();
            // let mut drop_required: HashMap<String, usize> = HashMap::new(); // per-token "how many drops before buy"

            let drop_len = drop_streaks.entry(mint.clone()).or_insert(0);
            let drop_scales = last_drop_scales.entry(mint.clone()).or_insert_with(Vec::new);
            let up_scales = last_up_scales.entry(mint.clone()).or_insert_with(Vec::new);
            let required = drop_required.entry(mint.clone()).or_insert(3); // Default: 3-drops

            if hist.len() >= 2 {
                let prev = *hist.get(hist.len() - 2).unwrap();
                let change = ((current_price - prev) / prev) * 100.0;

                // Detect drop/up and cache scales
                if change < -0.2 {
                    *drop_len += 1;
                    drop_scales.push(change);
                    if drop_scales.len() > 16 {
                        drop_scales.remove(0);
                    }

                    // Log focus on token if about to buy
                    if *drop_len + 1 >= *required {
                        println!(
                            "👀 Focus: {} drop streak {} (Δ{:.2}%)  drop_scales={:?}",
                            symbol,
                            *drop_len + 1,
                            change,
                            drop_scales
                        );
                    }
                } else if change > 0.2 {
                    // Up candle: reset streak, store up scale
                    if *drop_len > 0 {
                        up_scales.push(change);
                        if up_scales.len() > 16 {
                            up_scales.remove(0);
                        }
                    }
                    *drop_len = 0;
                }
            }

            // Entry logic: only enter if N drops (default 3), not open, and open count < max
            let open_positions = OPEN_POSITIONS.read().await;
            let open = open_positions.contains_key(&mint);
            let can_open_more = open_positions.len() < MAX_OPEN_POSITIONS;
            drop(open_positions);

            if *drop_len >= *required && !open && can_open_more {
                println!(
                    "🚀 ENTRY BUY {}: drop streak {}: {:?} (histlen {} last={:.9})",
                    symbol,
                    *drop_len,
                    &drop_scales,
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
                    // On buy: reset streak and cache count for next time (adaptively increase required drops if price keeps dropping after buy)
                    if drop_scales.len() >= *required && drop_scales.last().unwrap() < &-3.0 {
                        *required = (*required + 1).min(6); // try increase if after buy we get dumped again
                    } else if drop_scales.last().unwrap_or(&0.0) > &-0.5 && *required > 2 {
                        *required -= 1; // lower requirement if too shallow
                    }
                    *drop_len = 0;
                }
            }

            /* ---------- END ENTRY block ---------- */

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

                /* ───────── SMART TRAILING-STOP PARAMS (CHANDLIER + RATCHET) ───────── */
                const TP_START: f64 = 8.0; // start trailing at +8 %
                const DROP_TIGHT: f64 = -0.75; // tightest stop (-0.75%)
                const DROP_MID: f64 = -2.0; // loose stop (-2%) for 8–30%
                const DROP_WIDE: f64 = -4.5; // extra loose for super high
                const DROP_ULTRA: f64 = -7.0; // rare, for over 100% runs
                const PROFIT_1: f64 = 30.0;
                const PROFIT_2: f64 = 60.0;
                const PROFIT_3: f64 = 100.0;
                const ATR_PERIOD: usize = 60;
                const ATR_BASE_MULT: f64 = 1.5;
                const PROFIT_SHARPEN: f64 = 0.14; // more dynamic after 30%

                let fee_percent = (TRANSACTION_FEE_SOL / pos.entry_price) * 100.0;
                let profit_pct =
                    ((current_price - pos.entry_price - TRANSACTION_FEE_SOL) / pos.entry_price) *
                    100.0;
                let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;

                // Calculate trailing distance based on profit zone
                let trail = if profit_pct >= TP_START {
                    let mut t = match profit_pct {
                        x if x >= PROFIT_3 => DROP_ULTRA,
                        x if x >= PROFIT_2 => DROP_ULTRA * 0.7 + DROP_WIDE * 0.3,
                        x if x >= PROFIT_1 => DROP_WIDE * 0.8 + DROP_MID * 0.2,
                        x if x >= 20.0 => DROP_MID * 0.7 + DROP_TIGHT * 0.3,
                        _ => DROP_MID,
                    };

                    // Use ATR (volatility) in tight zones
                    if profit_pct < PROFIT_1 {
                        let atr_p = atr_pct(hist, ATR_PERIOD).unwrap_or(0.4);
                        let dyn_trail = -(
                            atr_p * ATR_BASE_MULT +
                            (profit_pct * PROFIT_SHARPEN).max(0.25)
                        );
                        t = t.max(dyn_trail);
                    }

                    // Always use at least tightest
                    t = t.max(DROP_TIGHT);

                    t
                } else {
                    f64::NEG_INFINITY
                };

                // Only sell if after-fee profit is positive and drop_from_peak triggers
                if trail > f64::NEG_INFINITY && drop_from_peak <= trail && profit_pct > 0.0 {
                    if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                        println!(
                            "🔴 TRAILING-SELL {symbol} +{:.2}% | peak +{:.2}% | trail {:.2}% | {tx}",
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
                }
            }
        } // end for mint

        sleep(Duration::from_secs(POSITIONS_CHECK_TIME)).await;
    }
}

// ────────────── ACTIONS ──────────────

fn format_duration_ago(from: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(from);

    if diff.num_seconds() < 60 {
        format!("{}s ago", diff.num_seconds())
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
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

async fn print_open_positions() {
    use comfy_table::{ Table, presets::UTF8_FULL };

    let positions_guard = OPEN_POSITIONS.read().await;
    let closed_guard = RECENT_CLOSED_POSITIONS.read().await;

    // ── quick stats ───────────────────────────
    let open_count = positions_guard.len();
    let mut total_unrealized_sol = 0.0;

    // ── prepare open-positions table ──────────
    let mut positions_vec: Vec<_> = positions_guard.iter().collect();
    positions_vec.sort_by_key(|(_, pos)| pos.open_time);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header([
            "Mint",
            "Entry Price",
            "Current Price",
            "Profit %",
            "Peak Price",
            "DCA Count",
            "Tokens",
            "SOL Spent",
            "Open Time",
        ]);

    for (mint, pos) in positions_vec {
        let current_price = PRICE_CACHE.read()
            .unwrap()
            .get(mint)
            .map(|&(_ts, price)| price)
            .unwrap_or(0.0);

        let profit_pct = if pos.entry_price > 0.0 && current_price > 0.0 {
            ((current_price - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        };

        // accumulate unrealized P/L in SOL
        total_unrealized_sol += current_price * pos.token_amount - pos.sol_spent;

        table.add_row([
            mint.clone(),
            format!("{:.12}", pos.entry_price),
            format!("{:.12}", current_price),
            format!("{:+.2}%", profit_pct),
            format!("{:.12}", pos.peak_price),
            pos.dca_count.to_string(),
            format!("{:.9}", pos.token_amount),
            format!("{:.9}", pos.sol_spent),
            format_duration_ago(pos.open_time),
        ]);
    }

    println!(
        "\n📂 [Open Positions] — count: {} | unrealized P/L: {:+.3} SOL\n{}\n",
        open_count,
        total_unrealized_sol,
        table
    );

    // ── recent-closed table (FIXED) ───────────
    if !closed_guard.is_empty() {
        let mut closed_vec: Vec<_> = closed_guard.values().cloned().collect();
        closed_vec.sort_by_key(|pos| pos.close_time.unwrap_or(pos.open_time));

        let mut table_closed = Table::new();
        table_closed
            .load_preset(UTF8_FULL)
            .set_header([
                "Mint",
                "Entry Price",
                "Close Price",
                "Profit %",
                "Peak Price",
                "Tokens",
                "SOL Spent",
                "SOL Received",
                "Open Time",
                "Close Time",
            ]);

        for pos in closed_vec {
            let close_price = if pos.token_amount > 0.0 {
                pos.sol_received / pos.token_amount
            } else {
                0.0
            };
            let profit_pct = if pos.sol_spent > 0.0 {
                ((pos.sol_received - pos.sol_spent) / pos.sol_spent) * 100.0
            } else {
                0.0
            };

            table_closed.add_row([
                "(closed)".into(),
                format!("{:.9}", pos.entry_price),
                format!("{:.9}", close_price),
                format!("{:+.2}%", profit_pct),
                format!("{:.9}", pos.peak_price),
                format!("{:.9}", pos.token_amount),
                format!("{:.9}", pos.sol_spent),
                format!("{:.9}", pos.sol_received),
                format_duration_ago(pos.open_time),
                pos.close_time.map(format_duration_ago).unwrap_or_else(|| "-".into()),
            ]);
        }

        println!("📁 [Recent Closed Positions]\n{}\n", table_closed);
    }
}
