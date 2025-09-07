//! Debug tool for Pool Service: fetch a single pool + vaults and run decoder.

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
        }
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
            println!("✅ Identified as Raydium Legacy AMM");
            ProgramKind::RaydiumLegacyAmm
        }
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
            println!("✅ Identified as Raydium CPMM");
            ProgramKind::RaydiumCpmm
        }
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => {
            println!("✅ Identified as Pump.fun");
            ProgramKind::PumpFun
        }
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
            println!("✅ Identified as Meteora DLMM");
            ProgramKind::MeteoraDlmm
        }
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => {
            println!("✅ Identified as Meteora DAMM");
            ProgramKind::MeteoraDamm
        }
        "CLMMLQNz3yAFJmJW9pdJDZu6wBdhGxb6BKC8pu9hKxmJ" => {
            println!("✅ Identified as Raydium CLMM");
            ProgramKind::RaydiumClmm
        }
        "MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG" => {
            println!("✅ Identified as Moonit AMM");
            ProgramKind::Moonit
        }
        _ => {
            println!("❓ Unknown program ID, using heuristics based on data length:");
            match data_len {
                752 => {
                    println!("  📏 752 bytes suggests Raydium Legacy AMM");
                    ProgramKind::RaydiumLegacyAmm
                }
                1544 => {
                    println!("  📏 1544 bytes suggests Raydium CPMM");
                    ProgramKind::RaydiumCpmm
                }
                132 => {
                    println!("  📏 132 bytes suggests Pump.fun");
                    ProgramKind::PumpFun
                }
                653 => {
                    println!("  📏 653 bytes suggests Orca Whirlpool");
                    ProgramKind::OrcaWhirlpool
                }
                409 => {
                    println!("  📏 409 bytes suggests Moonit AMM");
                    ProgramKind::Moonit
                }
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

fn extract_orca_whirlpool_vaults(
    pool_data: &[u8]
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    if pool_data.len() < 653 {
        return Err("Insufficient data for Orca Whirlpool pool".into());
    }

    // Orca Whirlpool structure offsets
    let vault_a_offset = 101; // Token vault A
    let vault_b_offset = 133; // Token vault B

    let vault_a = Pubkey::try_from(&pool_data[vault_a_offset..vault_a_offset + 32])?;
    let vault_b = Pubkey::try_from(&pool_data[vault_b_offset..vault_b_offset + 32])?;

    println!("🏦 Extracted Orca Whirlpool vaults:");
    println!("  Vault A: {}", vault_a);
    println!("  Vault B: {}", vault_b);

    Ok((vault_a, vault_b))
}

fn extract_orca_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 653 {
        return None;
    }

    // Debug: Let's see what's at different offsets
    println!("🔍 Orca Whirlpool raw data analysis:");
    for offset in [90, 95, 99, 131, 135, 179, 211, 240, 280].iter() {
        if *offset + 32 <= data.len() {
            if let Some(pk) = read_pubkey_at(data, *offset) {
                println!("  Offset {}: {}", offset, pk);
            }
        }
    }

    // Based on the raw analysis, the valid-looking pubkeys are at different offsets
    // Let's try offsets 90 and 95 which look more realistic
    let candidate_mint_a = read_pubkey_at(data, 90)?;
    let candidate_vault_a = read_pubkey_at(data, 95)?;

    // And also try other valid-looking candidates
    let candidate_mint_b = read_pubkey_at(data, 135)?;
    let candidate_vault_b = read_pubkey_at(data, 240)?;

    println!("� Testing candidate offsets:");
    println!("  Candidate Mint A (90): {}", candidate_mint_a);
    println!("  Candidate Vault A (95): {}", candidate_vault_a);
    println!("  Candidate Mint B (135): {}", candidate_mint_b);
    println!("  Candidate Vault B (240): {}", candidate_vault_b);

    // Check if any of these candidates match our expected patterns
    let sol_mint_str = "So11111111111111111111111111111111111111112";
    let target_token = "HzHwfQwXyQ77E5yPFU1sLVeDuc7Zg4PeyXXVF7qtGxch";

    // Check all candidates for SOL or target token
    for (desc, mint) in [
        ("Candidate Mint A", &candidate_mint_a),
        ("Candidate Mint B", &candidate_mint_b),
    ] {
        if mint == sol_mint_str {
            println!("✅ Found SOL at {}: {}", desc, mint);
        }
        if mint == target_token {
            println!("✅ Found target token at {}: {}", desc, mint);
        }
    }

    // For now, let's try the candidates that looked most valid
    Some((candidate_vault_a.to_string(), candidate_vault_b.to_string()))
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
            "INFO",
            &format!(
                "Pool acct len={} owner={} lamports={}",
                pool_account.data.len(),
                pool_account.owner,
                pool_account.lamports
            )
        );
    }

    let mut accounts: HashMap<String, AccountData> = HashMap::new();
    accounts.insert(pool_pubkey.to_string(), AccountData {
        pubkey: pool_pubkey,
        data: pool_account.data.clone(),
        slot: 0,
        fetched_at: std::time::Instant::now(),
        lamports: pool_account.lamports,
        owner: pool_account.owner,
    });

    let program_kind = match args.program {
        PoolKindArg::Auto => {
            let detected = detect_program_type(&pool_account.owner, pool_account.data.len());
            println!("Using auto-detected program type: {:?}", detected);
            detected
        }
        PoolKindArg::Pumpfun => ProgramKind::PumpFun,
        PoolKindArg::RaydiumCpmm => ProgramKind::RaydiumCpmm,
        PoolKindArg::RaydiumClmm => ProgramKind::RaydiumClmm,
        PoolKindArg::RaydiumLegacy => ProgramKind::RaydiumLegacyAmm,
        PoolKindArg::MeteoraDlmm => ProgramKind::MeteoraDlmm,
        PoolKindArg::MeteoraDamm => ProgramKind::MeteoraDamm,
    };

    if
        program_kind == ProgramKind::PumpFun ||
        program_kind == ProgramKind::RaydiumLegacyAmm ||
        program_kind == ProgramKind::RaydiumClmm ||
        program_kind == ProgramKind::MeteoraDlmm ||
        program_kind == ProgramKind::MeteoraDamm ||
        program_kind == ProgramKind::OrcaWhirlpool
    {
        // Legacy scan: show candidate pubkeys at common offsets for investigation
        if program_kind == ProgramKind::RaydiumLegacyAmm && args.verbose {
            for off in [0x150usize, 0x160, 0x170, 0x180, 0x190, 0x1a0, 0x1b0, 0x1c0, 0x1d0, 0x1e0] {
                if let Some(pk) = read_pubkey_at(&pool_account.data, off) {
                    log(LogTag::PoolCalculator, "DEBUG", &format!("OFFSET 0x{:x} -> {}", off, pk));
                }
            }
        }

        // DLMM scan: show pubkeys at DLMM offsets
        if program_kind == ProgramKind::MeteoraDlmm && args.verbose {
            for (name, off) in [
                ("token_x_mint", 88usize),
                ("token_y_mint", 120),
                ("reserve_x", 152),
                ("reserve_y", 184),
            ] {
                if let Some(pk) = read_pubkey_at(&pool_account.data, off) {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("DLMM {} @ offset {} -> {}", name, off, pk)
                    );
                }
            }
        }

        // DAMM scan: show pubkeys at DAMM offsets
        if program_kind == ProgramKind::MeteoraDamm && args.verbose {
            // Show the basic structure
            for (name, off) in [
                ("token_a_mint", 168usize),
                ("token_b_mint", 200),
                ("token_a_vault", 232),
                ("token_b_vault", 264),
            ] {
                if let Some(pk) = read_pubkey_at(&pool_account.data, off) {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("DAMM {} @ offset {} -> {}", name, off, pk)
                    );
                }
            }

            // Scan for more vault addresses in the range 250-350
            log(LogTag::PoolCalculator, "DEBUG", "Scanning for vault addresses in range 250-350:");
            for off in (250..350).step_by(32) {
                if let Some(pk) = read_pubkey_at(&pool_account.data, off) {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("DAMM scan @ offset {} -> {}", off, pk)
                    );
                }
            }
        }

        let vault_pair = if program_kind == ProgramKind::RaydiumLegacyAmm {
            extract_legacy_vaults(&pool_account.data)
        } else if program_kind == ProgramKind::RaydiumClmm {
            extract_clmm_vaults(&pool_account.data)
        } else if program_kind == ProgramKind::MeteoraDlmm {
            extract_dlmm_vaults(&pool_account.data)
        } else if program_kind == ProgramKind::MeteoraDamm {
            extract_damm_vaults(&pool_account.data)
        } else if program_kind == ProgramKind::OrcaWhirlpool {
            extract_orca_vaults(&pool_account.data)
        } else {
            extract_pumpfun_vaults(&pool_account.data)
        };
        if let Some((token_vault, sol_vault)) = vault_pair {
            if args.verbose {
                log(
                    LogTag::PoolCalculator,
                    "INFO",
                    &format!("Derived vaults token_vault={} sol_vault={}", token_vault, sol_vault)
                );
            }
            let vault_keys: Vec<Pubkey> = [token_vault.clone(), sol_vault.clone()]
                .into_iter()
                .filter_map(|s| Pubkey::from_str(&s).ok())
                .collect();
            if args.verbose {
                log(
                    LogTag::PoolCalculator,
                    "INFO",
                    &format!("Fetching {} vault accounts", vault_keys.len())
                );
            }
            if !vault_keys.is_empty() {
                if let Ok(vault_accounts) = rpc.get_multiple_accounts(&vault_keys).await {
                    for (i, acct_opt) in vault_accounts.into_iter().enumerate() {
                        if let Some(acct) = acct_opt {
                            accounts.insert(vault_keys[i].to_string(), AccountData {
                                pubkey: vault_keys[i],
                                data: acct.data.clone(),
                                slot: 0,
                                fetched_at: std::time::Instant::now(),
                                lamports: acct.lamports,
                                owner: acct.owner,
                            });
                        }
                    }
                }
                // Fallback: individually fetch any missing vaults (retry a few times)
                let mut attempts = 0;
                while
                    attempts < 5 &&
                    (!accounts.contains_key(&token_vault) || !accounts.contains_key(&sol_vault))
                {
                    attempts += 1;
                    if args.verbose {
                        log(
                            LogTag::PoolCalculator,
                            "WARN",
                            &format!("Retry attempt {} for missing vaults", attempts)
                        );
                    }
                    for (addr_str, pk) in [
                        (token_vault.as_str(), &vault_keys[0]),
                        (sol_vault.as_str(), &vault_keys[1]),
                    ] {
                        if !accounts.contains_key(addr_str) {
                            if let Ok(acct) = rpc.get_account(pk).await {
                                accounts.insert(addr_str.to_string(), AccountData {
                                    pubkey: *pk,
                                    data: acct.data.clone(),
                                    slot: 0,
                                    fetched_at: std::time::Instant::now(),
                                    lamports: acct.lamports,
                                    owner: acct.owner,
                                });
                                if args.verbose {
                                    log(
                                        LogTag::PoolCalculator,
                                        "INFO",
                                        &format!(
                                            "Fetched missing vault {} len={}",
                                            addr_str,
                                            acct.data.len()
                                        )
                                    );
                                }
                            }
                        }
                    }
                    if !accounts.contains_key(&token_vault) || !accounts.contains_key(&sol_vault) {
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                    }
                }
            }
            if args.verbose {
                for (k, v) in &accounts {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("Account key={} len={}", k, v.data.len())
                    );
                }
            }
            // Legacy fallback: if legacy and token vault missing, scan candidate offsets for token mint match
            if
                program_kind == ProgramKind::RaydiumLegacyAmm &&
                !accounts.contains_key(&token_vault)
            {
                if args.verbose {
                    log(
                        LogTag::PoolCalculator,
                        "WARN",
                        "Legacy token vault missing, scanning candidate offsets"
                    );
                }
                legacy_bulk_scan(
                    &rpc,
                    &mut accounts,
                    &pool_account.data,
                    &args.token_mint,
                    args.verbose
                ).await;
            }
            // Debug mint of fetched sol_vault
            if program_kind == ProgramKind::RaydiumLegacyAmm && args.verbose {
                if let Some(vault_acc) = accounts.get(&sol_vault) {
                    if vault_acc.data.len() >= 32 {
                        let mint = Pubkey::new_from_array(
                            vault_acc.data[0..32].try_into().unwrap_or([0u8; 32])
                        );
                        log(
                            LogTag::PoolCalculator,
                            "DEBUG",
                            &format!("Sol vault {} has mint {}", sol_vault, mint)
                        );
                    }
                }
            }
        } else {
            eprintln!("Failed to parse PumpFun vault addresses");
        }
    }

    let mut price = decoders::decode_pool(
        program_kind,
        &accounts,
        &args.token_mint,
        &args.quote_mint
    );
    if let Some(ref mut p) = price {
        if p.pool_address.is_empty() {
            p.pool_address = pool_pubkey.to_string();
        }
    }
    report(price);
}

fn report(price: Option<PriceResult>) {
    match price {
        Some(p) =>
            println!(
                "PriceResult mint={} price_sol={} sol_reserves={} token_reserves={} confidence={} pool={}",
                p.mint,
                p.price_sol,
                p.sol_reserves,
                p.token_reserves,
                p.confidence,
                p.pool_address
            ),
        None => println!("No price calculated"),
    }
}

fn extract_pumpfun_vaults(data: &[u8]) -> Option<(String, String)> {
    // Try PumpFun layout first
    if data.len() >= 200 {
        let mut o = 8; // discriminator
        o += 1 + 2; // bump + index
        o += 32; // creator
        o += 32 + 32; // base + quote mints
        o += 32; // lp mint
        if let Some(base) = read_pubkey(data, &mut o) {
            if let Some(quote) = read_pubkey(data, &mut o) {
                return Some((base, quote));
            }
        }
    }
    // Legacy Raydium heuristic (vaults at 0x160 token, 0x150 SOL). Order returns (token_vault, sol_vault)
    if data.len() > 0x1c0 {
        let token_vault = read_pubkey_at(data, 0x160)?;
        let sol_vault = read_pubkey_at(data, 0x150)?;
        return Some((token_vault, sol_vault));
    }
    None
}

fn extract_legacy_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() <= 0x1c0 {
        return None;
    }
    let sol_vault = read_pubkey_at(data, 0x150)?; // SOL vault
    let token_vault = read_pubkey_at(data, 0x160)?; // token vault
    Some((token_vault, sol_vault))
}

async fn legacy_bulk_scan(
    rpc: &screenerbot::rpc::RpcClient,
    accounts: &mut HashMap<String, AccountData>,
    data: &[u8],
    token_mint: &str,
    verbose: bool
) {
    let mut candidates: Vec<Pubkey> = Vec::new();
    // Extend scan range to find token vault
    for off in [
        0x150usize, 0x160, 0x170, 0x180, 0x1c0, 0x1d0, 0x1e0, 0x200, 0x210, 0x220, 0x230, 0x240,
        0x250, 0x260, 0x270, 0x280, 0x290, 0x2a0, 0x2b0, 0x2c0,
    ] {
        if let Some(pk_str) = read_pubkey_at(data, off) {
            if let Ok(pk) = Pubkey::from_str(&pk_str) {
                candidates.push(pk);
            }
        }
    }
    for pk in candidates {
        if accounts.contains_key(&pk.to_string()) {
            continue;
        }
        if let Ok(acct) = rpc.get_account(&pk).await {
            let mint = if acct.data.len() >= 32 {
                Pubkey::new_from_array(acct.data[0..32].try_into().unwrap_or([0u8; 32]))
            } else {
                Pubkey::default()
            };
            let is_token_vault = mint.to_string() == token_mint;
            let is_sol_vault = mint.to_string() == "So11111111111111111111111111111111111111112";
            accounts.insert(pk.to_string(), AccountData {
                pubkey: pk,
                data: acct.data.clone(),
                slot: 0,
                fetched_at: std::time::Instant::now(),
                lamports: acct.lamports,
                owner: acct.owner,
            });
            if verbose {
                log(
                    LogTag::PoolCalculator,
                    "INFO",
                    &format!(
                        "Scanned vault candidate {} len={} mint={} {}{}",
                        pk,
                        acct.data.len(),
                        mint,
                        if is_token_vault {
                            "[TOKEN_VAULT]"
                        } else {
                            ""
                        },
                        if is_sol_vault {
                            "[SOL_VAULT]"
                        } else {
                            ""
                        }
                    )
                );
            }
        }
    }
}

fn read_pubkey_at(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }
    let pk = Pubkey::new_from_array(data[offset..offset + 32].try_into().ok()?);
    Some(pk.to_string())
}

fn read_pubkey(data: &[u8], offset: &mut usize) -> Option<String> {
    if *offset + 32 > data.len() {
        return None;
    }
    let pk = Pubkey::new_from_array(data[*offset..*offset + 32].try_into().ok()?);
    *offset += 32;
    Some(pk.to_string())
}

fn extract_dlmm_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 216 {
        return None;
    }

    // Extract pubkeys at DLMM offsets
    let token_x_mint = read_pubkey_at(data, 88)?;
    let token_y_mint = read_pubkey_at(data, 120)?;
    let reserve_x = read_pubkey_at(data, 152)?;
    let reserve_y = read_pubkey_at(data, 184)?;

    // Check which token is SOL to determine vault order
    let sol_mint = "So11111111111111111111111111111111111111112";

    if token_y_mint == sol_mint {
        // token_x is the custom token, token_y is SOL
        // return (token_vault, sol_vault)
        Some((reserve_x, reserve_y))
    } else if token_x_mint == sol_mint {
        // token_x is SOL, token_y is the custom token
        // return (token_vault, sol_vault)
        Some((reserve_y, reserve_x))
    } else {
        None
    }
}

fn extract_damm_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 1112 {
        log(LogTag::PoolCalculator, "ERROR", &format!("DAMM data too short: {} bytes", data.len()));
        return None;
    }

    // Extract pubkeys at correct fixed offsets (corrected based on debug scan analysis)
    // offset 168: token_a_mint (our target token)
    // offset 200: token_b_mint (SOL)
    // offset 232: token_a_vault (target token vault)
    // offset 264: token_b_vault (SOL vault)

    let token_a_mint = read_pubkey_at(data, 168)?;
    let token_b_mint = read_pubkey_at(data, 200)?;
    let token_a_vault = read_pubkey_at(data, 232)?;
    let token_b_vault = read_pubkey_at(data, 264)?; // token_b_vault at offset 264

    log(
        LogTag::PoolCalculator,
        "DEBUG",
        &format!(
            "DAMM structure: token_a_mint={}, token_b_mint={}, token_a_vault={}, token_b_vault={}",
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault
        )
    );

    // Check if token_b is SOL (structure: token_a = target token, token_b = SOL)
    let sol_mint = "So11111111111111111111111111111111111111112";

    if token_b_mint == sol_mint {
        // Return (token_vault, sol_vault)
        log(
            LogTag::PoolCalculator,
            "DEBUG",
            "DAMM: token_a_vault=token_vault, token_b_vault=sol_vault"
        );
        Some((token_a_vault, token_b_vault))
    } else {
        log(
            LogTag::PoolCalculator,
            "ERROR",
            &format!("DAMM: unexpected token_b_mint: {}", token_b_mint)
        );
        None
    }
}

fn extract_clmm_vaults(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 800 {
        return None;
    }

    // Based on Raydium CLMM PoolState struct layout
    // Skip discriminator (8 bytes), bump (1 byte), amm_config (32 bytes), owner (32 bytes)
    let base_offset = 8 + 1 + 32 + 32;

    // Skip token_mint_0 (32 bytes) and token_mint_1 (32 bytes)
    let vault_offset = base_offset + 32 + 32;

    // Extract vault pubkeys at calculated offsets
    let token_vault_0 = read_pubkey_at(data, vault_offset)?; // token_vault_0
    let token_vault_1 = read_pubkey_at(data, vault_offset + 32)?; // token_vault_1

    // Extract token mints to determine which vault corresponds to which token
    let token_mint_0 = read_pubkey_at(data, base_offset)?;
    let token_mint_1 = read_pubkey_at(data, base_offset + 32)?;

    let sol_mint = "So11111111111111111111111111111111111111112";

    if token_mint_0 == sol_mint {
        // token_mint_0 is SOL, token_mint_1 is the custom token
        // return (token_vault, sol_vault)
        Some((token_vault_1, token_vault_0))
    } else if token_mint_1 == sol_mint {
        // token_mint_1 is SOL, token_mint_0 is the custom token
        // return (token_vault, sol_vault)
        Some((token_vault_0, token_vault_1))
    } else {
        None
    }
}
// End of file
