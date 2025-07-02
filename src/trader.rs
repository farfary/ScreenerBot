// Trader.rs

use crate::dexscreener::{ TOKENS, Token };
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use chrono::{ DateTime, Utc };
use tokio::io::{ self, AsyncBufReadExt, BufReader };
use comfy_table::{ Table, presets::UTF8_FULL };
use crate::swap_gmgn::*;

const TRADE_SIZE_SOL: f64 = 0.01; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 1; // example: allow up to 5 open positions
const MAX_DCA_COUNT: u32 = 3; // for example, max 3 DCA per position

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
    pub stop_loss_price: Option<f64>, // Optional stop-loss price
    pub take_profit_price: Option<f64>, // Optional take-profit price
    pub leverage: Option<f64>, // Optional leverage for margin trades
    pub trailing_stop_pct: Option<f64>, // Optional trailing stop percentage
    pub status: PositionStatus, // Status of the position (Open, Closed, etc.)
    pub avg_entry_price: f64, // Average entry price after DCA
    pub trade_type: TradeType, // Type of trade (Initial, DCA, etc.)
    pub risk_reward_ratio: Option<f64>, // Risk-reward ratio
    pub max_drawdown: Option<f64>, // Max drawdown
    pub capital_allocated: f64, // Capital allocated to the position
    pub profit_loss_usd: Option<f64>, // Profit/Loss in USD
    pub trade_fees: Option<f64>, // Trade fees (if applicable)
}

#[derive(Debug, Clone)]
pub enum PositionStatus {
    Open,
    Closed,
    Active, // e.g., awaiting DCA
    Pending, // Position is not yet entered, waiting for signal
}

#[derive(Debug, Clone)]
pub enum TradeType {
    Initial, // First position taken
    DCA, // Dollar-Cost Averaging
    Adjusted, // Adjusted position based on other factors
}

// ────────────── GLOBAL OPEN POSITIONS ──────────────
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);
pub static RECENT_CLOSED_POSITIONS: Lazy<RwLock<Vec<Position>>> = Lazy::new(|| {
    RwLock::new(Vec::new())
});

// ────────────── ACTIONS ──────────────

async fn print_positions() {
    let positions = OPEN_POSITIONS.read().await;
    let closed = RECENT_CLOSED_POSITIONS.read().await;
    let tokens = TOKENS.read().await;

    // ────────────── OPEN POSITIONS ──────────────
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(
            vec![
                "Mint",
                "Symbol",
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
        let current_price = tokens
            .iter()
            .find(|t| &t.mint == mint)
            .and_then(|t| t.price_usd.parse::<f64>().ok())
            .unwrap_or(0.0);

        let profit_pct = if pos.entry_price > 0.0 && current_price > 0.0 {
            ((current_price - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        };

        let symbol = tokens
            .iter()
            .find(|t| &t.mint == mint)
            .map(|t| t.symbol.clone())
            .unwrap_or_default();

        table.add_row(
            vec![
                mint.clone(),
                symbol,
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

async fn sell_token(
    symbol: &str,
    mint: &str,
    sell_price: f64,
    entry: f64,
    peak: f64,
    drop_pct: f64,
    sol_spent: f64,
    token_amount: f64,
    open_time: DateTime<Utc>,
    stop_loss_price: Option<f64>,
    take_profit_price: Option<f64>,
    leverage: Option<f64>,
    trailing_stop_pct: Option<f64>,
    trade_fees: Option<f64>
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

    if let Some(sl) = stop_loss_price {
        println!("   • Stop Loss Price : {:.9}", sl);
    }
    if let Some(tp) = take_profit_price {
        println!("   • Take Profit Price : {:.9}", tp);
    }
    if let Some(l) = leverage {
        println!("   • Leverage         : {:.1}", l);
    }
    if let Some(ts) = trailing_stop_pct {
        println!("   • Trailing Stop    : {:.2}%", ts);
    }
    if let Some(fee) = trade_fees {
        println!("   • Trade Fees       : {:.9}", fee);
    }

    println!("💰 [Screener] Executed SELL {}\n", symbol);

    {
        let mut closed = RECENT_CLOSED_POSITIONS.write().await;

        let closed_pos = Position {
            entry_price: entry,
            peak_price: peak,
            dca_count: 0,
            token_amount,
            sol_spent,
            sol_received,
            open_time,
            close_time: Some(close_time),
            stop_loss_price,
            take_profit_price,
            leverage,
            trailing_stop_pct,
            status: PositionStatus::Closed,
            avg_entry_price: entry,
            trade_type: TradeType::Adjusted,
            risk_reward_ratio: None,
            max_drawdown: None,
            capital_allocated: sol_spent,
            profit_loss_usd: None,
            trade_fees,
        };

        closed.push(closed_pos);

        if closed.len() > 10 {
            closed.remove(0);
        }
    }
}

pub async fn start_trader_loop() {
    println!("🚀 [Screener] Trader loop started!");

    // ────────────── Main trader loop ──────────────
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

                // ────────────── PRICE DROP ENTRY ──────────────
                if let Some(prev) = last_price {
                    let change_pct = ((current_price - prev) / prev) * 100.0;
                    if change_pct <= -1.0 {
                        let mut positions = OPEN_POSITIONS.write().await;

                        if positions.len() >= MAX_OPEN_POSITIONS {
                            println!(
                                "🚫 Max open positions ({}) reached, skip new for {}",
                                MAX_OPEN_POSITIONS,
                                token.symbol
                            );
                        } else if !positions.contains_key(&token.mint) {
                            let amount_sol = TRADE_SIZE_SOL;
                            let amount_u64 = (amount_sol * 1_000_000_000.0) as u64;

                            match buy_gmgn(&token.mint, amount_u64).await {
                                Ok(tx_hash) => {
                                    println!("✅ GMGN BUY success: {}", tx_hash);
                                    let tokens_bought = amount_sol / current_price;
                                    positions.insert(token.mint.clone(), Position {
                                        entry_price: current_price,
                                        peak_price: current_price,
                                        dca_count: 1,
                                        token_amount: tokens_bought,
                                        sol_spent: amount_sol,
                                        sol_received: 0.0,
                                        open_time: Utc::now(),
                                        close_time: None,
                                        stop_loss_price: Some(current_price * 0.9), // example, 10% below entry price
                                        take_profit_price: Some(current_price * 1.1), // example, 10% above entry price
                                        leverage: Some(2.0), // example leverage
                                        trailing_stop_pct: Some(5.0), // example trailing stop percentage
                                        status: PositionStatus::Open,
                                        avg_entry_price: current_price,
                                        trade_type: TradeType::Initial,
                                        risk_reward_ratio: Some(2.0), // e.g., 2:1 risk/reward ratio
                                        max_drawdown: Some(0.15), // e.g., max 15% drawdown allowed
                                        capital_allocated: amount_sol,
                                        profit_loss_usd: None,
                                        trade_fees: Some(0.001), // example trade fee in SOL
                                    });
                                }
                                Err(e) => {
                                    println!("❌ GMGN BUY failed: {:?}", e);
                                }
                            }
                        }
                    }
                }

                // ────────────── DCA + Trailing Stop ──────────────
                {
                    let mut positions = OPEN_POSITIONS.write().await;
                    if let Some(pos) = positions.get_mut(&token.mint) {
                        let now = Utc::now();
                        let elapsed = now - pos.open_time;
                        let drop_pct =
                            ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

                        if pos.dca_count >= MAX_DCA_COUNT {
                            println!("🚫 Max DCA count reached for {}", token.symbol);
                        } else if drop_pct > -10.0 {
                            println!(
                                "⏳ Not enough drop for {}: {:.2}% (need ≤ -10%)",
                                token.symbol,
                                drop_pct
                            );
                        } else if elapsed.num_minutes() < 5 {
                            println!(
                                "⏳ Waiting 5 min before next DCA for {} (elapsed: {}s)",
                                token.symbol,
                                elapsed.num_seconds()
                            );
                        } else {
                            let amount_sol = TRADE_SIZE_SOL;
                            let amount_u64 = (amount_sol * 1_000_000_000.0) as u64;

                            match buy_gmgn(&token.mint, amount_u64).await {
                                Ok(tx_hash) => {
                                    println!("✅ GMGN DCA BUY success: {}", tx_hash);
                                    let added = amount_sol / current_price;
                                    pos.token_amount += added;
                                    pos.sol_spent += amount_sol;
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
                                Err(e) => {
                                    println!("❌ GMGN DCA BUY failed: {:?}", e);
                                }
                            }
                        }

                        if current_price > pos.peak_price {
                            pos.peak_price = current_price;
                            println!(
                                "📈 [Peak] {} new peak → {:.9} SOL",
                                token.symbol,
                                pos.peak_price
                            );
                        }

                        let drop = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                        let profit_pct =
                            ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                        if drop <= -1.0 && profit_pct > 0.0 {
                            match sell_all_gmgn(&token.mint).await {
                                Ok(tx_hash) => {
                                    println!("✅ GMGN SELL success: {}", tx_hash);
                                    sell_token(
                                        &token.symbol,
                                        &token.mint,
                                        current_price,
                                        pos.entry_price,
                                        pos.peak_price,
                                        drop,
                                        pos.sol_spent,
                                        pos.token_amount,
                                        pos.open_time,
                                        pos.stop_loss_price,
                                        pos.take_profit_price,
                                        pos.leverage,
                                        pos.trailing_stop_pct,
                                        pos.trade_fees
                                    ).await;
                                    positions.remove(&token.mint);
                                }
                                Err(e) => {
                                    println!("❌ GMGN SELL failed: {:?}", e);
                                }
                            }
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

    // Enter listener to show table too
    tokio::spawn(async move {
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(_)) = lines.next_line().await {
            print_positions().await;
        }
    });
}
