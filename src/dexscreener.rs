#![allow(warnings)]

use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::Ordering;
use reqwest::Client;
use colored::Colorize;
use crate::configs::{BLACKLIST, ARGS};
use crate::utilitis::*;

const MAX_TOKENS: usize = 20;
const MIN_PRICE_SOL: f64 = 0.0000001;
const MAX_PRICE_SOL: f64 = 0.01;

#[derive(Debug, Clone)]
struct TxnCount {
    buys: u64,
    sells: u64,
}

#[derive(Debug, Clone)]
struct Txns {
    m5: TxnCount,
    h1: TxnCount,
    h6: TxnCount,
    h24: TxnCount,
}

#[derive(Debug, Clone)]
struct Volume {
    m5: f64,
    h1: f64,
    h6: f64,
    h24: f64,
}

#[derive(Debug, Clone)]
struct PriceChange {
    m5: f64,
    h1: f64,
    h6: f64,
    h24: f64,
}

#[derive(Debug, Clone)]
struct Liquidity {
    usd: f64,
    base: f64,
    quote: f64,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub mint: String,
    pub balance: String,
    pub ata_pubkey: String,
    pub program_id: String,
    pub name: String,
    pub symbol: String,
    pub dex_id: String,
    pub url: String,
    pub pair_address: String,
    pub labels: Vec<String>,
    pub quote_address: String,
    pub quote_name: String,
    pub quote_symbol: String,
    pub price_native: String,
    pub price_usd: String,
    pub last_price_usd: String,
    pub volume_usd: String,
    pub fdv_usd: String,
    pub image_url: String,
    pub txns: Txns,
    pub volume: Volume,
    pub price_change: PriceChange,
    pub liquidity: Liquidity,
    pub pair_created_at: u64,
}

// ────────────── GLOBAL STATIC ──────────────
pub static TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| RwLock::new(Vec::new()));

// ────────────── CHUNK HELPER ──────────────
fn chunked(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub fn start_dexscreener_loop() {
    let debug = ARGS.iter().any(|a| a == "--debug-dexscreener");
    let client = Client::new();
    let client_insert = client.clone();

    tokio::spawn(async move {
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }

            if debug {
                println!("\n🔄 [Screener] Fetching DexScreener token lists...");
            }

            let mut new_tokens: Vec<Token> = Vec::new();
            let endpoints = [
                "https://api.dexscreener.com/token-profiles/latest/v1",
                "https://api.dexscreener.com/token-boosts/latest/v1",
                "https://api.dexscreener.com/token-boosts/top/v1",
            ];

            // ── first-pass lists ───────────────────────────────────────────
            for url in endpoints {
                if debug {
                    println!("🌐 [Screener] Requesting: {}", url);
                }
                if let Ok(resp) = client_insert.get(url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            if debug {
                                println!("✅ {} tokens from {}", arr.len(), url);
                            }
                            for item in arr {
                                let mint = item["tokenAddress"].as_str().unwrap_or_default().to_string();
                                if BLACKLIST.read().await.contains(&mint) {
                                    continue;
                                }
                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".into(),
                                    ata_pubkey: item["url"].as_str().unwrap_or_default().into(),
                                    program_id: item["chainId"].as_str().unwrap_or_default().into(),
                                    name: "".into(),
                                    symbol: "".into(),
                                    dex_id: String::new(),
                                    url: String::new(),
                                    pair_address: String::new(),
                                    labels: Vec::new(),
                                    quote_address: String::new(),
                                    quote_name: String::new(),
                                    quote_symbol: String::new(),
                                    price_native: String::new(),
                                    price_usd: String::new(),
                                    last_price_usd: String::new(),
                                    volume_usd: String::new(),
                                    fdv_usd: String::new(),
                                    image_url: String::new(),
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
                                });
                            }
                        }
                    }
                }
            }

            // ── second-pass details ────────────────────────────────────────
            let mints: Vec<String> = new_tokens.iter().map(|t| t.mint.clone()).collect();
            let batches = chunked(mints, 30);

            for (i, batch) in batches.iter().enumerate() {
                let joined = batch.join(",");
                let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", joined);
                if debug {
                    println!("🔗 DexScreener info (batch {}/{})", i + 1, batches.len());
                }
                if let Ok(resp) = client_insert.get(&url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                let base_address = item["baseToken"]["address"].as_str().unwrap_or_default();
                                if let Some(tok) = new_tokens.iter_mut().find(|t| t.mint == base_address) {
                                    tok.dex_id = item["dexId"].as_str().unwrap_or_default().into();
                                    tok.url = item["url"].as_str().unwrap_or_default().into();
                                    tok.pair_address = item["pairAddress"].as_str().unwrap_or_default().into();
                                    tok.labels = item["labels"]
                                        .as_array()
                                        .unwrap_or(&vec![])
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect();
                                    tok.name = item["baseToken"]["name"].as_str().unwrap_or_default().into();
                                    tok.symbol = item["baseToken"]["symbol"].as_str().unwrap_or_default().into();
                                    tok.quote_address = item["quoteToken"]["address"].as_str().unwrap_or_default().into();
                                    tok.quote_name = item["quoteToken"]["name"].as_str().unwrap_or_default().into();
                                    tok.quote_symbol = item["quoteToken"]["symbol"].as_str().unwrap_or_default().into();
                                    tok.price_native = item["priceNative"].as_str().unwrap_or_default().into();
                                    tok.price_usd = item["priceUsd"].as_str().unwrap_or_default().into();
                                    tok.volume_usd = item["volume"]["h24"].as_f64().unwrap_or(0.0).to_string();
                                    tok.fdv_usd = item["fdv"].as_f64().unwrap_or(0.0).to_string();
                                    tok.image_url = item["info"]["imageUrl"].as_str().unwrap_or_default().into();
                                    tok.txns = Txns {
                                        m5: TxnCount {
                                            buys: item["txns"]["m5"]["buys"].as_u64().unwrap_or(0),
                                            sells: item["txns"]["m5"]["sells"].as_u64().unwrap_or(0),
                                        },
                                        h1: TxnCount {
                                            buys: item["txns"]["h1"]["buys"].as_u64().unwrap_or(0),
                                            sells: item["txns"]["h1"]["sells"].as_u64().unwrap_or(0),
                                        },
                                        h6: TxnCount {
                                            buys: item["txns"]["h6"]["buys"].as_u64().unwrap_or(0),
                                            sells: item["txns"]["h6"]["sells"].as_u64().unwrap_or(0),
                                        },
                                        h24: TxnCount {
                                            buys: item["txns"]["h24"]["buys"].as_u64().unwrap_or(0),
                                            sells: item["txns"]["h24"]["sells"].as_u64().unwrap_or(0),
                                        },
                                    };
                                    tok.volume = Volume {
                                        m5: item["volume"]["m5"].as_f64().unwrap_or(0.0),
                                        h1: item["volume"]["h1"].as_f64().unwrap_or(0.0),
                                        h6: item["volume"]["h6"].as_f64().unwrap_or(0.0),
                                        h24: item["volume"]["h24"].as_f64().unwrap_or(0.0),
                                    };
                                    tok.price_change = PriceChange {
                                        m5: item["priceChange"]["m5"].as_f64().unwrap_or(0.0),
                                        h1: item["priceChange"]["h1"].as_f64().unwrap_or(0.0),
                                        h6: item["priceChange"]["h6"].as_f64().unwrap_or(0.0),
                                        h24: item["priceChange"]["h24"].as_f64().unwrap_or(0.0),
                                    };
                                    tok.liquidity = Liquidity {
                                        usd: item["liquidity"]["usd"].as_f64().unwrap_or(0.0),
                                        base: item["liquidity"]["base"].as_f64().unwrap_or(0.0),
                                        quote: item["liquidity"]["quote"].as_f64().unwrap_or(0.0),
                                    };
                                    tok.pair_created_at = item["pairCreatedAt"].as_u64().unwrap_or(0);
                                }
                            }
                        }
                    }
                }
            }

            for t in &new_tokens {
                println!(
                    "Symbol: {}, Price: {}, Volume: {}, FDV: {}",
                    t.symbol,
                    t.price_native,
                    t.volume_usd,
                    t.fdv_usd
                );
            }

            const MIN_VOLUME_USD: f64 = 2000.0;
            const MIN_FDV_USD: f64 = 10000.0;
            const MAX_FDV_USD: f64 = 10_000_000.0;

            new_tokens.retain(|t| {
                let price = t.price_native.parse::<f64>().unwrap_or(0.0);
                let vol = t.volume_usd.parse::<f64>().unwrap_or(0.0);
                let fdv = t.fdv_usd.parse::<f64>().unwrap_or(0.0);
                price >= MIN_PRICE_SOL &&
                price <= MAX_PRICE_SOL &&
                vol >= MIN_VOLUME_USD &&
                fdv >= MIN_FDV_USD &&
                fdv <= MAX_FDV_USD &&
                !t.symbol.is_empty() &&
                !t.name.is_empty() &&
                !t.price_usd.is_empty()
            });

            if debug {
                println!("✅ {} tokens remain after price filter", new_tokens.len());
            }

            new_tokens.sort_unstable_by(|a, b| {
                let va = a.volume_usd.parse::<f64>().unwrap_or(0.0);
                let vb = b.volume_usd.parse::<f64>().unwrap_or(0.0);
                vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
            });
            new_tokens.truncate(MAX_TOKENS);

            let existing_mints: Vec<String> = {
                let lock = TOKENS.read().await;
                lock.iter().map(|t| t.mint.clone()).collect()
            };

            let mut filtered_tokens = Vec::with_capacity(MAX_TOKENS);
            for t in new_tokens {
                if BLACKLIST.read().await.contains(&t.mint) {
                    continue;
                }
                if !t.symbol.is_empty() && !t.price_usd.is_empty() && !t.name.is_empty() {
                    if !existing_mints.contains(&t.mint) {
                        if debug {
                            println!(
                                "{} {} {}\n  {} {}\n  {} {}\n  {} {} SOL\n  {} ${}\n  {} ${}\n  {} ${}\n  {} {}\n  {} {}\n  {} {}",
                                "🆕".bold(),
                                t.symbol.green().bold(),
                                "-".normal(),
                                "Name:".blue().bold(),
                                t.name.white(),
                                "Mint:".blue().bold(),
                                t.mint.dimmed(),
                                "Native:".blue().bold(),
                                t.price_native.cyan().bold(),
                                "USD:".blue().bold(),
                                t.price_usd.yellow().bold(),
                                "Volume24h:".blue().bold(),
                                t.volume_usd.magenta().bold(),
                                "FDV:".blue().bold(),
                                t.fdv_usd.blue().bold(),
                                "ATA:".blue().bold(),
                                t.ata_pubkey.dimmed(),
                                "ProgramID:".blue().bold(),
                                t.program_id.dimmed(),
                                "ImageURL:".blue().bold(),
                                t.image_url.underline().white()
                            );
                        } else {
                            println!("🆕 Added: {} ({})", t.symbol, t.mint);
                        }
                    }
                    filtered_tokens.push(t);
                }
            }

            {
                let mut lock = TOKENS.write().await;
                lock.clear();
                lock.extend(filtered_tokens);
            }

            println!("✅ TOKENS updated: {}", TOKENS.read().await.len());
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}
