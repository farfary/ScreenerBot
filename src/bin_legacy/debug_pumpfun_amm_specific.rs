//! PumpFun AMM specific decoder debugging tool
//!
//! Validates PumpFun AMM pool parsing, vault balances, LP supply calculation, and compares
//! calculated SOL price with DexScreener. Tests decoder implementation.
//!
//! Usage:
//!   cargo run --bin debug_pumpfun_amm_specific -- --pool <POOL_ADDRESS> [--show-hex]

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::{pumpfun_amm::PumpFunAmmDecoder, PoolDecoder};
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{PUMP_FUN_AMM_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
    name = "debug_pumpfun_amm_specific",
    about = "Debug a PumpFun AMM pool and compare price vs DexScreener"
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
        "debug_pumpfun_amm_specific".to_string(),
        "--debug-pool-decoders".to_string(),
    ]);

    println!("\n🔍 PUMPFUN AMM SPECIFIC DEBUGGER\n===============================");
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
        "Owner is PumpFun AMM: {}",
        if pool_acc.owner.to_string() == PUMP_FUN_AMM_PROGRAM_ID {
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

    // Parse PumpFun AMM structure manually to debug field extraction
    println!("\n🔍 PUMPFUN AMM STRUCTURE ANALYSIS\n================================");

    if pool_acc.data.len() >= 250 {
        // PumpFun AMM pool layout (approximate):
        // discriminator(8) + pool_bump(1) + index(2) + creator(32) + creator(32) +
        // base_mint(32) + quote_mint(32) + lp_mint(32) + vault1(32) + vault2(32) + lp_supply(8) + ...

        let base_mint = read_pubkey(&pool_acc.data, 43); // 8+1+2+32
        let quote_mint = read_pubkey(&pool_acc.data, 75); // 8+1+2+32+32
        let lp_mint = read_pubkey(&pool_acc.data, 107); // 8+1+2+32+32+32
        let vault1 = read_pubkey(&pool_acc.data, 139); // 8+1+2+32+32+32+32
        let vault2 = read_pubkey(&pool_acc.data, 171); // 8+1+2+32+32+32+32+32
        let lp_supply = read_u64(&pool_acc.data, 203); // 8+1+2+32+32+32+32+32+32

        println!("Base mint: {:?}", base_mint);
        println!("Quote mint: {:?}", quote_mint);
        println!("LP mint: {:?}", lp_mint);
        println!("Vault 1: {:?}", vault1);
        println!("Vault 2: {:?}", vault2);
        println!("LP supply: {:?}", lp_supply);

        // Identify SOL and token sides
        let (sol_mint, token_mint, sol_vault, token_vault) =
            if let (Some(base), Some(quote), Some(v1), Some(v2)) = (
                base_mint.as_ref(),
                quote_mint.as_ref(),
                vault1.as_ref(),
                vault2.as_ref(),
            ) {
                if base == SOL_MINT {
                    (base.clone(), quote.clone(), v1.clone(), v2.clone())
                } else if quote == SOL_MINT {
                    (quote.clone(), base.clone(), v2.clone(), v1.clone())
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

        // Check if token mint matches base or quote from structure
        let token_is_base = base_mint.as_ref() == Some(&token_mint);
        println!("Token is base mint: {}", token_is_base);

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

        // Verify vault mints match expectations
        let sol_vault_mint = decode_token_mint(&sol_vault_acc.data);
        let token_vault_mint = decode_token_mint(&token_vault_acc.data);

        println!("\n🔍 VAULT VERIFICATION\n====================");
        println!("SOL vault mint: {:?}", sol_vault_mint);
        println!("Token vault mint: {:?}", token_vault_mint);

        let sol_correct = sol_vault_mint.as_ref() == Some(&sol_mint);
        let token_correct = token_vault_mint.as_ref() == Some(&token_mint);
        println!("SOL vault mint correct: {}", sol_correct);
        println!("Token vault mint correct: {}", token_correct);

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

        if let Some(lp_supply_val) = lp_supply {
            println!("LP supply: {}", lp_supply_val);
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
        let result1 = PumpFunAmmDecoder::decode_and_calculate(&accounts, &token_mint, SOL_MINT);
        let result2 = PumpFunAmmDecoder::decode_and_calculate(&accounts, SOL_MINT, &token_mint);

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
        println!("⚠️ Pool data too short for PumpFun AMM analysis");
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

fn decode_token_mint(data: &[u8]) -> Option<String> {
    if data.len() < 32 {
        return None;
    }
    // Token account mint is at offset 0
    let bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let pubkey = Pubkey::new_from_array(bytes);
    Some(pubkey.to_string())
}
