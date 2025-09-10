//! Raydium CLMM specific decoder debugging tool
//!
//! This tool mirrors the Orca Whirlpool debugger but for Raydium CLMM pools.
//! It fetches the pool and vault accounts, decodes key fields, computes price
//! using sqrt_price_x64 (+ decimals), runs the main decoder in both orientations,
//! and compares the result to GeckoTerminal API (native SOL and quote prices).
//!
//! Usage:
//!   cargo run --bin debug_raydium_clmm_specific -- --pool <POOL_ADDRESS>

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{ log, LogTag };
use screenerbot::pools::decoders::raydium_clmm::RaydiumClmmDecoder;
use screenerbot::pools::decoders::PoolDecoder;
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{ RAYDIUM_CLMM_PROGRAM_ID, SOL_MINT };
use screenerbot::rpc::{ get_rpc_client, parse_pubkey };
use screenerbot::tokens::{ decimals::SOL_DECIMALS, get_token_decimals_sync };
use screenerbot::tokens::dexscreener::{ init_dexscreener_api, get_global_dexscreener_api };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
    name = "debug_raydium_clmm_specific",
    about = "Debug a Raydium CLMM pool with verbose parsing and price checks"
)]
struct Args {
    /// Pool address to debug
    #[arg(short, long)]
    pool: String,

    /// Show initial bytes as hex
    #[arg(long)]
    show_hex: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.pool.is_empty() {
        eprintln!("--pool is required");
        std::process::exit(1);
    }

    // Enable decoder debug logs from library
    set_cmd_args(
        vec!["debug_raydium_clmm_specific".to_string(), "--debug-pool-decoders".to_string()]
    );

    println!("\n🔍 RAYDIUM CLMM SPECIFIC DEBUGGER");
    println!("=================================");
    println!("Pool address: {}", args.pool);

    // Init RPC
    log(LogTag::System, "INIT", "Initializing RPC client...");
    if let Err(e) = screenerbot::rpc::init_rpc_client() {
        eprintln!("RPC init failed: {}", e);
        std::process::exit(1);
    }

    // Fetch pool account
    let rpc = get_rpc_client();
    let pool_pk = parse_pubkey(&args.pool).map_err(|e| format!("invalid pool pubkey: {}", e))?;
    let pool_acc = rpc
        .client()
        .get_account(&pool_pk)
        .map_err(|e| format!("failed to get pool account: {}", e))?;

    println!("\n📦 POOL ACCOUNT");
    println!("==============");
    println!("Owner: {}", pool_acc.owner);
    println!("Data size: {} bytes", pool_acc.data.len());
    println!("Owner is Raydium CLMM: {}", if pool_acc.owner.to_string() == RAYDIUM_CLMM_PROGRAM_ID {
        "✅"
    } else {
        "❌"
    });

    if args.show_hex {
        println!("\n📄 RAW HEX (first 192 bytes)");
        for (i, chunk) in pool_acc.data.chunks(16).take(12).enumerate() {
            print!("{:04x}: ", i * 16);
            for b in chunk {
                print!("{:02x} ", b);
            }
            println!();
        }
    }

    // Parse key fields using the same offsets as decoder.parse_clmm_pool
    let parsed = parse_clmm_minimal(&pool_acc.data);
    if parsed.is_none() {
        eprintln!("Failed to parse CLMM pool (size or offsets mismatch)");
        std::process::exit(1);
    }
    let info = parsed.unwrap();

    println!("\n🔧 PARSED FIELDS");
    println!("================");
    println!("token_mint_0: {}", info.token_mint_0);
    println!("token_mint_1: {}", info.token_mint_1);
    println!("token_vault_0: {}", info.token_vault_0);
    println!("token_vault_1: {}", info.token_vault_1);
    println!("mint_decimals_0: {}", info.mint_decimals_0);
    println!("mint_decimals_1: {}", info.mint_decimals_1);
    println!("tick_spacing: {}", info.tick_spacing);
    println!("sqrt_price_x64: {}", info.sqrt_price_x64);
    println!("tick_current: {}", info.tick_current);

    // Fetch vault accounts
    println!("\n🏦 FETCHING VAULT ACCOUNTS\n==========================");
    let (vault0_pk, vault1_pk) = (
        parse_pubkey(&info.token_vault_0)?,
        parse_pubkey(&info.token_vault_1)?,
    );
    let vault0_acc = rpc.client().get_account(&vault0_pk)?;
    let vault1_acc = rpc.client().get_account(&vault1_pk)?;

    let (amt0, amt1) = (
        decode_token_amount(&vault0_acc.data).unwrap_or(0),
        decode_token_amount(&vault1_acc.data).unwrap_or(0),
    );

    println!("vault_0 mint: {}", read_pubkey(&vault0_acc.data, 0).unwrap_or_default());
    println!("vault_1 mint: {}", read_pubkey(&vault1_acc.data, 0).unwrap_or_default());
    println!("vault_0 amount(raw): {}", amt0);
    println!("vault_1 amount(raw): {}", amt1);

    // Build accounts map for decoder test
    let mut accounts = HashMap::new();
    accounts.insert(args.pool.clone(), AccountData {
        pubkey: pool_pk,
        data: pool_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: pool_acc.lamports,
        owner: pool_acc.owner,
    });
    accounts.insert(info.token_vault_0.clone(), AccountData {
        pubkey: vault0_pk,
        data: vault0_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: vault0_acc.lamports,
        owner: vault0_acc.owner,
    });
    accounts.insert(info.token_vault_1.clone(), AccountData {
        pubkey: vault1_pk,
        data: vault1_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: vault1_acc.lamports,
        owner: vault1_acc.owner,
    });

    // Decide orientation relative to SOL (if present)
    let has_sol = info.token_mint_0 == SOL_MINT || info.token_mint_1 == SOL_MINT;
    let target_mint = if has_sol {
        if info.token_mint_0 == SOL_MINT { &info.token_mint_1 } else { &info.token_mint_0 }
    } else {
        // Fallback: use token_mint_0 for target when no SOL in pair
        &info.token_mint_0
    };

    // Run decoder in both orientations
    println!("\n🧪 DECODER CHECK\n================");
    let res1 = RaydiumClmmDecoder::decode_and_calculate(&accounts, target_mint, &SOL_MINT);
    match &res1 {
        Some(r) =>
            println!(
                "Orientation TOKEN/SOL → price_sol={:.12} mint={} pool={}",
                r.price_sol,
                r.mint,
                r.pool_address
            ),
        None => println!("Orientation TOKEN/SOL → None"),
    }
    let res2 = RaydiumClmmDecoder::decode_and_calculate(&accounts, &SOL_MINT, target_mint);
    match &res2 {
        Some(r) =>
            println!(
                "Orientation SOL/TOKEN → price_sol={:.12} mint={} pool={}",
                r.price_sol,
                r.mint,
                r.pool_address
            ),
        None => println!("Orientation SOL/TOKEN → None"),
    }

    // Manual price from sqrt_price_x64
    println!("\n🧮 MANUAL SQRT PRICE\n====================");
    let q64 = (2_f64).powi(64);
    let sqrt_p = (info.sqrt_price_x64 as f64) / q64;
    let raw = sqrt_p * sqrt_p; // token1/token0
    println!("sqrt_price: {}", sqrt_p);
    println!("raw price (t1/t0): {}", raw);

    let (token0_dec, token1_dec) = (
        get_token_decimals_sync(&info.token_mint_0).unwrap_or(info.mint_decimals_0 as u8),
        get_token_decimals_sync(&info.token_mint_1).unwrap_or(info.mint_decimals_1 as u8),
    );
    println!("decimals (cached or pool): token0={} token1={}", token0_dec, token1_dec);

    // Compute base token (token_mint_0) price in: quote token units, and SOL if available
    let price_quote_per_base = raw * (10_f64).powi((token0_dec as i32) - (token1_dec as i32));
    println!(
        "base_token_price_quote_token (decoded): {:.12} (quote per base)",
        price_quote_per_base
    );

    // If SOL is in pair, compute SOL per target mint (non-SOL side)
    if has_sol {
        let price_sol = if info.token_mint_1 == SOL_MINT {
            // t1=SOL, t0=token → raw is SOL/token
            raw * (10_f64).powi((token0_dec as i32) - (SOL_DECIMALS as i32))
        } else {
            // t0=SOL, t1=token → raw is token/SOL, invert
            (1.0 / raw) *
                (10_f64).powi(
                    ((if info.token_mint_1 == SOL_MINT { token0_dec } else { token1_dec }) as i32) -
                        (SOL_DECIMALS as i32)
                )
        };
        println!("SOL per target (manual): {:.12}", price_sol);
    } else {
        println!("Pool has no SOL side; decoder will skip in main service (single-pool SOL mode)");
    }

    // DexScreener API diff (SOL price)
    println!("\n📊 DEXSCREENER DIFF (SOL)\n========================");
    if has_sol {
        // Initialize DexScreener API once
        if let Err(e) = init_dexscreener_api().await {
            eprintln!("Failed to init DexScreener API: {}", e);
        } else if let Ok(api_arc) = get_global_dexscreener_api().await {
            let mut api = api_arc.lock().await;
            match api.get_token_data(target_mint).await {
                Ok(Some(api_token)) => {
                    if let Some(api_price_sol) = api_token.price_sol {
                        // Choose decoder result (prefer TOKEN/SOL orientation)
                        let dec = res1.as_ref().or(res2.as_ref());
                        if let Some(best) = dec {
                            let decoded = best.price_sol;
                            let diff_abs = (decoded - api_price_sol).abs();
                            let diff_pct = if api_price_sol > 0.0 {
                                (diff_abs / api_price_sol) * 100.0
                            } else {
                                0.0
                            };
                            println!(
                                "Decoded SOL price: {:.12}\nDexScreener SOL:  {:.12}\nDiff: {:.12} SOL ({:.4}%)",
                                decoded,
                                api_price_sol,
                                diff_abs,
                                diff_pct
                            );
                        } else {
                            println!("Decoder returned None; cannot compare.");
                        }
                    } else {
                        println!(
                            "DexScreener had no price_sol for target mint; likely no SOL pair."
                        );
                    }
                }
                Ok(None) => println!("DexScreener returned no data for target mint."),
                Err(e) => println!("DexScreener error: {}", e),
            }
        } else {
            println!("DexScreener API not initialized; skipping diff.");
        }
    } else {
        println!(
            "Pool has no SOL side; DexScreener diff is skipped (decoder also skips such pools)."
        );
    }

    println!("\n✅ DEBUG COMPLETE");
    Ok(())
}

struct MinimalClmmInfo {
    token_mint_0: String,
    token_mint_1: String,
    token_vault_0: String,
    token_vault_1: String,
    mint_decimals_0: u8,
    mint_decimals_1: u8,
    tick_spacing: u16,
    sqrt_price_x64: u128,
    tick_current: i32,
}

fn parse_clmm_minimal(data: &[u8]) -> Option<MinimalClmmInfo> {
    if data.len() < 1200 {
        return None;
    }
    let mut off = 8; // skip discriminator
    off += 1; // bump
    off += 32; // amm_config
    off += 32; // owner

    let token_mint_0 = read_pubkey(data, off)?;
    off += 32;
    let token_mint_1 = read_pubkey(data, off)?;
    off += 32;
    let token_vault_0 = read_pubkey(data, off)?;
    off += 32;
    let token_vault_1 = read_pubkey(data, off)?;
    off += 32;
    let _observation_key = read_pubkey(data, off)?;
    off += 32;

    let mint_decimals_0 = *data.get(off)?;
    off += 1;
    let mint_decimals_1 = *data.get(off)?;
    off += 1;
    let tick_spacing = u16::from_le_bytes(
        data
            .get(off..off + 2)?
            .try_into()
            .ok()?
    );
    off += 2;
    let _liquidity = u128::from_le_bytes(
        data
            .get(off..off + 16)?
            .try_into()
            .ok()?
    );
    off += 16;
    let sqrt_price_x64 = u128::from_le_bytes(
        data
            .get(off..off + 16)?
            .try_into()
            .ok()?
    );
    off += 16;
    let tick_current = i32::from_le_bytes(
        data
            .get(off..off + 4)?
            .try_into()
            .ok()?
    );

    Some(MinimalClmmInfo {
        token_mint_0,
        token_mint_1,
        token_vault_0,
        token_vault_1,
        mint_decimals_0,
        mint_decimals_1,
        tick_spacing,
        sqrt_price_x64,
        tick_current,
    })
}

fn read_pubkey(data: &[u8], off: usize) -> Option<String> {
    let bytes: [u8; 32] = data
        .get(off..off + 32)?
        .try_into()
        .ok()?;
    Some(Pubkey::new_from_array(bytes).to_string())
}

fn decode_token_amount(data: &[u8]) -> Option<u64> {
    if data.len() < 72 {
        return None;
    }
    let amt = u64::from_le_bytes(
        data
            .get(64..72)?
            .try_into()
            .ok()?
    );
    Some(amt)
}

// No in-binary API calls; compare externally to keep the tool focused and compile-safe.
