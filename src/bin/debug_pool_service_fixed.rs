//! Debug tool for Pool Service: fetch a single pool + vaults and run decoder with complete Orca Whirlpool support.

use clap::{ Parser, ValueEnum };
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::{ decoders, AccountData, PriceResult };
use screenerbot::pools::types::{ ProgramKind, SOL_MINT };
use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, ValueEnum)]
enum PoolKindArg {
    Auto,
    Pumpfun,
    RaydiumCpmm,
    RaydiumClmm,
    RaydiumLegacy,
    MeteoraDlmm,
    MeteoraDamm,
    OrcaWhirlpool,
}

#[derive(Parser, Debug)]
#[command(name = "debug_pool_service", about = "Decode a pool and compute price")]
struct Args {
    #[arg(long)] token_mint: String,
    #[arg(long)] pool: String,
    #[arg(long, value_enum)] program: PoolKindArg,
    #[arg(long, default_value = SOL_MINT)] quote_mint: String,
    #[arg(long, default_value_t = false)] verbose: bool,
    /// Inject internal '--debug-pool-calculator' flag for detailed decoder logs
    #[arg(long, default_value_t = false)]
    internal_calculator_debug: bool,
}

/// Detect program type based on pool account owner
fn detect_program_type(owner: &Pubkey, data_len: usize) -> ProgramKind {
    println!("🔍 Analyzing pool account...");
    println!("📊 Program ID: {}", owner);
    println!("📏 Data length: {} bytes", data_len);
    
    let program_str = owner.to_string();
    let program_kind = match program_str.as_str() {
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
            println!("✅ Identified as Orca Whirlpool");
            ProgramKind::OrcaWhirlpool
        },
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
            println!("✅ Identified as Raydium Legacy AMM");
            ProgramKind::RaydiumLegacyAmm
        },
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
            println!("✅ Identified as Raydium CPMM");
            ProgramKind::RaydiumCpmm
        },
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
            println!("✅ Identified as Raydium CLMM");
            ProgramKind::RaydiumClmm
        },
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => {
            println!("✅ Identified as Pump.fun");
            ProgramKind::PumpFun
        },
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
            println!("✅ Identified as Meteora DLMM");
            ProgramKind::MeteoraDlmm
        },
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => {
            println!("✅ Identified as Meteora DAMM");
            ProgramKind::MeteoraDamm
        },
        _ => {
            println!("❓ Unknown program ID, using heuristics based on data length:");
            match data_len {
                752 => {
                    println!("  📏 752 bytes suggests Raydium Legacy AMM");
                    ProgramKind::RaydiumLegacyAmm
                },
                1544 => {
                    println!("  📏 1544 bytes suggests Raydium CPMM");
                    ProgramKind::RaydiumCpmm
                },
                132 => {
                    println!("  📏 132 bytes suggests Pump.fun");
                    ProgramKind::PumpFun
                },
                653 => {
                    println!("  📏 653 bytes suggests Orca Whirlpool");
                    ProgramKind::OrcaWhirlpool
                },
                _ => {
                    println!("  ❌ Unknown data length, defaulting to Raydium Legacy AMM");
                    ProgramKind::RaydiumLegacyAmm
                }
            }
        }
    };
    
    println!("🎯 Final detection: {:?}", program_kind);
    program_kind
}

fn read_pubkey_at(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }
    
    let bytes = &data[offset..offset + 32];
    let pubkey = Pubkey::try_from(bytes).ok()?;
    Some(pubkey.to_string())
}

fn extract_orca_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 653 {
        return None;
    }
    
    // Use the exact Orca Whirlpool structure offsets as found in official source
    let mut offset = 8; // Skip discriminator
    
    // Skip whirlpools_config (32 bytes)
    offset += 32;
    // Skip whirlpool_bump (1 byte)
    offset += 1;
    // Skip tick_spacing (2 bytes)
    offset += 2;
    // Skip fee_tier_index_seed (2 bytes)
    offset += 2;
    // Skip fee_rate (2 bytes)
    offset += 2;
    // Skip protocol_fee_rate (2 bytes)
    offset += 2;
    // Skip liquidity (16 bytes)
    offset += 16;
    // Skip sqrt_price (16 bytes)
    offset += 16;
    // Skip tick_current_index (4 bytes)
    offset += 4;
    // Skip protocol_fee_owed_a (8 bytes)
    offset += 8;
    // Skip protocol_fee_owed_b (8 bytes)
    offset += 8;
    
    // Now we're at token_mint_a (offset should be 99)
    let token_mint_a_offset = offset;
    let token_mint_a = read_pubkey_at(data, token_mint_a_offset)?;
    offset += 32;
    
    // token_vault_a (offset should be 131)
    let token_vault_a_offset = offset;
    let token_vault_a = read_pubkey_at(data, token_vault_a_offset)?;
    offset += 32;
    
    // Skip fee_growth_global_a (16 bytes)
    offset += 16;
    
    // token_mint_b (offset should be 179)
    let token_mint_b_offset = offset;
    let token_mint_b = read_pubkey_at(data, token_mint_b_offset)?;
    offset += 32;
    
    // token_vault_b (offset should be 211)
    let token_vault_b_offset = offset;
    let token_vault_b = read_pubkey_at(data, token_vault_b_offset)?;
    
    println!("🪙 Orca Whirlpool token structure (official offsets):");
    println!("  Token Mint A ({}): {}", token_mint_a_offset, token_mint_a);
    println!("  Token Vault A ({}): {}", token_vault_a_offset, token_vault_a);
    println!("  Token Mint B ({}): {}", token_mint_b_offset, token_mint_b);
    println!("  Token Vault B ({}): {}", token_vault_b_offset, token_vault_b);
    
    // Determine which is SOL and which is the token
    let sol_mint_str = "So11111111111111111111111111111111111111112";
    
    if token_mint_a == sol_mint_str {
        // A is SOL, B is token
        println!("✅ Found SOL as Token A, returning (token_vault_B, sol_vault_A)");
        Some((token_vault_b.to_string(), token_vault_a.to_string())) // (token_vault, sol_vault)
    } else if token_mint_b == sol_mint_str {
        // B is SOL, A is token  
        println!("✅ Found SOL as Token B, returning (token_vault_A, sol_vault_B)");
        Some((token_vault_a.to_string(), token_vault_b.to_string())) // (token_vault, sol_vault)
    } else {
        println!("❌ Neither mint is SOL in this Orca Whirlpool");
        println!("🔍 Target token: HzHwfQwXyQ77E5yPFU1sLVeDuc7Zg4PeyXXVF7qtGxch");
        println!("🔧 This might be a token/token pool or the target token might not match");
        
        // For debugging, let's still return the vaults in order
        Some((token_vault_a.to_string(), token_vault_b.to_string()))
    }
}

fn extract_raydium_vaults(data: &[u8]) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    // Use same offsets as the CLMM decoder for consistency
    // Based on Raydium CLMM PoolState struct
    if data.len() < 200 {
        return Err(format!("CLMM pool data too short: {} bytes", data.len()).into());
    }

    // Skip discriminator (8 bytes) and bump (1 byte)
    let mut offset = 8 + 1;
    // Skip amm_config (32 bytes) and owner (32 bytes)
    offset += 32 + 32;
    // Skip token mints (32 + 32 bytes)
    offset += 32 + 32;
    
    // Extract token vaults
    let vault_0_bytes = &data[offset..offset + 32];
    let vault_0 = Pubkey::new_from_array(
        vault_0_bytes.try_into()
            .map_err(|_| "Failed to convert vault_0 bytes")?
    );
    offset += 32;
    
    let vault_1_bytes = &data[offset..offset + 32];
    let vault_1 = Pubkey::new_from_array(
        vault_1_bytes.try_into()
            .map_err(|_| "Failed to convert vault_1 bytes")?
    );
    
    println!("Base Vault (vault_0): {}", vault_0);
    println!("Quote Vault (vault_1): {}", vault_1);
    
    Ok((vault_0, vault_1))
}

fn extract_meteora_vaults(pool_data: &[u8]) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    if pool_data.len() < 500 {
        return Err("Insufficient data for Meteora pool".into());
    }
    
    let vault_a_offset = 100;
    let vault_b_offset = 132;
    
    let vault_a = Pubkey::try_from(&pool_data[vault_a_offset..vault_a_offset + 32])?;
    let vault_b = Pubkey::try_from(&pool_data[vault_b_offset..vault_b_offset + 32])?;
    
    println!("🏦 Extracted Meteora vaults:");
    println!("  Vault A: {}", vault_a);
    println!("  Vault B: {}", vault_b);
    
    Ok((vault_a, vault_b))
}

fn extract_raydium_cpmm_vaults(data: &[u8]) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    if data.len() < 8 + 32 * 10 {
        return Err("Pool data too short for CPMM".into());
    }

    let mut offset = 8; // Skip discriminator

    // Skip amm_config and pool_creator  
    let _amm_config = read_pubkey_at_offset(data, &mut offset)?;
    let _pool_creator = read_pubkey_at_offset(data, &mut offset)?;
    
    // Extract vault addresses
    let token_0_vault = read_pubkey_at_offset(data, &mut offset)?;
    let token_1_vault = read_pubkey_at_offset(data, &mut offset)?;

    println!("Token 0 Vault: {}", token_0_vault);
    println!("Token 1 Vault: {}", token_1_vault);

    Ok((token_0_vault, token_1_vault))
}

fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<Pubkey, Box<dyn std::error::Error>> {
    if *offset + 32 > data.len() {
        return Err("Insufficient data for pubkey".into());
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(
        pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey")?
    );

    Ok(pubkey)
}

async fn get_vault_balances(
    vault_a: &Pubkey,
    vault_b: &Pubkey,
) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let rpc = get_rpc_client();
    
    println!("💰 Fetching vault balances...");
    println!("  Vault A: {}", vault_a);
    println!("  Vault B: {}", vault_b);
    
    let accounts = rpc.get_multiple_accounts(&[*vault_a, *vault_b]).await?;
    if accounts.len() < 2 {
        return Err("Not enough vault accounts fetched".into());
    }
    
    let vault_a_account = accounts[0]
        .as_ref()
        .ok_or("Vault A account not found")?;
    let vault_b_account = accounts[1]
        .as_ref()
        .ok_or("Vault B account not found")?;

    // Check minimum data length for token accounts
    if vault_a_account.data.len() < 72 {
        return Err(format!("Vault A data too short: {} bytes", vault_a_account.data.len()).into());
    }
    if vault_b_account.data.len() < 72 {
        return Err(format!("Vault B data too short: {} bytes", vault_b_account.data.len()).into());
    }
    
    // Token accounts store balance at offset 64 (8 bytes, little endian)
    let balance_a = u64::from_le_bytes(
        vault_a_account.data[64..72]
            .try_into()
            .map_err(|_| "Failed to read vault A balance")?
    );
    let balance_b = u64::from_le_bytes(
        vault_b_account.data[64..72]
            .try_into()
            .map_err(|_| "Failed to read vault B balance")?
    );
    
    println!("💰 Vault balances:");
    println!("  Vault A: {} tokens", balance_a);
    println!("  Vault B: {} tokens", balance_b);
    
    Ok((balance_a, balance_b))
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.internal_calculator_debug {
        // Reconstruct minimal arg list with internal flag so is_debug_pool_calculator_enabled() returns true
        set_cmd_args(vec!["debug_pool_service".to_string(), "--debug-pool-calculator".to_string()]);
    }

    if args.verbose {
        log(
            LogTag::PoolCalculator,
            "START",
            &format!("token={} pool={} program={:?}", args.token_mint, args.pool, args.program)
        );
    }

    if args.token_mint == SOL_MINT {
        eprintln!("Token mint must not be SOL");
        return;
    }

    let rpc = get_rpc_client();
    let pool_pubkey = Pubkey::from_str(&args.pool).expect("Invalid pool pubkey");

    let pool_account = rpc.get_account(&pool_pubkey).await.expect("Failed to fetch pool account");
    if args.verbose {
        log(
            LogTag::PoolCalculator,
            "POOL",
            &format!("owner={} len={}", pool_account.owner, pool_account.data.len())
        );
    }

    // Determine program type
    let program_kind = match args.program {
        PoolKindArg::Auto => detect_program_type(&pool_account.owner, pool_account.data.len()),
        PoolKindArg::Pumpfun => ProgramKind::PumpFun,
        PoolKindArg::RaydiumCpmm => ProgramKind::RaydiumCpmm,
        PoolKindArg::RaydiumClmm => ProgramKind::RaydiumClmm,
        PoolKindArg::RaydiumLegacy => ProgramKind::RaydiumLegacyAmm,
        PoolKindArg::MeteoraDlmm => ProgramKind::MeteoraDlmm,
        PoolKindArg::MeteoraDamm => ProgramKind::MeteoraDamm,
        PoolKindArg::OrcaWhirlpool => ProgramKind::OrcaWhirlpool,
    };

    println!("🎯 Using program type: {:?}", program_kind);

    // Extract vault addresses based on program type
    let vaults = match program_kind {
        ProgramKind::OrcaWhirlpool => {
            if let Some((vault_a, vault_b)) = extract_orca_vaults(&pool_account.data) {
                (
                    Pubkey::from_str(&vault_a).expect("Invalid vault A"),
                    Pubkey::from_str(&vault_b).expect("Invalid vault B")
                )
            } else {
                eprintln!("❌ Failed to extract Orca Whirlpool vaults");
                return;
            }
        },
        ProgramKind::RaydiumLegacyAmm => {
            match extract_raydium_vaults(&pool_account.data) {
                Ok(vaults) => vaults,
                Err(e) => {
                    eprintln!("❌ Failed to extract Raydium Legacy vaults: {}", e);
                    return;
                }
            }
        },
        ProgramKind::RaydiumCpmm => {
            match extract_raydium_cpmm_vaults(&pool_account.data) {
                Ok(vaults) => vaults,
                Err(e) => {
                    eprintln!("❌ Failed to extract Raydium CPMM vaults: {}", e);
                    return;
                }
            }
        },
        ProgramKind::RaydiumClmm => {
            match extract_raydium_vaults(&pool_account.data) {
                Ok(vaults) => vaults,
                Err(e) => {
                    eprintln!("❌ Failed to extract Raydium CLMM vaults: {}", e);
                    return;
                }
            }
        },
        ProgramKind::MeteoraDlmm | ProgramKind::MeteoraDamm => {
            match extract_meteora_vaults(&pool_account.data) {
                Ok(vaults) => vaults,
                Err(e) => {
                    eprintln!("❌ Failed to extract Meteora vaults: {}", e);
                    return;
                }
            }
        },
        ProgramKind::PumpFun => {
            eprintln!("⚠️ Pump.fun pools don't use traditional vault structure");
            return;
        },
        _ => {
            eprintln!("❌ Unsupported program type for vault extraction: {:?}", program_kind);
            return;
        }
    };

    // Get vault balances
    match get_vault_balances(&vaults.0, &vaults.1).await {
        Ok((balance_a, balance_b)) => {
            println!("✅ Successfully extracted vault balances");
            println!("  Total reserves: {} + {} tokens", balance_a, balance_b);
        },
        Err(e) => {
            eprintln!("❌ Failed to get vault balances: {}", e);
        }
    }

    // Now run the actual decoder
    println!("\n🔍 Testing decoder implementation...");
    
    let token_mint_pubkey = Pubkey::from_str(&args.token_mint).expect("Invalid token mint");
    let quote_mint_pubkey = Pubkey::from_str(&args.quote_mint).expect("Invalid quote mint");

    let pool_account_data = AccountData {
        pubkey: pool_pubkey,
        data: pool_account.data,
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: pool_account.lamports,
        owner: pool_account.owner,
    };

    let mut accounts: HashMap<String, AccountData> = HashMap::new();
    accounts.insert(pool_pubkey.to_string(), pool_account_data);

    // Add vault accounts
    match rpc.get_multiple_accounts(&[vaults.0, vaults.1]).await {
        Ok(vault_accounts) => {
            if let Some(vault_a_account) = vault_accounts[0].as_ref() {
                accounts.insert(vaults.0.to_string(), AccountData {
                    pubkey: vaults.0,
                    data: vault_a_account.data.clone(),
                    slot: 0,
                    fetched_at: std::time::Instant::now(),
                    lamports: vault_a_account.lamports,
                    owner: vault_a_account.owner,
                });
            }
            if let Some(vault_b_account) = vault_accounts[1].as_ref() {
                accounts.insert(vaults.1.to_string(), AccountData {
                    pubkey: vaults.1,
                    data: vault_b_account.data.clone(),
                    slot: 0,
                    fetched_at: std::time::Instant::now(),
                    lamports: vault_b_account.lamports,
                    owner: vault_b_account.owner,
                });
            }
        },
        Err(e) => {
            eprintln!("⚠️ Warning: Failed to fetch vault accounts for decoder: {}", e);
        }
    }

    println!("📊 Running decoder with {} accounts", accounts.len());

    match decoders::decode_pool(
        program_kind,
        &accounts,
        &args.token_mint,
        &args.quote_mint,
    ) {
        Some(price_result) => {
            println!("🎉 Successfully decoded pool!");
            println!("💰 Price Result:");
            println!("  Price SOL: {}", price_result.price_sol);
            println!("  Price USD: {}", price_result.price_usd);
            println!("  Reserves: {} token, {} SOL", price_result.token_reserves, price_result.sol_reserves);
            
            if args.verbose {
                log(
                    LogTag::PoolCalculator,
                    "SUCCESS",
                    &format!("price={} reserves={}+{}", 
                        price_result.price_sol, 
                        price_result.token_reserves, 
                        price_result.sol_reserves
                    )
                );
            }
        },
        None => {
            eprintln!("❌ Decoder failed: No result returned");
            
            if args.verbose {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    "decoder returned None"
                );
            }
        }
    }
}
