#![allow(warnings)]
use crate::prelude::*;
use crate::rate_limiter::{ RateLimitedRequest, DEXSCREENER_LIMITER };
use crate::price_validation::{ get_price_state, PriceState };
use crate::dexscreener::TOKENS;

use std::{ fs, str::FromStr };
use chrono::{ DateTime, Utc };
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer } };
use solana_account_decoder::UiAccountData;
use once_cell::sync::Lazy;
use bs58;
use anyhow::{ anyhow, Result, bail };
use spl_token::state::{ Mint, Account };
use solana_program::program_pack::Pack;
use std::collections::HashMap;
use std::path::Path;
use std::io::{ Write };
use reqwest::blocking::Client;
use serde_json::Value;
use std::{ fs::{ File }, time::{ Instant } };
use rayon::prelude::*;
use std::{ sync::RwLock };
use std::time::{ SystemTime, UNIX_EPOCH };
use solana_client::{ rpc_config::RpcTransactionConfig };
use solana_sdk::{ commitment_config::CommitmentConfig, signature::Signature };
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    UiTransactionEncoding,
    UiTransactionTokenBalance,
};
use std::sync::atomic::{ AtomicBool, Ordering };
use tokio::time::{ sleep, Duration };
use tokio::task;
use futures::FutureExt;
use std::collections::HashSet;
use std::fs::{ OpenOptions };
use std::io::{ BufRead, BufReader };
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: String,
    pub source: String, // "dexscreener"
    pub name: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub tx_count_24h: Option<u64>,
}

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// ────────────── CONFIG & RPC ──────────────
#[derive(Debug, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
}

pub static CONFIGS: Lazy<Configs> = Lazy::new(|| {
    let raw = fs::read_to_string("configs.json").expect("❌ Failed to read configs.json");
    serde_json::from_str(&raw).expect("❌ Failed to parse configs.json")
});

pub static RPC: Lazy<RpcClient> = Lazy::new(|| { RpcClient::new(CONFIGS.rpc_url.clone()) });

// ────────────── SCAN HELPER ──────────────
/// Scans all token accounts under `owner` for a given SPL program,
/// returning `(mint_address, raw_amount_u64)` for each ATA.
fn scan_program_tokens(owner: &Pubkey, program_id: &Pubkey) -> Vec<(String, u64)> {
    let filter = TokenAccountsFilter::ProgramId(*program_id);
    let accounts = RPC.get_token_accounts_by_owner(owner, filter).expect("❌ RPC error");

    let mut result = Vec::new();
    for keyed in accounts {
        let acc = keyed.account;
        if let UiAccountData::Json(parsed) = acc.data {
            let info = &parsed.parsed["info"];
            let mint = info["mint"].as_str().expect("Missing mint").to_string();
            let raw_amount_str = info["tokenAmount"]["amount"]
                .as_str()
                .expect("Missing raw amount");
            let raw_amount = raw_amount_str.parse::<u64>().expect("Invalid raw amount");
            result.push((mint, raw_amount));
        }
    }
    result
}

// ────────────── PUBLIC API ──────────────
/// Returns _all_ SPL (and Token-2022) balances for the main wallet,
/// as a list of `(mint_address, raw_amount)`.
pub fn get_all_tokens() -> Vec<(String, u64)> {
    // decode keypair
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();

    let mut tokens = Vec::new();

    // standard SPL
    tokens.extend(scan_program_tokens(&owner, &spl_token::id()));

    // Token-2022
    let token2022 = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
    tokens.extend(scan_program_tokens(&owner, &token2022));

    tokens
}

/// Returns the total raw token amount for `token_mint`,
/// summing across all ATAs (SPL + Token-2022).
pub fn get_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut total = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    total += s.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }

    println!("🔢 On-chain balance for {}: {}", token_mint, total);
    total
}

/// Returns the largest single-ATA raw amount for `token_mint`.
pub fn get_biggest_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut biggest = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    let v = s.parse::<u64>().unwrap_or(0);
                    if v > biggest {
                        biggest = v;
                    }
                }
            }
        }
    }

    println!("🔢 Biggest single ATA for {}: {}", token_mint, biggest);
    biggest
}

/// Enhanced summary function that serves as the main bot interface
/// Provides comprehensive portfolio analysis and trading insights
pub async fn print_summary() {
    // Wrap the entire function in error handling to prevent crashes
    if let Err(e) = print_summary_inner().await {
        eprintln!("💥 [PRINT SUMMARY] Error occurred: {:?}", e);
    }
}

/// Internal implementation of print_summary with proper error handling
async fn print_summary_inner() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use comfy_table::{ Table, presets::UTF8_FULL };
    use std::time::Duration;

    // Add timeout for acquiring locks to prevent hanging
    let positions_guard = tokio::time
        ::timeout(Duration::from_secs(5), OPEN_POSITIONS.read()).await
        .map_err(|_| "Timeout acquiring OPEN_POSITIONS lock")?;

    let closed_guard = tokio::time
        ::timeout(Duration::from_secs(5), CLOSED_POSITIONS.read()).await
        .map_err(|_| "Timeout acquiring CLOSED_POSITIONS lock")?;

    // ── Enhanced stats calculation with comprehensive metrics ───────────────────────────
    let open_count = positions_guard.len();
    let closed_count = closed_guard.len();
    // Get position mints early for later use in price status checking
    let position_mints: Vec<String> = positions_guard.keys().cloned().collect();
    let mut total_unrealized_sol = 0.0;
    let mut total_invested_sol = 0.0;
    let mut winners = 0;
    let mut losers = 0;
    let mut best_performer = ("".to_string(), 0.0, "".to_string());
    let mut worst_performer = ("".to_string(), 0.0, "".to_string());
    let mut total_drawdown = 0.0;
    let mut avg_holding_hours = 0.0;
    let mut positions_with_dca = 0;
    let mut closed_winners = 0; // Track closed winners at function level

    // ── Bot Performance Header ───────────────────────────────────────────────
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    🤖 SCREENER BOT DASHBOARD 🤖                   ║");
    println!("║              Automated Solana Token Trading Summary               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let now = Utc::now();
    println!("⏰ Analysis Time: {} UTC", now.format("%Y-%m-%d %H:%M:%S"));

    // ── Full Watchlist Tokens Display ──────────────────────────────────────────────────
    match tokio::time::timeout(Duration::from_secs(2), TOKENS.read()).await {
        Ok(tokens_guard) => {
            if !tokens_guard.is_empty() {
                // Filter out excluded tokens from watchlist display
                let blacklist = crate::configs::BLACKLIST.read().await;
                let mut all_watchlist_tokens: Vec<_> = tokens_guard
                    .iter()
                    .filter(|token| !blacklist.contains(&token.mint))
                    .collect();
                drop(blacklist);

                // Sort by market cap (highest first) for full watchlist display
                all_watchlist_tokens.sort_by(|a, b| {
                    let a_mcap = a.fdv_usd.parse::<f64>().unwrap_or(0.0);
                    let b_mcap = b.fdv_usd.parse::<f64>().unwrap_or(0.0);
                    b_mcap.partial_cmp(&a_mcap).unwrap_or(std::cmp::Ordering::Equal)
                });

                let mut full_watchlist_table = Table::new();
                full_watchlist_table
                    .load_preset(UTF8_FULL)
                    .set_header([
                        "#",
                        "Symbol",
                        "Name",
                        "Price USD",
                        "5m %",
                        "1h %",
                        "24h %",
                        "Volume 24h",
                        "Liquidity",
                        "MCap",
                        "Buys 1h",
                        "Rug Score",
                        "Mint Address",
                    ]);

                for (index, token) in all_watchlist_tokens.iter().enumerate() {
                    let price_usd = token.price_usd.parse::<f64>().unwrap_or(0.0);
                    let volume_24h = token.volume.h24;
                    let liquidity_usd = token.liquidity.usd;
                    let mcap = token.fdv_usd.parse::<f64>().unwrap_or(0.0);
                    let buys_1h = token.txns.h1.buys;

                    // Format price change with emojis
                    let change_5m = token.price_change.m5;
                    let change_1h = token.price_change.h1;
                    let change_24h = token.price_change.h24;

                    let format_change = |change: f64| -> String {
                        if change > 5.0 {
                            format!("🚀+{:.1}%", change)
                        } else if change > 0.0 {
                            format!("📈+{:.1}%", change)
                        } else if change < -5.0 {
                            format!("💀{:.1}%", change)
                        } else if change < 0.0 {
                            format!("📉{:.1}%", change)
                        } else {
                            "➡️0.0%".to_string()
                        }
                    };

                    // Format large numbers
                    let format_large_number = |value: f64| -> String {
                        if value >= 1_000_000.0 {
                            format!("{:.2}M", value / 1_000_000.0)
                        } else if value >= 1_000.0 {
                            format!("{:.1}K", value / 1_000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    };

                    // Truncate name if too long
                    let display_name = if token.name.len() > 12 {
                        format!("{}...", &token.name[..9])
                    } else {
                        token.name.clone()
                    };

                    // Format rug score with emoji
                    let rug_score_display = if token.rug_check.rugged {
                        "🚨 RUG".to_string()
                    } else if token.rug_check.score_normalised >= 80 {
                        format!("✅ {}", token.rug_check.score_normalised)
                    } else if token.rug_check.score_normalised >= 60 {
                        format!("⚠️ {}", token.rug_check.score_normalised)
                    } else if token.rug_check.score_normalised > 0 {
                        format!("🔴 {}", token.rug_check.score_normalised)
                    } else {
                        "❓ N/A".to_string()
                    };

                    full_watchlist_table.add_row([
                        (index + 1).to_string(),
                        token.symbol.clone(),
                        display_name,
                        if price_usd > 0.0 {
                            if price_usd < 0.000001 {
                                format!("${:.9}", price_usd)
                            } else {
                                format!("${:.6}", price_usd)
                            }
                        } else {
                            "N/A".to_string()
                        },
                        format_change(change_5m),
                        format_change(change_1h),
                        format_change(change_24h),
                        format!("${}", format_large_number(volume_24h)),
                        format!("${}", format_large_number(liquidity_usd)),
                        format!("${}", format_large_number(mcap)),
                        buys_1h.to_string(),
                        rug_score_display,
                        token.mint.clone(),
                    ]);
                }

                println!("\n🎯 [FULL WATCHLIST TOKENS] ═════════════════════════════════════════");
                println!(
                    "📊 {} tracked tokens (excluded tokens filtered) • Sorted by Market Cap • Live data from DexScreener",
                    all_watchlist_tokens.len()
                );
                println!("🔍 Rug scores from RugCheck • Volume & liquidity in USD");
                println!("═══════════════════════════════════════════════════════════════════");
                println!("{}", full_watchlist_table);
                println!("═══════════════════════════════════════════════════════════════════\n");
            } else {
                println!(
                    "\n🎯 [FULL WATCHLIST] No tokens loaded yet - waiting for DexScreener data\n"
                );
            }
        }
        Err(_) => {
            println!(
                "\n🎯 [FULL WATCHLIST] Token data being updated - skipping watchlist display this cycle\n"
            );
        }
    }

    // ── prepare open-positions table with enhanced columns ──────────
    let mut positions_vec: Vec<_> = positions_guard.iter().collect();

    // Sort by open_time (oldest first, so newest will be at bottom)
    positions_vec.sort_by(|(_, a), (_, b)| { a.open_time.cmp(&b.open_time) });

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header([
            "#",
            "Mint Address",
            "Entry",
            "Current",
            "Profit %",
            "P/L SOL",
            "Peak",
            "DD %",
            "DCA",
            "Tokens",
            "Invested",
            "Value",
            "Age",
            "Status",
        ]);

    for (index, (mint, pos)) in positions_vec.iter().enumerate() {
        let current_price = match
            tokio::time::timeout(Duration::from_secs(2), async {
                match PRICE_CACHE.read() {
                    Ok(cache) =>
                        cache
                            .get(*mint)
                            .map(|&(_ts, price)| price)
                            .unwrap_or(0.0),
                    Err(e) => {
                        println!("⚠️ [PRICE_CACHE] Error acquiring lock: {:?}", e);
                        0.0
                    }
                }
            }).await
        {
            Ok(price) => price,
            Err(_) => {
                println!("⚠️ [PRICE_CACHE] Timeout acquiring lock for mint: {}", mint);
                0.0
            }
        };

        // Check if price is valid/loaded
        let price_loaded = current_price > 0.0;
        let current_price_str = if price_loaded {
            format!("{:.8}", current_price)
        } else {
            "Price not loaded".to_string()
        };

        let (current_value, profit_sol, profit_pct) = if price_loaded {
            let current_value = current_price * pos.token_amount;
            let profit_sol = current_value - pos.sol_spent - TRANSACTION_FEE_SOL;
            let profit_pct = if pos.sol_spent > 0.0 {
                (profit_sol / pos.sol_spent) * 100.0
            } else {
                0.0
            };
            (current_value, profit_sol, profit_pct)
        } else {
            (0.0, 0.0, 0.0)
        };

        // Calculate drawdown from peak
        let peak_value = pos.peak_price * pos.token_amount;
        let drawdown_pct = if peak_value > 0.0 && price_loaded {
            ((peak_value - current_value) / peak_value) * 100.0
        } else {
            0.0
        };

        // Calculate holding time
        let holding_hours = now.signed_duration_since(pos.open_time).num_hours() as f64;
        avg_holding_hours += holding_hours;

        // Format age for display
        let age_display = format_position_age(pos.open_time);

        // Update stats only if price loaded
        if price_loaded {
            total_unrealized_sol += profit_sol;
            total_invested_sol += pos.sol_spent;
            total_drawdown += drawdown_pct;

            if pos.dca_count > 0 {
                positions_with_dca += 1;
            }

            if profit_pct > 0.0 {
                winners += 1;
                if profit_pct > best_performer.1 {
                    best_performer = (
                        (*mint).clone(),
                        profit_pct,
                        mint[..(8).min(mint.len())].to_string(),
                    );
                }
            } else if profit_pct < 0.0 {
                losers += 1;
                if profit_pct < worst_performer.1 {
                    worst_performer = (
                        (*mint).clone(),
                        profit_pct,
                        mint[..(8).min(mint.len())].to_string(),
                    );
                }
            }
        } else {
            // For positions without valid prices, don't include in stats but still track them
            total_invested_sol += pos.sol_spent;
            println!("⚠️ [SUMMARY] {} - Price not loaded, excluding from profit calculations", mint);
        }

        // Full mint address (no shortening)
        let full_mint = (*mint).clone();

        // Position status
        let status = if !price_loaded {
            "❓ NO_PRICE"
        } else if profit_pct > 20.0 {
            "🚀 MOON"
        } else if profit_pct > 10.0 {
            "📈 PUMP"
        } else if profit_pct > 0.0 {
            "✅ PROF"
        } else if profit_pct > -10.0 {
            "⚠️ DOWN"
        } else if profit_pct > -25.0 {
            "📉 LOSS"
        } else {
            "💀 RIP"
        };

        // Add row to table
        table.add_row([
            (index + 1).to_string(),
            full_mint,
            format!("{:.8}", pos.entry_price),
            current_price_str,
            if price_loaded { format!("{:+.1}%", profit_pct) } else { "N/A".to_string() },
            if price_loaded { format!("{:+.4}", profit_sol) } else { "N/A".to_string() },
            format!("{:.8}", pos.peak_price),
            if price_loaded { format!("{:.1}%", drawdown_pct) } else { "N/A".to_string() },
            pos.dca_count.to_string(),
            format!("{:.1}K", pos.token_amount / 1000.0),
            format!("{:.4}", pos.sol_spent),
            if price_loaded { format!("{:.4}", current_value) } else { "N/A".to_string() },
            age_display,
            status.to_string(),
        ]);
    }

    // Calculate averages
    if open_count > 0 {
        avg_holding_hours /= open_count as f64;
    }
    let avg_drawdown = if open_count > 0 { total_drawdown / (open_count as f64) } else { 0.0 };

    // ── Enhanced summary with comprehensive portfolio metrics ───────────
    let portfolio_pct = if total_invested_sol > 0.0 {
        (total_unrealized_sol / total_invested_sol) * 100.0
    } else {
        0.0
    };

    let win_rate = if open_count > 0 {
        ((winners as f64) / (open_count as f64)) * 100.0
    } else {
        0.0
    };

    let dca_usage_rate = if open_count > 0 {
        ((positions_with_dca as f64) / (open_count as f64)) * 100.0
    } else {
        0.0
    };

    println!("\n🎯 [PORTFOLIO OVERVIEW] ════════════════════════════════════════════");
    println!(
        "📊 Active Positions: {} | Winners: {} ({:.1}%) | Losers: {} ({:.1}%)",
        open_count,
        winners,
        ((winners as f64) / (open_count.max(1) as f64)) * 100.0,
        losers,
        ((losers as f64) / (open_count.max(1) as f64)) * 100.0
    );
    println!(
        "💰 Total Invested: {:.3} SOL | Current Value: {:.3} SOL | P/L: {:+.3} SOL ({:+.1}%)",
        total_invested_sol,
        total_invested_sol + total_unrealized_sol,
        total_unrealized_sol,
        portfolio_pct
    );
    println!(
        "📈 DCA Usage: {:.1}% | Avg Hold Time: {:.1}h | Avg Drawdown: {:.1}%",
        dca_usage_rate,
        avg_holding_hours,
        avg_drawdown
    );

    if !best_performer.0.is_empty() {
        println!(
            "🏆 Best: {} {:+.1}% | 📉 Worst: {} {:+.1}%",
            best_performer.2,
            best_performer.1,
            worst_performer.2,
            worst_performer.1
        );
    }

    // Risk Analysis
    let risk_level = if portfolio_pct < -20.0 {
        "🔴 HIGH RISK"
    } else if portfolio_pct < -10.0 {
        "🟡 MEDIUM RISK"
    } else if portfolio_pct > 10.0 {
        "🟢 PROFITABLE"
    } else {
        "🔵 STABLE"
    };

    println!("🎲 Portfolio Risk: {} | Bot Status: ACTIVE", risk_level);
    println!("════════════════════════════════════════════════════════════════════\n");

    // ── Top 25 Watchlist Tokens ──────────────────────────────────────────────────
    // Make this section optional to prevent blocking when token loading is happening
    match tokio::time::timeout(Duration::from_secs(2), TOKENS.read()).await {
        Ok(tokens_guard) => {
            if !tokens_guard.is_empty() {
                // Filter out excluded tokens from watchlist display
                let blacklist = crate::configs::BLACKLIST.read().await;
                let mut watchlist_tokens: Vec<_> = tokens_guard
                    .iter()
                    .filter(|token| !blacklist.contains(&token.mint))
                    .collect();
                drop(blacklist);

                // Sort by volume (highest first)
                watchlist_tokens.sort_by(|a, b| {
                    let a_vol = a.volume.h24;
                    let b_vol = b.volume.h24;
                    b_vol.partial_cmp(&a_vol).unwrap_or(std::cmp::Ordering::Equal)
                });

                // Take top 25
                watchlist_tokens.truncate(25);

                let mut watchlist_table = Table::new();
                watchlist_table
                    .load_preset(UTF8_FULL)
                    .set_header([
                        "#",
                        "Symbol",
                        "Name",
                        "Price USD",
                        "5m %",
                        "1h %",
                        "24h %",
                        "Volume 24h",
                        "Liquidity",
                        "MCap",
                        "Buys 1h",
                        "Mint Address",
                    ]);

                for (index, token) in watchlist_tokens.iter().enumerate() {
                    let price_usd = token.price_usd.parse::<f64>().unwrap_or(0.0);
                    let volume_24h = token.volume.h24;
                    let liquidity_usd = token.liquidity.usd;
                    let mcap = token.fdv_usd.parse::<f64>().unwrap_or(0.0);
                    let buys_1h = token.txns.h1.buys;

                    // Format price change with colors
                    let change_5m = token.price_change.m5;
                    let change_1h = token.price_change.h1;
                    let change_24h = token.price_change.h24;

                    let format_change = |change: f64| -> String {
                        if change > 0.0 {
                            format!("📈+{:.1}%", change)
                        } else if change < 0.0 {
                            format!("📉{:.1}%", change)
                        } else {
                            "0.0%".to_string()
                        }
                    };

                    // Format large numbers
                    let format_large_number = |value: f64| -> String {
                        if value >= 1_000_000.0 {
                            format!("{:.1}M", value / 1_000_000.0)
                        } else if value >= 1_000.0 {
                            format!("{:.1}K", value / 1_000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    };

                    // Truncate name if too long
                    let display_name = if token.name.len() > 15 {
                        format!("{}...", &token.name[..12])
                    } else {
                        token.name.clone()
                    };

                    watchlist_table.add_row([
                        (index + 1).to_string(),
                        token.symbol.clone(),
                        display_name,
                        if price_usd > 0.0 {
                            format!("${:.6}", price_usd)
                        } else {
                            "N/A".to_string()
                        },
                        format_change(change_5m),
                        format_change(change_1h),
                        format_change(change_24h),
                        format!("${}", format_large_number(volume_24h)),
                        format!("${}", format_large_number(liquidity_usd)),
                        format!("${}", format_large_number(mcap)),
                        buys_1h.to_string(),
                        token.mint.clone(),
                    ]);
                }

                println!("📊 [TOP 25 WATCHLIST TOKENS] ═══════════════════════════════════════");
                println!(
                    "🎯 Sorted by 24h volume (excluded tokens filtered) • Updated from DexScreener & RugCheck APIs"
                );
                println!("══════════════════════════════════════════════════════════════════\n");
                println!("{}\n", watchlist_table);
            } else {
                println!("📊 [WATCHLIST] No tokens loaded yet - waiting for DexScreener data\n");
            }
        }
        Err(_) => {
            // Timeout acquiring TOKENS lock - this is expected during token loading
            println!(
                "📊 [WATCHLIST] Token data being updated - skipping watchlist display this cycle\n"
            );
            println!(
                "🔄 [INFO] DexScreener is currently refreshing token data, watchlist will return next cycle\n"
            );
        }
    }

    if open_count > 0 {
        println!("📋 [OPEN POSITIONS] ────────────────────────────────────────────────");
        println!("{}\n", table);
    } else {
        println!("📭 No open positions - Bot is monitoring for opportunities\n");
    }

    // ── Enhanced recent-closed positions table with comprehensive data ───────────
    if !closed_guard.is_empty() {
        // First, analyze ALL closed positions for comprehensive metrics
        let closed_positions_map: HashMap<String, Position> = closed_guard.clone();
        let mut all_closed_with_mints: Vec<(String, Position)> = closed_positions_map
            .into_iter()
            .collect();
        all_closed_with_mints.sort_by(|a, b|
            b.1.close_time.unwrap_or(b.1.open_time).cmp(&a.1.close_time.unwrap_or(a.1.open_time))
        );

        // Calculate metrics on ALL closed positions
        let mut closed_total_profit = 0.0;
        // closed_winners is already declared at function level
        let mut total_closed_hold_time = 0.0;
        let mut best_closed_trade = ("".to_string(), 0.0, "".to_string());
        let mut worst_closed_trade = ("".to_string(), 0.0, "".to_string());

        // Process ALL closed positions for analysis
        for (mint, pos) in &all_closed_with_mints {
            let profit_pct = if pos.sol_spent > 0.0 {
                ((pos.sol_received - pos.sol_spent) / pos.sol_spent) * 100.0
            } else {
                0.0
            };

            let profit_sol = pos.sol_received - pos.sol_spent;
            closed_total_profit += profit_sol;

            if profit_sol > 0.0 {
                closed_winners += 1;
                if profit_pct > best_closed_trade.1 {
                    best_closed_trade = (
                        mint.clone(),
                        profit_pct,
                        mint[..(8).min(mint.len())].to_string(),
                    );
                }
            } else if profit_pct < worst_closed_trade.1 {
                worst_closed_trade = (
                    mint.clone(),
                    profit_pct,
                    mint[..(8).min(mint.len())].to_string(),
                );
            }

            // Calculate position duration
            if let Some(close_time) = pos.close_time {
                let diff = close_time.signed_duration_since(pos.open_time);
                let hours = diff.num_hours() as f64;
                total_closed_hold_time += hours;
            }
        }

        // Now create table with only the most recent 15 positions for display
        let mut closed_with_mints: Vec<(String, Position)> = all_closed_with_mints.clone();
        closed_with_mints.truncate(15);

        // Reverse the order so most recent appears at the bottom of the table
        closed_with_mints.reverse();

        let mut table_closed = Table::new();
        table_closed
            .load_preset(UTF8_FULL)
            .set_header([
                "#",
                "Mint Address",
                "Entry",
                "Exit",
                "Profit %",
                "P/L SOL",
                "Peak",
                "Max %",
                "Hold Time",
                "DCA",
                "Reason",
                "Closed",
            ]);

        // Populate table with only recent 15 positions (analysis already done above)
        for (index, (mint, pos)) in closed_with_mints.iter().enumerate() {
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

            let profit_sol = pos.sol_received - pos.sol_spent;

            // Calculate max potential gain from peak
            let max_gain_pct = if pos.entry_price > 0.0 {
                ((pos.peak_price - pos.entry_price) / pos.entry_price) * 100.0
            } else {
                0.0
            };

            // Calculate position duration
            let duration = if let Some(close_time) = pos.close_time {
                let diff = close_time.signed_duration_since(pos.open_time);
                let hours = diff.num_hours() as f64;

                if hours < 24.0 {
                    format!("{:.1}h", hours)
                } else {
                    let days = hours / 24.0;
                    format!("{:.1}d", days)
                }
            } else {
                "-".to_string()
            };

            // Full mint address (no shortening)
            let full_mint = mint.clone();

            // Determine close reason based on profit and patterns
            let close_reason = if profit_pct > 20.0 {
                "🎯 TARGET"
            } else if profit_pct > 0.0 {
                "✅ PROFIT"
            } else if profit_pct > -10.0 {
                "🛑 STOP"
            } else if profit_pct > -25.0 {
                "📉 CUT"
            } else {
                "💀 RUG"
            };

            table_closed.add_row([
                (index + 1).to_string(),
                full_mint,
                format!("{:.8}", pos.entry_price),
                format!("{:.8}", close_price),
                format!("{:+.1}%", profit_pct),
                format!("{:+.4}", profit_sol),
                format!("{:.8}", pos.peak_price),
                format!("{:.1}%", max_gain_pct),
                duration,
                pos.dca_count.to_string(),
                close_reason.to_string(),
                pos.close_time.map(format_duration_ago).unwrap_or_else(|| "-".into()),
            ]);
        }

        // Calculate metrics based on ALL closed positions
        let closed_win_rate = if all_closed_with_mints.len() > 0 {
            ((closed_winners as f64) / (all_closed_with_mints.len() as f64)) * 100.0
        } else {
            0.0
        };

        let avg_closed_hold_time = if all_closed_with_mints.len() > 0 {
            total_closed_hold_time / (all_closed_with_mints.len() as f64)
        } else {
            0.0
        };

        println!("📁 [CLOSED POSITIONS ANALYSIS] ════════════════════════════════════");
        println!(
            "📊 Total Closed P/L: {:+.3} SOL | Win Rate: {:.1}% ({}/{}) | Avg Hold: {:.1}h",
            closed_total_profit,
            closed_win_rate,
            closed_winners,
            all_closed_with_mints.len(),
            avg_closed_hold_time
        );

        if !best_closed_trade.0.is_empty() {
            println!(
                "🏆 Best Trade: {} {:+.1}% | 📉 Worst: {} {:+.1}%",
                best_closed_trade.2,
                best_closed_trade.1,
                worst_closed_trade.2,
                worst_closed_trade.1
            );
        }

        println!("═══════════════════════════════════════════════════════════════════");
        println!("📋 Recent 15 Closed Positions (detailed table):");
        println!("{}\n", table_closed);
    }

    // Drop the guards here as we're done with the raw data
    drop(positions_guard);
    drop(closed_guard);

    // ── Bot Analytics & Recommendations ──────────────────────────────────────
    println!("🤖 [BOT ANALYTICS] ═════════════════════════════════════════════════");

    let total_positions = open_count + closed_count;
    let overall_win_rate = if total_positions > 0 {
        let total_winners = winners + closed_winners;
        ((total_winners as f64) / (total_positions as f64)) * 100.0
    } else {
        0.0
    };

    println!(
        "📈 Overall Performance: {}/{} trades | {:.1}% win rate",
        winners + closed_winners,
        total_positions,
        overall_win_rate
    );

    println!("💡 Bot Recommendations:");
    if portfolio_pct < -15.0 {
        println!("   🔴 High losses detected - Consider reducing position sizes");
        println!("   🔴 Review entry criteria - Current strategy may need adjustment");
    } else if portfolio_pct > 15.0 {
        println!("   🟢 Strong performance - Consider scaling up successful strategies");
        println!("   🟢 Current parameters working well");
    } else if win_rate < 40.0 && open_count > 5 {
        println!("   🟡 Low win rate - Monitor entry signals more carefully");
        println!("   🟡 Consider tightening stop-loss criteria");
    } else {
        println!("   🔵 Performance within acceptable range");
        println!("   🔵 Continue current strategy with minor optimizations");
    }

    if avg_holding_hours > 48.0 {
        println!("   ⏰ Long holding times detected - Consider faster profit-taking");
    }

    if dca_usage_rate > 70.0 {
        println!("   💰 High DCA usage - Monitor for better entry timing");
    }

    println!("═══════════════════════════════════════════════════════════════════");
    println!("🔄 Bot Status: ACTIVE | Next scan in progress...\n");

    // ── Price Loading Status Warning ──────────────────────────────────────────
    // Use the position_mints we collected earlier
    if !position_mints.is_empty() {
        let missing_prices = position_mints
            .iter()
            .filter(|mint| { !matches!(get_price_state(mint), PriceState::Loaded(_)) })
            .count();

        if missing_prices > 0 {
            println!(
                "⚠️ [PRICE STATUS] {}/{} positions have missing/invalid prices",
                missing_prices,
                position_mints.len()
            );
            println!("🚫 [TRADING] All buy/sell/DCA actions are BLOCKED until prices are loaded");
            println!(
                "📡 [SYSTEM] Rate limiting active: DexScreener (300/min), GeckoTerminal (30/min)"
            );
        } else {
            println!("✅ [PRICE STATUS] All position prices loaded - Trading system active");
        }
    }

    println!("═══════════════════════════════════════════════════════════════════");

    // Return success
    Ok(())
}

/// Return the `decimals` of a mint account on–chain, with disk cache.
/// Cache is in ".token_decimals_cache.json"
pub fn get_token_decimals(rpc: &RpcClient, mint: &Pubkey) -> Result<u8> {
    let cache_path = ".token_decimals_cache.json";
    let mut cache: HashMap<String, u8> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mint_str = mint.to_string();
    if let Some(&decimals) = cache.get(&mint_str) {
        return Ok(decimals);
    }

    let acc = rpc.get_account(mint)?;
    if acc.owner != spl_token::id() {
        return Err(anyhow!("account {mint} is not an SPL-Token mint"));
    }
    let mint_state = Mint::unpack(&acc.data).map_err(|e| anyhow!("failed to unpack Mint: {e}"))?;
    let decimals = mint_state.decimals;

    cache.insert(mint_str, decimals);

    // Write cache (ignore errors)
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });
    println!("🔍 Fetching decimals for mint: {mint}");
    println!("🔍 Account owner: {}", acc.owner);
    println!("🔍 Mint state decimals: {decimals}");
    Ok(decimals)
}

/// Return the `mint` address of a token account, with disk cache.
/// Cache is in ".token_account_mint_cache.json"
pub fn get_token_account_mint(rpc: &RpcClient, token_account: &Pubkey) -> Result<Pubkey> {
    let cache_path = ".token_account_mint_cache.json";
    let mut cache: HashMap<String, String> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let acct_str = token_account.to_string();
    if let Some(mint_str) = cache.get(&acct_str) {
        return Pubkey::from_str(mint_str).map_err(|e| anyhow!("Invalid cached mint: {e}"));
    }

    let acc = rpc.get_account(token_account)?;
    if acc.owner != spl_token::id() {
        return Err(anyhow!("account {token_account} is not an SPL-Token account"));
    }
    let account_state = Account::unpack(&acc.data).map_err(|e|
        anyhow!("failed to unpack Token Account: {e}")
    )?;
    let mint = account_state.mint;

    cache.insert(acct_str, mint.to_string());
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });

    Ok(mint)
}

// Cache: token_mint -> (timestamp_secs, price)
pub static PRICE_CACHE: Lazy<RwLock<HashMap<String, (u64, f64)>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// Pull every *Solana* `pairAddress` for the given token mint from DexScreener, with 2h cache.
pub async fn fetch_solana_pairs(token_mint: &str) -> Result<Vec<Pubkey>> {
    let cache_path = ".solana_pairs_cache.json";
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let expire_secs = 2 * 3600;

    // {mint: [timestamp, [array of pair pubkeys as string]]}
    let mut cache: HashMap<String, (u64, Vec<String>)> = if Path::new(cache_path).exists() {
        fs::read_to_string(cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Check cache
    if let Some((ts, pairs)) = cache.get(token_mint) {
        if now < *ts + expire_secs {
            return Ok(
                pairs
                    .iter()
                    .filter_map(|s| Pubkey::from_str(s).ok())
                    .collect()
            );
        }
    }

    // Fetch fresh from DexScreener
    println!("🔄 Fetching pools from DexScreener...");
    let pools = fetch_combined_pools(token_mint).await?;

    if pools.is_empty() {
        bail!("No Solana pools found for mint {} from DexScreener", token_mint);
    }

    let addresses: Vec<String> = pools
        .iter()
        .map(|p| p.address.clone())
        .collect();

    // Update cache
    cache.insert(token_mint.to_string(), (now, addresses.clone()));
    let _ = fs::File::create(cache_path).and_then(|mut f| {
        let s = serde_json::to_string(&cache).unwrap();
        f.write_all(s.as_bytes())
    });

    // Return as Vec<Pubkey>
    Ok(
        addresses
            .iter()
            .filter_map(|s| Pubkey::from_str(s).ok())
            .collect()
    )
}

/// Return the effective price you actually paid (SOL per token)
/// and print full debug info.
///
/// Change `SWAP_FEE_SOL` below if you want a different hard-coded fee.
pub fn effective_swap_price(
    rpc: &RpcClient,
    tx_sig_str: &str,
    wallet: &Pubkey,
    token_mint: &Pubkey,
    lamports_in: u64
) -> Result<f64> {
    // ───── hard-coded fee you want to deduct ─────
    const SWAP_FEE_SOL: f64 = 0.000005; // <── tweak here
    const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
    let fee_lamports = (SWAP_FEE_SOL * LAMPORTS_PER_SOL) as u64;
    // ---------------------------------------------

    println!("🔍 fetching tx {tx_sig_str}");
    let sig = Signature::from_str(tx_sig_str)?;

    let tx = rpc.get_transaction_with_config(&sig, RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    })?;
    let meta = tx.transaction.meta.ok_or_else(|| anyhow!("transaction meta missing"))?;

    // ---- helpers ----------------------------------------------------------
    fn opt_ref<'a, T>(os: &'a OptionSerializer<T>) -> Option<&'a T> {
        Option::<&T>::from(os.as_ref())
    }
    fn balance(
        list: &OptionSerializer<Vec<UiTransactionTokenBalance>>,
        owner: &Pubkey,
        mint: &Pubkey
    ) -> f64 {
        opt_ref(list)
            .and_then(|v| {
                v.iter().find(|b| {
                    b.mint == mint.to_string() && opt_ref(&b.owner) == Some(&owner.to_string())
                })
            })
            .and_then(|b| b.ui_token_amount.ui_amount)
            .unwrap_or(0.0)
    }
    // -----------------------------------------------------------------------

    let pre = balance(&meta.pre_token_balances, wallet, token_mint);
    let post = balance(&meta.post_token_balances, wallet, token_mint);
    let delta = post - pre;

    println!("🧮 balances  → pre: {pre}, post: {post}, delta: {delta}");

    if delta <= 0.0 {
        return Err(anyhow!("no token balance increase detected"));
    }

    // subtract fee before computing price
    let effective_lamports = lamports_in.saturating_sub(fee_lamports);
    let sol_spent = (effective_lamports as f64) / LAMPORTS_PER_SOL;
    let price = sol_spent / delta;

    println!(
        "💰 lamports_in: {lamports_in}  (= {:.9} SOL)",
        (lamports_in as f64) / LAMPORTS_PER_SOL
    );
    println!("💸 minus fee  : {fee_lamports} (= {SWAP_FEE_SOL:.9} SOL)");
    println!("💰 spent used : {effective_lamports} (= {sol_spent:.9} SOL)");
    println!("📈 effective price: {price:.12} SOL per token");

    Ok(price)
}

pub fn install_sigint_handler() -> Result<()> {
    // plain Ctrl‑C (works cross‑platform)
    ctrlc::set_handler(|| {
        SHUTDOWN.store(true, Ordering::SeqCst);
    })?;

    // Spawn async SIGTERM listener for Unix
    #[cfg(unix)]
    {
        use tokio::signal::unix::{ signal, SignalKind };
        tokio::spawn(async {
            let mut sigterm = signal(SignalKind::terminate()).expect(
                "cannot install SIGTERM handler"
            );
            sigterm.recv().await;
            SHUTDOWN.store(true, Ordering::SeqCst);
        });
    }

    Ok(())
}

pub fn format_duration_ago(from: DateTime<Utc>) -> String {
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

/// Format duration as current age (not "ago") with support for days, hours, and minutes
pub fn format_position_age(from: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(from);

    let days = diff.num_days();
    let hours = diff.num_hours() % 24;
    let minutes = diff.num_minutes() % 60;

    if days > 0 {
        if hours > 0 { format!("{}d {}h", days, hours) } else { format!("{}d", days) }
    } else if hours > 0 {
        if minutes > 0 { format!("{}h {}m", hours, minutes) } else { format!("{}h", hours) }
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", diff.num_seconds().max(0))
    }
}

/// Waits until either Ctrl‑C (SIGINT) or SIGTERM (from systemd) is received.
pub async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{ signal, SignalKind };
        let mut term = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv()             => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

// Global set for permanently skipped tokens (now async Mutex)
pub static SKIPPED_SELLS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| {
    // Initialize from file, can't do async here, so use blocking
    let mut set = HashSet::new();
    if let Ok(file) = File::open(".skipped_sells") {
        for line in BufReader::new(file).lines().flatten() {
            set.insert(line.trim().to_string());
        }
    }
    Mutex::new(set)
});

pub async fn add_skipped_sell(mint: &str) {
    {
        let mut set = SKIPPED_SELLS.lock().await;
        if set.insert(mint.to_string()) {
            // File I/O must be blocking (but this is rare)
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(".skipped_sells") {
                let _ = writeln!(f, "{mint}");
            }
        }
    }
}

/// Fetch pools from DexScreener API and convert to PoolInfo format
pub async fn fetch_dexscreener_pools(token_mint: &str) -> Result<Vec<PoolInfo>> {
    let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", token_mint);
    println!("📊 Fetching DexScreener pools...");
    let client = reqwest::Client::new(); // Use async client
    let response = client.get_with_rate_limit(&url, &DEXSCREENER_LIMITER).await?;
    let json: Value = response.json().await?;

    let mut pools = Vec::new();
    if let Some(arr) = json.get("pairs").and_then(|v| v.as_array()) {
        for p in arr {
            if p.get("chainId").and_then(|v| v.as_str()) != Some("solana") {
                continue;
            }
            if let Some(addr) = p.get("pairAddress").and_then(|v| v.as_str()) {
                let name = p
                    .get("baseToken")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|base| {
                        let quote = p
                            .get("quoteToken")
                            .and_then(|v| v.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown");
                        format!("{} / {}", base, quote)
                    });

                let liquidity_usd = p
                    .get("liquidity")
                    .and_then(|v| v.get("usd"))
                    .and_then(|v| v.as_f64());

                let volume_24h_usd = p
                    .get("volume")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| v.as_f64());

                let tx_count_24h = p
                    .get("txns")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| {
                        let buys = v
                            .get("buys")
                            .and_then(|b| b.as_u64())
                            .unwrap_or(0);
                        let sells = v
                            .get("sells")
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                        Some(buys + sells)
                    });

                pools.push(PoolInfo {
                    address: addr.to_string(),
                    source: "dexscreener".to_string(),
                    name,
                    liquidity_usd,
                    volume_24h_usd,
                    tx_count_24h,
                });
            }
        }
    }

    println!("📊 Found {} pools from DexScreener", pools.len());
    Ok(pools)
}

/// Fetch pools from DexScreener only
pub async fn fetch_combined_pools(token_mint: &str) -> Result<Vec<PoolInfo>> {
    let mut all_pools = Vec::new();
    let mut seen_addresses = HashSet::new();

    // Fetch from DexScreener
    match fetch_dexscreener_pools(token_mint).await {
        Ok(dex_pools) => {
            for pool in dex_pools {
                if seen_addresses.insert(pool.address.clone()) {
                    all_pools.push(pool);
                }
            }
        }
        Err(e) => println!("⚠️ DexScreener fetch failed: {}", e),
    }

    // Sort by liquidity (highest first), then by volume
    all_pools.sort_by(|a, b| {
        let a_liq = a.liquidity_usd.unwrap_or(0.0);
        let b_liq = b.liquidity_usd.unwrap_or(0.0);
        let a_vol = a.volume_24h_usd.unwrap_or(0.0);
        let b_vol = b.volume_24h_usd.unwrap_or(0.0);

        b_liq
            .partial_cmp(&a_liq)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b_vol.partial_cmp(&a_vol).unwrap_or(std::cmp::Ordering::Equal))
    });

    println!("� Found {} pools from DexScreener", all_pools.len());

    // Print summary of pools found
    for (i, pool) in all_pools.iter().take(5).enumerate() {
        println!(
            "  {}. {} [{}] - Liq: ${:.0} Vol: ${:.0} Txs: {}",
            i + 1,
            pool.name.as_ref().unwrap_or(&"Unknown".to_string()),
            pool.source,
            pool.liquidity_usd.unwrap_or(0.0),
            pool.volume_24h_usd.unwrap_or(0.0),
            pool.tx_count_24h.unwrap_or(0)
        );
    }

    Ok(all_pools)
}

/// Updated function that returns PoolInfo structs instead of just Pubkeys
pub async fn fetch_solana_pools_detailed(token_mint: &str) -> Result<Vec<PoolInfo>> {
    fetch_combined_pools(token_mint).await
}
