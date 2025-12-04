/// Debug tool for analyzing PumpFun pool data
///
/// This tool fetches raw account data for a specific pool and analyzes:
/// - Pool account structure and data
/// - Mint accounts
/// - Vault accounts
/// - Owner verification
/// - Data layout and offsets
///
/// Usage:
/// cargo run --bin debug_pumpfun_pool -- <POOL_ADDRESS>
///
/// Example:
/// cargo run --bin debug_pumpfun_pool -- Dm8vW6XQYxEbF4hjkLkeh1T23pohGB9Sae4p3G8QZwRP
use screenerbot::constants::{
  PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID, SOL_DECIMALS, SOL_MINT, SYSTEM_PROGRAM_ID,
};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
  // Initialize config first
  screenerbot::config::load_config().expect("Failed to load config");

  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    eprintln!("Usage: {} <POOL_ADDRESS>", args[0]);
    eprintln!("\nExample:");
 eprintln!("{} Dm8vW6XQYxEbF4hjkLkeh1T23pohGB9Sae4p3G8QZwRP", args[0]);
    std::process::exit(1);
  }

  let pool_address = &args[1];

  println!("╔════════════════════════════════════════════════════════════════════════════╗");
 println!("║ PumpFun Pool Debug Tool - Raw Data Analysis ║");
  println!("╚════════════════════════════════════════════════════════════════════════════╝");
  println!();
  println!("Pool Address: {}", pool_address);
  println!();

  // Parse pool address
  let pool_pubkey = match Pubkey::from_str(pool_address) {
    Ok(pk) => pk,
    Err(e) => {
 eprintln!("Invalid pool address: {}", e);
      std::process::exit(1);
    }
  };

  // Get RPC client - create simple client for debugging
  let rpc_url = screenerbot::config::with_config(|cfg| cfg.rpc.urls[0].clone());
  let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

  // Fetch pool account
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 1: Fetching Pool Account Data");
  println!("─────────────────────────────────────────────────────────────────────────────");

  // Use simple get_account instead of get_account_with_config
  let pool_account = match rpc_client.get_account(&pool_pubkey) {
    Ok(account) => account,
    Err(e) => {
 eprintln!("Failed to fetch pool account: {}", e);
      std::process::exit(1);
    }
  };

  let pool_account = match pool_account {
    account => account,
  };

 println!("Pool account fetched successfully");
  println!();
  println!("Owner: {}", pool_account.owner);
  println!("Lamports: {}", pool_account.lamports);
  println!("Data length: {} bytes", pool_account.data.len());
  println!("Executable: {}", pool_account.executable);
  println!("Rent Epoch: {}", pool_account.rent_epoch);
  println!();

  // Determine program type
  let owner_str = pool_account.owner.to_string();
  let program_type = if owner_str == PUMP_FUN_AMM_PROGRAM_ID {
    "PumpFun AMM"
  } else if owner_str == PUMP_FUN_LEGACY_PROGRAM_ID {
    "PumpFun Legacy"
  } else {
    "Unknown"
  };

  println!("Program Type: {}", program_type);
  println!();

  // Analyze data layout
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 2: Analyzing Pool Data Structure");
  println!("─────────────────────────────────────────────────────────────────────────────");

  let data = &pool_account.data;

  if data.len() < 8 {
 eprintln!("Data too short to contain discriminator");
    std::process::exit(1);
  }

  // Print discriminator
  let discriminator = &data[0..8];
  println!("Discriminator (first 8 bytes):");
 print!("Hex: ");
  for byte in discriminator {
    print!("{:02x} ", byte);
  }
  println!();
 print!("Dec: ");
  for byte in discriminator {
    print!("{:3} ", byte);
  }
  println!();
  println!();

  // Enhanced: scan for known values/creator from env vars to find true offsets
  println!("Scanning for known field values in raw bytes (from env vars, if provided)…\n");
  let mut known_u64s: Vec<(String, u64)> = Vec::new();
  // Helper to read env u64
  let read_u64 = |key: &str| -> Option<u64> {
    std::env::var(key)
      .ok()
      .and_then(|v| v.trim().parse::<u64>().ok())
  };
  if let Some(v) = read_u64("PUMPDEBUG_VIRT_TOKEN") {
    known_u64s.push(("virtual_token_reserves".to_string(), v));
  }
  if let Some(v) = read_u64("PUMPDEBUG_VIRT_SOL") {
    known_u64s.push(("virtual_sol_reserves".to_string(), v));
  }
  if let Some(v) = read_u64("PUMPDEBUG_REAL_TOKEN") {
    known_u64s.push(("real_token_reserves".to_string(), v));
  }
  if let Some(v) = read_u64("PUMPDEBUG_REAL_SOL") {
    known_u64s.push(("real_sol_reserves".to_string(), v));
  }
  if let Some(v) = read_u64("PUMPDEBUG_TOTAL_SUPPLY") {
    known_u64s.push(("token_total_supply".to_string(), v));
  }

  let creator_from_env = std::env::var("PUMPDEBUG_CREATOR_PUBKEY")
    .ok()
    .and_then(|s| Pubkey::from_str(&s).ok());

  if creator_from_env.is_none() && known_u64s.is_empty() {
 println!("No PUMPDEBUG_* env vars set. You can set:");
 println!("- PUMPDEBUG_CREATOR_PUBKEY");
 println!("- PUMPDEBUG_VIRT_TOKEN, PUMPDEBUG_VIRT_SOL");
 println!("- PUMPDEBUG_REAL_TOKEN, PUMPDEBUG_REAL_SOL");
 println!("- PUMPDEBUG_TOTAL_SUPPLY");
 println!("Proceeding with structural parse only.\n");
  }

  // Scan u64s
  let mut u64_hits: Vec<(usize, u64)> = Vec::new();
  for off in 0..=data.len().saturating_sub(8) {
    let val = u64::from_le_bytes([
      data[off],
      data[off + 1],
      data[off + 2],
      data[off + 3],
      data[off + 4],
      data[off + 5],
      data[off + 6],
      data[off + 7],
    ]);
    u64_hits.push((off, val));
  }

  // Index hits for known u64s
  let mut labeled_hits: Vec<(String, usize, u64)> = Vec::new();
  for (label, target) in &known_u64s {
    let mut matches: Vec<(usize, u64)> = u64_hits
      .iter()
      .cloned()
      .filter(|(_, v)| v == target)
      .collect();
    matches.sort_by_key(|(o, _)| *o);
    if matches.is_empty() {
 println!("No match for {} = {}", label, target);
    } else {
      println!(
 "{} candidates for {} = {}:",
        matches.len(),
        label,
        target
      );
      for (i, (off, val)) in matches.iter().enumerate() {
 println!("- [{}] offset {} (val {})", i, off, val);
      }
      // Record the first for clustering
      labeled_hits.push((label.clone(), matches[0].0, matches[0].1));
    }
  }

  // Scan for creator pubkey
  let mut creator_hit: Option<usize> = None;
  if let Some(creator_pk) = creator_from_env {
    let bytes = creator_pk.to_bytes();
    for off in 0..=data.len().saturating_sub(32) {
      if data[off..off + 32] == bytes {
        creator_hit = Some(off);
        break;
      }
    }
    match creator_hit {
 Some(off) => println!("Creator pubkey found at offset {}: {}", off, creator_pk),
 None => println!("Creator pubkey not found in data: {}", creator_pk),
    }
  }

  // Try to infer a cluster if we have multiple u64 hits
  if labeled_hits.len() >= 2 {
    labeled_hits.sort_by_key(|(_, off, _)| *off);
    println!("\nInferred u64 cluster (sorted by offset):");
    for (label, off, val) in &labeled_hits {
 println!("- {:>20} @ {:>3} = {}", label, off, val);
    }

    // Heuristic: guess that reserves fields are near each other; find minimal span
    let span_start = labeled_hits.first().unwrap().1;
    let span_end = labeled_hits.last().unwrap().1 + 8;
    println!(
 "Cluster span: [{}..{}) ({} bytes)\n",
      span_start,
      span_end,
      span_end.saturating_sub(span_start)
    );
  }

  // Optional: search likely bools (0x00/0x01) near cluster
  if !labeled_hits.is_empty() {
    let center =
      labeled_hits.iter().map(|(_, off, _)| *off).sum::<usize>() / labeled_hits.len();
    let start = center.saturating_sub(24);
    let end = (center + 24).min(data.len());
    let mut bool_candidates: Vec<(usize, u8)> = Vec::new();
    for i in start..end {
      if data[i] == 0 || data[i] == 1 {
        bool_candidates.push((i, data[i]));
      }
    }
    if !bool_candidates.is_empty() {
      println!(
 "Nearby bool candidates around center {} (±24 bytes):",
        center
      );
      for (off, b) in &bool_candidates {
 println!("- offset {} value {}", off, b);
      }
    }
  }

  // Print small hex windows for hits
  let dump_window = |off: usize, label: &str| {
    let win = 8usize; // bytes before/after
    let s = off.saturating_sub(win);
    let e = (off + 8 + win).min(data.len());
 print!("[{}] hex @{}: ", label, off);
    for i in s..e {
      print!("{:02x} ", data[i]);
    }
    println!();
  };
  for (label, off, _) in &labeled_hits {
    dump_window(*off, label);
  }
  if let Some(off) = creator_hit {
    dump_window(off, "creator");
  }

  println!();
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 2B: Deep Scan - Finding ALL Pubkeys in Account Data");
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!();

  // Scan every possible 32-byte window for valid pubkeys
  let mut found_pubkeys: Vec<(usize, Pubkey)> = Vec::new();
  for offset in 0..=data.len().saturating_sub(32) {
    if let Ok(pk) = Pubkey::try_from(&data[offset..offset + 32]) {
      // Only include non-default pubkeys
      if pk != Pubkey::default() {
        found_pubkeys.push((offset, pk));
      }
    }
  }

  println!("Found {} potential pubkey fields:", found_pubkeys.len());
  for (off, pk) in &found_pubkeys {
 println!("Offset {:>3}: {}", off, pk);
  }
  println!();

  // Now fetch and classify each pubkey
  println!("Analyzing each pubkey to determine its type...");
  println!();

  let mut token_mint: Option<(usize, Pubkey, u8)> = None; // offset, pubkey, decimals

  for (off, pk) in &found_pubkeys {
    let pk_str = pk.to_string();
    print!("Offset {:>3} ({}): ", off, pk);

    // Check for known system addresses first
    if pk_str == SOL_MINT {
 println!("WRAPPED SOL (WSOL)");
      continue;
    }
    if pk_str == SYSTEM_PROGRAM_ID {
      println!("System Program (11111...)");
      continue;
    }

    // Fetch account
    match rpc_client.get_account(pk) {
      Ok(account) => {
        let owner = account.owner.to_string();

        // Check if it's a token mint
        if owner == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
          && account.data.len() >= 82
        {
          // Parse mint account structure
          let supply = u64::from_le_bytes(account.data[36..44].try_into().unwrap());
          let decimals = account.data[44];
          let is_initialized = account.data[45] == 1;

          println!(
 "TOKEN MINT (decimals={}, supply={}, initialized={})",
            decimals, supply, is_initialized
          );

          // Store as candidate token mint
          if token_mint.is_none() {
            token_mint = Some((*off, *pk, decimals));
          }
        }
        // Check if it's a token account
        else if owner == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
          && account.data.len() >= 165
        {
          let mint = Pubkey::try_from(&account.data[0..32]).ok();
          let amount = u64::from_le_bytes(account.data[64..72].try_into().unwrap());
          println!("Token Account (mint={:?}, balance={})", mint, amount);
        }
        // Check for known program IDs
        else if owner == SYSTEM_PROGRAM_ID {
          println!("System-owned account (wallet/PDA)");
        } else {
          println!("Unknown account type (owner={})", owner);
        }
      }
      Err(_e) => {
 println!("Account not found on-chain");
      }
    }
  }

  println!();

  // Now compute price if we found the token mint
  if let Some((mint_offset, mint_pk, decimals)) = token_mint {
    println!("═════════════════════════════════════════════════════════════════════════════");
    println!("PRICE CALCULATION");
    println!("═════════════════════════════════════════════════════════════════════════════");
    println!();
    println!(
      "Token Mint: {} (offset {}, decimals {})",
      mint_pk, mint_offset, decimals
    );
    println!();

    // Use the discovered reserve offsets
    let virt_token = labeled_hits
      .iter()
      .find(|(l, _, _)| l == "virtual_token_reserves")
      .map(|(_, _, v)| *v);
    let virt_sol = labeled_hits
      .iter()
      .find(|(l, _, _)| l == "virtual_sol_reserves")
      .map(|(_, _, v)| *v);
    let real_token = labeled_hits
      .iter()
      .find(|(l, _, _)| l == "real_token_reserves")
      .map(|(_, _, v)| *v);
    let real_sol = labeled_hits
      .iter()
      .find(|(l, _, _)| l == "real_sol_reserves")
      .map(|(_, _, v)| *v);

    if let (Some(vt), Some(vs), Some(rt), Some(rs)) =
      (virt_token, virt_sol, real_token, real_sol)
    {
      println!("Reserve fields:");
 println!("Virtual Token: {}", vt);
 println!("Virtual SOL: {}", vs);
 println!("Real Token: {}", rt);
 println!("Real SOL: {}", rs);
      println!();

      // Calculate price: SOL per token
      // For bonding curve: typically price = sol_reserves / token_reserves
      // We'll use virtual reserves as they represent the current curve state

      let token_amount = vt as f64 / 10_f64.powi(decimals as i32);
      let sol_amount = vs as f64 / 10_f64.powi(SOL_DECIMALS as i32);

      if token_amount > 0.0 {
        let price_sol = sol_amount / token_amount;

        println!("Token amount (human): {:.6}", token_amount);
 println!("SOL amount (human): {:.9}", sol_amount);
        println!();
        println!(
          "╔═════════════════════════════════════════════════════════════════════════╗"
        );
        println!("║ PRICE: {:.15} SOL per token", price_sol);
        println!(
          "╚═════════════════════════════════════════════════════════════════════════╝"
        );
        println!();

        // Also calculate with real reserves for comparison
        let token_amount_real = rt as f64 / 10_f64.powi(decimals as i32);
        let sol_amount_real = rs as f64 / 10_f64.powi(SOL_DECIMALS as i32);
        if token_amount_real > 0.0 {
          let price_sol_real = sol_amount_real / token_amount_real;
          println!(
            "Price (using real reserves): {:.15} SOL per token",
            price_sol_real
          );
          println!();
        }
      } else {
 println!("Cannot calculate price: token amount is zero");
      }
    } else {
 println!("Missing reserve fields - cannot calculate price");
    }
  } else {
 println!("No token mint found in account data - cannot calculate price");
  }

  println!();
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 3: OLD DECODER ANALYSIS (for comparison)");
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!();

  // Try to parse as PumpFun Legacy structure
  if data.len() >= 200 {
    println!("OLD: Parsing as PumpFun Legacy structure (INCORRECT OFFSETS):");
    println!();

    // PumpFun Legacy offsets (based on our decoder):
    // discriminator(8) + pool_bump(1) + index(2) + creator(32) + base_mint(32) + quote_mint(32) + lp_mint(32) + vault1(32) + vault2(32)

    let mut offset = 8;

    // Pool bump
    if offset < data.len() {
      println!("Offset {}: pool_bump = {}", offset, data[offset]);
      offset += 1;
    }

    // Index (2 bytes)
    if offset + 2 <= data.len() {
      let index = u16::from_le_bytes([data[offset], data[offset + 1]]);
      println!("Offset {}: index = {}", offset, index);
      offset += 2;
    }

    // Creator (32 bytes)
    if offset + 32 <= data.len() {
      let creator = Pubkey::try_from(&data[offset..offset + 32]).ok();
      if let Some(creator) = creator {
        println!("Offset {}: creator = {}", offset, creator);
      }
      offset += 32;
    }

    println!();

    // Base mint
    let base_mint_offset = offset;
    if base_mint_offset + 32 <= data.len() {
      let base_mint = Pubkey::try_from(&data[base_mint_offset..base_mint_offset + 32]).ok();
      if let Some(mint) = base_mint {
        println!("Offset {}: BASE_MINT = {}", base_mint_offset, mint);
        offset += 32;
      }
    }

    // Quote mint
    let quote_mint_offset = offset;
    if quote_mint_offset + 32 <= data.len() {
      let quote_mint =
        Pubkey::try_from(&data[quote_mint_offset..quote_mint_offset + 32]).ok();
      if let Some(mint) = quote_mint {
        println!("Offset {}: QUOTE_MINT = {}", quote_mint_offset, mint);
        offset += 32;
      }
    }

    // LP mint
    let lp_mint_offset = offset;
    if lp_mint_offset + 32 <= data.len() {
      let lp_mint = Pubkey::try_from(&data[lp_mint_offset..lp_mint_offset + 32]).ok();
      if let Some(mint) = lp_mint {
        println!("Offset {}: LP_MINT = {}", lp_mint_offset, mint);
        offset += 32;
      }
    }

    println!();

    // Vault1
    let vault1_offset = offset;
    if vault1_offset + 32 <= data.len() {
      let vault1 = Pubkey::try_from(&data[vault1_offset..vault1_offset + 32]).ok();
      if let Some(vault) = vault1 {
        println!("Offset {}: VAULT1 = {}", vault1_offset, vault);
        offset += 32;
      }
    }

    // Vault2
    let vault2_offset = offset;
    if vault2_offset + 32 <= data.len() {
      let vault2 = Pubkey::try_from(&data[vault2_offset..vault2_offset + 32]).ok();
      if let Some(vault) = vault2 {
        println!("Offset {}: VAULT2 = {}", vault2_offset, vault);
        offset += 32;
      }
    }

    println!();
    println!("Remaining data: {} bytes", data.len() - offset);
  } else {
    println!(
 "Data length ({} bytes) is less than 200 bytes",
      data.len()
    );
 println!("Cannot parse full structure");
  }

  println!();

  // Now fetch and analyze the mints and vaults (OLD WRONG OFFSETS - kept for comparison)
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 4: OLD Mints Analysis (WRONG OFFSETS - for comparison)");
  println!("─────────────────────────────────────────────────────────────────────────────");

  if data.len() >= 200 {
    let base_mint_offset = 8 + 1 + 2 + 32; // Skip discriminator, bump, index, creator
    let quote_mint_offset = base_mint_offset + 32;

    println!("\nOLD decoder tried these offsets (INCORRECT):");
    if let Ok(base_mint) = Pubkey::try_from(&data[base_mint_offset..base_mint_offset + 32]) {
 println!("BASE MINT @ {}: {}", base_mint_offset, base_mint);
    }

    if let Ok(quote_mint) = Pubkey::try_from(&data[quote_mint_offset..quote_mint_offset + 32]) {
 println!("QUOTE MINT @ {}: {}", quote_mint_offset, quote_mint);
    }
  }

  // Fetch and analyze vaults (OLD)
  println!();
  println!("─────────────────────────────────────────────────────────────────────────────");
  println!("STEP 5: Summary - Why Decoder Failed");
  println!("─────────────────────────────────────────────────────────────────────────────");

  println!();
  println!("ROOT CAUSE:");
 println!("This is a PumpFun BONDING CURVE account, NOT an AMM pool.");
 println!("The decoder assumed AMM structure with base/quote mints and vaults.");
 println!("Bonding curves have a different layout focused on reserve tracking.");
  println!();

  println!();
  println!("╔════════════════════════════════════════════════════════════════════════════╗");
 println!("║ Debug Complete ║");
  println!("╚════════════════════════════════════════════════════════════════════════════╝");
}

fn is_sol_mint(mint: &str) -> bool {
  mint == SOL_MINT || mint == SYSTEM_PROGRAM_ID
}

fn analyze_mint(rpc_client: &RpcClient, mint_pubkey: &Pubkey) {
  let mint_str = mint_pubkey.to_string();

  // Check if it's SOL or System Program
  if mint_str == SOL_MINT {
 println!("Type: Wrapped SOL (WSOL)");
 println!("Status: Valid SOL mint");
    return;
  }

  if mint_str == SYSTEM_PROGRAM_ID {
 println!("Type: System Program");
 println!("Status: This is the System Program, not a token mint!");
    return;
  }

  // Fetch mint account
  let config = RpcAccountInfoConfig {
    encoding: None,
    commitment: Some(CommitmentConfig::confirmed()),
    data_slice: None,
    min_context_slot: None,
  };

  match rpc_client.get_account(mint_pubkey) {
    Ok(account) => {
 println!("Owner: {}", account.owner);
 println!("Data length: {} bytes", account.data.len());

      // Check if it's a token mint (SPL Token program)
 if account.owner.to_string() == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"{
 println!("Type: SPL Token Mint");

        // Try to parse decimals (offset 44, 1 byte)
        if account.data.len() > 44 {
          let decimals = account.data[44];
 println!("Decimals: {}", decimals);
        }
 println!("Status: Valid token mint");
      } else {
 println!("Type: Unknown");
 println!("Status: Not a standard SPL token mint");
      }
    }
    Err(e) => {
 println!("Status: Failed to fetch mint: {}", e);
    }
  }
}

fn analyze_token_account(rpc_client: &RpcClient, account_pubkey: &Pubkey) {
  let account_str = account_pubkey.to_string();

  // Check if it's System Program
  if account_str == SYSTEM_PROGRAM_ID {
 println!("Type: System Program");
 println!("Status: This is the System Program, not a token account!");
    return;
  }

  // Fetch token account
  let config = RpcAccountInfoConfig {
    encoding: None,
    commitment: Some(CommitmentConfig::confirmed()),
    data_slice: None,
    min_context_slot: None,
  };

  match rpc_client.get_account(account_pubkey) {
    Ok(account) => {
 println!("Owner: {}", account.owner);
 println!("Data length: {} bytes", account.data.len());
 println!("Lamports: {}", account.lamports);

      // Check if it's a token account (SPL Token program)
 if account.owner.to_string() == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"{
 println!("Type: SPL Token Account");

        // Parse token account structure
        if account.data.len() >= 72 {
          // Mint (offset 0, 32 bytes)
          if let Ok(mint) = Pubkey::try_from(&account.data[0..32]) {
 println!("Mint: {}", mint);
          }

          // Owner (offset 32, 32 bytes)
          if let Ok(owner) = Pubkey::try_from(&account.data[32..64]) {
 println!("Account Owner: {}", owner);
          }

          // Amount (offset 64, 8 bytes)
          let amount = u64::from_le_bytes(account.data[64..72].try_into().unwrap());
 println!("Balance: {} (raw)", amount);
        }
 println!("Status: Valid token account");
      } else {
 println!("Type: Unknown account type");
 println!("Status: Not a standard SPL token account");
      }
    }
    Err(e) => {
 println!("Status: Failed to fetch account: {}", e);
    }
  }
}
