//! Debug tool for testing pool decoders directly with RPC data
//!
//! This tool fetches pool account data directly from RPC (bypassing the pool fetcher)
//! and uses the individual decoders to decode and calculate prices with detailed debugging.
//!
//! Features:
//! - Direct RPC account fetching (not using pools/fetcher.rs)
//! - Tests all supported pool decoders
//! - Detailed decoding information
//! - Raw account data display
//! - Price calculation verification
//! - Program identification
//! - Reserve analysis
//!
//! Usage Examples:
//! cargo run --bin debug_pool_decoders -- --pool <POOL_ADDRESS>
//! cargo run --bin debug_pool_decoders -- --pool <POOL_ADDRESS> --verbose
//! cargo run --bin debug_pool_decoders -- --pool <POOL_ADDRESS> --show-raw-data

use base64::Engine as _;
use clap::Parser;
use screenerbot::logger::{self as logger, LogTag};
use screenerbot::pools::decoders::fluxbeam_amm::{FluxbeamAmmDecoder, FluxbeamPoolInfo};
use screenerbot::pools::decoders::raydium_cpmm::{RaydiumCpmmDecoder, RaydiumCpmmPoolInfo};
use screenerbot::pools::types::{PriceResult, ProgramKind, SOL_MINT};
use screenerbot::rpc::{get_rpc_client, parse_pubkey};
use screenerbot::tokens::decimals::SOL_DECIMALS;
use solana_sdk::account::Account;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "debug_pool_decoders",
    about = "Debug and test pool decoders with direct RPC data"
)]
struct Args {
    /// Pool address to analyze
    #[arg(short, long)]
    pool: String,

    /// Show verbose decoding details
    #[arg(short, long)]
    verbose: bool,

    /// Show raw account data in hex and base64
    #[arg(long)]
    show_raw_data: bool,

    /// Test specific decoder only (raydium-cpmm, raydium-clmm, orca-whirlpool, etc.)
    #[arg(long)]
    decoder: Option<String>,

    /// Show account sizes and owners
    #[arg(long)]
    show_account_info: bool,

    /// Maximum accounts to fetch for analysis
    #[arg(long, default_value = "10")]
    max_accounts: usize,
}

/// Pool account data fetched from RPC
#[derive(Debug, Clone)]
struct PoolAccountData {
    address: String,
    owner: String,
    lamports: u64,
    data: Vec<u8>,
    executable: bool,
    rent_epoch: u64,
}

/// Decoder test result
#[derive(Debug)]
struct DecoderTestResult {
    decoder_name: String,
    program_kind: ProgramKind,
    success: bool,
    price_result: Option<PriceResult>,
    error: Option<String>,
    decode_time_ms: u64,
    reserves_info: Option<ReservesInfo>,
}

/// Reserve information extracted from pool
#[derive(Debug)]
struct ReservesInfo {
    token_a_mint: String,
    token_b_mint: String,
    token_a_reserve: u64,
    token_b_reserve: u64,
    token_a_decimals: u8,
    token_b_decimals: u8,
    sol_reserve: u64,
    token_reserve: u64,
    sol_decimals: u8,
    token_decimals: u8,
    token_mint: String,
}

impl PoolAccountData {
    /// Create from Solana Account
    fn from_account(address: String, account: Account) -> Self {
        Self {
            address,
            owner: account.owner.to_string(),
            lamports: account.lamports,
            data: account.data,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }

    /// Get data size
    fn data_size(&self) -> usize {
        self.data.len()
    }

    /// Get data as hex string (first 64 bytes)
    fn data_hex_preview(&self) -> String {
        let preview_len = std::cmp::min(64, self.data.len());
        self.data[..preview_len]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    /// Get data as base64 string (first 256 bytes)
    fn data_base64_preview(&self) -> String {
        let preview_len = std::cmp::min(256, self.data.len());
        base64::engine::general_purpose::STANDARD.encode(&self.data[..preview_len])
    }
}

/// Fetch pool account data directly from RPC
async fn fetch_pool_account_data(pool_address: &str) -> Result<PoolAccountData, String> {
    let pool_pubkey =
        parse_pubkey(pool_address).map_err(|e| format!("Invalid pool address: {}", e))?;

    logger::info(
        LogTag::PoolService,
        &format!("Fetching account data for pool: {}", &pool_address[..8]),
    );

    let rpc_client = get_rpc_client();
    let client = rpc_client.client();

    match client.get_account(&pool_pubkey) {
        Ok(account) => {
            logger::info(
                LogTag::PoolService,
                &format!(
                    "Fetched account: {} bytes, owner: {}",
                    account.data.len(),
                    &account.owner.to_string()[..8]
                ),
            );
            Ok(PoolAccountData::from_account(
                pool_address.to_string(),
                account,
            ))
        }
        Err(e) => {
            let error_msg = format!("Failed to fetch account data: {}", e);
            logger::info(LogTag::PoolService, &error_msg);
            Err(error_msg)
        }
    }
}

/// Identify the program kind from the account owner
fn identify_program_kind(owner: &str) -> ProgramKind {
    ProgramKind::from_program_id(owner)
}

/// Test a specific decoder with the pool data
async fn test_decoder(
    decoder_name: &str,
    program_kind: ProgramKind,
    pool_data: &PoolAccountData,
    verbose: bool,
) -> DecoderTestResult {
    let start_time = std::time::Instant::now();

    logger::info(
        LogTag::PoolService,
        &format!("Testing {} decoder", decoder_name),
    );

    let (success, price_result, error, reserves_info) = match decoder_name {
        "raydium-cpmm" => test_raydium_cpmm_decoder(pool_data, verbose).await,
        "raydium-clmm" => test_raydium_clmm_decoder(pool_data, verbose).await,
        "raydium-legacy" => test_raydium_legacy_decoder(pool_data, verbose).await,
        "orca-whirlpool" => test_orca_whirlpool_decoder(pool_data, verbose).await,
        "meteora-damm" => test_meteora_damm_decoder(pool_data, verbose).await,
        "meteora-dlmm" => test_meteora_dlmm_decoder(pool_data, verbose).await,
        "pumpfun-amm" => test_pumpfun_amm_decoder(pool_data, verbose).await,
        "pumpfun-legacy" => test_pumpfun_legacy_decoder(pool_data, verbose).await,
        "moonit-amm" => test_moonit_amm_decoder(pool_data, verbose).await,
        "fluxbeam-amm" => test_fluxbeam_amm_decoder(pool_data, verbose).await,
        _ => (false, None, Some("Unknown decoder".to_string()), None),
    };

    let decode_time_ms = start_time.elapsed().as_millis() as u64;

    DecoderTestResult {
        decoder_name: decoder_name.to_string(),
        program_kind,
        success,
        price_result,
        error,
        decode_time_ms,
        reserves_info,
    }
}

/// Test Raydium CPMM decoder
async fn test_raydium_cpmm_decoder(
    pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Raydium CPMM decoder");
    }

    match RaydiumCpmmDecoder::decode_raydium_cpmm_pool(&pool_data.data, &pool_data.address) {
        Some(pool_info) => {
            if verbose {
                logger::info(
                    LogTag::PoolService,
                    &format!(
                        "Raydium CPMM decoded successfully: token0={}, token1={}",
                        &pool_info.token_0_mint[..8],
                        &pool_info.token_1_mint[..8]
                    ),
                );
            }

            // Extract reserve information
            let reserves_info = extract_raydium_cpmm_reserves_from_info(&pool_info, verbose).await;

            // For actual price calculation, we would need vault account data
            // For now, just show the pool structure
            if let Some(ref reserves) = reserves_info {
                if verbose {
                    logger::info(
                        LogTag::PoolService,
                        &format!(
                            "Reserves extracted: SOL={}, Token={}, Token mint={}",
                            reserves.sol_reserve,
                            reserves.token_reserve,
                            &reserves.token_mint[..8]
                        ),
                    );
                }
            }

            (
                true,
                None,
                Some("Price calculation requires vault account data".to_string()),
                reserves_info,
            )
        }
        None => {
            let error_msg = "Raydium CPMM decode failed: could not parse pool data".to_string();
            if verbose {
                logger::info(LogTag::PoolService, &error_msg);
            }
            (false, None, Some(error_msg), None)
        }
    }
}

/// Test Raydium CLMM decoder
async fn test_raydium_clmm_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Raydium CLMM decoder");
    }

    // Note: This is a placeholder - actual CLMM decoder implementation would go here
    let error_msg = "Raydium CLMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test Raydium Legacy AMM decoder
async fn test_raydium_legacy_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Raydium Legacy AMM decoder");
    }

    // Note: This is a placeholder - actual Legacy AMM decoder implementation would go here
    let error_msg = "Raydium Legacy AMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test Orca Whirlpool decoder
async fn test_orca_whirlpool_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Orca Whirlpool decoder");
    }

    // Note: This is a placeholder - actual Whirlpool decoder implementation would go here
    let error_msg = "Orca Whirlpool decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test Meteora DAMM decoder
async fn test_meteora_damm_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Meteora DAMM decoder");
    }

    // Note: This is a placeholder - actual DAMM decoder implementation would go here
    let error_msg = "Meteora DAMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test Meteora DLMM decoder
async fn test_meteora_dlmm_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Meteora DLMM decoder");
    }

    // Note: This is a placeholder - actual DLMM decoder implementation would go here
    let error_msg = "Meteora DLMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test PumpFun AMM decoder
async fn test_pumpfun_amm_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing PumpFun AMM decoder");
    }

    // Note: This is a placeholder - actual PumpFun AMM decoder implementation would go here
    let error_msg = "PumpFun AMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test PumpFun Legacy decoder
async fn test_pumpfun_legacy_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing PumpFun Legacy decoder");
    }

    // Note: This is a placeholder - actual PumpFun Legacy decoder implementation would go here
    let error_msg = "PumpFun Legacy decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test Moonit AMM decoder
async fn test_moonit_amm_decoder(
    _pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing Moonit AMM decoder");
    }

    // Note: This is a placeholder - actual Moonit AMM decoder implementation would go here
    let error_msg = "Moonit AMM decoder not yet implemented in debug tool".to_string();
    if verbose {
        logger::info(LogTag::PoolService, &error_msg);
    }
    (false, None, Some(error_msg), None)
}

/// Test FluxBeam AMM decoder
async fn test_fluxbeam_amm_decoder(
    pool_data: &PoolAccountData,
    verbose: bool,
) -> (
    bool,
    Option<PriceResult>,
    Option<String>,
    Option<ReservesInfo>,
) {
    if verbose {
        logger::info(LogTag::PoolService, "Testing FluxBeam AMM decoder");
    }

    match FluxbeamAmmDecoder::parse_fluxbeam_pool(&pool_data.data) {
        Some(pool_info) => {
            if verbose {
                logger::info(
                    LogTag::PoolService,
                    &format!(
                        "FluxBeam AMM decoded successfully: tokenA={}, tokenB={}",
                        &pool_info.token_a_mint[..8],
                        &pool_info.token_b_mint[..8]
                    ),
                );
            }

            // Extract reserve information - FluxBeam pools store vault addresses directly
            let reserves_info = extract_fluxbeam_reserves_from_info(&pool_info, verbose).await;

            // For actual price calculation, we would need vault account data
            if let Some(ref reserves) = reserves_info {
                if verbose {
                    logger::info(
                        LogTag::PoolService,
                        &format!(
                            "Reserves extracted: SOL vault={}, Token vault={}, Token mint={}",
                            &reserves.sol_reserve,
                            &reserves.token_reserve,
                            &reserves.token_mint[..8]
                        ),
                    );
                }
            }

            (
                true,
                None,
                Some(
                    "Pool decoded successfully - price calculation requires vault account data"
                        .to_string(),
                ),
                reserves_info,
            )
        }
        None => {
            let error_msg = "FluxBeam AMM decode failed: could not parse pool data".to_string();
            if verbose {
                logger::info(LogTag::PoolService, &error_msg);
            }
            (false, None, Some(error_msg), None)
        }
    }
}

/// Extract reserves information from FluxbeamPoolInfo
async fn extract_fluxbeam_reserves_from_info(
    pool_info: &FluxbeamPoolInfo,
    verbose: bool,
) -> Option<ReservesInfo> {
    if verbose {
        logger::info(
            LogTag::PoolService,
            &format!(
                "Extracting reserves from FluxBeam pool: tokenA={}, tokenB={}",
                &pool_info.token_a_mint[..8],
                &pool_info.token_b_mint[..8]
            ),
        );
    }

    // Determine which token is SOL
    let sol_mint_str = SOL_MINT;

    if pool_info.token_a_mint == sol_mint_str {
        // Token A is SOL, Token B is the target token
        Some(ReservesInfo {
            token_a_mint: pool_info.token_a_mint.clone(),
            token_b_mint: pool_info.token_b_mint.clone(),
            token_a_reserve: 0,  // Would need vault account data
            token_b_reserve: 0,  // Would need vault account data
            token_a_decimals: 9, // SOL decimals
            token_b_decimals: 9, // Default, would need to fetch from mint
            sol_reserve: 0,      // Would need vault account data
            token_reserve: 0,    // Would need vault account data
            sol_decimals: SOL_DECIMALS,
            token_decimals: 9, // Default, would need to fetch from mint
            token_mint: pool_info.token_b_mint.clone(),
        })
    } else if pool_info.token_b_mint == sol_mint_str {
        // Token B is SOL, Token A is the target token
        Some(ReservesInfo {
            token_a_mint: pool_info.token_a_mint.clone(),
            token_b_mint: pool_info.token_b_mint.clone(),
            token_a_reserve: 0,  // Would need vault account data
            token_b_reserve: 0,  // Would need vault account data
            token_a_decimals: 9, // Default, would need to fetch from mint
            token_b_decimals: 9, // SOL decimals
            sol_reserve: 0,      // Would need vault account data
            token_reserve: 0,    // Would need vault account data
            sol_decimals: SOL_DECIMALS,
            token_decimals: 9, // Default, would need to fetch from mint
            token_mint: pool_info.token_a_mint.clone(),
        })
    } else {
        if verbose {
            logger::info(
                LogTag::PoolService,
                "Pool does not contain SOL - cannot extract SOL reserves",
            );
        }
        None
    }
}

/// Extract reserves information from RaydiumCpmmPoolInfo
async fn extract_raydium_cpmm_reserves_from_info(
    pool_info: &RaydiumCpmmPoolInfo,
    verbose: bool,
) -> Option<ReservesInfo> {
    if verbose {
        logger::info(
            LogTag::PoolService,
            &format!(
                "Extracting reserves from CPMM pool: token0={}, token1={}",
                &pool_info.token_0_mint[..8],
                &pool_info.token_1_mint[..8]
            ),
        );
    }

    // Determine which token is SOL
    let sol_mint_str = SOL_MINT;

    if pool_info.token_0_mint == sol_mint_str {
        // Token 0 is SOL, Token 1 is the target token
        Some(ReservesInfo {
            token_a_mint: pool_info.token_0_mint.clone(),
            token_b_mint: pool_info.token_1_mint.clone(),
            token_a_reserve: 0, // Would need vault account data
            token_b_reserve: 0, // Would need vault account data
            token_a_decimals: pool_info.token_0_decimals,
            token_b_decimals: pool_info.token_1_decimals,
            sol_reserve: 0,   // Would need vault account data
            token_reserve: 0, // Would need vault account data
            sol_decimals: SOL_DECIMALS,
            token_decimals: pool_info.token_1_decimals,
            token_mint: pool_info.token_1_mint.clone(),
        })
    } else if pool_info.token_1_mint == sol_mint_str {
        // Token 1 is SOL, Token 0 is the target token
        Some(ReservesInfo {
            token_a_mint: pool_info.token_0_mint.clone(),
            token_b_mint: pool_info.token_1_mint.clone(),
            token_a_reserve: 0, // Would need vault account data
            token_b_reserve: 0, // Would need vault account data
            token_a_decimals: pool_info.token_0_decimals,
            token_b_decimals: pool_info.token_1_decimals,
            sol_reserve: 0,   // Would need vault account data
            token_reserve: 0, // Would need vault account data
            sol_decimals: SOL_DECIMALS,
            token_decimals: pool_info.token_0_decimals,
            token_mint: pool_info.token_0_mint.clone(),
        })
    } else {
        if verbose {
            logger::info(
                LogTag::PoolService,
                "Pool does not contain SOL - cannot extract SOL reserves",
            );
        }
        None
    }
}

/// Print account information
fn print_account_info(pool_data: &PoolAccountData, show_raw_data: bool) {
    println!("🏦 Account Information:");
    println!("   Address: {}", pool_data.address);
    println!("   Owner: {}", pool_data.owner);
    println!(
        "   Lamports: {} ({:.9} SOL)",
        pool_data.lamports,
        (pool_data.lamports as f64) / 1e9
    );
    println!("   Data Size: {} bytes", pool_data.data_size());
    println!("   Executable: {}", pool_data.executable);
    println!("   Rent Epoch: {}", pool_data.rent_epoch);

    if show_raw_data {
        println!("\n📄 Raw Data Preview:");
        println!("   Hex (first 64 bytes): {}", pool_data.data_hex_preview());
        println!(
            "   Base64 (first 256 bytes): {}",
            pool_data.data_base64_preview()
        );
    }
    println!();
}

/// Print decoder test results
fn print_decoder_results(results: &[DecoderTestResult]) {
    println!("🔬 Decoder Test Results:");
    println!("========================");

    for result in results {
        let status = if result.success {
            "✅ SUCCESS"
        } else {
            "❌ FAILED"
        };
        println!(
            "   {} - {} ({:.1}ms)",
            result.decoder_name, status, result.decode_time_ms
        );

        if let Some(ref price_result) = result.price_result {
            println!("      💰 Price: {:.12} SOL", price_result.price_sol);
            println!("      🏊 Liquidity: {:.6} SOL", price_result.sol_reserves);
            println!("      🎯 Token: {}", &price_result.mint[..8]);
        }

        if let Some(ref reserves) = result.reserves_info {
            println!("      📊 Reserves:");
            println!(
                "         SOL: {} ({} decimals)",
                reserves.sol_reserve, reserves.sol_decimals
            );
            println!(
                "         Token: {} ({} decimals)",
                reserves.token_reserve, reserves.token_decimals
            );
            println!("         Token Mint: {}", &reserves.token_mint[..8]);
        }

        if let Some(ref error) = result.error {
            println!("      ⚠️  Error: {}", error);
        }

        println!();
    }
}

/// Print summary statistics
fn print_summary(results: &[DecoderTestResult]) {
    let successful = results.iter().filter(|r| r.success).count();
    let total = results.len();
    let avg_decode_time = if total > 0 {
        (results.iter().map(|r| r.decode_time_ms).sum::<u64>() as f64) / (total as f64)
    } else {
        0.0
    };

    println!("📈 Summary:");
    println!("   Successful Decoders: {}/{}", successful, total);
    println!("   Average Decode Time: {:.1}ms", avg_decode_time);

    let with_prices = results.iter().filter(|r| r.price_result.is_some()).count();
    if with_prices > 0 {
        println!("   Decoders with Valid Prices: {}", with_prices);
    }

    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🔍 Pool Decoder Debug Tool");
    println!("==========================");
    println!("Pool: {}", args.pool);
    println!();

    // Fetch pool account data from RPC
    logger::info(LogTag::PoolService, "Fetching pool account data from RPC");
    let pool_data = match fetch_pool_account_data(&args.pool).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Failed to fetch pool data: {}", e);
            return Err(e.into());
        }
    };

    // Print account information
    if args.show_account_info || args.verbose {
        print_account_info(&pool_data, args.show_raw_data);
    }

    // Identify program kind
    let program_kind = identify_program_kind(&pool_data.owner);
    println!(
        "🏷️  Program Kind: {} ({:?})",
        program_kind.display_name(),
        program_kind
    );
    println!("   Owner: {}", pool_data.owner);
    println!();

    // Determine which decoders to test
    let decoders_to_test = if let Some(specific_decoder) = args.decoder {
        vec![specific_decoder]
    } else {
        vec![
            "raydium-cpmm".to_string(),
            "raydium-clmm".to_string(),
            "raydium-legacy".to_string(),
            "orca-whirlpool".to_string(),
            "meteora-damm".to_string(),
            "meteora-dlmm".to_string(),
            "pumpfun-amm".to_string(),
            "pumpfun-legacy".to_string(),
            "moonit-amm".to_string(),
            "fluxbeam-amm".to_string(),
        ]
    };

    // Test decoders
    logger::info(LogTag::PoolService, "Starting decoder tests");
    let mut results = Vec::new();

    for decoder_name in decoders_to_test {
        let result = test_decoder(&decoder_name, program_kind, &pool_data, args.verbose).await;
        results.push(result);
    }

    // Print results
    print_decoder_results(&results);
    print_summary(&results);

    // Final status
    let successful_decoders: Vec<_> = results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.decoder_name.as_str())
        .collect();

    if successful_decoders.is_empty() {
        println!("❌ No decoders were successful for this pool");
        println!("   This could mean:");
        println!("   - Unsupported pool format");
        println!("   - Invalid pool data");
        println!("   - Pool is not a standard AMM");
    } else {
        println!("✅ Successful decoders: {}", successful_decoders.join(", "));

        let with_valid_prices: Vec<_> = results
            .iter()
            .filter(|r| r.price_result.is_some())
            .map(|r| r.decoder_name.as_str())
            .collect();

        if !with_valid_prices.is_empty() {
            println!(
                "💰 Decoders with valid prices: {}",
                with_valid_prices.join(", ")
            );
        }
    }

    logger::info(LogTag::PoolService, "Decoder testing completed");
    Ok(())
}
