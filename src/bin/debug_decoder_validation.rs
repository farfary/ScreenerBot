//! Debug tool for comprehensive decoder validation and testing
//!
//! This tool searches through all tokens in the database and finds pools for each supported
//! program type to validate decoder implementations and price calculations using the pools module.
//!
//! Features:
//! - Scans all tokens in database for pools
//! - Finds one TOKEN/SOL and one SOL/TOKEN pool for each supported program type
//! - Tests all decoders using the pools module calculator (no duplicate logic)
//! - Compares calculated prices with API prices (DexScreener)
//! - Identifies price differences > 20% as suspicious/problematic
//! - Provides detailed analysis of decoder accuracy and errors
//! - Focuses only on SOL pairs (TOKEN/SOL or SOL/TOKEN)
//!
//! Usage Examples:
//! cargo run --bin debug_decoder_validation
//! cargo run --bin debug_decoder_validation -- --max-tokens 500 --min-liquidity 1000
//! cargo run --bin debug_decoder_validation -- --program-filter raydium --verbose

use clap::Parser;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::calculator::PriceCalculator;
use screenerbot::pools::decoders::raydium_cpmm::RaydiumCpmmDecoder;
use screenerbot::pools::fetcher::AccountData;
use screenerbot::pools::types::{ProgramKind, SOL_MINT};
use screenerbot::pools::utils::is_stablecoin_mint;
use screenerbot::rpc::{get_rpc_client, parse_pubkey};
use screenerbot::tokens::database::TokenDatabase;
use screenerbot::tokens::dexscreener::{get_global_dexscreener_api, init_dexscreener_api};
use screenerbot::tokens::types::Token;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[derive(Parser, Debug)]
#[command(
    name = "debug_decoder_validation",
    about = "Comprehensive decoder validation and price testing tool"
)]
struct Args {
    /// Maximum tokens to scan from database
    #[arg(long, default_value = "200")]
    max_tokens: usize,

    /// Minimum token liquidity USD to consider
    #[arg(long, default_value = "100")]
    min_liquidity: f64,

    /// Filter by program type (e.g., "raydium", "orca", "meteora")
    #[arg(long)]
    program_filter: Option<String>,

    /// Price difference threshold % to flag as problematic
    #[arg(long, default_value = "20.0")]
    price_diff_threshold: f64,

    /// Show verbose debugging information
    #[arg(short, long)]
    verbose: bool,

    /// Maximum pools per token to check
    #[arg(long, default_value = "10")]
    max_pools_per_token: usize,

    /// Skip price validation (only test decoders)
    #[arg(long)]
    skip_price_validation: bool,

    /// Enable pool decoders debug logging
    #[arg(long)]
    debug_pool_decoders: bool,

    /// Enable pool calculator debug logging
    #[arg(long)]
    debug_pool_calculator: bool,

    /// Enable all debug modes
    #[arg(long)]
    debug_all: bool,
}

/// Information about a found pool for testing
#[derive(Debug, Clone)]
struct TestPool {
    token_mint: String,
    token_symbol: String,
    pool_address: String,
    program_kind: ProgramKind,
    program_id: String,
    liquidity_usd: f64,
    pair_type: PairType,
    base_mint: String,
    quote_mint: String,
}

/// Type of SOL pair
#[derive(Debug, Clone, PartialEq)]
enum PairType {
    TokenSol, // TOKEN/SOL (token is base, SOL is quote)
    SolToken, // SOL/TOKEN (SOL is base, token is quote)
}

/// Result of decoder testing for a pool
#[derive(Debug)]
struct DecoderTestResult {
    pool: TestPool,
    decoder_success: bool,
    decoder_error: Option<String>,
    decode_time_ms: u64,
    calculated_price_sol: Option<f64>,
    api_price_sol: Option<f64>,
    price_diff_percent: Option<f64>,
    price_validation_passed: bool,
    pool_info_extracted: bool,
    reserves_info: Option<String>,
}

/// Summary statistics for each program type
#[derive(Debug)]
struct ProgramStats {
    program_kind: ProgramKind,
    total_pools_found: usize,
    token_sol_pools: usize,
    sol_token_pools: usize,
    decoder_successes: usize,
    decoder_failures: usize,
    price_validation_successes: usize,
    price_validation_failures: usize,
    avg_decode_time_ms: f64,
    suspicious_price_diffs: usize,
    error_messages: Vec<String>,
}

impl TestPool {
    /// Get the target token mint (non-SOL token)
    fn get_target_token_mint(&self) -> &str {
        if self.base_mint == SOL_MINT {
            &self.quote_mint
        } else {
            &self.base_mint
        }
    }
}

/// Fetch pools for a token and categorize by program type and pair direction
async fn find_pools_for_token(token: &Token, args: &Args) -> Result<Vec<TestPool>, String> {
    let mut test_pools = Vec::new();

    // Get DexScreener API
    let dex_api = get_global_dexscreener_api()
        .await
        .map_err(|e| format!("Failed to get DexScreener API: {}", e))?;
    let mut api_lock = dex_api.lock().await;

    // Get all pools for this token from DexScreener
    let pairs_result = api_lock.get_solana_token_pairs(&token.mint).await;
    drop(api_lock);

    let pairs = match pairs_result {
        Ok(pairs) => pairs,
        Err(e) => {
            if args.verbose {
                log(
                    LogTag::System,
                    "API_ERROR",
                    &format!("Failed to get pairs for token {}: {}", &token.mint[..8], e),
                );
            }
            return Ok(test_pools);
        }
    };

    if pairs.is_empty() {
        return Ok(test_pools);
    }

    let rpc_client = get_rpc_client();
    let sol_mint = SOL_MINT;

    // Process each pool from DexScreener
    for pair in pairs.iter().take(args.max_pools_per_token) {
        let liquidity_usd = pair.liquidity.as_ref().map(|l| l.usd).unwrap_or(0.0);

        if liquidity_usd < args.min_liquidity {
            continue;
        }

        // Check if this is a SOL pair
        let base_mint = &pair.base_token.address;
        let quote_mint = &pair.quote_token.address;

        let pair_type = if base_mint == sol_mint && quote_mint != sol_mint {
            PairType::SolToken
        } else if quote_mint == sol_mint && base_mint != sol_mint {
            PairType::TokenSol
        } else {
            continue; // Skip non-SOL pairs
        };

        // Skip stablecoin pairs
        if is_stablecoin_mint(base_mint) || is_stablecoin_mint(quote_mint) {
            continue;
        }

        // Parse pool address and get program info from RPC
        let pool_pubkey = match Pubkey::from_str(&pair.pair_address) {
            Ok(pubkey) => pubkey,
            Err(_) => {
                continue;
            }
        };

        // Get pool account to determine actual program
        let account_info = match rpc_client.client().get_account(&pool_pubkey) {
            Ok(account) => account,
            Err(_) => {
                continue;
            }
        };

        let program_id = account_info.owner.to_string();
        let program_kind = ProgramKind::from_program_id(&program_id);

        // Apply program filter if specified
        if let Some(filter) = &args.program_filter {
            let program_name = program_kind.display_name().to_lowercase();
            if !program_name.contains(&filter.to_lowercase()) {
                continue;
            }
        }

        // Skip unknown programs
        if program_kind == ProgramKind::Unknown {
            continue;
        }

        let test_pool = TestPool {
            token_mint: token.mint.clone(),
            token_symbol: token.symbol.clone(),
            pool_address: pair.pair_address.clone(),
            program_kind,
            program_id,
            liquidity_usd,
            pair_type,
            base_mint: base_mint.clone(),
            quote_mint: quote_mint.clone(),
        };

        test_pools.push(test_pool);
    }

    Ok(test_pools)
}

/// Test decoder on a specific pool using the pools module calculator
async fn test_decoder_on_pool(pool: &TestPool, args: &Args) -> DecoderTestResult {
    let start_time = Instant::now();

    if args.verbose {
        log(
            LogTag::System,
            "DECODER_TEST",
            &format!(
                "Testing {} decoder on pool {} ({:?})",
                pool.program_kind.display_name(),
                &pool.pool_address[..8],
                pool.pair_type
            ),
        );
    }

    // Fetch pool account data from RPC
    let pool_pubkey = match parse_pubkey(&pool.pool_address) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            return DecoderTestResult {
                pool: pool.clone(),
                decoder_success: false,
                decoder_error: Some(format!("Invalid pool address: {}", e)),
                decode_time_ms: 0,
                calculated_price_sol: None,
                api_price_sol: None,
                price_diff_percent: None,
                price_validation_passed: false,
                pool_info_extracted: false,
                reserves_info: None,
            };
        }
    };

    let rpc_client = get_rpc_client();
    let pool_account = match rpc_client.client().get_account(&pool_pubkey) {
        Ok(account) => account,
        Err(e) => {
            return DecoderTestResult {
                pool: pool.clone(),
                decoder_success: false,
                decoder_error: Some(format!("Failed to fetch pool account: {}", e)),
                decode_time_ms: 0,
                calculated_price_sol: None,
                api_price_sol: None,
                price_diff_percent: None,
                price_validation_passed: false,
                pool_info_extracted: false,
                reserves_info: None,
            };
        }
    };

    // Create the main pool account data
    let mut pool_accounts = HashMap::new();
    pool_accounts.insert(
        pool_pubkey.to_string(),
        AccountData {
            pubkey: pool_pubkey,
            data: pool_account.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: pool_account.lamports,
            owner: pool_account.owner,
        },
    );

    // Fetch auxiliary accounts based on DEX type
    match pool.program_kind {
        ProgramKind::RaydiumCpmm => {
            // For Raydium CPMM, we need to fetch vault accounts for proper price calculation
            if let Some(pool_info) =
                RaydiumCpmmDecoder::decode_raydium_cpmm_pool(&pool_account.data, &pool.pool_address)
            {
                // Fetch vault accounts
                if let Ok(vault_0_pubkey) = parse_pubkey(&pool_info.token_0_vault) {
                    if let Ok(vault_0_account) = rpc_client.client().get_account(&vault_0_pubkey) {
                        pool_accounts.insert(
                            vault_0_pubkey.to_string(),
                            AccountData {
                                pubkey: vault_0_pubkey,
                                data: vault_0_account.data,
                                slot: 0,
                                fetched_at: std::time::Instant::now(),
                                lamports: vault_0_account.lamports,
                                owner: vault_0_account.owner,
                            },
                        );
                    }
                }

                if let Ok(vault_1_pubkey) = parse_pubkey(&pool_info.token_1_vault) {
                    if let Ok(vault_1_account) = rpc_client.client().get_account(&vault_1_pubkey) {
                        pool_accounts.insert(
                            vault_1_pubkey.to_string(),
                            AccountData {
                                pubkey: vault_1_pubkey,
                                data: vault_1_account.data,
                                slot: 0,
                                fetched_at: std::time::Instant::now(),
                                lamports: vault_1_account.lamports,
                                owner: vault_1_account.owner,
                            },
                        );
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "CPMM_VAULTS",
                        &format!(
                            "Fetched vault accounts: {} and {} for CPMM pool",
                            &pool_info.token_0_vault[..8],
                            &pool_info.token_1_vault[..8]
                        ),
                    );
                }
            }
        }

        ProgramKind::OrcaWhirlpool => {
            // For Orca Whirlpool, we need to fetch token vault accounts
            use screenerbot::pools::decoders::orca_whirlpool::OrcaWhirlpoolDecoder;
            if let Some(vault_accounts) =
                OrcaWhirlpoolDecoder::extract_reserve_accounts(&pool_account.data)
            {
                let vault_count = vault_accounts.len();

                if args.verbose {
                    log(
                        LogTag::System,
                        "ORCA_VAULT_EXTRACT",
                        &format!(
                            "Extracted {} vault addresses: {:?}",
                            vault_count, vault_accounts
                        ),
                    );
                }

                for vault_address in &vault_accounts {
                    if args.verbose {
                        log(
                            LogTag::System,
                            "ORCA_VAULT_FETCH",
                            &format!("Attempting to fetch vault account: {}", vault_address),
                        );
                    }

                    match parse_pubkey(vault_address) {
                        Ok(vault_pubkey) => match rpc_client.client().get_account(&vault_pubkey) {
                            Ok(vault_account) => {
                                let data_len = vault_account.data.len();
                                pool_accounts.insert(
                                    vault_address.clone(),
                                    AccountData {
                                        pubkey: vault_pubkey,
                                        data: vault_account.data,
                                        slot: 0,
                                        fetched_at: std::time::Instant::now(),
                                        lamports: vault_account.lamports,
                                        owner: vault_account.owner,
                                    },
                                );

                                if args.verbose {
                                    log(
                                        LogTag::System,
                                        "ORCA_VAULT_SUCCESS",
                                        &format!(
                                            "Successfully fetched vault account: {} ({} bytes)",
                                            vault_address, data_len
                                        ),
                                    );
                                }
                            }
                            Err(e) => {
                                if args.verbose {
                                    log(
                                        LogTag::System,
                                        "ORCA_VAULT_ERROR",
                                        &format!(
                                            "Failed to fetch vault account {}: {}",
                                            vault_address, e
                                        ),
                                    );
                                }
                            }
                        },
                        Err(e) => {
                            if args.verbose {
                                log(
                                    LogTag::System,
                                    "ORCA_VAULT_PARSE_ERROR",
                                    &format!(
                                        "Failed to parse vault address {}: {}",
                                        vault_address, e
                                    ),
                                );
                            }
                        }
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "ORCA_VAULTS",
                        &format!(
                            "Fetched {} vault accounts for Orca Whirlpool pool",
                            vault_count
                        ),
                    );
                }
            } else {
                if args.verbose {
                    log(
                        LogTag::System,
                        "ORCA_VAULT_EXTRACT_FAIL",
                        "Failed to extract vault accounts from Orca pool data",
                    );
                }
            }
        }

        ProgramKind::RaydiumLegacyAmm => {
            // For Raydium Legacy AMM, we need to fetch coin and pc vault accounts
            use screenerbot::pools::decoders::raydium_legacy_amm::RaydiumLegacyAmmDecoder;
            if let Some(vault_accounts) =
                RaydiumLegacyAmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
                let vault_count = vault_accounts.len();
                for vault_address in &vault_accounts {
                    if let Ok(vault_pubkey) = parse_pubkey(vault_address) {
                        if let Ok(vault_account) = rpc_client.client().get_account(&vault_pubkey) {
                            pool_accounts.insert(
                                vault_address.clone(),
                                AccountData {
                                    pubkey: vault_pubkey,
                                    data: vault_account.data,
                                    slot: 0,
                                    fetched_at: std::time::Instant::now(),
                                    lamports: vault_account.lamports,
                                    owner: vault_account.owner,
                                },
                            );
                        }
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "LEGACY_VAULTS",
                        &format!(
                            "Fetched {} vault accounts for Raydium Legacy AMM pool",
                            vault_count
                        ),
                    );
                }
            }
        }

        ProgramKind::RaydiumClmm => {
            // For Raydium CLMM, we need to fetch vault accounts
            use screenerbot::pools::decoders::raydium_clmm::RaydiumClmmDecoder;
            if let Some(vault_accounts) =
                RaydiumClmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
                let vault_count = vault_accounts.len();
                for vault_address in &vault_accounts {
                    if let Ok(vault_pubkey) = parse_pubkey(vault_address) {
                        if let Ok(vault_account) = rpc_client.client().get_account(&vault_pubkey) {
                            pool_accounts.insert(
                                vault_address.clone(),
                                AccountData {
                                    pubkey: vault_pubkey,
                                    data: vault_account.data,
                                    slot: 0,
                                    fetched_at: std::time::Instant::now(),
                                    lamports: vault_account.lamports,
                                    owner: vault_account.owner,
                                },
                            );
                        }
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "CLMM_VAULTS",
                        &format!(
                            "Fetched {} vault accounts for Raydium CLMM pool",
                            vault_count
                        ),
                    );
                }
            }
        }

        ProgramKind::MeteoraDlmm => {
            // For Meteora DLMM, we need to fetch reserve accounts
            use screenerbot::pools::decoders::meteora_dlmm::MeteoraDlmmDecoder;
            if let Some(vault_accounts) =
                MeteoraDlmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
                let vault_count = vault_accounts.len();
                for vault_address in &vault_accounts {
                    if let Ok(vault_pubkey) = parse_pubkey(vault_address) {
                        if let Ok(vault_account) = rpc_client.client().get_account(&vault_pubkey) {
                            pool_accounts.insert(
                                vault_address.clone(),
                                AccountData {
                                    pubkey: vault_pubkey,
                                    data: vault_account.data,
                                    slot: 0,
                                    fetched_at: std::time::Instant::now(),
                                    lamports: vault_account.lamports,
                                    owner: vault_account.owner,
                                },
                            );
                        }
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "METEORA_DLMM_VAULTS",
                        &format!(
                            "Fetched {} vault accounts for Meteora DLMM pool",
                            vault_count
                        ),
                    );
                }
            }
        }

        ProgramKind::MeteoraDamm => {
            // For Meteora DAMM, we need to fetch reserve accounts
            use screenerbot::pools::decoders::meteora_damm::MeteoraDammDecoder;
            if let Some(vault_accounts) =
                MeteoraDammDecoder::extract_reserve_accounts(&pool_account.data)
            {
                let vault_count = vault_accounts.len();
                for vault_address in &vault_accounts {
                    if let Ok(vault_pubkey) = parse_pubkey(vault_address) {
                        if let Ok(vault_account) = rpc_client.client().get_account(&vault_pubkey) {
                            pool_accounts.insert(
                                vault_address.clone(),
                                AccountData {
                                    pubkey: vault_pubkey,
                                    data: vault_account.data,
                                    slot: 0,
                                    fetched_at: std::time::Instant::now(),
                                    lamports: vault_account.lamports,
                                    owner: vault_account.owner,
                                },
                            );
                        }
                    }
                }

                if args.verbose {
                    log(
                        LogTag::System,
                        "METEORA_DAMM_VAULTS",
                        &format!(
                            "Fetched {} vault accounts for Meteora DAMM pool",
                            vault_count
                        ),
                    );
                }
            }
        }

        ProgramKind::PumpFunAmm | ProgramKind::PumpFunLegacy => {
            // PumpFun pools typically contain all needed data in the pool account itself
            // Additional vault fetching might be needed based on implementation
            if args.verbose {
                log(
                    LogTag::System,
                    "PUMPFUN_CHECK",
                    "PumpFun pool - checking if additional accounts needed",
                );
            }
        }

        _ => {
            // For other pool types, the pool account might contain all needed data
            if args.verbose {
                log(
                    LogTag::System,
                    "UNKNOWN_DEX",
                    &format!(
                        "Unknown DEX type: {} - using pool account only",
                        pool.program_kind.display_name()
                    ),
                );
            }
        }
    }

    // Create calculator instance with empty pool directory (we don't need it for direct calculation)
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    let pool_directory = Arc::new(RwLock::new(HashMap::new()));
    let calculator = PriceCalculator::new(pool_directory);

    // Determine base and quote mints based on pair type
    let (base_mint, quote_mint) = match pool.pair_type {
        PairType::TokenSol => (pool.get_target_token_mint(), SOL_MINT),
        PairType::SolToken => (SOL_MINT, pool.get_target_token_mint()),
    };

    // Use the pools module calculator to get price
    let (decoder_success, decoder_error, pool_info_extracted, reserves_info, calculated_price_sol) =
        match calculator.calculate_price_sync(
            &pool_accounts,
            pool.program_kind,
            base_mint,
            quote_mint,
            &pool.pool_address,
        ) {
            Some(price_result) => {
                let reserves_info = format!(
                    "SOL reserves: {:.9}, Token reserves: {:.6}, Price: {:.12} SOL",
                    price_result.sol_reserves, price_result.token_reserves, price_result.price_sol
                );

                if args.verbose {
                    log(
                        LogTag::System,
                        "CALC_SUCCESS",
                        &format!(
                            "{} calculation successful: {}",
                            pool.program_kind.display_name(),
                            reserves_info
                        ),
                    );
                }

                (
                    true,
                    None,
                    true,
                    Some(reserves_info),
                    Some(price_result.price_sol),
                )
            }
            None => {
                let error = format!(
                    "{} decoder failed or not implemented",
                    pool.program_kind.display_name()
                );
                if args.verbose {
                    log(LogTag::System, "CALC_ERROR", &error);
                }
                (false, Some(error), false, None, None)
            }
        };

    let decode_time_ms = start_time.elapsed().as_millis() as u64;

    // Get API price for comparison if not skipping validation
    let (api_price_sol, price_diff_percent, price_validation_passed) = if args.skip_price_validation
    {
        (None, None, true) // Always pass validation when skipping
    } else {
        get_api_price_and_compare(pool, calculated_price_sol, args).await
    };

    DecoderTestResult {
        pool: pool.clone(),
        decoder_success,
        decoder_error,
        decode_time_ms,
        calculated_price_sol,
        api_price_sol,
        price_diff_percent,
        price_validation_passed,
        pool_info_extracted,
        reserves_info,
    }
}

/// Get API price and compare with calculated price
async fn get_api_price_and_compare(
    pool: &TestPool,
    calculated_price_sol: Option<f64>,
    args: &Args,
) -> (Option<f64>, Option<f64>, bool) {
    let target_token = pool.get_target_token_mint();

    // Get API price from DexScreener
    let api_price_sol = if let Ok(api) = get_global_dexscreener_api().await {
        if let Ok(mut guard) = tokio::time::timeout(Duration::from_millis(5000), api.lock()).await {
            guard.get_price(target_token).await
        } else {
            None
        }
    } else {
        None
    };

    let (price_diff_percent, price_validation_passed) =
        if let (Some(calculated), Some(api)) = (calculated_price_sol, api_price_sol) {
            if api > 0.0 {
                let diff = ((calculated - api).abs() / api) * 100.0;
                let passed = diff <= args.price_diff_threshold;
                (Some(diff), passed)
            } else {
                (None, false) // Fail if API price is invalid
            }
        } else {
            // Fail validation if we couldn't calculate a price (decoder not working properly)
            // Only pass if we explicitly skip price validation
            (None, false)
        };

    (api_price_sol, price_diff_percent, price_validation_passed)
}

/// Organize test pools by program type and pair direction
fn organize_pools_by_program(
    test_pools: Vec<TestPool>,
) -> HashMap<ProgramKind, (Vec<TestPool>, Vec<TestPool>)> {
    let mut organized: HashMap<ProgramKind, (Vec<TestPool>, Vec<TestPool>)> = HashMap::new();

    for pool in test_pools {
        let entry = organized
            .entry(pool.program_kind)
            .or_insert_with(|| (Vec::new(), Vec::new()));

        match pool.pair_type {
            PairType::TokenSol => entry.0.push(pool),
            PairType::SolToken => entry.1.push(pool),
        }
    }

    // Sort pools by liquidity (descending) within each category
    for (token_sol_pools, sol_token_pools) in organized.values_mut() {
        token_sol_pools.sort_by(|a, b| {
            b.liquidity_usd
                .partial_cmp(&a.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sol_token_pools.sort_by(|a, b| {
            b.liquidity_usd
                .partial_cmp(&a.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    organized
}

/// Print detailed test results
fn print_test_results(results: &[DecoderTestResult], args: &Args) {
    println!("\n🔬 DECODER TEST RESULTS");
    println!("=======================");

    for result in results {
        let status = if result.decoder_success { "✅" } else { "❌" };
        let pair_type = match result.pool.pair_type {
            PairType::TokenSol => "TOKEN/SOL",
            PairType::SolToken => "SOL/TOKEN",
        };

        println!(
            "\n{} {} - {} ({}) - {}",
            status,
            result.pool.program_kind.display_name(),
            result.pool.token_symbol,
            pair_type,
            &result.pool.pool_address[..8]
        );

        println!("   💰 Liquidity: ${:.2}", result.pool.liquidity_usd);
        println!("   ⏱️  Decode Time: {}ms", result.decode_time_ms);

        if let Some(error) = &result.decoder_error {
            println!("   ❌ Error: {}", error);
        }

        if result.pool_info_extracted {
            println!("   ✅ Pool info extracted successfully");
            if let Some(reserves) = &result.reserves_info {
                println!("   📊 Reserves: {}", reserves);
            }
        }

        if !args.skip_price_validation {
            match (result.calculated_price_sol, result.api_price_sol) {
                (Some(calc), Some(api)) => {
                    println!("   💲 Calculated: {:.12} SOL", calc);
                    println!("   📡 API Price: {:.12} SOL", api);
                    if let Some(diff) = result.price_diff_percent {
                        let warning = if diff > args.price_diff_threshold {
                            "⚠️"
                        } else {
                            "✅"
                        };
                        println!("   {} Price Diff: {:.2}%", warning, diff);
                    }
                }
                (Some(calc), None) => {
                    println!("   💲 Calculated: {:.12} SOL", calc);
                    println!("   📡 API Price: unavailable");
                }
                (None, Some(api)) => {
                    println!("   💲 Calculated: not available");
                    println!("   📡 API Price: {:.12} SOL", api);
                }
                (None, None) => {
                    println!("   💲 No price data available");
                }
            }
        }
    }
}

/// Generate and print summary statistics
fn print_summary_statistics(results: &[DecoderTestResult], args: &Args) {
    let mut stats: HashMap<ProgramKind, ProgramStats> = HashMap::new();

    // Collect statistics
    for result in results {
        let stat = stats
            .entry(result.pool.program_kind)
            .or_insert_with(|| ProgramStats {
                program_kind: result.pool.program_kind,
                total_pools_found: 0,
                token_sol_pools: 0,
                sol_token_pools: 0,
                decoder_successes: 0,
                decoder_failures: 0,
                price_validation_successes: 0,
                price_validation_failures: 0,
                avg_decode_time_ms: 0.0,
                suspicious_price_diffs: 0,
                error_messages: Vec::new(),
            });

        stat.total_pools_found += 1;

        match result.pool.pair_type {
            PairType::TokenSol => {
                stat.token_sol_pools += 1;
            }
            PairType::SolToken => {
                stat.sol_token_pools += 1;
            }
        }

        if result.decoder_success {
            stat.decoder_successes += 1;
        } else {
            stat.decoder_failures += 1;
            if let Some(error) = &result.decoder_error {
                stat.error_messages.push(error.clone());
            }
        }

        if result.price_validation_passed {
            stat.price_validation_successes += 1;
        } else {
            stat.price_validation_failures += 1;
        }

        if let Some(diff) = result.price_diff_percent {
            if diff > args.price_diff_threshold {
                stat.suspicious_price_diffs += 1;
            }
        }

        stat.avg_decode_time_ms += result.decode_time_ms as f64;
    }

    // Calculate averages
    for stat in stats.values_mut() {
        if stat.total_pools_found > 0 {
            stat.avg_decode_time_ms /= stat.total_pools_found as f64;
        }
    }

    // Print summary
    println!("\n📊 SUMMARY STATISTICS");
    println!("=====================");

    let mut sorted_stats: Vec<_> = stats.values().collect();
    sorted_stats.sort_by_key(|s| s.total_pools_found);
    sorted_stats.reverse();

    for stat in sorted_stats {
        println!(
            "\n🏷️  {} ({} pools)",
            stat.program_kind.display_name(),
            stat.total_pools_found
        );
        println!("   TOKEN/SOL pairs: {}", stat.token_sol_pools);
        println!("   SOL/TOKEN pairs: {}", stat.sol_token_pools);
        println!(
            "   Decoder success rate: {}/{} ({:.1}%)",
            stat.decoder_successes,
            stat.total_pools_found,
            if stat.total_pools_found > 0 {
                ((stat.decoder_successes as f64) / (stat.total_pools_found as f64)) * 100.0
            } else {
                0.0
            }
        );

        if !args.skip_price_validation {
            println!(
                "   Price validation rate: {}/{} ({:.1}%)",
                stat.price_validation_successes,
                stat.total_pools_found,
                if stat.total_pools_found > 0 {
                    ((stat.price_validation_successes as f64) / (stat.total_pools_found as f64))
                        * 100.0
                } else {
                    0.0
                }
            );

            if stat.suspicious_price_diffs > 0 {
                println!(
                    "   ⚠️  Suspicious price diffs (>{}%): {}",
                    args.price_diff_threshold, stat.suspicious_price_diffs
                );
            }
        }

        println!("   Avg decode time: {:.1}ms", stat.avg_decode_time_ms);

        if !stat.error_messages.is_empty() {
            println!("   Common errors:");
            let mut error_counts: HashMap<&String, usize> = HashMap::new();
            for error in &stat.error_messages {
                *error_counts.entry(error).or_insert(0) += 1;
            }
            let mut sorted_errors: Vec<_> = error_counts.iter().collect();
            sorted_errors.sort_by_key(|(_, count)| *count);
            sorted_errors.reverse();

            for (error, count) in sorted_errors.iter().take(3) {
                println!("     - {} ({}x)", error, count);
            }
        }
    }

    // Overall summary
    let total_pools = results.len();
    let total_successes = results.iter().filter(|r| r.decoder_success).count();
    let total_price_validation_passed =
        results.iter().filter(|r| r.price_validation_passed).count();
    let total_suspicious = results
        .iter()
        .filter(|r| {
            r.price_diff_percent
                .map(|d| d > args.price_diff_threshold)
                .unwrap_or(false)
        })
        .count();

    println!("\n🎯 OVERALL SUMMARY");
    println!("==================");
    println!("Total pools tested: {}", total_pools);
    println!(
        "Decoder success rate: {}/{} ({:.1}%)",
        total_successes,
        total_pools,
        if total_pools > 0 {
            ((total_successes as f64) / (total_pools as f64)) * 100.0
        } else {
            0.0
        }
    );

    if !args.skip_price_validation {
        println!(
            "Price validation rate: {}/{} ({:.1}%)",
            total_price_validation_passed,
            total_pools,
            if total_pools > 0 {
                ((total_price_validation_passed as f64) / (total_pools as f64)) * 100.0
            } else {
                0.0
            }
        );

        if total_suspicious > 0 {
            println!(
                "⚠️  Suspicious price differences: {} pools",
                total_suspicious
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set global arguments so debug flags work properly
    let mut global_args = std::env::args().collect::<Vec<String>>();

    // Add debug flags based on the parsed arguments
    if args.debug_pool_decoders {
        global_args.push("--debug-pool-decoders".to_string());
    }
    if args.debug_pool_calculator {
        global_args.push("--debug-pool-calculator".to_string());
    }
    if args.debug_all {
        global_args.extend_from_slice(&[
            "--debug-pool-decoders".to_string(),
            "--debug-pool-calculator".to_string(),
            "--debug-pool-fetcher".to_string(),
            "--debug-pool-analyzer".to_string(),
        ]);
    }

    // Set the global arguments
    screenerbot::arguments::set_cmd_args(global_args);

    println!("🔍 Decoder Validation Tool");
    println!("===========================");
    println!("Max tokens: {}", args.max_tokens);
    println!("Min liquidity: ${}", args.min_liquidity);
    println!("Price diff threshold: {}%", args.price_diff_threshold);
    if let Some(filter) = &args.program_filter {
        println!("Program filter: {}", filter);
    }
    if args.debug_pool_decoders {
        println!("Debug pool decoders: ENABLED");
    }
    if args.debug_pool_calculator {
        println!("Debug pool calculator: ENABLED");
    }
    println!();

    let start_time = Instant::now();

    // Initialize services
    log(LogTag::System, "INIT", "Initializing services...");

    if let Err(e) = screenerbot::rpc::init_rpc_client() {
        log(
            LogTag::System,
            "WARN",
            &format!("RPC initialization failed: {}", e),
        );
    }

    // Always initialize DexScreener API since we need it to discover pools
    init_dexscreener_api().await?;
    log(LogTag::System, "INIT", "DexScreener API initialized");

    // Open token database
    log(LogTag::System, "DB", "Opening token database...");
    let db = TokenDatabase::new()?;
    let mut tokens = db.get_all_tokens().await?;

    // Filter tokens by liquidity
    tokens.retain(|token| {
        token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0) >= args.min_liquidity
    });

    // Sort by liquidity descending
    tokens.sort_by(|a, b| {
        let a_liq = a.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
        let b_liq = b.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
        b_liq
            .partial_cmp(&a_liq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let tokens = tokens.into_iter().take(args.max_tokens).collect::<Vec<_>>();

    log(
        LogTag::System,
        "DB",
        &format!("Scanning {} tokens for pools", tokens.len()),
    );

    // Find pools for each token
    let mut all_test_pools = Vec::new();
    let mut error_count = 0;

    for (i, token) in tokens.iter().enumerate() {
        if i > 0 && i % 50 == 0 {
            log(
                LogTag::System,
                "PROGRESS",
                &format!("Processed {}/{} tokens", i, tokens.len()),
            );
        }

        // Rate limiting
        if i > 0 {
            sleep(Duration::from_millis(200)).await; // 5 requests per second
        }

        match find_pools_for_token(token, &args).await {
            Ok(pools) => {
                if args.verbose && !pools.is_empty() {
                    log(
                        LogTag::System,
                        "POOLS_FOUND",
                        &format!(
                            "Token {} ({}): found {} pools",
                            &token.mint[..8],
                            &token.symbol,
                            pools.len()
                        ),
                    );
                }
                all_test_pools.extend(pools);
            }
            Err(e) => {
                error_count += 1;
                if args.verbose {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Token {}: {}", &token.mint[..8], e),
                    );
                }
            }
        }
    }

    log(
        LogTag::System,
        "DISCOVERY",
        &format!(
            "Found {} total pools from {} tokens ({} errors)",
            all_test_pools.len(),
            tokens.len(),
            error_count
        ),
    );

    if all_test_pools.is_empty() {
        println!("❌ No pools found matching criteria");
        return Ok(());
    }

    // Organize pools by program type
    let organized_pools = organize_pools_by_program(all_test_pools);

    // Print pool organization summary
    println!("\n📋 POOL ORGANIZATION BY PROGRAM");
    println!("===============================");
    for (program_kind, (token_sol_pools, sol_token_pools)) in &organized_pools {
        println!(
            "🏷️  {} - TOKEN/SOL: {}, SOL/TOKEN: {}",
            program_kind.display_name(),
            token_sol_pools.len(),
            sol_token_pools.len()
        );

        // Show top pools for each direction
        if !token_sol_pools.is_empty() {
            let top = &token_sol_pools[0];
            println!(
                "   Best TOKEN/SOL: {} (${:.0}) - {}",
                top.token_symbol,
                top.liquidity_usd,
                &top.pool_address[..8]
            );
        }
        if !sol_token_pools.is_empty() {
            let top = &sol_token_pools[0];
            println!(
                "   Best SOL/TOKEN: {} (${:.0}) - {}",
                top.token_symbol,
                top.liquidity_usd,
                &top.pool_address[..8]
            );
        }
    }

    // Select best pools for testing (one TOKEN/SOL and one SOL/TOKEN per program)
    let mut test_pools = Vec::new();
    for (_program_kind, (token_sol_pools, sol_token_pools)) in organized_pools {
        // Take best TOKEN/SOL pool if available
        if let Some(pool) = token_sol_pools.first() {
            test_pools.push(pool.clone());
        }

        // Take best SOL/TOKEN pool if available
        if let Some(pool) = sol_token_pools.first() {
            test_pools.push(pool.clone());
        }
    }

    if test_pools.is_empty() {
        println!("❌ No suitable pools found for testing");
        return Ok(());
    }

    println!("\n🧪 TESTING {} POOLS", test_pools.len());
    println!("====================");

    // Test each selected pool
    let mut results = Vec::new();
    for (i, pool) in test_pools.iter().enumerate() {
        println!(
            "Testing {}/{}: {} {} ({:?})",
            i + 1,
            test_pools.len(),
            pool.program_kind.display_name(),
            pool.token_symbol,
            pool.pair_type
        );

        let result = test_decoder_on_pool(pool, &args).await;
        results.push(result);

        // Rate limiting between tests
        if i < test_pools.len() - 1 {
            sleep(Duration::from_millis(500)).await;
        }
    }

    // Print results
    print_test_results(&results, &args);
    print_summary_statistics(&results, &args);

    let elapsed = start_time.elapsed();
    println!("\n⏱️  Total execution time: {:.2}s", elapsed.as_secs_f64());

    log(LogTag::System, "COMPLETE", "Decoder validation completed");
    Ok(())
}
