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

// Constants
const TRADE_SIZE_SOL: f64 = 0.004; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 3; // example: allow up to 5 open positions
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

    for (mint, pos) in positions.iter() {
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

        for pos in closed.iter() {
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

            // current price (blocking RPC on helper thread)
            let current_price = match
                task::spawn_blocking({
                    let m = mint.clone();
                    move || price_from_biggest_pool(&RPC, &m)
                }).await
            {
                Ok(Ok(p)) if p > 0.0 => p,
                Ok(Err(e)) => {
                    eprintln!("❌ price error for {symbol}: {e}");
                    if
                        e.to_string().contains("no valid pools") ||
                        e.to_string().contains("Unsupported program id") ||
                        e.to_string().contains("is not an SPL-Token mint") ||
                        e.to_string().contains("AccountNotFound")
                        
                    {
                        println!("⚠️ Blacklisting mint: {}", mint);
                        crate::configs::add_to_blacklist(&mint).await;
                        println!("🛑 {} added to blacklist", symbol);
                    }
                    continue;
                }
                _ => {
                    continue;
                }
            };

            let now = Instant::now();

            /* ---------- ENTRY: dropped ≥25% in last 5 minutes ---------- */
            if let Some(&(ts, old_price)) = last_prices.get(&mint) {
                let elapsed = now.duration_since(ts);
                if elapsed.as_secs() >= 300 {
                    let drop_pct = ((current_price - old_price) / old_price) * 100.0;
                    if drop_pct <= -10.0 {
                        println!(
                            "🚨 {} has dropped {:.2}% over the last 5m (from {:.9} → {:.9}), placing entry",
                            symbol,
                            drop_pct,
                            old_price,
                            current_price
                        );
                        let need_entry = {
                            let pos = OPEN_POSITIONS.read().await;
                            pos.len() < MAX_OPEN_POSITIONS && !pos.contains_key(&mint)
                        };
                        if need_entry {
                            let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                            if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                                println!("✅ GMGN BUY success: {tx}");
                                let bought = TRADE_SIZE_SOL / current_price;
                                {
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
                    // reset snapshot
                    last_prices.insert(mint.clone(), (now, current_price));
                }
            } else {
                // first snapshot
                last_prices.insert(mint.clone(), (now, current_price));
            }

            /* ---------- DCA & trailing stop ---------- */
            if let Some(mut pos) = OPEN_POSITIONS.read().await.get(&mint).cloned() {
                let now = Utc::now();
                let elapsed = now - pos.open_time;

                // DCA
                let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let should_dca =
                    pos.dca_count < MAX_DCA_COUNT &&
                    current_price < pos.last_dca_price &&
                    drop_pct <= -20.0 &&
                    elapsed.num_minutes() >= 5;
                if should_dca {
                    let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                    if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                        println!("✅ GMGN DCA BUY success: {tx}");
                        let added = TRADE_SIZE_SOL / current_price;
                        pos.token_amount += added;
                        pos.sol_spent += TRADE_SIZE_SOL + TRANSACTION_FEE_SOL;
                        pos.dca_count += 1;
                        pos.entry_price = pos.sol_spent / pos.token_amount;
                        pos.last_dca_price = current_price;
                        {
                            let mut w = OPEN_POSITIONS.write().await;
                            w.insert(mint.clone(), pos.clone());
                        }
                    }
                }

                // update peak
                if current_price > pos.peak_price {
                    {
                        let mut w = OPEN_POSITIONS.write().await;
                        if let Some(p) = w.get_mut(&mint) {
                            p.peak_price = current_price;
                        }
                    }
                }

                // trailing stop / take-profit
                let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                if profit_pct >= 10.0 && drop_from_peak <= 0.0 {
                    if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                        println!("✅ GMGN SELL success: {tx}");
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

                    }
                }
            }
        } // end for mint

        sleep(Duration::from_secs(5)).await;
    }
}
