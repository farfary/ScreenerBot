//! Meteora DAMM v2 specific decoder debugging tool
//!
//! Validates offsets, vault balances, sqrt_price math (Q64.64), orientation, and compares
//! calculated SOL price with DexScreener. Runs decoder in both orientations.
//!
//! Usage:
//!   cargo run --bin debug_meteora_damm_specific -- --pool <POOL_ADDRESS> [--show-hex]

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::{meteora_damm::MeteoraDammDecoder, PoolDecoder};
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{METEORA_DAMM_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
    name = "debug_meteora_damm_specific",
    about = "Debug a Meteora DAMM v2 pool and compare price vs DexScreener"
)]
struct Args {
    /// Pool address to debug
    #[arg(short, long)]
    pool: String,

    /// Show first bytes of pool data as hex
    #[arg(long)]
    show_hex: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    set_cmd_args(vec![
        "debug_meteora_damm_specific".to_string(),
        "--debug-pool-decoders".to_string(),
    ]);

    println!("\n🔍 METEORA DAMM v2 SPECIFIC DEBUGGER\n====================================");
    println!("Pool address: {}", args.pool);

    log(LogTag::System, "INIT", "Initializing RPC client...");
    init_rpc_client().map_err(|e| format!("RPC init failed: {}", e))?;
    let rpc = get_rpc_client();

    let pool_pk = parse_pubkey(&args.pool)?;
    let pool_acc = rpc.client().get_account(&pool_pk)?;

    println!("\n📦 POOL ACCOUNT\n==============");
    println!("Owner: {}", pool_acc.owner);
    println!("Data size: {} bytes", pool_acc.data.len());
    println!(
        "Owner is Meteora DAMM: {}",
        if pool_acc.owner.to_string() == METEORA_DAMM_PROGRAM_ID {
            "✅"
        } else {
            "❌"
        }
    );

    if args.show_hex {
        println!("\n📄 RAW HEX (first 256 bytes)");
        for (i, chunk) in pool_acc.data.chunks(16).take(16).enumerate() {
            print!("{:04x}: ", i * 16);
            for b in chunk {
                print!("{:02x} ", b);
            }
            println!();
        }
    }

    // Parse key fields via fixed offsets (empirically shifted -8 bytes from theoretical layout)
    let token_a_mint = read_pubkey(&pool_acc.data, 168).ok_or("parse token_a_mint")?;
    let token_b_mint = read_pubkey(&pool_acc.data, 200).ok_or("parse token_b_mint")?;
    let token_a_vault = read_pubkey(&pool_acc.data, 232).ok_or("parse token_a_vault")?;
    let token_b_vault = read_pubkey(&pool_acc.data, 264).ok_or("parse token_b_vault")?;
    let protocol_a_fee = read_u64(&pool_acc.data, 392).unwrap_or(0);
    let protocol_b_fee = read_u64(&pool_acc.data, 400).unwrap_or(0);
    let partner_a_fee = read_u64(&pool_acc.data, 408).unwrap_or(0);
    let partner_b_fee = read_u64(&pool_acc.data, 416).unwrap_or(0);
    let sqrt_456 = read_u128(&pool_acc.data, 456).unwrap_or(0);
    let sqrt_464 = read_u128(&pool_acc.data, 464).unwrap_or(0);

    println!("\n🔧 PARSED FIELDS\n================");
    println!("token_a_mint: {}", token_a_mint);
    println!("token_b_mint: {}", token_b_mint);
    println!("token_a_vault: {}", token_a_vault);
    println!("token_b_vault: {}", token_b_vault);
    println!(
        "fees: protocol_a={} protocol_b={} partner_a={} partner_b={}",
        protocol_a_fee, protocol_b_fee, partner_a_fee, partner_b_fee
    );
    println!("sqrt_price@456: {}", sqrt_456);
    println!("sqrt_price@464: {}", sqrt_464);

    // Fetch vault accounts
    let a_vault_pk = parse_pubkey(&token_a_vault)?;
    let b_vault_pk = parse_pubkey(&token_b_vault)?;
    let a_vault_acc = rpc.client().get_account(&a_vault_pk)?;
    let b_vault_acc = rpc.client().get_account(&b_vault_pk)?;
    let a_amt_raw = decode_token_amount(&a_vault_acc.data).unwrap_or(0);
    let b_amt_raw = decode_token_amount(&b_vault_acc.data).unwrap_or(0);

    println!("\n🏦 VAULTS\n=========");
    println!(
        "a_vault mint: {}",
        read_pubkey(&a_vault_acc.data, 0).unwrap_or_default()
    );
    println!(
        "b_vault mint: {}",
        read_pubkey(&b_vault_acc.data, 0).unwrap_or_default()
    );
    println!("a_vault amount(raw): {}", a_amt_raw);
    println!("b_vault amount(raw): {}", b_amt_raw);

    // Orient relative to SOL
    let (token_mint, sol_vault_pk, token_vault_pk, sol_fees, token_fees) =
        if token_b_mint == SOL_MINT {
            (
                token_a_mint.clone(),
                b_vault_pk,
                a_vault_pk,
                protocol_b_fee + partner_b_fee,
                protocol_a_fee + partner_a_fee,
            )
        } else if token_a_mint == SOL_MINT {
            (
                token_b_mint.clone(),
                a_vault_pk,
                b_vault_pk,
                protocol_a_fee + partner_a_fee,
                protocol_b_fee + partner_b_fee,
            )
        } else {
            println!("Pool has no SOL side; aborting");
            return Ok(());
        };

    // Effective reserves (minus fees)
    let sol_acc = if sol_vault_pk == a_vault_pk {
        &a_vault_acc
    } else {
        &b_vault_acc
    };
    let tok_acc = if token_vault_pk == a_vault_pk {
        &a_vault_acc
    } else {
        &b_vault_acc
    };
    let sol_raw = decode_token_amount(&sol_acc.data).unwrap_or(0);
    let tok_raw = decode_token_amount(&tok_acc.data).unwrap_or(0);
    let sol_eff = sol_raw.saturating_sub(sol_fees);
    let tok_eff = tok_raw.saturating_sub(token_fees);

    // Decimals
    let tok_dec = get_token_decimals_sync(&token_mint).unwrap_or(6);
    let sol_dec = SOL_DECIMALS;
    let sol = (sol_eff as f64) / (10f64).powi(sol_dec as i32);
    let tok = (tok_eff as f64) / (10f64).powi(tok_dec as i32);

    println!("\n🧮 MANUAL PRICES\n================");
    if tok > 0.0 {
        let vault_ratio = sol / tok;
        println!("Vault ratio (after fees): {:.12} SOL/token", vault_ratio);
    } else {
        println!("Token effective reserve zero; vault ratio skipped");
    }

    // Sqrt-based price (try both offsets; pick reasonable)
    let (sqrt_sel, sqrt_based_price) = select_sqrt_and_price(
        sqrt_456,
        sqrt_464,
        tok_dec,
        sol_dec,
        &token_a_mint,
        &token_b_mint,
    );
    println!("sqrt selected offset: {}", sqrt_sel);
    println!("Sqrt-based price: {:.12} SOL/token", sqrt_based_price);

    // Build accounts map and run decoder (both orientations)
    let mut accounts = HashMap::new();
    accounts.insert(
        args.pool.clone(),
        AccountData {
            pubkey: pool_pk,
            data: pool_acc.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: pool_acc.lamports,
            owner: pool_acc.owner,
        },
    );
    accounts.insert(
        token_a_vault.clone(),
        AccountData {
            pubkey: a_vault_pk,
            data: a_vault_acc.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: a_vault_acc.lamports,
            owner: a_vault_acc.owner,
        },
    );
    accounts.insert(
        token_b_vault.clone(),
        AccountData {
            pubkey: b_vault_pk,
            data: b_vault_acc.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: b_vault_acc.lamports,
            owner: b_vault_acc.owner,
        },
    );

    println!("\n🧪 DECODER CHECK\n================");
    let d1 = MeteoraDammDecoder::decode_and_calculate(&accounts, &token_mint, &SOL_MINT);
    if let Some(r) = &d1 {
        println!(
            "Orientation TOKEN/SOL → {:.12} (pool {})",
            r.price_sol, r.pool_address
        );
    } else {
        println!("Orientation TOKEN/SOL → None");
    }
    let d2 = MeteoraDammDecoder::decode_and_calculate(&accounts, &SOL_MINT, &token_mint);
    if let Some(r) = &d2 {
        println!(
            "Orientation SOL/TOKEN → {:.12} (pool {})",
            r.price_sol, r.pool_address
        );
    } else {
        println!("Orientation SOL/TOKEN → None");
    }

    // DexScreener diff
    println!("\n📊 DEXSCREENER DIFF (SOL)\n========================");
    init_dexscreener_api().await.ok();
    if let Ok(api_arc) = get_global_dexscreener_api().await {
        let mut api = api_arc.lock().await;
        match api.get_token_data(&token_mint).await {
            Ok(Some(api_token)) => {
                if let Some(api_sol) = api_token.price_sol {
                    let best = d1.as_ref().or(d2.as_ref());
                    if let Some(best) = best {
                        let dec = best.price_sol;
                        let diff_abs = (dec - api_sol).abs();
                        let diff_pct = if api_sol > 0.0 {
                            (diff_abs / api_sol) * 100.0
                        } else {
                            0.0
                        };
                        println!(
                            "Decoded SOL price: {:.12}\nDexScreener SOL:  {:.12}\nDiff: {:.12} SOL ({:.4}%)",
                            dec,
                            api_sol,
                            diff_abs,
                            diff_pct
                        );
                    } else {
                        println!("Decoder returned None; cannot compare.");
                    }
                } else {
                    println!("DexScreener had no price_sol for the token (no SOL pair).");
                }
            }
            Ok(None) => println!("DexScreener returned no data for token."),
            Err(e) => println!("DexScreener error: {}", e),
        }
    } else {
        println!("DexScreener API not initialized");
    }

    println!("\n✅ DEBUG COMPLETE");
    Ok(())
}

fn read_pubkey(data: &[u8], off: usize) -> Option<String> {
    let bytes: [u8; 32] = data.get(off..off + 32)?.try_into().ok()?;
    Some(Pubkey::new_from_array(bytes).to_string())
}

fn read_u64(data: &[u8], off: usize) -> Option<u64> {
    let bytes: [u8; 8] = data.get(off..off + 8)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn read_u128(data: &[u8], off: usize) -> Option<u128> {
    let bytes: [u8; 16] = data.get(off..off + 16)?.try_into().ok()?;
    Some(u128::from_le_bytes(bytes))
}

fn decode_token_amount(data: &[u8]) -> Option<u64> {
    if data.len() < 72 {
        return None;
    }
    let amt = u64::from_le_bytes(data.get(64..72)?.try_into().ok()?);
    Some(amt)
}

fn select_sqrt_and_price(
    sqrt_456: u128,
    sqrt_464: u128,
    token_decimals: u8,
    sol_decimals: u8,
    token_a_mint: &str,
    _token_b_mint: &str,
) -> (&'static str, f64) {
    let cands = [("456", sqrt_456), ("464", sqrt_464)];
    for (lab, val) in cands {
        if val == 0 {
            continue;
        }
        let sqrt_f64 = val as f64;
        let base = (sqrt_f64 / (2f64).powi(64)).powi(2);
        let dec_adj = (10f64).powi((token_decimals as i32) - (sol_decimals as i32));
        let mut price = base * dec_adj;
        if token_a_mint == SOL_MINT {
            price = if price > 0.0 { 1.0 / price } else { 0.0 };
        }
        if price.is_finite() && price > 0.0 && price < 1e6 {
            return (lab, price);
        }
    }
    ("none", 0.0)
}
