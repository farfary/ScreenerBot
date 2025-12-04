//! Raydium Legacy AMM specific decoder debugging tool
//!
//! This tool mirrors the CLMM debugger but for Raydium Legacy AMM pools.
//! It fetches the pool and vault accounts, decodes key fields using fixed
//! legacy offsets, computes price via reserves (+ decimals), runs the main
//! decoder in both orientations, and compares the result to DexScreener SOL price.
//!
//! Usage:
//! cargo run --bin debug_raydium_legacy_amm_specific -- --pool <POOL_ADDRESS>

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::decoders::raydium_legacy_amm::RaydiumLegacyAmmDecoder;
use screenerbot::pools::decoders::PoolDecoder;
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{RAYDIUM_LEGACY_AMM_PROGRAM_ID, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, parse_pubkey};
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::{decimals::SOL_DECIMALS, get_token_decimals_sync};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(
  name = "debug_raydium_legacy_amm_specific",
  about = "Debug a Raydium Legacy AMM pool with verbose parsing and price checks"
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
  set_cmd_args(vec![
    "debug_raydium_legacy_amm_specific".to_string(),
    "--debug-pool-decoders".to_string(),
  ]);

  println!("\n RAYDIUM LEGACY AMM SPECIFIC DEBUGGER");
  println!("======================================");
  println!("Pool address: {}", args.pool);

  // Init RPC
  logger::info(
    LogTag::System, "Initializing RPC client...");
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

  println!("\n POOL ACCOUNT");
  println!("==============");
  println!("Owner: {}", pool_acc.owner);
  println!("Data size: {} bytes", pool_acc.data.len());
  println!(
    "Owner is Raydium Legacy AMM: {}",
    if pool_acc.owner.to_string() == RAYDIUM_LEGACY_AMM_PROGRAM_ID {
      ""
    } else {
      ""
    }
  );

  if args.show_hex {
    println!("\n RAW HEX (first 192 bytes)");
    for (i, chunk) in pool_acc.data.chunks(16).take(12).enumerate() {
      print!("{:04x}: ", i * 16);
      for b in chunk {
        print!("{:02x} ", b);
      }
      println!();
    }
  }

  // Parse minimal legacy fields by fixed offsets
  let info = match parse_legacy_minimal(&pool_acc.data) {
    Some(v) => v,
    None => {
      eprintln!("Failed to parse Legacy AMM pool (size or offsets mismatch)");
      std::process::exit(1);
    }
  };

  println!("\n PARSED FIELDS");
  println!("================");
  println!("coin_mint: {}", info.coin_mint);
  println!("pc_mint: {}", info.pc_mint);
  println!("coin_vault: {}", info.coin_vault);
  println!("pc_vault: {}", info.pc_vault);

  // Probe candidate vault offsets and try to match by mint
  println!("\n FETCHING VAULT ACCOUNTS\n==========================");
  let candidates_off = [0x150usize, 0x160, 0x170, 0x180];
  let mut candidates: Vec<(usize, String)> = Vec::new();
  for off in candidates_off {
    if let Some(pk) = read_pubkey(&pool_acc.data, off) {
      candidates.push((off, pk));
    }
  }

  println!("Candidates at offsets:");
  for (off, pk) in &candidates {
 println!("0x{off:03x}: {pk}");
  }

  let mut vault_map: HashMap<String, (usize, solana_sdk::account::Account)> = HashMap::new();
  for (off, pk_s) in &candidates {
    if let Ok(pk) = parse_pubkey(pk_s) {
      match rpc.client().get_account(&pk) {
        Ok(acc) => {
 println!("Fetched account for {pk_s} (owner: {})", acc.owner);
          vault_map.insert(pk_s.clone(), (*off, acc));
        }
        Err(e) => {
 println!("Failed to fetch {pk_s}: {}", e);
        }
      }
    }
  }

  // Choose coin_vault/pc_vault by matching mint fields
  let mut coin_vault_pk = None;
  let mut pc_vault_pk = None;
  for (pk_s, (_off, acc)) in &vault_map {
    if acc.data.len() >= 32 {
      if let Some(mint_s) = read_pubkey(&acc.data, 0) {
        if mint_s == info.coin_mint {
          coin_vault_pk = Some(parse_pubkey(pk_s)?);
        }
        if mint_s == info.pc_mint {
          pc_vault_pk = Some(parse_pubkey(pk_s)?);
        }
      }
    }
  }

  // Fallback to parsed if matching by mint fails
  let coin_vault_pk = coin_vault_pk.unwrap_or(parse_pubkey(&info.coin_vault)?);
  let pc_vault_pk = pc_vault_pk.unwrap_or(parse_pubkey(&info.pc_vault)?);

  let coin_vault_acc = rpc.client().get_account(&coin_vault_pk)?;
  let pc_vault_acc = rpc.client().get_account(&pc_vault_pk)?;

  let (coin_amt, pc_amt) = (
    decode_token_amount(&coin_vault_acc.data).unwrap_or(0),
    decode_token_amount(&pc_vault_acc.data).unwrap_or(0),
  );

  println!(
    "coin_vault final: {} (mint {})",
    coin_vault_pk,
    read_pubkey(&coin_vault_acc.data, 0).unwrap_or_default()
  );
  println!(
 "pc_vault final: {} (mint {})",
    pc_vault_pk,
    read_pubkey(&pc_vault_acc.data, 0).unwrap_or_default()
  );
  println!("coin_vault amount(raw): {}", coin_amt);
  println!("pc_vault amount(raw): {}", pc_amt);

  // Build accounts map for decoder test
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
    coin_vault_pk.to_string(),
    AccountData {
      pubkey: coin_vault_pk,
      data: coin_vault_acc.data.clone(),
      slot: 0,
      fetched_at: std::time::Instant::now(),
      lamports: coin_vault_acc.lamports,
      owner: coin_vault_acc.owner,
    },
  );
  accounts.insert(
    pc_vault_pk.to_string(),
    AccountData {
      pubkey: pc_vault_pk,
      data: pc_vault_acc.data.clone(),
      slot: 0,
      fetched_at: std::time::Instant::now(),
      lamports: pc_vault_acc.lamports,
      owner: pc_vault_acc.owner,
    },
  );

  // Decide orientation relative to SOL (if present)
  let has_sol = info.coin_mint == SOL_MINT || info.pc_mint == SOL_MINT;
  let target_mint = if has_sol {
    if info.coin_mint == SOL_MINT {
      &info.pc_mint
    } else {
      &info.coin_mint
    }
  } else {
    &info.coin_mint
  };

  // Run decoder in both orientations
  println!("\n DECODER CHECK\n================");
  let res1 = RaydiumLegacyAmmDecoder::decode_and_calculate(&accounts, target_mint, &SOL_MINT);
  match &res1 {
    Some(r) => println!(
      "Orientation TOKEN/SOL → price_sol={:.12} mint={} pool={}",
      r.price_sol, r.mint, r.pool_address
    ),
    None => println!("Orientation TOKEN/SOL → None"),
  }
  let res2 = RaydiumLegacyAmmDecoder::decode_and_calculate(&accounts, &SOL_MINT, target_mint);
  match &res2 {
    Some(r) => println!(
      "Orientation SOL/TOKEN → price_sol={:.12} mint={} pool={}",
      r.price_sol, r.mint, r.pool_address
    ),
    None => println!("Orientation SOL/TOKEN → None"),
  }

  // Manual price from reserves
  println!("\n MANUAL RESERVES PRICE\n========================");
  if has_sol {
    let (sol_raw, tok_raw, tok_dec) = if info.pc_mint == SOL_MINT {
      (
        pc_amt,
        coin_amt,
        get_token_decimals_sync(&info.coin_mint).unwrap_or(6),
      )
    } else {
      (
        coin_amt,
        pc_amt,
        get_token_decimals_sync(&info.pc_mint).unwrap_or(6),
      )
    };
    let sol = (sol_raw as f64) / (10f64).powi(SOL_DECIMALS as i32);
    let tok = (tok_raw as f64) / (10f64).powi(tok_dec as i32);
    if tok > 0.0 {
      let price_sol = sol / tok;
      println!("SOL per target (manual): {:.12}", price_sol);
    } else {
      println!("Token reserve is zero; manual price skipped");
    }
  } else {
    println!("Pool has no SOL side; legacy decoder will skip in main service");
  }

  // DexScreener API diff (SOL price)
  println!("\n DEXSCREENER DIFF (SOL)\n========================");
  if has_sol {
    // Initialize DexScreener API once
    if let Err(e) = init_dexscreener_api().await {
      eprintln!("Failed to init DexScreener API: {}", e);
    } else if let Ok(api_arc) = get_global_dexscreener_api().await {
      let mut api = api_arc.lock().await;
      match api.get_token_data(target_mint).await {
        Ok(Some(api_token)) => {
          if let Some(api_price_sol) = api_token.price_dexscreener_sol {
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
                "Decoded SOL price: {:.12}\nDexScreener SOL: {:.12}\nDiff: {:.12} SOL ({:.4}%)",
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

  println!("\n DEBUG COMPLETE");
  Ok(())
}

struct MinimalLegacyInfo {
  coin_mint: String,
  pc_mint: String,
  coin_vault: String,
  pc_vault: String,
}

fn parse_legacy_minimal(data: &[u8]) -> Option<MinimalLegacyInfo> {
  if data.len() < 0x1c0 {
    return None;
  }

  let coin_vault = read_pubkey(data, 0x150)?;
  let pc_vault = read_pubkey(data, 0x160)?;
  let mint_a = read_pubkey(data, 0x190)?; // token mint
  let mint_b = read_pubkey(data, 0x1b0)?; // other mint (often SOL)

  // Orient so that one side is SOL if present
  let (coin_mint, pc_mint, coin_vault, pc_vault) = if mint_a == SOL_MINT {
    // mint_a is SOL → swap so coin_mint is the token
    (mint_b, mint_a, pc_vault, coin_vault)
  } else if mint_b == SOL_MINT {
    (mint_a, mint_b, coin_vault, pc_vault)
  } else {
    (mint_a, mint_b, coin_vault, pc_vault)
  };

  Some(MinimalLegacyInfo {
    coin_mint,
    pc_mint,
    coin_vault,
    pc_vault,
  })
}

fn read_pubkey(data: &[u8], off: usize) -> Option<String> {
  let bytes: [u8; 32] = data.get(off..off + 32)?.try_into().ok()?;
  Some(Pubkey::new_from_array(bytes).to_string())
}

fn decode_token_amount(data: &[u8]) -> Option<u64> {
  if data.len() < 72 {
    return None;
  }
  let amt = u64::from_le_bytes(data.get(64..72)?.try_into().ok()?);
  Some(amt)
}
