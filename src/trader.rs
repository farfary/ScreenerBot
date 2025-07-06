#![allow(warnings)]

use crate::dexscreener::{ TOKENS, Token };
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use tokio::io::{ self, AsyncBufReadExt, AsyncReadExt, BufReader };
use comfy_table::{ Table, presets::UTF8_FULL };
use crate::swap_gmgn::*;
use crate::helpers::*;
use crate::pool_price::*;
use tokio::task::spawn_blocking;
use crate::configs::BLACKLIST;
use crate::persistence::*;
use std::sync::atomic::Ordering;

use serde::{ Serialize, Deserialize };
use tokio::{ fs };
use anyhow::Result;
use crate::utilitis::*;
// put this near your other imports
use tokio::time::{ timeout };

// Constants
const TRADE_SIZE_SOL: f64 = 0.01; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 20; // example: allow up to 5 open positions
const MAX_DCA_COUNT: u8 = 3; // for example, max 3 DCA per position
const TRANSACTION_FEE_SOL: f64 = 0.0001; // Transaction fee for buying and selling

// ────────────── ACTIONS ──────────────

use chrono::{ Duration as ChronoDuration };

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

async fn print_open_positions() {
    let positions = OPEN_POSITIONS.read().await;
    let closed = RECENT_CLOSED_POSITIONS.read().await;

    // Sort open positions by open_time ascending (latest at bottom)
    let mut positions_vec: Vec<_> = positions.iter().collect();
    positions_vec.sort_by_key(|(_, pos)| pos.open_time);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(
            vec![
                "Mint",
                "Entry Price",
                "Current Price",
                "Profit %",
                "Peak Price",
                "DCA Count",
                "Tokens",
                "SOL Spent",
                "Open Time"
            ]
        );

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

        table.add_row(
            vec![
                mint.clone(),
                format!("{:.12}", pos.entry_price),
                format!("{:.12}", current_price),
                format!("{:+.2}%", profit_pct),
                format!("{:.12}", pos.peak_price),
                pos.dca_count.to_string(),
                format!("{:.9}", pos.token_amount),
                format!("{:.9}", pos.sol_spent),
                format_duration_ago(pos.open_time)
            ]
        );
    }

    println!("\n📂 [Open Positions]\n{}\n", table);

    if !closed.is_empty() {
        // Sort closed positions by close_time ascending (latest at bottom)
        let mut closed_vec: Vec<_> = closed.iter().cloned().collect();
        closed_vec.sort_by_key(|pos| pos.close_time.unwrap_or(pos.open_time));

        let mut table_closed = Table::new();
        table_closed
            .load_preset(UTF8_FULL)
            .set_header(
                vec![
                    "Mint",
                    "Entry Price",
                    "Close Price",
                    "Profit %",
                    "Peak Price",
                    "Tokens",
                    "SOL Spent",
                    "SOL Received",
                    "Open Time",
                    "Close Time"
                ]
            );

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

            table_closed.add_row(
                vec![
                    "(closed)".into(),
                    format!("{:.9}", pos.entry_price),
                    format!("{:.9}", close_price),
                    format!("{:+.2}%", profit_pct),
                    format!("{:.9}", pos.peak_price),
                    format!("{:.9}", pos.token_amount),
                    format!("{:.9}", pos.sol_spent),
                    format!("{:.9}", pos.sol_received),
                    format_duration_ago(pos.open_time),
                    pos.close_time.map(format_duration_ago).unwrap_or_else(|| "-".into())
                ]
            );
        }

        println!("📁 [Recent Closed Positions]\n{}\n", table_closed);
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

        closed.push(Position {
            entry_price: entry,
            peak_price: peak,
            dca_count,
            token_amount,
            sol_spent,
            sol_received,
            open_time,
            close_time: Some(close_time),
            last_dca_price, // ← NEW field
        });

        if closed.len() > 10 {
            closed.remove(0);
        }
    }
}

use crate::configs::RPC; // import your static RPC client

use futures::future::join_all;

use futures::FutureExt;
use tokio::{ task };

use tokio::runtime::Handle;

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

use std::collections::{ VecDeque };

async fn trader_main_loop() {
    use std::time::Instant;
    println!("🔥 Entered MAIN TRADER LOOP TASK");

    // wait until TOKENS is non-empty
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
    let mut notified_profit_bucket: HashMap<String, i32> = HashMap::new();
    let mut price_histories: HashMap<String, VecDeque<f64>> = HashMap::new();

    const PRICE_HISTORY_CAP: usize = 60; // 5 min @ 5 s/loop
    const RSI_PERIOD: usize = 14;
    const OVERSOLD_RSI: f64 = 30.0;
    const BOUNCE_REQ: f64 = 0.25; // %

    // track price snapshots for 5-min drop checks
    let mut last_prices: HashMap<String, (Instant, f64)> = HashMap::new();

    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            return;
        }

        /* ── build mint list ───────────────────────────────────────────── */
        let mut mints: Vec<String> = {
            let t = TOKENS.read().await;
            t.iter()
                .map(|tok| tok.mint.clone())
                .collect()
        };
        {
            let pos = OPEN_POSITIONS.read().await;
            for m in pos.keys() {
                if !mints.contains(m) {
                    mints.push(m.clone());
                }
            }
        }

        /* ── iterate mints ─────────────────────────────────────────────── */
        for mint in mints {
            if SHUTDOWN.load(Ordering::SeqCst) {
                return;
            }
            if BLACKLIST.read().await.contains(&mint) {
                continue;
            }

            let symbol = TOKENS.read().await
                .iter()
                .find(|t| t.mint == mint)
                .map(|t| t.symbol.clone())
                .unwrap_or_else(|| mint.chars().take(4).collect());

            /* ---------- CURRENT PRICE (with timeout) ------------------- */
            let price_handle = tokio::task::spawn_blocking({
                let m = mint.clone();
                move || price_from_biggest_pool(&RPC, &m) // ← sync call
            });

            let current_price = match timeout(Duration::from_secs(5), price_handle).await {
                /* finished in time -------------------------------------------------- */
                Ok(join_res) =>
                    match join_res {
                        Ok(Ok(p)) if p > 0.0 => p, // valid price
                        Ok(Ok(_)) => {
                            // price <= 0 ⇒ skip
                            continue;
                        }
                        Ok(Err(e)) => {
                            // decode error
                            eprintln!("❌ price error for {symbol}: {e}");
                            if
                                e.to_string().contains("no valid pools") ||
                                e.to_string().contains("Unsupported program id") ||
                                e.to_string().contains("is not an SPL-Token mint") ||
                                e.to_string().contains("AccountNotFound")
                            {
                                println!("⚠️ Blacklisting mint: {mint}");
                                crate::configs::add_to_blacklist(&mint).await;
                                println!("🛑 {symbol} added to blacklist");
                            }
                            continue;
                        }
                        Err(join_err) => {
                            // worker panicked
                            eprintln!("❌ worker panic fetching price for {symbol}: {join_err}");
                            continue;
                        }
                    }

                /* timed out --------------------------------------------------------- */
                Err(_) => {
                    eprintln!("⏰ price timeout (>5 s) for {symbol}");
                    continue;
                }
            };

            let hist = price_histories.entry(mint.clone()).or_insert_with(VecDeque::new);
            hist.push_back(current_price);
            if hist.len() > PRICE_HISTORY_CAP {
                hist.pop_front();
            }

            let now = Instant::now();

            /* ---------- ENTRY: dropped n in last x minutes ---------- */
            // PARAMETERS ─────────────────────────────────────────────────────────────
            // How many price points to keep in memory (for 5min window with 5s loop)
            const PRICE_HISTORY_CAP: usize = 60;

            // How many periods for RSI calculation (standard is 14)
            const RSI_PERIOD: usize = 14;

            // Entry only if price has dropped at least this % over the last 5min
            const DROP_TRIGGER_PCT: f64 = -7.0;

            // Entry only if we have bounced this % off the local 5min low (to avoid catching falling knife)
            const BOUNCE_REQ_PCT: f64 = 0.25;

            // Entry only if RSI is below this (oversold zone, helps catch bounces)
            const OVERSOLD_RSI: f64 = 30.0;

            // REPLACEMENT BLOCK (drop in as-is) ─────────────────────────────────────
            if hist.len() == PRICE_HISTORY_CAP {
                // 5-min drop versus oldest price
                let oldest = hist[0];
                let drop_pct = ((current_price - oldest) / oldest) * 100.0;

                if drop_pct <= DROP_TRIGGER_PCT {
                    // How much we've bounced up off the lowest price in last 5min
                    let min_5m = hist.iter().copied().fold(f64::INFINITY, f64::min);
                    let bounce_pct = ((current_price - min_5m) / min_5m) * 100.0;

                    // Quick in-place RSI calculation (classic 14 periods)
                    let mut gains = 0.0;
                    let mut losses = 0.0;
                    for i in PRICE_HISTORY_CAP - RSI_PERIOD..PRICE_HISTORY_CAP - 1 {
                        let diff = hist[i + 1] - hist[i];
                        if diff >= 0.0 {
                            gains += diff;
                        } else {
                            losses += -diff;
                        }
                    }
                    let rsi = if losses == 0.0 {
                        0.0
                    } else {
                        let rs = gains / losses;
                        100.0 - 100.0 / (1.0 + rs)
                    };

                    // Only enter if we don't already hold & have available slots
                    let need_entry = {
                        let pos = OPEN_POSITIONS.read().await;
                        pos.len() < MAX_OPEN_POSITIONS && !pos.contains_key(&mint)
                    };

                    // ENTRY DECISION
                    if need_entry && bounce_pct >= BOUNCE_REQ_PCT && rsi < OVERSOLD_RSI {
                        println!(
                            "🚨 {symbol} drop {drop_pct:.2}% | bounce {bounce_pct:.2}% | RSI {rsi:.1} ⇒ BUY"
                        );
                        let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                        if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                            println!("✅ GMGN BUY success: {tx}");
                            let bought = TRADE_SIZE_SOL / current_price;
                            let mut pos = OPEN_POSITIONS.write().await;
                            pos.insert(mint.clone(), Position {
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

            const TP_START_PROFIT_PCT: f64 = 8.0; // enable trailing ≥ +8 %
            const TP_BASE_DROP_PCT: f64 = -2.0; // start trail at −2 %
            const TP_STEP_PCT: f64 = 4.0; // widen trail 1 % per +4 % profit
            const TP_MAX_DROP_PCT: f64 = -10.0; // never looser than −10 %

            // ───────── DCA + TRAILING (single block) ───────────────────────────────
            if let Some(mut pos) = pos_opt {
                let now = Utc::now();
                let elapsed = now - pos.open_time;
                let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let next_trig =
                    DCA_BASE_TRIGGER_PCT + (pos.dca_count as f64) * DCA_STEP_TRIGGER_PCT;
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
                            "🟢 DCA #{:02} {} @ {:.9} (∆ {:.2} %)  |  {tx}",
                            pos.dca_count,
                            symbol,
                            current_price,
                            drop_pct
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

                /* ——— dynamic trailing stop ——— */
                let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;

                if profit_pct >= TP_START_PROFIT_PCT {
                    let extra = ((profit_pct - TP_START_PROFIT_PCT) / TP_STEP_PCT).floor();
                    let mut trail = TP_BASE_DROP_PCT - extra; // wider trail with profit
                    if trail < TP_MAX_DROP_PCT {
                        trail = TP_MAX_DROP_PCT;
                    }

                    if drop_from_peak <= trail {
                        if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                            println!(
                                "🔴 SELL {}  +{:.2}% | peak {:.2}% | trail {:.2}%  |  {tx}",
                                symbol,
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
            }
        } // end for mint

        sleep(Duration::from_secs(5)).await;
    }
}
