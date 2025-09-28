//! Meteora DBC (Dynamic Bonding Curve) specific decoder debugging tool
//!
//! Validates DBC pool parsing, vault balances, fee handling, and compares
//! calculated SOL price with DexScreener. Tests decoder implementation.
//!
//! Usage:
//!   cargo run --bin debug_meteora_dbc_specific -- --pool <POOL_ADDRESS> [--show-hex]

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::{meteora_dbc::MeteoraDbcDecoder, PoolDecoder};
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{METEORA_DBC_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
    name = "debug_meteora_dbc_specific",
    about = "Debug a Meteora DBC pool and compare price vs DexScreener"
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
        "debug_meteora_dbc_specific".to_string(),
        "--debug-pool-decoders".to_string(),
    ]);

    println!("\n🔍 METEORA DBC SPECIFIC DEBUGGER\n=================================");
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
        "Owner is Meteora DBC: {}",
        if pool_acc.owner.to_string() == METEORA_DBC_PROGRAM_ID {
            "✅"
        } else {
            "❌"
        }
    );

    if args.show_hex {
        println!("\n📄 RAW HEX (first 256 bytes)");
        for (i, chunk) in pool_acc.data.chunks(16).take(16).enumerate() {
            print!("{:04x}: ", i * 16);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            println!();
        }
    }

    // Try to scan for vault addresses in pool data
    println!("\n🔍 SCANNING FOR VAULT ADDRESSES\n===============================");
    let mut found_vaults = Vec::new();

    // Scan for 32-byte sequences that look like pubkeys
    for offset in (0..pool_acc.data.len().saturating_sub(32)).step_by(4) {
        if let Some(potential_pubkey) = read_pubkey(&pool_acc.data, offset) {
            // Check if this looks like a vault by fetching it
            if let Ok(vault_pk) = parse_pubkey(&potential_pubkey) {
                if let Ok(vault_acc) = rpc.client().get_account(&vault_pk) {
                    if vault_acc.data.len() >= 165 {
                        // SPL token account size
                        if let Some(mint) = read_pubkey(&vault_acc.data, 0) {
                            found_vaults.push((offset, potential_pubkey.clone(), mint.clone()));
                            println!(
                                "Found vault @ offset {}: {} (mint: {})",
                                offset, potential_pubkey, mint
                            );
                        }
                    }
                }
            }
        }
    }

    if found_vaults.len() < 2 {
        println!(
            "⚠️ Found {} vaults, need at least 2 for SOL pair",
            found_vaults.len()
        );
        return Ok(());
    }

    // Identify SOL and token vaults
    let mut sol_vault = None;
    let mut token_vault = None;
    let mut token_mint = String::new();

    for (_, vault_addr, mint) in &found_vaults {
        if mint == SOL_MINT {
            sol_vault = Some(vault_addr.clone());
        } else if !mint.is_empty() && mint != "11111111111111111111111111111111" {
            token_vault = Some(vault_addr.clone());
            token_mint = mint.clone();
        }
    }

    let sol_vault = sol_vault.ok_or("No SOL vault found")?;
    let token_vault = token_vault.ok_or("No token vault found")?;

    println!("\n🏦 IDENTIFIED VAULTS\n===================");
    println!("SOL vault: {}", sol_vault);
    println!("Token vault: {}", token_vault);
    println!("Token mint: {}", token_mint);

    // Fetch vault accounts and get balances
    let sol_vault_pk = parse_pubkey(&sol_vault)?;
    let token_vault_pk = parse_pubkey(&token_vault)?;
    let sol_vault_acc = rpc.client().get_account(&sol_vault_pk)?;
    let token_vault_acc = rpc.client().get_account(&token_vault_pk)?;

    let sol_balance_raw = decode_token_amount(&sol_vault_acc.data).unwrap_or(0);
    let token_balance_raw = decode_token_amount(&token_vault_acc.data).unwrap_or(0);

    println!("\n💰 RAW BALANCES\n===============");
    println!("SOL vault balance (raw): {}", sol_balance_raw);
    println!("Token vault balance (raw): {}", token_balance_raw);

    // Try to locate fees in pool data (heuristic scan)
    println!("\n💸 FEE DETECTION\n===============");
    let mut potential_fees = Vec::new();

    for offset in (100..pool_acc.data.len().saturating_sub(16)).step_by(8) {
        if let Some(fee1) = read_u64(&pool_acc.data, offset) {
            if let Some(fee2) = read_u64(&pool_acc.data, offset + 8) {
                // Look for reasonable fee values (not zero, not too large)
                if (fee1 > 0 || fee2 > 0) && fee1 < 1u64 << 50 && fee2 < 1u64 << 50 {
                    potential_fees.push((offset, fee1, fee2));
                }
            }
        }
    }

    println!("Found {} potential fee pairs:", potential_fees.len());
    for (offset, fee1, fee2) in &potential_fees {
        println!("  @ offset {}: {} / {}", offset, fee1, fee2);
    }

    // Use first reasonable fee pair as quote fees (protocol + partner)
    let (protocol_quote_fee, partner_quote_fee) = (0, 0); // Skip fees for debugging

    println!(
        "Using no fees for debugging: protocol_quote_fee={}, partner_quote_fee={}",
        protocol_quote_fee, partner_quote_fee
    );

    // Calculate effective SOL balance after fees
    let sol_after_fees = sol_balance_raw
        .saturating_sub(protocol_quote_fee)
        .saturating_sub(partner_quote_fee);

    // Get decimals and calculate human-readable amounts
    let token_decimals = get_token_decimals_sync(&token_mint).unwrap_or(9);
    let sol_decimals = SOL_DECIMALS;

    let sol_amount = (sol_after_fees as f64) / (10f64).powi(sol_decimals as i32);
    let token_amount = (token_balance_raw as f64) / (10f64).powi(token_decimals as i32);

    println!("\n🧮 CALCULATED AMOUNTS\n====================");
    println!("SOL amount (after fees): {:.9} SOL", sol_amount);
    println!(
        "Token amount: {:.9} tokens (decimals: {})",
        token_amount, token_decimals
    );

    if token_amount > 0.0 {
        let manual_price = sol_amount / token_amount;
        println!("Manual price: {:.12} SOL/token", manual_price);
    }

    // Test decoder
    println!("\n🧪 DECODER TEST\n===============");
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
        sol_vault.clone(),
        AccountData {
            pubkey: sol_vault_pk,
            data: sol_vault_acc.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: sol_vault_acc.lamports,
            owner: sol_vault_acc.owner,
        },
    );

    accounts.insert(
        token_vault.clone(),
        AccountData {
            pubkey: token_vault_pk,
            data: token_vault_acc.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: token_vault_acc.lamports,
            owner: token_vault_acc.owner,
        },
    );

    // Test both orientations
    let result1 = MeteoraDbcDecoder::decode_and_calculate(&accounts, &token_mint, SOL_MINT);
    let result2 = MeteoraDbcDecoder::decode_and_calculate(&accounts, SOL_MINT, &token_mint);

    if let Some(r) = &result1 {
        println!(
            "Orientation TOKEN/SOL → {:.12} SOL/token (pool {})",
            r.price_sol, r.pool_address
        );
    } else {
        println!("Orientation TOKEN/SOL → None");
    }

    if let Some(r) = &result2 {
        println!(
            "Orientation SOL/TOKEN → {:.12} SOL/token (pool {})",
            r.price_sol, r.pool_address
        );
    } else {
        println!("Orientation SOL/TOKEN → None");
    }

    // Compare with DexScreener
    println!("\n📊 DEXSCREENER COMPARISON\n========================");
    init_dexscreener_api().await.ok();

    if let Ok(api_arc) = get_global_dexscreener_api().await {
        let mut api = api_arc.lock().await;
        match api.get_token_data(&token_mint).await {
            Ok(Some(api_token)) => {
                if let Some(api_sol_price) = api_token.price_sol {
                    let decoder_price = result1.as_ref().or(result2.as_ref()).map(|r| r.price_sol);

                    if let Some(decoded_price) = decoder_price {
                        let diff_abs = (decoded_price - api_sol_price).abs();
                        let diff_pct = (diff_abs / api_sol_price) * 100.0;

                        println!(
                            "Decoded SOL price: {:.12}\nDexScreener SOL:   {:.12}\nDiff: {:.12} SOL ({:.4}%)",
                            decoded_price,
                            api_sol_price,
                            diff_abs,
                            diff_pct
                        );
                    } else {
                        println!("Decoder returned no price");
                    }
                } else {
                    println!("DexScreener had no SOL price for token");
                }
            }
            Ok(None) => println!("DexScreener returned no data for token"),
            Err(e) => println!("DexScreener error: {}", e),
        }
    } else {
        println!("DexScreener API not initialized");
    }

    println!("\n✅ DEBUG COMPLETE");
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }
    let bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
    let pubkey = Pubkey::new_from_array(bytes);

    // Basic sanity check - reject all-zeros or all-ones
    if bytes.iter().all(|&b| b == 0) || bytes.iter().all(|&b| b == 255) {
        return None;
    }

    Some(pubkey.to_string())
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 > data.len() {
        return None;
    }
    let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn decode_token_amount(data: &[u8]) -> Option<u64> {
    if data.len() < 72 {
        return None;
    }
    // Token account amount is at offset 64
    let bytes: [u8; 8] = data[64..72].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}
