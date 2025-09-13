//! Meteora DAMM v2 specific decoder debugging tool
//!
//! Validates offsets, vault balances, sqrt_price math (Q64.64), orientation, and compares
//! calculated SOL price with DexScreener. Runs decoder in both orientations.
//!
//! Usage:
//!   cargo run --bin debug_meteora_damm_specific -- --pool <POOL_ADDRESS> [--show-hex]

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{ log, LogTag };
use screenerbot::pools::decoders::{ meteora_damm::MeteoraDammDecoder, PoolDecoder };
use screenerbot::pools::types::{ METEORA_DAMM_PROGRAM_ID, SOL_MINT };
use screenerbot::pools::fetcher::AccountData;
use screenerbot::rpc::{ get_rpc_client, init_rpc_client, parse_pubkey };
use screenerbot::tokens::{ decimals::SOL_DECIMALS, get_token_decimals_sync };
use screenerbot::tokens::dexscreener::{ init_dexscreener_api, get_global_dexscreener_api };
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
    set_cmd_args(
        vec!["debug_meteora_damm_specific".to_string(), "--debug-pool-decoders".to_string()]
    );

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
    println!("Owner is Meteora DAMM: {}", if pool_acc.owner.to_string() == METEORA_DAMM_PROGRAM_ID {
        "✅"
    } else {
        "❌"
    });

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

    // Parse key fields via correct empirical offsets (verified against JSON)
    let token_a_mint = read_pubkey(&pool_acc.data, 168).ok_or("parse token_a_mint")?;
    let token_b_mint = read_pubkey(&pool_acc.data, 200).ok_or("parse token_b_mint")?;
    let token_a_vault = read_pubkey(&pool_acc.data, 232).ok_or("parse token_a_vault")?;
    let token_b_vault = read_pubkey(&pool_acc.data, 264).ok_or("parse token_b_vault")?;
    let liquidity = read_u128(&pool_acc.data, 304).unwrap_or(0);
    let protocol_a_fee = read_u64(&pool_acc.data, 392).unwrap_or(0);
    let protocol_b_fee = read_u64(&pool_acc.data, 400).unwrap_or(0);
    let partner_a_fee = read_u64(&pool_acc.data, 408).unwrap_or(0);
    let partner_b_fee = read_u64(&pool_acc.data, 416).unwrap_or(0);
    let sqrt_min_price = read_u128(&pool_acc.data, 424).unwrap_or(0);
    let sqrt_max_price = read_u128(&pool_acc.data, 440).unwrap_or(0);
    let sqrt_456 = read_u128(&pool_acc.data, 456).unwrap_or(0);
    let sqrt_464 = read_u128(&pool_acc.data, 464).unwrap_or(0);

    // Try alternative liquidity offset (JSON shows large value, may be at different position)
    let liquidity_alt1 = read_u128(&pool_acc.data, 320).unwrap_or(0);
    let liquidity_alt2 = read_u128(&pool_acc.data, 336).unwrap_or(0);

    println!("\n🔧 PARSED FIELDS\n================");
    println!("token_a_mint: {}", token_a_mint);
    println!("token_b_mint: {}", token_b_mint);
    println!("token_a_vault: {}", token_a_vault);
    println!("token_b_vault: {}", token_b_vault);
    println!("liquidity@304: {}", liquidity);
    println!("liquidity@320: {} (alt1)", liquidity_alt1);
    println!("liquidity@336: {} (alt2)", liquidity_alt2);
    println!(
        "fees: protocol_a={} protocol_b={} partner_a={} partner_b={}",
        protocol_a_fee,
        protocol_b_fee,
        partner_a_fee,
        partner_b_fee
    );
    println!("sqrt_min_price: {}", sqrt_min_price);
    println!("sqrt_max_price: {}", sqrt_max_price);
    println!("sqrt_price@456: {}", sqrt_456);
    println!("sqrt_price@464: {}", sqrt_464);

    // JSON verification - the liquidity value from JSON is 670203503310646500374104774470
    let json_liquidity = 670203503310646500374104774470u128;
    let selected_liquidity = if liquidity_alt1 == json_liquidity {
        println!("✅ Found JSON liquidity match at offset 320");
        liquidity_alt1
    } else if liquidity_alt2 == json_liquidity {
        println!("✅ Found JSON liquidity match at offset 336");
        liquidity_alt2
    } else {
        println!(
            "⚠️  No liquidity match found. JSON={}, read values: {}@304, {}@320, {}@336",
            json_liquidity,
            liquidity,
            liquidity_alt1,
            liquidity_alt2
        );
        // Use the largest non-zero value as likely correct
        let max_liquidity = [liquidity, liquidity_alt1, liquidity_alt2]
            .iter()
            .max()
            .copied()
            .unwrap_or(0);
        println!("🔍 Using largest liquidity value: {}", max_liquidity);
        max_liquidity
    };

    // Fetch vault accounts
    let a_vault_pk = parse_pubkey(&token_a_vault)?;
    let b_vault_pk = parse_pubkey(&token_b_vault)?;
    let a_vault_acc = rpc.client().get_account(&a_vault_pk)?;
    let b_vault_acc = rpc.client().get_account(&b_vault_pk)?;
    let a_amt_raw = decode_token_amount(&a_vault_acc.data).unwrap_or(0);
    let b_amt_raw = decode_token_amount(&b_vault_acc.data).unwrap_or(0);

    println!("\n🏦 VAULTS\n=========");
    println!("a_vault mint: {}", read_pubkey(&a_vault_acc.data, 0).unwrap_or_default());
    println!("b_vault mint: {}", read_pubkey(&b_vault_acc.data, 0).unwrap_or_default());
    println!("a_vault amount(raw): {}", a_amt_raw);
    println!("b_vault amount(raw): {}", b_amt_raw);

    // Orient relative to SOL
    let (token_mint, sol_vault_pk, token_vault_pk, sol_fees, token_fees) = if
        token_b_mint == SOL_MINT
    {
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
    let sol_acc = if sol_vault_pk == a_vault_pk { &a_vault_acc } else { &b_vault_acc };
    let tok_acc = if token_vault_pk == a_vault_pk { &a_vault_acc } else { &b_vault_acc };
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

    // Enhanced sqrt-based price with detailed analysis
    let (sqrt_sel, sqrt_based_price, sqrt_details) = select_sqrt_and_price_detailed(
        sqrt_456,
        sqrt_464,
        tok_dec,
        sol_dec,
        &token_a_mint,
        &token_b_mint
    );
    println!("sqrt selected offset: {}", sqrt_sel);
    println!("Sqrt-based price: {:.12} SOL/token", sqrt_based_price);
    if !sqrt_details.is_empty() {
        println!("Sqrt calculation details:\n{}", sqrt_details);
    }

    // DAMM liquidity-based price (Uniswap v3 style)
    if selected_liquidity > 0 {
        println!("\n🔬 DAMM LIQUIDITY ANALYSIS\n=========================");
        let sqrt_current = if sqrt_456 > 0 { sqrt_456 } else { sqrt_464 };
        if sqrt_current > 0 && sqrt_min_price > 0 && sqrt_max_price > 0 {
            // Calculate theoretical amounts from liquidity formula
            let liq_price = calculate_damm_liquidity_price(
                selected_liquidity,
                sqrt_current,
                sqrt_min_price,
                sqrt_max_price,
                tok_dec,
                sol_dec,
                &token_a_mint
            );
            println!("Liquidity-based price: {:.12} SOL/token", liq_price);

            // Compare with vault ratio
            if tok > 0.0 {
                let vault_ratio = sol / tok;
                let diff_pct = if vault_ratio > 0.0 {
                    ((liq_price - vault_ratio).abs() / vault_ratio) * 100.0
                } else {
                    0.0
                };
                println!("Diff vs vault ratio: {:.4}%", diff_pct);
            }
        } else {
            println!("Missing sqrt price or price bounds for liquidity calculation");
        }
    } else {
        println!("No liquidity data for enhanced analysis");
    }

    // Build accounts map and run decoder (both orientations)
    let mut accounts = HashMap::new();
    accounts.insert(args.pool.clone(), AccountData {
        pubkey: pool_pk,
        data: pool_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: pool_acc.lamports,
        owner: pool_acc.owner,
    });
    accounts.insert(token_a_vault.clone(), AccountData {
        pubkey: a_vault_pk,
        data: a_vault_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: a_vault_acc.lamports,
        owner: a_vault_acc.owner,
    });
    accounts.insert(token_b_vault.clone(), AccountData {
        pubkey: b_vault_pk,
        data: b_vault_acc.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: b_vault_acc.lamports,
        owner: b_vault_acc.owner,
    });

    println!("\n🧪 DECODER CHECK\n================");
    let d1 = MeteoraDammDecoder::decode_and_calculate(&accounts, &token_mint, &SOL_MINT);
    if let Some(r) = &d1 {
        println!("Orientation TOKEN/SOL → {:.12} (pool {})", r.price_sol, r.pool_address);
    } else {
        println!("Orientation TOKEN/SOL → None");
    }
    let d2 = MeteoraDammDecoder::decode_and_calculate(&accounts, &SOL_MINT, &token_mint);
    if let Some(r) = &d2 {
        println!("Orientation SOL/TOKEN → {:.12} (pool {})", r.price_sol, r.pool_address);
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
    let bytes: [u8; 32] = data
        .get(off..off + 32)?
        .try_into()
        .ok()?;
    Some(Pubkey::new_from_array(bytes).to_string())
}

fn read_u64(data: &[u8], off: usize) -> Option<u64> {
    let bytes: [u8; 8] = data
        .get(off..off + 8)?
        .try_into()
        .ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn read_u128(data: &[u8], off: usize) -> Option<u128> {
    let bytes: [u8; 16] = data
        .get(off..off + 16)?
        .try_into()
        .ok()?;
    Some(u128::from_le_bytes(bytes))
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

fn select_sqrt_and_price_detailed(
    sqrt_456: u128,
    sqrt_464: u128,
    token_decimals: u8,
    sol_decimals: u8,
    token_a_mint: &str,
    _token_b_mint: &str
) -> (&'static str, f64, String) {
    let mut details = String::new();
    let cands = [
        ("456", sqrt_456),
        ("464", sqrt_464),
    ];
    for (lab, val) in cands {
        if val == 0 {
            details.push_str(&format!("  {} = 0 (skipped)\n", lab));
            continue;
        }
        let sqrt_f64 = val as f64;
        let normalized_sqrt = sqrt_f64 / (2f64).powi(64);
        let base_price = normalized_sqrt.powi(2);
        let dec_adj_factor = (10f64).powi((sol_decimals as i32) - (token_decimals as i32));
        let mut price = base_price * dec_adj_factor;
        if token_a_mint == SOL_MINT {
            price = if price > 0.0 { 1.0 / price } else { 0.0 };
        }

        details.push_str(
            &format!(
                "  {}: raw={} sqrt_f64={:.6e} norm={:.12e} base={:.12e} dec_adj={:.6e} final={:.12e}\n",
                lab,
                val,
                sqrt_f64,
                normalized_sqrt,
                base_price,
                dec_adj_factor,
                price
            )
        );

        if price.is_finite() && price > 0.0 && price < 1e6 {
            return (lab, price, details);
        }
    }
    ("none", 0.0, details)
}

fn calculate_damm_liquidity_price(
    liquidity: u128,
    sqrt_current: u128,
    sqrt_min: u128,
    sqrt_max: u128,
    token_decimals: u8,
    sol_decimals: u8,
    token_a_mint: &str
) -> f64 {
    // DAMM uses Uniswap v3 style concentrated liquidity formulas
    // Calculate theoretical amounts from liquidity and price bounds

    let sqrt_curr_f64 = (sqrt_current as f64) / (2f64).powi(64);
    let sqrt_min_f64 = (sqrt_min as f64) / (2f64).powi(64);
    let sqrt_max_f64 = (sqrt_max as f64) / (2f64).powi(64);
    let liq_f64 = liquidity as f64;

    // Calculate amounts based on current position within price range
    // For DAMM: amount_a = L * (sqrt_max - sqrt_curr) / (sqrt_max * sqrt_curr)
    // amount_b = L * (sqrt_curr - sqrt_min)

    let amount_a = if sqrt_curr_f64 < sqrt_max_f64 {
        (liq_f64 * (sqrt_max_f64 - sqrt_curr_f64)) / (sqrt_max_f64 * sqrt_curr_f64)
    } else {
        0.0
    };

    let amount_b = if sqrt_curr_f64 > sqrt_min_f64 {
        (liq_f64 * (sqrt_curr_f64 - sqrt_min_f64)) / (2f64).powi(128)
    } else {
        0.0
    };

    println!(
        "  L={:.0} sqrt_curr={:.12e} sqrt_min={:.12e} sqrt_max={:.12e}",
        liq_f64,
        sqrt_curr_f64,
        sqrt_min_f64,
        sqrt_max_f64
    );
    println!("  Theoretical: amount_a={:.6e} amount_b={:.6e}", amount_a, amount_b);

    // Convert to human readable units and calculate price
    if amount_a > 0.0 && amount_b > 0.0 {
        let human_a = amount_a / (10f64).powi(token_decimals as i32);
        let human_b = amount_b / (10f64).powi(sol_decimals as i32);

        let price = if token_a_mint == SOL_MINT {
            human_a / human_b // SOL per token
        } else {
            human_b / human_a // SOL per token
        };

        println!(
            "  Human readable: token={:.6e} sol={:.6e} price={:.12e}",
            human_a,
            human_b,
            price
        );
        price
    } else {
        // Fallback to sqrt price calculation
        let base_price = sqrt_curr_f64.powi(2);
        let dec_adj_factor = (10f64).powi((sol_decimals as i32) - (token_decimals as i32));
        let mut price = base_price * dec_adj_factor;
        if token_a_mint == SOL_MINT {
            price = if price > 0.0 { 1.0 / price } else { 0.0 };
        }
        println!("  Fallback to sqrt: {:.12e}", price);
        price
    }
}
