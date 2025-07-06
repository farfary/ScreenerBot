#![allow(warnings)]

use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::Ordering;
use reqwest::Client;
use crate::configs::BLACKLIST;
use crate::utilitis::*;

#[derive(Debug, Clone)]
pub struct Token {
    pub mint: String,
    pub balance: String,
    pub ata_pubkey: String,
    pub program_id: String,
    pub name: String,
    pub symbol: String,
    pub price_native: String, // Store native SOL price here
    pub price_usd: String,
    pub last_price_usd: String,
    pub volume_usd: String,
    pub fdv_usd: String,
    pub image_url: String,
}

// ────────────── GLOBAL STATIC ──────────────
pub static TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| RwLock::new(Vec::new()));

// ────────────── CHUNK HELPER ──────────────
fn chunked(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// Required import:
use colored::Colorize;

pub fn start_dexscreener_loop() {
    const MAX_TOKENS: usize = 80;
    // check debug flag from cached args
    let debug = crate::configs::ARGS.iter().any(|a| a == "--debug-dexscreener");

    let client = Client::new();
    let client_insert = client.clone();

    tokio::spawn(async move {
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) { break }

            // minimal start/stop unless debug
            if debug {
                println!("\n🔄 [Screener] Fetching DexScreener token lists...");
            }

            let mut new_tokens: Vec<Token> = Vec::new();
            let endpoints = [
                "https://api.dexscreener.com/token-profiles/latest/v1",
                "https://api.dexscreener.com/token-boosts/latest/v1",
                "https://api.dexscreener.com/token-boosts/top/v1",
            ];

            /* ── first-pass lists ─────────────────────────────────────────── */
            for url in endpoints {
                if debug {
                    println!("🌐 [Screener] Requesting: {url}");
                }
                if let Ok(resp) = client_insert.get(url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            if debug {
                                println!("✅ {arr_len} tokens from {url}", arr_len = arr.len());
                            }
                            for item in arr {
                                let mint = item["tokenAddress"].as_str().unwrap_or_default();
                                if BLACKLIST.read().await.contains(mint) {
                                    continue;
                                }
                                new_tokens.push(Token {
                                    mint: mint.to_string(),
                                    balance: "0".into(),
                                    ata_pubkey: item["url"].as_str().unwrap_or_default().into(),
                                    program_id: item["chainId"].as_str().unwrap_or_default().into(),
                                    name: "".into(),
                                    symbol: "".into(),
                                    price_native: "".into(),
                                    price_usd: "".into(),
                                    last_price_usd: "".into(),
                                    volume_usd: "".into(),
                                    fdv_usd: "".into(),
                                    image_url: "".into(),
                                });
                            }
                        }
                    }
                }
            }

            /* ── second-pass details ──────────────────────────────────────── */
            let mints: Vec<String> = new_tokens.iter().map(|t| t.mint.clone()).collect();
            let batches = chunked(mints, 30);

            for (i, batch) in batches.iter().enumerate() {
                let joined = batch.join(",");
                let url = format!("https://api.dexscreener.com/tokens/v1/solana/{joined}");
                if debug {
                    println!("🔗 DexScreener info (batch {}/{})", i + 1, batches.len());
                }
                if let Ok(resp) = client_insert.get(&url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                let mint = item["baseToken"]["address"]
                                    .as_str()
                                    .unwrap_or_default();
                                if let Some(tok) = new_tokens.iter_mut().find(|t| t.mint == mint) {
                                    tok.name = item["baseToken"]["name"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                    tok.symbol = item["baseToken"]["symbol"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                    tok.price_native = item["priceNative"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                    tok.price_usd = item["priceUsd"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                    tok.volume_usd = item["volume"]["h24"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                    tok.fdv_usd = item["fdv"]
                                        .as_f64()
                                        .unwrap_or_default()
                                        .to_string();
                                    tok.image_url = item["info"]["imageUrl"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into();
                                }
                            }
                        }
                    }
                }
            }

            /* ── keep top-N and push into shared TOKENS ──────────────────── */
            new_tokens.sort_unstable_by(|a, b| {
                let va = a.volume_usd.parse::<f64>().unwrap_or(0.0);
                let vb = b.volume_usd.parse::<f64>().unwrap_or(0.0);
                vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
            });
            new_tokens.truncate(MAX_TOKENS);

            // Capture existing mints to only log truly new ones
            let existing_mints: Vec<String> = {
                let lock = TOKENS.read().await;
                lock.iter().map(|t| t.mint.clone()).collect()
            };

            // Filter and log only new additions
            let mut filtered_tokens = Vec::with_capacity(MAX_TOKENS);
            for t in new_tokens {
                if BLACKLIST.read().await.contains(&t.mint) {
                    continue;
                }
                if !t.symbol.is_empty() && !t.price_usd.is_empty() && !t.name.is_empty() {
                    if !existing_mints.contains(&t.mint) {
                        if debug {
                            // full info as before
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
                                t.image_url.underline().white(),
                            );
                        } else {
                            // minimal added-token log
                            println!("🆕 Added: {} ({})", t.symbol, t.mint);
                        }
                    }
                    filtered_tokens.push(t);
                }
            }

            // Replace TOKENS with the new list
            {
                let mut lock = TOKENS.write().await;
                lock.clear();
                lock.extend(filtered_tokens);
            }

            // always show this stop line
            println!("✅ TOKENS updated: {}", TOKENS.read().await.len());

            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}


