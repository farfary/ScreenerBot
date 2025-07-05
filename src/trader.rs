#![allow(warnings)]

use crate::dexscreener::{ TOKENS, Token };
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use tokio::io::{ self, AsyncBufReadExt, BufReader };
use comfy_table::{ Table, presets::UTF8_FULL };
use crate::swap_gmgn::*;
use crate::helpers::*;
use crate::utilitis::{ price_from_biggest_pool };
use tokio::task::spawn_blocking;
use crate::configs::BLACKLIST;
use crate::persistence::*;

use serde::{Serialize, Deserialize};
use tokio::{fs};
use anyhow::Result;
use crate::utilitis::PRICE_CACHE;


// Constants
const TRADE_SIZE_SOL: f64 = 0.004; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 3; // example: allow up to 5 open positions
const MAX_DCA_COUNT: u8 = 3; // for example, max 3 DCA per position
const TRANSACTION_FEE_SOL: f64 = 0.0001; // Transaction fee for buying and selling


// ────────────── ACTIONS ──────────────


async fn print_open_positions() {
    let positions = OPEN_POSITIONS.read().await;
    let closed = RECENT_CLOSED_POSITIONS.read().await;

    // ────────────── OPEN POSITIONS ──────────────
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
        // Read current price from in-memory cache
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
                format!("{:.9}", pos.entry_price),
                format!("{:.9}", current_price),
                format!("{:+.2}%", profit_pct),
                format!("{:.9}", pos.peak_price),
                pos.dca_count.to_string(),
                format!("{:.9}", pos.token_amount),
                format!("{:.9}", pos.sol_spent),
                pos.open_time.to_rfc3339()
            ]
        );
    }

    println!("\n📂 [Open Positions]\n{}\n", table);

    // ────────────── RECENT CLOSED POSITIONS ──────────────
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
                    pos.open_time.to_rfc3339(),
                    pos.close_time.map(|t| t.to_rfc3339()).unwrap_or_else(|| "-".into())
                ]
            );
        }

        println!("📁 [Recent Closed Positions]\n{}\n", table_closed);
    }
}

// ── utils.rs (or wherever you keep helpers) ──────────────────────────────────
pub async fn sell_token(
    symbol:          &str,
    mint:            &str,
    sell_price:      f64,
    entry:           f64,
    peak:            f64,
    drop_pct:        f64,
    sol_spent:       f64,
    token_amount:    f64,
    dca_count:       u8,
    last_dca_price:  f64,
    open_time:       DateTime<Utc>,
) {
    let close_time   = Utc::now();
    let sol_received = token_amount * sell_price - TRANSACTION_FEE_SOL;
    let profit_sol   = sol_received - sol_spent - TRANSACTION_FEE_SOL;
    let profit_pct   = (profit_sol / sol_spent) * 100.0;

    println!("\n🔴 [SELL] Close position with trailing stop");
    println!("   • Token           : {} ({})", symbol, mint);
    println!("   • Entry Price     : {:.9} SOL", entry);
    println!("   • Peak Price      : {:.9} SOL", peak);
    println!("   • Sell Price      : {:.9} SOL", sell_price);
    println!("   • Tokens Sold     : {:.9}",  token_amount);
    println!("   • SOL Spent       : {:.9} SOL", sol_spent);
    println!("   • SOL Received    : {:.9} SOL", sol_received);
    println!("   • Profit (SOL)    : {:.9} SOL", profit_sol);
    println!("   • Profit Percent  : {:.2}%",  profit_pct);
    println!("   • Drop From Peak  : {:.2}%",  drop_pct);
    println!("   • DCA Count       : {}",       dca_count);
    println!("   • Last DCA Price  : {:.9} SOL", last_dca_price);
    println!("   • Open Time       : {}", open_time);
    println!("   • Close Time      : {}", close_time);
    println!("💰 [Screener] Executed SELL {}\n", symbol);

    // ✅ store in RECENT_CLOSED_POSITIONS
    {
        let mut closed = RECENT_CLOSED_POSITIONS.write().await;

        closed.push(Position {
            entry_price:    entry,
            peak_price:     peak,
            dca_count,
            token_amount,
            sol_spent,
            sol_received,
            open_time,
            close_time:     Some(close_time),
            last_dca_price,          // ← NEW field
        });

        if closed.len() > 10 {
            closed.remove(0);
        }
    }
}


use crate::configs::RPC; // import your static RPC client

use futures::future::join_all;

pub async fn start_trader_loop() {
    println!("🚀 [Screener] Trader loop started!");

    tokio::spawn(async move {
        println!("🔥 Entered MAIN TRADER LOOP TASK");

        // Wait ONCE for TOKENS to be loaded
        loop {
            let lock = TOKENS.read().await;
            println!("⏳ Waiting for TOKENS to be loaded... (len={})", lock.len());
            if !lock.is_empty() {
                println!("✅ TOKENS loaded! Proceeding with trader loop.");
                break;
            }
            drop(lock);
            sleep(Duration::from_secs(1)).await;
        }

        // track last prices per mint
        let mut last_prices: HashMap<String, f64> = HashMap::new();

        loop {
            // snapshot tokens
            let tokens_snapshot = {
                let lock = TOKENS.read().await;
                lock.clone()
            };

            // build list of mints
            let mut mints: Vec<String> = tokens_snapshot
                .iter()
                .map(|t| t.mint.clone())
                .collect();
            {
                let pos_lock = OPEN_POSITIONS.read().await;
                for m in pos_lock.keys() {
                    if !mints.contains(m) {
                        mints.push(m.clone());
                    }
                }
            }

            for mint in mints {
                // skip black-listed
                {
                    let bl = BLACKLIST.read().await;
                    if bl.contains(&mint) {
                        continue;
                    }
                }

                // symbol or fallback
                let symbol = tokens_snapshot
                    .iter()
                    .find(|t| t.mint == mint)
                    .map(|t| t.symbol.clone())
                    .unwrap_or_else(|| mint.chars().take(4).collect());

                // fetch price
                let price_res = tokio::task::spawn_blocking({
                    let m = mint.clone();
                    move || price_from_biggest_pool(&RPC, &m)
                }).await;
                let current_price = match price_res {
                    Ok(Ok(p)) if p > 0.0 => p,
                    Ok(Err(e)) => {
                        eprintln!("❌ price error for {}: {}", symbol, e);
                        if
                            e.to_string().contains("no valid pools") ||
                            e.to_string().contains("Unsupported program id")
                        {
                            let mut bl = BLACKLIST.write().await;
                            if bl.insert(mint.clone()) {
                                println!("🛑 {} added to blacklist", symbol);
                            }
                        }
                        continue;
                    }
                    _ => {
                        continue;
                    }
                };

                // compare to last tick
                if let Some(prev) = last_prices.get(&mint).cloned() {
                    let pct = ((current_price - prev) / prev) * 100.0;
                    if pct.abs() >= 1.0 {
                        println!(
                            "💹 {} price change: {:.9} → {:.9} ({:+.2}%)",
                            symbol,
                            prev,
                            current_price,
                            pct
                        );
                    }
                    // ENTRY on drop ≥5%
                    if pct <= -15.0 {
                        let mut positions = OPEN_POSITIONS.write().await;
                        if positions.len() < MAX_OPEN_POSITIONS && !positions.contains_key(&mint) {
                            let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                            if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                                println!("✅ GMGN BUY success: {}", tx);
                                let bought = TRADE_SIZE_SOL / current_price;
                                positions.insert(mint.clone(), Position {
                                    entry_price: current_price,
                                    peak_price: current_price,
                                    dca_count: 1,
                                    token_amount: bought,
                                    sol_spent: TRADE_SIZE_SOL + TRANSACTION_FEE_SOL,
                                    sol_received: 0.0,
                                    open_time: Utc::now(),
                                    close_time: None,
                                    last_dca_price: current_price, // ← NEW
                                });
                            }
                        }
                        drop(positions);                      // release lock
                        let _ = save_open().await;            // ← save to disk
                    }
                }

                // DCA + TRAILING STOP
                {
                    let mut positions = OPEN_POSITIONS.write().await;
                    if let Some(pos) = positions.get_mut(&mint) {
                        let now = Utc::now();
                        let elapsed = now - pos.open_time;

                        // DCA
                        let drop_pct =
                            ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

                        if
                            pos.dca_count < MAX_DCA_COUNT &&
                            // ↓ only buy if we’re strictly LOWER than the previous DCA price
                            current_price < pos.last_dca_price &&
                            drop_pct <= -20.0 &&
                            elapsed.num_minutes() >= 5
                        {
                            let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                            if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                                println!("✅ GMGN DCA BUY success: {}", tx);

                                let added = TRADE_SIZE_SOL / current_price;
                                pos.token_amount += added;
                                pos.sol_spent += TRADE_SIZE_SOL + TRANSACTION_FEE_SOL;
                                pos.dca_count += 1;
                                pos.entry_price = pos.sol_spent / pos.token_amount;
                                pos.last_dca_price = current_price; // ← update the reference price

                                println!(
                                    "🟢 [DCA] {} new avg entry: {:.9} SOL (DCA {})",
                                    symbol,
                                    pos.entry_price,
                                    pos.dca_count
                                );
                            }
                            let _ = save_open().await;        // ← save DCA update
                        }

                        // update peak
                        if current_price > pos.peak_price {
                            pos.peak_price = current_price;
                            println!("📈 [Peak] {} new peak → {:.9} SOL", symbol, pos.peak_price);
                            let _ = save_open().await;        // ← save new peak
                        }

                        // trailing stop / take-profit with profit check
                        let profit_pct =
                            ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                        let drop_from_peak =
                            ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                        if profit_pct >= 10.0 && drop_from_peak <= 0.0 {
                            // pass current_price to ensure sell at profit
                            if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                                println!("✅ GMGN SELL success: {}", tx);
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
                                    pos.open_time,
                                ).await;
                                positions.remove(&mint);
                                drop(positions);
                                let _ = save_open().await;    // ← save removal
                                let _ = save_closed().await;  // ← save closed vec
                            }
                        }
                    }
                }

                // update last price
                last_prices.insert(mint, current_price);
            }

            sleep(Duration::from_secs(1)).await;
        }
    });

    // keyboard shortcut to print open positions
    tokio::spawn(async move {
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(_)) = lines.next_line().await {
            print_open_positions().await;
        }
    });
}
