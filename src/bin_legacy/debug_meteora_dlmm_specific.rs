//! Meteora DLMM (Dynamic Liquidity Market Maker) specific decoder debugging tool
//!
//! Validates DLMM pool parsing, vault balances, active_id/bin_step handling, and compares
//! calculated SOL price with DexScreener. Tests decoder implementation.
//!
//! Usage:
//!   cargo run --bin debug_meteora_dlmm_specific -- --pool <POOL_ADDRESS> [--show-hex]

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::{meteora_dlmm::MeteoraDlmmDecoder, PoolDecoder};
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{METEORA_DLMM_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
    name = "debug_meteora_dlmm_specific",
    about = "Debug a Meteora DLMM pool and compare price vs DexScreener"
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
        "debug_meteora_dlmm_specific".to_string(),
        "--debug-pool-decoders".to_string(),
    ]);

    println!("\n🔍 METEORA DLMM SPECIFIC DEBUGGER\n=================================");
    println!("Pool address: {}", args.pool);

    logger::info(
        LogTag::System, "Initializing RPC client...");
    init_rpc_client().map_err(|e| format!("RPC init failed: {}", e))?;
    let rpc = get_rpc_client();

    let pool_pk = parse_pubkey(&args.pool)?;
    let pool_acc = rpc.client().get_account(&pool_pk)?;

    println!("\n📦 POOL ACCOUNT\n==============");
    println!("Owner: {}", pool_acc.owner);
    println!("Data size: {} bytes", pool_acc.data.len());
    println!(
        "Owner is Meteora DLMM: {}",
        if pool_acc.owner.to_string() == METEORA_DLMM_PROGRAM_ID {
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

    // Parse DLMM structure manually to debug field extraction
    println!("\n🔍 DLMM STRUCTURE ANALYSIS\n=========================");

    if pool_acc.data.len() >= 216 {
        // Extract key fields at known offsets
        let token_x_mint = read_pubkey(&pool_acc.data, 88);
        let token_y_mint = read_pubkey(&pool_acc.data, 120);
        let reserve_x = read_pubkey(&pool_acc.data, 152);
        let reserve_y = read_pubkey(&pool_acc.data, 184);
        let active_id = read_i32(&pool_acc.data, 76);
        let bin_step = read_u16(&pool_acc.data, 80);

        println!("Token X mint: {:?}", token_x_mint);
        println!("Token Y mint: {:?}", token_y_mint);
        println!("Reserve X: {:?}", reserve_x);
        println!("Reserve Y: {:?}", reserve_y);
        println!("Active ID: {:?}", active_id);
        println!("Bin Step: {:?}", bin_step);

        // Scan for potential DLMM values
        println!("\n🔍 SCANNING FOR DLMM VALUES\n===========================");
        scan_for_i32_values(&pool_acc.data);
        scan_for_u16_values(&pool_acc.data);

        // Identify SOL and token sides
        let (sol_mint, token_mint, sol_vault, token_vault) =
            if let (Some(x_mint), Some(y_mint), Some(x_vault), Some(y_vault)) = (
                token_x_mint.as_ref(),
                token_y_mint.as_ref(),
                reserve_x.as_ref(),
                reserve_y.as_ref(),
            ) {
                if x_mint == SOL_MINT {
                    (
                        x_mint.clone(),
                        y_mint.clone(),
                        x_vault.clone(),
                        y_vault.clone(),
                    )
                } else if y_mint == SOL_MINT {
                    (
                        y_mint.clone(),
                        x_mint.clone(),
                        y_vault.clone(),
                        x_vault.clone(),
                    )
                } else {
                    println!("⚠️ Neither token is SOL");
                    return Ok(());
                }
            } else {
                println!("⚠️ Could not extract all required fields");
                return Ok(());
            };

        println!("\n🏦 IDENTIFIED VAULTS\n===================");
        println!("SOL mint: {}", sol_mint);
        println!("Token mint: {}", token_mint);
        println!("SOL vault: {}", sol_vault);
        println!("Token vault: {}", token_vault);

        // Fetch vault accounts
        let sol_vault_pk = parse_pubkey(&sol_vault)?;
        let token_vault_pk = parse_pubkey(&token_vault)?;
        let sol_vault_acc = rpc.client().get_account(&sol_vault_pk)?;
        let token_vault_acc = rpc.client().get_account(&token_vault_pk)?;

        let sol_balance_raw = decode_token_amount(&sol_vault_acc.data).unwrap_or(0);
        let token_balance_raw = decode_token_amount(&token_vault_acc.data).unwrap_or(0);

        println!("\n💰 RAW BALANCES\n===============");
        println!("SOL vault balance (raw): {}", sol_balance_raw);
        println!("Token vault balance (raw): {}", token_balance_raw);

        // Calculate human-readable amounts
        let token_decimals = get_token_decimals_sync(&token_mint).unwrap_or(9);
        let sol_decimals = SOL_DECIMALS;

        let sol_amount = (sol_balance_raw as f64) / (10f64).powi(sol_decimals as i32);
        let token_amount = (token_balance_raw as f64) / (10f64).powi(token_decimals as i32);

        println!("\n🧮 CALCULATED AMOUNTS\n====================");
        println!("SOL amount: {:.9} SOL", sol_amount);
        println!(
            "Token amount: {:.9} tokens (decimals: {})",
            token_amount, token_decimals
        );

        if token_amount > 0.0 {
            let vault_price = sol_amount / token_amount;
            println!("Vault-based price: {:.12} SOL/token", vault_price);
        }

        // Calculate DLMM theoretical price if we have active_id and bin_step
        if let (Some(active_id_val), Some(bin_step_val)) = (active_id, bin_step) {
            println!("\n📈 DLMM THEORETICAL PRICE\n========================");
            let bin_step_factor = 1.0 + (bin_step_val as f64) / 10000.0;
            let theoretical_price = bin_step_factor.powf(active_id_val as f64);

            println!("Active ID: {}", active_id_val);
            println!("Bin Step: {}", bin_step_val);
            println!("Bin Step Factor: {:.8}", bin_step_factor);
            println!("Theoretical Price: {:.12}", theoretical_price);
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
        let result1 = MeteoraDlmmDecoder::decode_and_calculate(&accounts, &token_mint, SOL_MINT);
        let result2 = MeteoraDlmmDecoder::decode_and_calculate(&accounts, SOL_MINT, &token_mint);

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
                    if let Some(api_sol_price) = api_token.price_dexscreener_sol {
                        let decoder_price =
                            result1.as_ref().or(result2.as_ref()).map(|r| r.price_sol);

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
    } else {
        println!("⚠️ Pool data too short for DLMM analysis");
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

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    if offset + 4 > data.len() {
        return None;
    }
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    if offset + 2 > data.len() {
        return None;
    }
    let bytes: [u8; 2] = data[offset..offset + 2].try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn decode_token_amount(data: &[u8]) -> Option<u64> {
    if data.len() < 72 {
        return None;
    }
    // Token account amount is at offset 64
    let bytes: [u8; 8] = data[64..72].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn scan_for_i32_values(data: &[u8]) {
    println!("Scanning for interesting i32 values:");
    for offset in (0..data.len().saturating_sub(4)).step_by(4) {
        if let Some(value) = read_i32(data, offset) {
            // Look for reasonable active_id values (typically between -1000 and 1000)
            if value.abs() < 1000 && value != 0 {
                println!("  i32 @ offset {}: {}", offset, value);
            }
        }
    }
}

fn scan_for_u16_values(data: &[u8]) {
    println!("Scanning for interesting u16 values:");
    for offset in (0..data.len().saturating_sub(2)).step_by(2) {
        if let Some(value) = read_u16(data, offset) {
            // Look for reasonable bin_step values (typically 1-100)
            if value > 0 && value <= 100 {
                println!("  u16 @ offset {}: {}", offset, value);
            }
        }
    }
}
