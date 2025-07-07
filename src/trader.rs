// #![allow(warnings)]

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
use std::time::Instant;
use serde::{ Serialize, Deserialize };
use tokio::{ fs };
use anyhow::Result;
use crate::utilitis::*;
use tokio::time::{ timeout };
use crate::configs::RPC;
use futures::future::join_all;
use futures::FutureExt;
use tokio::{ task };
use tokio::runtime::Handle;
use chrono::{ Duration as ChronoDuration };

// Constants
const TRADE_SIZE_SOL: f64 = 0.01; // amount of SOL to spend on each buy
const MAX_OPEN_POSITIONS: usize = 20; // example: allow up to 5 open positions
const MAX_DCA_COUNT: u8 = 3; // for example, max 3 DCA per position
const TRANSACTION_FEE_SOL: f64 = 0.0001; // Transaction fee for buying and selling

/* ── tunables ─────────────────────────────────────── */
const SHORT_POLL_SECS: u64 = 10; // default
const LONG_POLL_SECS: u64 = 60; // pos > 6 h
const OLD_POS_HOURS: i64 = 6; // age threshold
const PRICE_HISTORY_CAP: usize = 60; // 5 min @ 5 s/loop
const RSI_PERIOD: usize = 14;

/* ───── SMART-DIP ENTRY ─────────────────────────────── */
const FAST_WIN: usize = 6; // 30 s high
const MED_WIN: usize = 36; // 3 min high
const FAST_DROP_PCT: f64 = -2.0; // need ≥−2 % in 30 s
const MED_DROP_PCT: f64 = -4.0; // AND ≥−4 % in 3 min
const ATR_PERIOD_BOUNCE: usize = 18; // 90 s ATR
const MIN_BOUNCE_ATR_MULT: f64 = 0.4; // bounce ≥ 0.4×ATR%
const MIN_BOUNCE_PCT: f64 = 0.15; // at least 0.15 %
const DIP_TIMEOUT_SEC: u64 = 300; // 5 min wave life
const FAST_EMA: usize = 12; // 1-min EMA for confirmation

/* ───────── new DipTracker ───────── */
#[derive(Debug)]
struct DipTracker {
    low_price: f64,
    start_time: Instant,
    last_low_time: Instant,
    armed: bool,
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
    let mut last_polled: HashMap<String, Instant> = HashMap::new(); // ⟵ NEW
    let mut dip_trackers: HashMap<String, DipTracker> = HashMap::new();

    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            return;
        }

        /* ── build mint list ──────────────────────────── */
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

        /* ── iterate mints ────────────────────────────── */
        for mint in mints {
            if SHUTDOWN.load(Ordering::SeqCst) {
                return;
            }
            if BLACKLIST.read().await.contains(&mint) {
                continue;
            }

            /* ── per-mint polling logic ───────────────── */
            let (is_old, poll_secs) = {
                let guard = OPEN_POSITIONS.read().await;
                let old = guard
                    .get(&mint)
                    .map(|p| (Utc::now() - p.open_time).num_hours() >= OLD_POS_HOURS)
                    .unwrap_or(false);
                (old, if old { LONG_POLL_SECS } else { SHORT_POLL_SECS })
            };
            if let Some(last) = last_polled.get(&mint) {
                if last.elapsed().as_secs() < poll_secs {
                    continue; // skip until interval elapsed
                }
            }
            last_polled.insert(mint.clone(), Instant::now()); // mark

            /* ── symbol string ────────────────────────── */
            let symbol = TOKENS.read().await
                .iter()
                .find(|t| t.mint == mint)
                .map(|t| t.symbol.clone())
                .unwrap_or_else(|| mint.chars().take(4).collect());

            /* ── PRICE (spawn_blocking with timeout)───── */
            let price_handle = tokio::task::spawn_blocking({
                let m = mint.clone();
                move || price_from_biggest_pool(&RPC, &m)
            });
            let current_price = match timeout(Duration::from_secs(5), price_handle).await {
                Ok(join_res) =>
                    match join_res {
                        Ok(Ok(p)) if p > 0.0 => p,
                        Ok(Ok(_)) => {
                            continue;
                        }
                        Ok(Err(e)) => {
                            eprintln!("❌ price error for {symbol}: {e}");
                            if
                                e.to_string().contains("no valid pools") ||
                                e.to_string().contains("Unsupported program id") ||
                                e.to_string().contains("is not an SPL-Token mint") ||
                                e.to_string().contains("AccountNotFound") ||
                                e.to_string().contains("base reserve is zero")
                            {
                                println!("⚠️ Blacklisting mint: {mint}");
                                crate::configs::add_to_blacklist(&mint).await;
                            }
                            continue;
                        }
                        Err(join_err) => {
                            eprintln!("❌ worker panic fetching price for {symbol}: {join_err}");
                            continue;
                        }
                    }
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

            /* ───────── new constants ───────── */
            const HOLD_SEC: u64 = 8; // price must stop making lower lows ≥ this
            const EXTRA_BOUNCE_FRAC: f64 = 0.15; // 15 % of the fast drop regained = enough

            /* ---------- ENTRY: bottom-catch buy ---------- */
            if hist.len() >= MED_WIN {
                let fast_hi = hist
                    .iter()
                    .rev()
                    .take(FAST_WIN)
                    .fold(f64::MIN, |m, &p| m.max(p));
                let med_hi = hist
                    .iter()
                    .rev()
                    .take(MED_WIN)
                    .fold(f64::MIN, |m, &p| m.max(p));

                let drop_fast = pct_change(fast_hi, current_price); // ≤ 0
                let drop_med = pct_change(med_hi, current_price); // ≤ 0

                /* per-mint tracker */
                let mut remove = false;
                {
                    let tr = dip_trackers.entry(mint.clone()).or_insert_with(|| DipTracker {
                        low_price: current_price,
                        start_time: Instant::now(),
                        last_low_time: Instant::now(),
                        armed: false,
                    });

                    /* (1) arm the tracker once both windows satisfy the thresholds */
                    if !tr.armed && drop_fast <= FAST_DROP_PCT && drop_med <= MED_DROP_PCT {
                        tr.armed = true;
                        tr.low_price = current_price;
                        tr.start_time = Instant::now();
                        tr.last_low_time = Instant::now();
                    }

                    /* (2) update low + time stamp while armed */
                    if tr.armed && current_price < tr.low_price {
                        tr.low_price = current_price;
                        tr.last_low_time = Instant::now();
                    }

                    /* (3) give up if the whole pattern grows too old */
                    if tr.armed && tr.start_time.elapsed().as_secs() > DIP_TIMEOUT_SEC {
                        remove = true;
                    }

                    /* (4) consolidation + small bounce → buy */
                    if tr.armed && tr.last_low_time.elapsed().as_secs() >= HOLD_SEC {
                        let bounce_pct = pct_change(tr.low_price, current_price);

                        /* dynamic bounce requirement:                                     *
                         *   – 15 % of the fast drop,                                       *
                         *   – or volatility-based (ATR × multiplier),                      *
                         *   – at least MIN_BOUNCE_PCT.                                     */
                        let atr_p = atr_pct(hist, ATR_PERIOD_BOUNCE).unwrap_or(0.3);
                        let need_b = (drop_fast.abs() * EXTRA_BOUNCE_FRAC)
                            .max(atr_p * MIN_BOUNCE_ATR_MULT)
                            .max(MIN_BOUNCE_PCT);

                        let ema_ok = ema(hist, FAST_EMA).map_or(true, |e| current_price > e);
                        let rsi_ok = rsi(hist, RSI_PERIOD).map_or(true, |v| v >= 30.0);
                        let capacity_ok = {
                            let pos = OPEN_POSITIONS.read().await;
                            pos.len() < MAX_OPEN_POSITIONS && !pos.contains_key(&mint)
                        };

                        if bounce_pct >= need_b && ema_ok && rsi_ok && capacity_ok {
                            println!(
                                "🚀 BOTTOM-CATCH ENTRY {symbol}: drop {:.2}% / bounce {:.2}% / need {:.2}%",
                                drop_fast,
                                bounce_pct,
                                need_b
                            );

                            let lamports = (TRADE_SIZE_SOL * 1_000_000_000.0) as u64;
                            if let Ok(tx) = buy_gmgn(&mint, lamports).await {
                                println!("✅ GMGN BUY success: {tx}");
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
                                remove = true; // consume this dip wave
                            }
                        }
                    }
                } /* mutable borrow ends */

                if remove {
                    dip_trackers.remove(&mint);
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

                /* ───────── TRAILING-STOP PARAMS ───────── */
                const TP_START: f64 = 10.0; // enable trailing only after +10 %
                const DROP_MIN: f64 = -1.0; // tightest it can ever be
                const DROP_MAX: f64 = -40.0; // loosest fallback guard
                const ATR_PERIOD: usize = 60; // 60 × 5 s  ≈ 5-minute ATR
                const ATR_BASE_MULT: f64 = 2.0; // base distance = 2 × ATR
                const PROFIT_FACTOR: f64 = 0.25; // adds 0.25 % trail per 1 % profit

                /* ------------- improved trailing-stop block ------------- */
                let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;

                let trail = if profit_pct >= TP_START {
                    /* volatility-adjusted distance */
                    let atr_p = atr_pct(hist, ATR_PERIOD).unwrap_or(0.5); // % of price
                    let mut t = -(atr_p * ATR_BASE_MULT).max(profit_pct * PROFIT_FACTOR);
                    t = t.clamp(DROP_MAX, DROP_MIN); // bounds
                    t
                } else {
                    f64::NEG_INFINITY // not active
                };

                if trail > f64::NEG_INFINITY && drop_from_peak <= trail {
                    if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                        println!(
                            "🔴 SELL {symbol} +{:.2}% | peak {:.2}% | trail {:.2}% | {tx}",
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

                // logic sell 1
                /* ——— dynamic trailing stop ——— */
                /*──────────────── Smart trailing-stop 5 % → 1000 % ────────────────*/
                // const TP_START_PCT: f64 = 5.0; // enable at +5 %
                // const TP_MIN_DROP: f64 = 0.0; // lock breakeven at +5 %
                // const TP_MID_PCT: f64 = 25.0; // reach −2 % trail by +25 %
                // const TP_MID_DROP: f64 = -2.0;

                // const TP_MAX_PCT: f64 = 1000.0; // theoretic upper bound
                // const TP_MAX_DROP: f64 = -40.0; // never wider than −40 %

                // #[inline]
                // fn calc_trail(profit_pct: f64) -> f64 {
                //     if profit_pct < TP_START_PCT {
                //         return f64::NEG_INFINITY; // trailing not active yet
                //     }
                //     if profit_pct <= TP_MID_PCT {
                //         // linear easing  0 → −2 %
                //         let t = (profit_pct - TP_START_PCT) / (TP_MID_PCT - TP_START_PCT);
                //         return TP_MIN_DROP + t * (TP_MID_DROP - TP_MIN_DROP);
                //     }
                //     // quadratic easing  −2 %  →  −40 %
                //     let norm = ((profit_pct - TP_MID_PCT) / (TP_MAX_PCT - TP_MID_PCT)).min(1.0);
                //     let widen = norm * norm; // slow at first, faster later
                //     let drop = TP_MID_DROP + widen * (TP_MAX_DROP - TP_MID_DROP);
                //     drop.max(TP_MAX_DROP) // clamp
                // }

                // /*──────── Replace your old trailing-stop block with this ────────*/
                // let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
                // let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
                // let trail = calc_trail(profit_pct);

                // if trail > f64::NEG_INFINITY && drop_from_peak <= trail {
                //     if let Ok(tx) = sell_all_gmgn(&mint, current_price).await {
                //         println!(
                //             "🔴 SELL {symbol} +{:.2}% | peak {:.2}% | trail {:.2}%  |  {tx}",
                //             profit_pct,
                //             drop_from_peak.abs(),
                //             trail
                //         );
                //         sell_token(
                //             &symbol,
                //             &mint,
                //             current_price,
                //             pos.entry_price,
                //             pos.peak_price,
                //             drop_from_peak,
                //             pos.sol_spent,
                //             pos.token_amount,
                //             pos.dca_count,
                //             pos.last_dca_price,
                //             pos.open_time
                //         ).await;
                //         OPEN_POSITIONS.write().await.remove(&mint);
                //         notified_profit_bucket.remove(&mint);
                //     }
                // }
            }
        } // end for mint

        sleep(Duration::from_secs(5)).await;
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

async fn print_open_positions() {
    use comfy_table::{ Table, presets::UTF8_FULL }; // make sure these are in scope

    let positions_guard = OPEN_POSITIONS.read().await;
    let closed_guard = RECENT_CLOSED_POSITIONS.read().await;

    /* ── quick stats ─────────────────────────────────────────── */
    let open_count = positions_guard.len();
    let mut total_unrealized_sol = 0.0;

    /* ── prepare open-positions table ────────────────────────── */
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

        /* accumulate unrealized P/L in SOL */
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

    /* ── recent-closed table (unchanged) ─────────────────────── */
    if !closed_guard.is_empty() {
        let mut closed_vec: Vec<_> = closed_guard.iter().cloned().collect();
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
