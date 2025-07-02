// Trader.rs

use crate::dexscreener::{ TOKENS, Token };
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use tokio::io::{ self, AsyncBufReadExt, BufReader };
use comfy_table::{ Table, presets::UTF8_FULL };

const TRADE_SIZE_SOL: f64 = 0.01; // amount of SOL to spend on each buy

#[derive(Debug, Clone)]
pub struct Position {
    pub entry_price: f64,
    pub peak_price: f64,
    pub dca_count: u32,
    pub token_amount: f64, // total tokens held
    pub sol_spent: f64, // total SOL spent buying
    pub sol_received: f64, // SOL received on sell
    pub open_time: DateTime<Utc>, // when position opened
    pub close_time: Option<DateTime<Utc>>, // when position closed
}

// ────────────── GLOBAL OPEN POSITIONS ──────────────
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// ────────────── ACTIONS ──────────────

async fn print_open_positions() {
    let positions = OPEN_POSITIONS.read().await;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(
            vec![
                "Mint",
                "Entry Price",
                "Peak Price",
                "DCA Count",
                "Tokens",
                "SOL Spent",
                "Open Time"
            ]
        );
    for (mint, pos) in positions.iter() {
        table.add_row(
            vec![
                mint.clone(),
                format!("{:.9}", pos.entry_price),
                format!("{:.9}", pos.peak_price),
                pos.dca_count.to_string(),
                format!("{:.9}", pos.token_amount),
                format!("{:.9}", pos.sol_spent),
                pos.open_time.to_rfc3339()
            ]
        );
    }
    println!("\n{}\n", table);
}

async fn buy_token(symbol: &str, mint: &str, price: f64, amount_sol: f64, reason: &str) {
    let tokens_bought = amount_sol / price;
    println!("\n🟢 [BUY] Open/Increase position");
    println!("   • Token        : {} ({})", symbol, mint);
    println!("   • Price        : {:.9} SOL", price);
    println!("   • SOL Spent    : {:.9} SOL", amount_sol);
    println!("   • Tokens Bought: {:.9}", tokens_bought);
    println!("   • Reason       : {}", reason);
    println!("   • Time         : {}", Utc::now());
    println!("✅ [Screener] Executed BUY {}\n", symbol);
    // Your actual buy logic here...
}

async fn sell_token(
    symbol: &str,
    mint: &str,
    sell_price: f64,
    entry: f64,
    peak: f64,
    drop_pct: f64,
    sol_spent: f64,
    token_amount: f64,
    open_time: DateTime<Utc>
) {
    let sol_received = token_amount * sell_price;
    let profit_sol = sol_received - sol_spent;
    let profit_pct = (profit_sol / sol_spent) * 100.0;
    let close_time = Utc::now();
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
    println!("   • Open Time       : {}", open_time);
    println!("   • Close Time      : {}", close_time);
    println!("💰 [Screener] Executed SELL {}\n", symbol);
    // Your actual sell logic here...
}

// ────────────── TRADER & INPUT LOOP ──────────────
pub async fn start_trader_loop() {
    println!("🚀 [Screener] Trader loop started!");

    // spawn task to listen for 'o' input and print open positions
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().eq_ignore_ascii_case("o") {
                let positions = OPEN_POSITIONS.read().await;
                println!("\n📂 [Screener] Open Positions:");
                if positions.is_empty() {
                    println!("   (none)\n");
                } else {
                    for (mint, pos) in positions.iter() {
                        println!(
                            "   • {}: entry {:.9} SOL, peak {:.9} SOL, tokens {:.9}, spent {:.9} SOL, opened {}",
                            mint,
                            pos.entry_price,
                            pos.peak_price,
                            pos.token_amount,
                            pos.sol_spent,
                            pos.open_time
                        );
                    }
                    println!();
                }
            }
        }
    });

    // spawn main trader logic
    tokio::spawn(async move {
        let mut last_prices: Vec<(String, f64)> = Vec::new();

        loop {
            let tokens_snapshot: Vec<Token> = {
                let lock = TOKENS.read().await;
                lock.clone()
            };

            for token in &tokens_snapshot {
                if token.price_usd.is_empty() {
                    continue;
                }
                let current_price = token.price_usd.parse::<f64>().unwrap_or(0.0);
                if current_price <= 0.0 {
                    continue;
                }

                let last_price = last_prices
                    .iter()
                    .find(|(m, _)| m == &token.mint)
                    .map(|(_, p)| *p);

                // PRICE DROP ENTRY
                if let Some(prev) = last_price {
                    let change_pct = ((current_price - prev) / prev) * 100.0;
                    if change_pct <= -1.0 {
                        let mut positions = OPEN_POSITIONS.write().await;
                        if !positions.contains_key(&token.mint) {
                            buy_token(
                                &token.symbol,
                                &token.mint,
                                current_price,
                                TRADE_SIZE_SOL,
                                &format!("Initial drop {:.2}%", change_pct)
                            ).await;

                            let tokens_bought = TRADE_SIZE_SOL / current_price;
                            positions.insert(token.mint.clone(), Position {
                                entry_price: current_price,
                                peak_price: current_price,
                                dca_count: 1,
                                token_amount: tokens_bought,
                                sol_spent: TRADE_SIZE_SOL,
                                sol_received: 0.0,
                                open_time: Utc::now(),
                                close_time: None,
                            });
                        }
                    }
                }

                // DCA + TRAILING STOP
                {
                    let mut positions = OPEN_POSITIONS.write().await;
                    if let Some(pos) = positions.get_mut(&token.mint) {
                        // DCA
                        if current_price < pos.entry_price * 0.9 {
                            buy_token(
                                &token.symbol,
                                &token.mint,
                                current_price,
                                TRADE_SIZE_SOL,
                                "DCA: Additional drop >10%"
                            ).await;

                            let added = TRADE_SIZE_SOL / current_price;
                            pos.token_amount += added;
                            pos.sol_spent += TRADE_SIZE_SOL;
                            pos.dca_count += 1;
                            pos.entry_price = pos.sol_spent / pos.token_amount;
                            println!(
                                "🟢 [DCA] {} new avg entry: {:.9} SOL (DCA count: {}, tokens: {:.9})",
                                token.symbol,
                                pos.entry_price,
                                pos.dca_count,
                                pos.token_amount
                            );
                        }
                        // Peak
                        if current_price > pos.peak_price {
                            pos.peak_price = current_price;
                            println!(
                                "📈 [Peak] {} new peak → {:.9} SOL",
                                token.symbol,
                                pos.peak_price
                            );
                        }
                        // Trailing stop & no-loss
                        let drop = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                        let profit_pct =
                            ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                        if drop <= -1.0 && profit_pct > 0.0 {
                            let open_time = pos.open_time;
                            sell_token(
                                &token.symbol,
                                &token.mint,
                                current_price,
                                pos.entry_price,
                                pos.peak_price,
                                drop,
                                pos.sol_spent,
                                pos.token_amount,
                                open_time
                            ).await;
                            positions.remove(&token.mint);
                        }
                    }
                }

                // SAVE LAST PRICE
                if let Some(i) = last_prices.iter().position(|(m, _)| m == &token.mint) {
                    last_prices[i] = (token.mint.clone(), current_price);
                } else {
                    last_prices.push((token.mint.clone(), current_price));
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    });

    // single-Enter listener
    tokio::spawn(async move {
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(_)) = lines.next_line().await {
            print_open_positions().await;
        }
    });
}
