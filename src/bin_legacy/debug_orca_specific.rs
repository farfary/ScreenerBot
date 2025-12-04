//! Specialized Orca Whirlpool decoder debugging tool
//!
//! This tool is designed to debug a specific Orca Whirlpool pool with extreme verbosity
//! to identify and fix decoding issues. It fetches all required accounts and traces
//! every step of the decoding process.
//!
//! Features:
//! - Fetches pool account and all vault accounts
//! - Byte-by-byte pool data analysis
//! - Complete offset mapping verification
//! - Token account balance extraction
//! - Price calculation step-by-step debugging
//! - Comparison with expected values from JSON data
//!
//! Usage:
//! cargo run --bin debug_orca_specific -- --pool <POOL_ADDRESS>

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::orca_whirlpool::OrcaWhirlpoolDecoder;
use screenerbot::pools::decoders::PoolDecoder;
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{ORCA_WHIRLPOOL_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
  name = "debug_orca_specific",
  about = "Debug specific Orca Whirlpool pool with extreme verbosity"
)]
struct Args {
  /// Pool address to debug
  #[arg(short, long, default_value = "")]
  pool: String,

  /// Show raw hex data
  #[arg(long)]
  show_hex: bool,

  /// Compare with expected JSON values
  #[arg(long)]
  compare_json: bool,
}

/// Expected values from the JSON data provided
#[derive(Debug)]
struct ExpectedValues {
  whirlpools_config: String,
  whirlpool_bump: u8,
  tick_spacing: u16,
  fee_rate: u16,
  protocol_fee_rate: u16,
  liquidity: u128,
  sqrt_price: u128,
  tick_current_index: i32,
  protocol_fee_owed_a: u64,
  protocol_fee_owed_b: u64,
  token_mint_a: String,
  token_vault_a: String,
  fee_growth_global_a: u128,
  token_mint_b: String,
  token_vault_b: String,
  fee_growth_global_b: u128,
}

impl ExpectedValues {
  fn new() -> Self {
    Self {
      whirlpools_config: "2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ".to_string(),
      whirlpool_bump: 254,
      tick_spacing: 8,
      fee_rate: 500,
      protocol_fee_rate: 1300,
      liquidity: 140999358787746,
      sqrt_price: 570014308189661989929,
      tick_current_index: 68618,
      protocol_fee_owed_a: 17256369,
      protocol_fee_owed_b: 16571571593,
      token_mint_a: "So11111111111111111111111111111111111111112".to_string(),
      token_vault_a: "ES7yhSrYeFo4U1PfJHNRkbfCWxCwPLk2DjrEbmN8bg58".to_string(),
      fee_growth_global_a: 39491275638829089,
      token_mint_b: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
      token_vault_b: "4dmvFGeQH2eqa3ktNHMgm4wZ8vuTukBiK9M7gxW5oR9F".to_string(),
      fee_growth_global_b: 30834751189288089992,
    }
  }
}

/// Parse pool data with extreme verbosity and return (mint_a, mint_b, vault_a, vault_b)
fn parse_pool_data_verbose(
  data: &[u8],
  expected: &ExpectedValues,
  show_hex: bool,
) -> Option<(String, String, String, String)> {
  println!("\n DETAILED POOL DATA ANALYSIS");
  println!("==============================");
  println!("Pool data size: {} bytes", data.len());

  if data.len() < 653 {
 println!("ERROR: Pool data too small (need at least 653 bytes)");
    return None;
  }

  if show_hex {
    println!("\n RAW HEX DATA (first 200 bytes):");
    for (i, chunk) in data.chunks(16).take(12).enumerate() {
      print!("{:04x}: ", i * 16);
      for byte in chunk {
        print!("{:02x} ", byte);
      }
      println!();
    }
  }

  let mut offset = 0;

  // Discriminator (8 bytes)
  println!("\n DISCRIMINATOR");
  println!("Offset: {} (0x{:x})", offset, offset);
  let discriminator = &data[offset..offset + 8];
  print!("Value: ");
  for byte in discriminator {
    print!("{:02x} ", byte);
  }
  println!();
  offset += 8;

  // Whirlpools Config (32 bytes)
  println!("\n WHIRLPOOLS_CONFIG");
  println!("Offset: {} (0x{:x})", offset, offset);
  let config_bytes = &data[offset..offset + 32];
  let config_pubkey = Pubkey::try_from(config_bytes).ok()?;
  let config_str = config_pubkey.to_string();
  println!("Parsed: {}", config_str);
  println!("Expected: {}", expected.whirlpools_config);
  println!(
    "Match: {}",
    if config_str == expected.whirlpools_config {
      ""
    } else {
      ""
    }
  );
  offset += 32;

  // Whirlpool Bump (1 byte)
  println!("\n WHIRLPOOL_BUMP");
  println!("Offset: {} (0x{:x})", offset, offset);
  let bump = data[offset];
  println!("Parsed: {}", bump);
  println!("Expected: {}", expected.whirlpool_bump);
  println!(
    "Match: {}",
    if bump == expected.whirlpool_bump {
      ""
    } else {
      ""
    }
  );
  offset += 1;

  // Tick Spacing (2 bytes)
  println!("\n TICK_SPACING");
  println!("Offset: {} (0x{:x})", offset, offset);
  let tick_spacing = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);
  println!("Parsed: {}", tick_spacing);
  println!("Expected: {}", expected.tick_spacing);
  println!(
    "Match: {}",
    if tick_spacing == expected.tick_spacing {
      ""
    } else {
      ""
    }
  );
  offset += 2;

  // Fee Tier Index Seed (2 bytes)
  println!("\n FEE_TIER_INDEX_SEED");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_tier_seed = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);
  println!("Parsed: {}", fee_tier_seed);
  println!("Expected: {} (from JSON: [8,0])", 8u16);
  offset += 2;

  // Fee Rate (2 bytes)
  println!("\n FEE_RATE");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);
  println!("Parsed: {}", fee_rate);
  println!("Expected: {}", expected.fee_rate);
  println!(
    "Match: {}",
    if fee_rate == expected.fee_rate {
      ""
    } else {
      ""
    }
  );
  offset += 2;

  // Protocol Fee Rate (2 bytes)
  println!("\n PROTOCOL_FEE_RATE");
  println!("Offset: {} (0x{:x})", offset, offset);
  let protocol_fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);
  println!("Parsed: {}", protocol_fee_rate);
  println!("Expected: {}", expected.protocol_fee_rate);
  println!(
    "Match: {}",
    if protocol_fee_rate == expected.protocol_fee_rate {
      ""
    } else {
      ""
    }
  );
  offset += 2;

  // Liquidity (16 bytes)
  println!("\n LIQUIDITY");
  println!("Offset: {} (0x{:x})", offset, offset);
  let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
  println!("Parsed: {}", liquidity);
  println!("Expected: {}", expected.liquidity);
  println!(
    "Match: {}",
    if liquidity == expected.liquidity {
      ""
    } else {
      ""
    }
  );
  offset += 16;

  // Sqrt Price (16 bytes) - CRITICAL for price calculation
  println!("\n SQRT_PRICE");
  println!("Offset: {} (0x{:x})", offset, offset);
  let sqrt_price = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
  println!("Parsed: {}", sqrt_price);
  println!("Expected: {}", expected.sqrt_price);
  println!(
    "Match: {}",
    if sqrt_price == expected.sqrt_price {
      ""
    } else {
      ""
    }
  );

  // Calculate price from sqrt_price
  if sqrt_price > 0 {
    let sqrt_price_scaled = (sqrt_price as f64) / (2_f64).powi(64);
    let price_raw = sqrt_price_scaled * sqrt_price_scaled;
    println!("Price calculation debug:");
 println!("sqrt_price: {}", sqrt_price);
 println!("sqrt_price_scaled: {}", sqrt_price_scaled);
 println!("price_raw (token_a/token_b): {}", price_raw);

    // Since token_a is SOL and token_b is the target token
    // price_raw = SOL/TOKEN, we want TOKEN/SOL
    let token_sol_price = 1.0 / price_raw;
 println!("token/sol price (inverted): {}", token_sol_price);
  }
  offset += 16;

  // Tick Current Index (4 bytes)
  println!("\n TICK_CURRENT_INDEX");
  println!("Offset: {} (0x{:x})", offset, offset);
  let tick_current = i32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
  println!("Parsed: {}", tick_current);
  println!("Expected: {}", expected.tick_current_index);
  println!(
    "Match: {}",
    if tick_current == expected.tick_current_index {
      ""
    } else {
      ""
    }
  );
  offset += 4;

  // Protocol Fee Owed A (8 bytes)
  println!("\n PROTOCOL_FEE_OWED_A");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_owed_a = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
  println!("Parsed: {}", fee_owed_a);
  println!("Expected: {}", expected.protocol_fee_owed_a);
  println!(
    "Match: {}",
    if fee_owed_a == expected.protocol_fee_owed_a {
      ""
    } else {
      ""
    }
  );
  offset += 8;

  // Protocol Fee Owed B (8 bytes)
  println!("\n PROTOCOL_FEE_OWED_B");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_owed_b = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
  println!("Parsed: {}", fee_owed_b);
  println!("Expected: {}", expected.protocol_fee_owed_b);
  println!(
    "Match: {}",
    if fee_owed_b == expected.protocol_fee_owed_b {
      ""
    } else {
      ""
    }
  );
  offset += 8;

  // Token Mint A (32 bytes)
  println!("\n🪙 TOKEN_MINT_A");
  println!("Offset: {} (0x{:x})", offset, offset);
  let mint_a_bytes = &data[offset..offset + 32];
  let mint_a_pubkey = Pubkey::try_from(mint_a_bytes).ok()?;
  let mint_a_str = mint_a_pubkey.to_string();
  println!("Parsed: {}", mint_a_str);
  println!("Expected: {}", expected.token_mint_a);
  println!(
    "Match: {}",
    if mint_a_str == expected.token_mint_a {
      ""
    } else {
      ""
    }
  );
  println!(
    "Is SOL: {}",
 if mint_a_str == SOL_MINT { ""} else { ""}
  );
  offset += 32;

  // Token Vault A (32 bytes)
  println!("\n TOKEN_VAULT_A");
  println!("Offset: {} (0x{:x})", offset, offset);
  let vault_a_bytes = &data[offset..offset + 32];
  let vault_a_pubkey = Pubkey::try_from(vault_a_bytes).ok()?;
  let vault_a_str = vault_a_pubkey.to_string();
  println!("Parsed: {}", vault_a_str);
  println!("Expected: {}", expected.token_vault_a);
  println!(
    "Match: {}",
    if vault_a_str == expected.token_vault_a {
      ""
    } else {
      ""
    }
  );
  offset += 32;

  // Fee Growth Global A (16 bytes)
  println!("\n FEE_GROWTH_GLOBAL_A");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_growth_a = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
  println!("Parsed: {}", fee_growth_a);
  println!("Expected: {}", expected.fee_growth_global_a);
  println!(
    "Match: {}",
    if fee_growth_a == expected.fee_growth_global_a {
      ""
    } else {
      ""
    }
  );
  offset += 16;

  // Token Mint B (32 bytes)
  println!("\n🪙 TOKEN_MINT_B");
  println!("Offset: {} (0x{:x})", offset, offset);
  let mint_b_bytes = &data[offset..offset + 32];
  let mint_b_pubkey = Pubkey::try_from(mint_b_bytes).ok()?;
  let mint_b_str = mint_b_pubkey.to_string();
  println!("Parsed: {}", mint_b_str);
  println!("Expected: {}", expected.token_mint_b);
  println!(
    "Match: {}",
    if mint_b_str == expected.token_mint_b {
      ""
    } else {
      ""
    }
  );
  println!(
    "Is SOL: {}",
 if mint_b_str == SOL_MINT { ""} else { ""}
  );
  offset += 32;

  // Token Vault B (32 bytes)
  println!("\n TOKEN_VAULT_B");
  println!("Offset: {} (0x{:x})", offset, offset);
  let vault_b_bytes = &data[offset..offset + 32];
  let vault_b_pubkey = Pubkey::try_from(vault_b_bytes).ok()?;
  let vault_b_str = vault_b_pubkey.to_string();
  println!("Parsed: {}", vault_b_str);
  println!("Expected: {}", expected.token_vault_b);
  println!(
    "Match: {}",
    if vault_b_str == expected.token_vault_b {
      ""
    } else {
      ""
    }
  );
  offset += 32;

  // Fee Growth Global B (16 bytes)
  println!("\n FEE_GROWTH_GLOBAL_B");
  println!("Offset: {} (0x{:x})", offset, offset);
  let fee_growth_b = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
  println!("Parsed: {}", fee_growth_b);
  println!("Expected: {}", expected.fee_growth_global_b);
  println!(
    "Match: {}",
    if fee_growth_b == expected.fee_growth_global_b {
      ""
    } else {
      ""
    }
  );
  offset += 16;

  println!("\n PARSING SUMMARY");
  println!("==================");
  println!("Total offset processed: {} bytes", offset);
  println!("Remaining data: {} bytes", data.len() - offset);

  // Identify token pair orientation
  println!("\n TOKEN PAIR ANALYSIS");
  println!("======================");
  if mint_a_str == SOL_MINT {
 println!("Token A is SOL, Token B is target token");
 println!("Target token: {}", mint_b_str);
 println!("SOL vault: {}", vault_a_str);
 println!("Token vault: {}", vault_b_str);
  } else if mint_b_str == SOL_MINT {
 println!("Token B is SOL, Token A is target token");
 println!("Target token: {}", mint_a_str);
 println!("SOL vault: {}", vault_b_str);
 println!("Token vault: {}", vault_a_str);
  } else {
 println!("ERROR: Neither token is SOL!");
    println!("Token A: {}", mint_a_str);
    println!("Token B: {}", mint_b_str);
  }

  Some((mint_a_str, mint_b_str, vault_a_str, vault_b_str))
}

/// Fetch and analyze token account data
async fn analyze_token_account(vault_address: &str, vault_name: &str) -> Option<u64> {
  println!("\n ANALYZING {} VAULT: {}", vault_name, vault_address);
  println!("===========================================");

  let rpc_client = get_rpc_client();
  let vault_pubkey = match parse_pubkey(vault_address) {
    Ok(pk) => pk,
    Err(e) => {
 println!("ERROR: Invalid vault pubkey: {}", e);
      return None;
    }
  };

  let vault_account = match rpc_client.client().get_account(&vault_pubkey) {
    Ok(account) => account,
    Err(e) => {
 println!("ERROR: Failed to fetch vault account: {}", e);
      return None;
    }
  };

 println!("Account fetched successfully");
  println!("Account owner: {}", vault_account.owner);
  println!("Account data size: {} bytes", vault_account.data.len());
  println!("Account lamports: {}", vault_account.lamports);

  if vault_account.data.len() < 72 {
 println!("ERROR: Token account data too small (need at least 72 bytes)");
    return None;
  }

  // Token account structure:
  // 0-32: mint (32 bytes)
  // 32-64: owner (32 bytes)
  // 64-72: amount (8 bytes)
  // 72-73: delegate option (1 byte)
  // etc.

  let mint_bytes = &vault_account.data[0..32];
  let mint_pubkey = Pubkey::try_from(mint_bytes).ok()?;
  let mint_str = mint_pubkey.to_string();
  println!("Token mint: {}", mint_str);

  let owner_bytes = &vault_account.data[32..64];
  let owner_pubkey = Pubkey::try_from(owner_bytes).ok()?;
  let owner_str = owner_pubkey.to_string();
  println!("Token owner: {}", owner_str);

  let amount_bytes = &vault_account.data[64..72];
  let amount = u64::from_le_bytes(amount_bytes.try_into().ok()?);
  println!("Token amount (raw): {}", amount);

  // Get token decimals for proper display
  if let Some(decimals) = get_token_decimals_sync(&mint_str) {
    let adjusted_amount = (amount as f64) / (10_f64).powi(decimals as i32);
    println!("Token amount (adjusted): {:.9}", adjusted_amount);
    println!("Token decimals: {}", decimals);
  } else {
 println!("WARNING: Could not get token decimals for {}", mint_str);
  }

  Some(amount)
}

/// Test the Orca decoder with verbose debugging
async fn test_orca_decoder_verbose(
  pool_accounts: &HashMap<String, AccountData>,
  target_token_mint: &str,
) {
  println!("\n TESTING ORCA DECODER");
  println!("========================");

  // Enable debug mode
  let mut cmd_args = vec!["debug_orca_specific".to_string()];
  cmd_args.push("--debug-pool-decoders".to_string());
  set_cmd_args(cmd_args);

  println!("Available accounts:");
  for (addr, _) in pool_accounts {
 println!("- {}", addr);
  }

  // Test with TOKEN/SOL orientation (BONK/SOL)
  println!("\n Testing TOKEN/SOL orientation...");
  let result1 = OrcaWhirlpoolDecoder::decode_and_calculate(
    pool_accounts,
    target_token_mint, // Target as base
 SOL_MINT, // SOL as quote
  );

  match result1 {
    Some(price_result) => {
 println!("TOKEN/SOL decode successful!");
      println!("Price: {:.12} SOL", price_result.price_sol);
      println!("SOL reserves: {:.9}", price_result.sol_reserves);
      println!("Token reserves: {:.6}", price_result.token_reserves);
    }
    None => {
 println!("TOKEN/SOL decode failed");
    }
  }

  // Test with SOL/TOKEN orientation
  println!("\n Testing SOL/TOKEN orientation...");
  let result2 = OrcaWhirlpoolDecoder::decode_and_calculate(
    pool_accounts,
 SOL_MINT, // SOL as base
    target_token_mint, // Target as quote
  );

  match result2 {
    Some(price_result) => {
 println!("SOL/TOKEN decode successful!");
      println!("Price: {:.12} SOL", price_result.price_sol);
      println!("SOL reserves: {:.9}", price_result.sol_reserves);
      println!("Token reserves: {:.6}", price_result.token_reserves);
    }
    None => {
 println!("SOL/TOKEN decode failed");
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();

  // Default to the pool from the JSON if not provided
  let pool_address = if args.pool.is_empty() {
    // We need to determine the pool address from the vault addresses or other means
    // For now, let's use a placeholder and require the user to provide it
 println!("ERROR: Please provide the pool address with --pool");
    std::process::exit(1);
  } else {
    args.pool
  };

 println!("ORCA WHIRLPOOL SPECIFIC DEBUGGER");
  println!("====================================");
  println!("Pool address: {}", pool_address);

  let expected = ExpectedValues::new();

  // Initialize RPC client
  logger::info(
    LogTag::System, "Initializing RPC client...");
  if let Err(e) = screenerbot::rpc::init_rpc_client() {
 println!("ERROR: RPC initialization failed: {}", e);
    std::process::exit(1);
  }

  // Fetch pool account
  println!("\n FETCHING POOL ACCOUNT");
  println!("========================");
  let rpc_client = get_rpc_client();
  let pool_pubkey = match parse_pubkey(&pool_address) {
    Ok(pk) => pk,
    Err(e) => {
 println!("ERROR: Invalid pool address: {}", e);
      std::process::exit(1);
    }
  };

  let pool_account = match rpc_client.client().get_account(&pool_pubkey) {
    Ok(account) => account,
    Err(e) => {
 println!("ERROR: Failed to fetch pool account: {}", e);
      std::process::exit(1);
    }
  };

 println!("Pool account fetched successfully");
  println!("Owner: {}", pool_account.owner);
  println!("Data size: {} bytes", pool_account.data.len());
  println!("Expected owner: {}", ORCA_WHIRLPOOL_PROGRAM_ID);
  println!(
    "Owner match: {}",
    if pool_account.owner.to_string() == ORCA_WHIRLPOOL_PROGRAM_ID {
      ""
    } else {
      ""
    }
  );

  // Parse pool data with extreme verbosity and capture parsed keys
  let (mint_a, mint_b, vault_a, vault_b) =
    match parse_pool_data_verbose(&pool_account.data, &expected, args.show_hex) {
      Some(t) => t,
      None => {
 println!("ERROR: Failed to parse pool core fields");
        std::process::exit(1);
      }
    };

  // Fetch vault accounts
  println!("\n FETCHING VAULT ACCOUNTS");
  println!("==========================");

  // Determine SOL orientation to know which vault is SOL and which is token
  let (sol_vault, token_vault, target_token_mint) = if mint_a == SOL_MINT {
    (vault_a.clone(), vault_b.clone(), mint_b.clone())
  } else if mint_b == SOL_MINT {
    (vault_b.clone(), vault_a.clone(), mint_a.clone())
  } else {
    // Fallback: assume mint_b is target, vault_b is token vault
    (vault_a.clone(), vault_b.clone(), mint_b.clone())
  };

  let sol_vault_balance = analyze_token_account(&sol_vault, "SOL").await;
  let token_vault_balance = analyze_token_account(&token_vault, "TOKEN").await;

  // Create accounts map for decoder testing
  let mut pool_accounts = HashMap::new();
  pool_accounts.insert(
    pool_address.clone(),
    AccountData {
      pubkey: pool_pubkey,
      data: pool_account.data.clone(),
      slot: 0,
      fetched_at: std::time::Instant::now(),
      lamports: pool_account.lamports,
      owner: pool_account.owner,
    },
  );

  // Add vault accounts if fetched successfully
  if let Ok(vault_a_pubkey) = parse_pubkey(&vault_a) {
    if let Ok(vault_a_account) = rpc_client.client().get_account(&vault_a_pubkey) {
      pool_accounts.insert(
        vault_a.clone(),
        AccountData {
          pubkey: vault_a_pubkey,
          data: vault_a_account.data,
          slot: 0,
          fetched_at: std::time::Instant::now(),
          lamports: vault_a_account.lamports,
          owner: vault_a_account.owner,
        },
      );
    }
  }

  if let Ok(vault_b_pubkey) = parse_pubkey(&vault_b) {
    if let Ok(vault_b_account) = rpc_client.client().get_account(&vault_b_pubkey) {
      pool_accounts.insert(
        vault_b.clone(),
        AccountData {
          pubkey: vault_b_pubkey,
          data: vault_b_account.data,
          slot: 0,
          fetched_at: std::time::Instant::now(),
          lamports: vault_b_account.lamports,
          owner: vault_b_account.owner,
        },
      );
    }
  }

  // Test decoder with verbose debugging
  test_orca_decoder_verbose(&pool_accounts, &target_token_mint).await;

  // Calculate expected price manually for comparison
  if let (Some(sol_balance), Some(token_balance)) = (sol_vault_balance, token_vault_balance) {
    println!("\n MANUAL PRICE CALCULATION");
    println!("============================");

    // Get token decimals
    let token_decimals = get_token_decimals_sync(&target_token_mint).unwrap_or(9);
    let sol_decimals = SOL_DECIMALS;

    let sol_adjusted = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
    let token_adjusted = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

    println!("SOL balance (raw): {}", sol_balance);
    println!("SOL balance (adjusted): {:.9}", sol_adjusted);
    println!("Token balance (raw): {}", token_balance);
    println!("Token balance (adjusted): {:.6}", token_adjusted);

    if token_adjusted > 0.0 {
      let simple_price = sol_adjusted / token_adjusted;
      println!("Simple price (SOL/Token): {:.12} SOL", simple_price);
    }
  }

  // Final summary with API diff
  println!("\n FINAL SUMMARY & API DIFF");
  println!("===========================");

  // Reuse the same accounts to compute decoder price (TOKEN/SOL orientation)
  let decoded_price_opt =
    OrcaWhirlpoolDecoder::decode_and_calculate(&pool_accounts, &target_token_mint, SOL_MINT);

  match decoded_price_opt {
    Some(decoded) => {
      let our_price = decoded.price_sol;
      println!("Our decoded price: {:.12} SOL", our_price);

      // Fetch DexScreener API price
      if let Err(e) = init_dexscreener_api().await {
 println!("API init failed: {}", e);
      }

      match get_global_dexscreener_api().await {
        Ok(api) => {
          let mut guard = api.lock().await;
          match guard.get_token_data(&target_token_mint).await {
            Ok(Some(api_token)) => {
              // Get price from DexScreener or pool price
              let api_price = api_token
                .price_dexscreener_sol
                .or(api_token.price_pool_sol)
                .unwrap_or(0.0);
              println!("DexScreener price: {:.12} SOL", api_price);

              let abs_diff = (our_price - api_price).abs();
              let pct_diff = if api_price > 0.0 {
                (abs_diff / api_price) * 100.0
              } else {
                0.0
              };
              println!("Diff: {:.12} SOL ({:.4}%)", abs_diff, pct_diff);
            }
            Ok(None) => {
              println!(
 "DexScreener returned no data for token {}",
                &expected.token_mint_b
              );
            }
            Err(err) => {
 println!("DexScreener error: {}", err);
            }
          }
        }
        Err(e) => {
 println!("DexScreener API not available: {}", e);
        }
      }
    }
    None => {
 println!("Could not decode price to compare with API");
    }
  }

  println!("\n DEBUGGING COMPLETE");
  Ok(())
}
