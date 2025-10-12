use chrono::{DateTime, Utc};
/// Debug tool for analyzing decimal fetching issues
///
/// This tool investigates why many tokens are missing decimals by:
/// 1. Analyzing tokens in database without decimals
/// 2. Checking failed decimals cache for patterns
/// 3. Testing batch decimal fetching on problem tokens
/// 4. Outputting detailed JSON reports for analysis
use screenerbot::logger::{init_file_logging, log, LogTag};
use screenerbot::rpc::get_rpc_client;
use screenerbot::rpc::init_rpc_client;
use screenerbot::tokens::blacklist;
use screenerbot::tokens::database::TokenDatabase;
use screenerbot::tokens::decimals::{
    batch_fetch_token_decimals, cleanup_retryable_failed_cache, get_database_stats,
    get_failed_cache_stats, get_token_decimals_from_chain, migrate_failed_tokens_to_blacklist,
};
use screenerbot::tokens::get_token_decimals_sync;
use serde::{Deserialize, Serialize};
use solana_program::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
struct DecimalAnalysisReport {
    pub timestamp: DateTime<Utc>,
    pub summary: DecimalSummary,
    pub tokens_without_decimals: Vec<TokenWithoutDecimals>,
    pub failed_tokens: Vec<FailedToken>,
    pub test_results: Vec<TestResult>,
    pub detailed_debug: Vec<DetailedDebugInfo>,
    pub database_stats: DatabaseStats,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DecimalSummary {
    pub total_tokens_in_db: usize,
    pub tokens_without_decimals: usize,
    pub tokens_with_decimals: usize,
    pub permanently_failed: usize,
    pub retryable_failed: usize,
    pub zero_liquidity_tokens: usize,
    pub success_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenWithoutDecimals {
    pub mint: String,
    pub symbol: String,
    pub liquidity_usd: f64,
    pub age_hours: Option<i64>,
    pub has_transactions: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct FailedToken {
    pub mint: String,
    pub error: String,
    pub is_permanent: bool,
    pub retry_worthy: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    pub mint: String,
    pub symbol: String,
    pub test_type: String,
    pub success: bool,
    pub decimals: Option<u8>,
    pub error: Option<String>,
    pub response_time_ms: u64,
    pub rpc_raw_response: Option<String>,
    pub mint_account_exists: Option<bool>,
    pub mint_account_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DetailedDebugInfo {
    pub mint: String,
    pub steps: Vec<DebugStep>,
    pub final_result: Option<u8>,
    pub total_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DebugStep {
    pub step_name: String,
    pub success: bool,
    pub details: String,
    pub time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseStats {
    pub decimals_cached: usize,
    pub failed_cached: usize,
    pub cache_hit_rate: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    log(
        LogTag::System,
        "START",
        "🔍 Starting decimal fetching analysis...",
    );

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut max_tokens_to_analyze = 100;
    let mut test_batch_size = 10;
    let mut output_file = "decimal_analysis_report.json".to_string();
    let mut cleanup_failed_cache = false;
    let mut enable_detailed_debug = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--max-tokens" => {
                if i + 1 < args.len() {
                    max_tokens_to_analyze = args[i + 1].parse().unwrap_or(100);
                    i += 1;
                }
            }
            "--batch-size" => {
                if i + 1 < args.len() {
                    test_batch_size = args[i + 1].parse().unwrap_or(10);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_file = args[i + 1].clone();
                    i += 1;
                }
            }
            "--cleanup-failed" => {
                cleanup_failed_cache = true;
            }
            "--detailed-debug" => {
                enable_detailed_debug = true;
            }
            "--test-failed" => {
                // Test some tokens from the failed cache to debug why they're failing
                log(
                    LogTag::System,
                    "TEST_FAILED",
                    "🔍 Testing failed tokens for debugging...",
                );
                let test_results = test_failed_tokens().await;
                for result in test_results {
                    log(
                        LogTag::System,
                        "TEST_FAILED",
                        &format!("Tested {}: {:?}", result.mint, result.final_result),
                    );
                }
                return Ok(());
            }
            "--test-retry-limit" => {
                // Test retry limit functionality with a specific problematic token
                log(
                    LogTag::System,
                    "TEST_RETRY",
                    "🔄 Testing retry limit functionality...",
                );
                test_retry_limit_functionality().await;
                return Ok(());
            }
            "--test-blacklist" => {
                // Test blacklist integration and migration
                log(
                    LogTag::System,
                    "TEST_BLACKLIST",
                    "🛡️ Testing blacklist integration...",
                );
                test_blacklist_integration().await;
                return Ok(());
            }
            "--help" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    log(
        LogTag::System,
        "CONFIG",
        &format!(
            "Analysis parameters: max_tokens={}, batch_size={}, output={}, cleanup={}, detailed_debug={}",
            max_tokens_to_analyze,
            test_batch_size,
            output_file,
            cleanup_failed_cache,
            enable_detailed_debug
        )
    );

    // Initialize RPC client
    match init_rpc_client() {
        Ok(_) => log(
            LogTag::System,
            "RPC",
            "✅ RPC client initialized successfully",
        ),
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("❌ Failed to initialize RPC client: {}", e),
            );
            return Err(e.into());
        }
    }

    // Initialize database
    let database = match TokenDatabase::new() {
        Ok(db) => {
            log(
                LogTag::System,
                "DB",
                "✅ Token database connected successfully",
            );
            db
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("❌ Failed to connect to database: {}", e),
            );
            return Err(e.into());
        }
    };

    // Optional: Cleanup retryable failed cache
    if cleanup_failed_cache {
        log(
            LogTag::System,
            "CLEANUP",
            "🧹 Cleaning up retryable failed cache...",
        );
        match cleanup_retryable_failed_cache() {
            Ok((removed, remaining)) => {
                log(
                    LogTag::System,
                    "CLEANUP",
                    &format!(
                        "✅ Cleaned failed cache: removed {} retryable errors, {} permanent errors remain",
                        removed,
                        remaining
                    )
                );
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "CLEANUP",
                    &format!("⚠️ Failed to cleanup cache: {}", e),
                );
            }
        }
    }

    // Start analysis
    let mut report = DecimalAnalysisReport {
        timestamp: Utc::now(),
        summary: DecimalSummary {
            total_tokens_in_db: 0,
            tokens_without_decimals: 0,
            tokens_with_decimals: 0,
            permanently_failed: 0,
            retryable_failed: 0,
            zero_liquidity_tokens: 0,
            success_rate: 0.0,
        },
        tokens_without_decimals: Vec::new(),
        failed_tokens: Vec::new(),
        test_results: Vec::new(),
        detailed_debug: Vec::new(),
        database_stats: DatabaseStats {
            decimals_cached: 0,
            failed_cached: 0,
            cache_hit_rate: 0.0,
        },
        recommendations: Vec::new(),
    };

    // Step 1: Get database statistics
    log(
        LogTag::System,
        "STATS",
        "📊 Analyzing database statistics...",
    );
    match get_database_stats() {
        Ok((cached_decimals, failed_decimals)) => {
            report.database_stats.decimals_cached = cached_decimals;
            report.database_stats.failed_cached = failed_decimals;
            log(
                LogTag::System,
                "STATS",
                &format!(
                    "Database: {} cached decimals, {} failed tokens",
                    cached_decimals, failed_decimals
                ),
            );
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to get database stats: {}", e),
            );
        }
    }

    // Step 2: Get failed cache statistics
    log(LogTag::System, "FAILED", "🔍 Analyzing failed tokens...");
    match get_failed_cache_stats() {
        Ok((total_failed, permanent_failed, sample_errors)) => {
            report.summary.permanently_failed = permanent_failed;
            report.summary.retryable_failed = total_failed - permanent_failed;

            log(
                LogTag::System,
                "FAILED",
                &format!(
                    "Failed tokens: {} total ({} permanent, {} retryable)",
                    total_failed,
                    permanent_failed,
                    total_failed - permanent_failed
                ),
            );

            if !sample_errors.is_empty() {
                log(LogTag::System, "FAILED", "Sample failed token errors:");
                for (i, error) in sample_errors.iter().enumerate().take(5) {
                    log(LogTag::System, "FAILED", &format!("  {}. {}", i + 1, error));
                }
            }
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to get failed cache stats: {}", e),
            );
        }
    }

    // Step 3: Analyze tokens in database
    log(
        LogTag::System,
        "TOKENS",
        "🔍 Analyzing tokens without decimals...",
    );
    let all_tokens = match database.get_all_tokens_with_update_time().await {
        Ok(tokens) => {
            log(
                LogTag::System,
                "TOKENS",
                &format!("Found {} tokens in database", tokens.len()),
            );
            tokens
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to get tokens from database: {}", e),
            );
            return Err(e.into());
        }
    };

    report.summary.total_tokens_in_db = all_tokens.len();

    // Get fresh tokens only (last 1 hour)
    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let fresh_tokens: Vec<_> = all_tokens
        .into_iter()
        .filter(|(_, _, last_updated, _)| *last_updated >= one_hour_ago)
        .collect();

    log(
        LogTag::System,
        "TOKENS",
        &format!(
            "Found {} fresh tokens (updated in last hour)",
            fresh_tokens.len()
        ),
    );

    // Analyze tokens for decimal status
    let mut tokens_without_decimals = Vec::new();
    let mut tokens_with_decimals = 0;
    let mut zero_liquidity_count = 0;

    for (mint, symbol, _, liquidity_usd) in fresh_tokens.iter().take(max_tokens_to_analyze) {
        // Check if token has cached decimals
        let has_decimals = get_token_decimals_sync(mint).is_some();

        if has_decimals {
            tokens_with_decimals += 1;
        } else {
            // Get full token info to analyze why decimals are missing
            if let Ok(Some(token)) = database.get_token_by_mint(mint) {
                let age_hours = token.pair_created_at.map(|created| {
                    let created_dt =
                        DateTime::from_timestamp(created, 0).unwrap_or_else(|| Utc::now());
                    let age = now - created_dt;
                    age.num_hours()
                });

                let token_without_decimals = TokenWithoutDecimals {
                    mint: mint.clone(),
                    symbol: symbol.clone(),
                    liquidity_usd: *liquidity_usd,
                    age_hours,
                    has_transactions: token.txns.is_some(),
                };

                if *liquidity_usd <= 0.0 {
                    zero_liquidity_count += 1;
                } else {
                    tokens_without_decimals.push(token_without_decimals);
                }
            }
        }
    }

    report.summary.tokens_with_decimals = tokens_with_decimals;
    report.summary.tokens_without_decimals = tokens_without_decimals.len();
    report.summary.zero_liquidity_tokens = zero_liquidity_count;
    report.summary.success_rate = if fresh_tokens.len() > 0 {
        ((tokens_with_decimals as f64) / (fresh_tokens.len() as f64)) * 100.0
    } else {
        0.0
    };

    log(
        LogTag::System,
        "ANALYSIS",
        &format!(
            "Analysis results: {}/{} tokens have decimals ({:.1}% success rate)",
            tokens_with_decimals,
            fresh_tokens.len(),
            report.summary.success_rate
        ),
    );

    // Step 4: Test batch fetching on problematic tokens
    log(
        LogTag::System,
        "TEST",
        "🧪 Testing batch decimal fetching...",
    );

    // Select high-liquidity tokens without decimals for testing
    let mut test_tokens: Vec<_> = tokens_without_decimals
        .iter()
        .filter(|t| t.liquidity_usd > 1000.0) // High liquidity tokens
        .take(test_batch_size)
        .collect();

    // If not enough high-liquidity tokens, add medium liquidity ones
    if test_tokens.len() < test_batch_size {
        let additional: Vec<_> = tokens_without_decimals
            .iter()
            .filter(|t| t.liquidity_usd > 100.0 && t.liquidity_usd <= 1000.0)
            .take(test_batch_size - test_tokens.len())
            .collect();
        test_tokens.extend(additional);
    }

    if !test_tokens.is_empty() {
        log(
            LogTag::System,
            "TEST",
            &format!(
                "Testing decimal fetching for {} tokens...",
                test_tokens.len()
            ),
        );

        let test_mints: Vec<String> = test_tokens.iter().map(|t| t.mint.clone()).collect();
        let start_time = std::time::Instant::now();

        let batch_results = batch_fetch_token_decimals(&test_mints).await;
        let elapsed_ms = start_time.elapsed().as_millis() as u64;

        for (i, (mint, result)) in batch_results.iter().enumerate() {
            let test_token = &test_tokens[i];
            let test_result = TestResult {
                mint: mint.clone(),
                symbol: test_token.symbol.clone(),
                test_type: "batch_fetch".to_string(),
                success: result.is_ok(),
                decimals: result.as_ref().ok().copied(),
                error: result.as_ref().err().map(|e| e.clone()),
                response_time_ms: elapsed_ms / (batch_results.len() as u64),
                rpc_raw_response: None,    // Will be filled by detailed debug
                mint_account_exists: None, // Will be filled by detailed debug
                mint_account_size: None,   // Will be filled by detailed debug
            };

            if test_result.success {
                log(
                    LogTag::System,
                    "TEST",
                    &format!(
                        "✅ {} ({}): {} decimals",
                        &test_result.mint,
                        test_result.symbol,
                        test_result.decimals.unwrap_or(0)
                    ),
                );
            } else {
                log(
                    LogTag::System,
                    "TEST",
                    &format!(
                        "❌ {} ({}): {}",
                        &test_result.mint,
                        test_result.symbol,
                        test_result
                            .error
                            .as_ref()
                            .unwrap_or(&"Unknown error".to_string())
                    ),
                );
            }

            report.test_results.push(test_result);
        }

        let successful_tests = report.test_results.iter().filter(|t| t.success).count();
        log(
            LogTag::System,
            "TEST",
            &format!(
                "Test results: {}/{} successful ({:.1}% success rate)",
                successful_tests,
                report.test_results.len(),
                ((successful_tests as f64) / (report.test_results.len() as f64)) * 100.0
            ),
        );
    } else {
        log(
            LogTag::System,
            "TEST",
            "⚠️ No suitable tokens found for testing",
        );
    }

    // Step 4.5: Detailed debugging of core fetching logic
    if enable_detailed_debug {
        log(
            LogTag::System,
            "DETAILED",
            "🔬 Starting detailed debugging of fetching logic...",
        );
        let detailed_debug_results = perform_detailed_debugging(&tokens_without_decimals, 5).await;
        report.detailed_debug = detailed_debug_results;
    } else {
        log(
            LogTag::System,
            "DETAILED",
            "🔬 Detailed debugging disabled (use --detailed-debug to enable)",
        );
    }

    // Step 5: Store problem tokens without decimals (limited)
    report.tokens_without_decimals = tokens_without_decimals.into_iter().take(50).collect();

    // Step 6: Generate recommendations
    generate_recommendations(&mut report);

    // Step 7: Save report to file
    log(
        LogTag::System,
        "SAVE",
        &format!("💾 Saving analysis report to {}", output_file),
    );
    let json_output = serde_json::to_string_pretty(&report)?;
    std::fs::write(&output_file, json_output)?;

    // Step 8: Print summary
    print_summary(&report);

    log(LogTag::System, "COMPLETE", "✅ Decimal analysis complete!");
    Ok(())
}

/// Perform detailed step-by-step debugging of decimal fetching for specific tokens
async fn perform_detailed_debugging(
    tokens_without_decimals: &[TokenWithoutDecimals],
    max_tokens: usize,
) -> Vec<DetailedDebugInfo> {
    let mut debug_results = Vec::new();

    // Select tokens for detailed debugging - prefer higher liquidity but don't require minimum
    let tokens_to_debug: Vec<_> = tokens_without_decimals
        .iter()
        .filter(|t| t.liquidity_usd >= 0.0) // Include all tokens for debugging
        .take(max_tokens)
        .collect();

    if tokens_to_debug.is_empty() {
        log(
            LogTag::System,
            "DETAILED",
            "No tokens found for detailed debugging",
        );
        // If no tokens without decimals, let's test a few known tokens
        let test_mints = vec![
            ("So11111111111111111111111111111111111111112", "SOL"), // Should work
            ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC"), // Should work
            ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "USDT"), // Should work
        ];

        log(
            LogTag::System,
            "DETAILED",
            "Testing known tokens for debugging",
        );
        for (mint, symbol) in test_mints {
            let debug_info = debug_single_token_fetching(mint, symbol).await;
            debug_results.push(debug_info);
        }
        return debug_results;
    }

    log(
        LogTag::System,
        "DETAILED",
        &format!(
            "Performing detailed debugging on {} tokens",
            tokens_to_debug.len()
        ),
    );

    for token in tokens_to_debug {
        let debug_info = debug_single_token_fetching(&token.mint, &token.symbol).await;
        debug_results.push(debug_info);
    }

    debug_results
}

/// Debug the complete decimal fetching process for a single token
async fn debug_single_token_fetching(mint_str: &str, symbol: &str) -> DetailedDebugInfo {
    let start_time = std::time::Instant::now();
    let mut steps = Vec::new();
    let mut final_decimals = None;

    log(
        LogTag::System,
        "DETAILED",
        &format!("🔬 Debugging token: {} ({})", mint_str, symbol),
    );

    // Step 1: Validate mint address format
    let step_start = std::time::Instant::now();
    let pubkey_result = Pubkey::from_str(mint_str);
    let step_time = step_start.elapsed().as_millis() as u64;

    match &pubkey_result {
        Ok(_) => {
            steps.push(DebugStep {
                step_name: "1. Mint Address Validation".to_string(),
                success: true,
                details: "Valid Solana public key format".to_string(),
                time_ms: step_time,
            });
        }
        Err(e) => {
            steps.push(DebugStep {
                step_name: "1. Mint Address Validation".to_string(),
                success: false,
                details: format!("Invalid public key: {}", e),
                time_ms: step_time,
            });
            return DetailedDebugInfo {
                mint: mint_str.to_string(),
                steps,
                final_result: None,
                total_time_ms: start_time.elapsed().as_millis() as u64,
            };
        }
    }

    let pubkey = pubkey_result.unwrap();

    // Step 2: Check cache+online
    let step_start = std::time::Instant::now();
    let cached_decimals = get_token_decimals_sync(mint_str);
    let step_time = step_start.elapsed().as_millis() as u64;

    match cached_decimals {
        Some(decimals) => {
            steps.push(DebugStep {
                step_name: "2. Cache/Online Check".to_string(),
                success: true,
                details: format!("Found: {} decimals", decimals),
                time_ms: step_time,
            });
            final_decimals = Some(decimals);
        }
        None => {
            steps.push(DebugStep {
                step_name: "2. Cache/Online Check".to_string(),
                success: false,
                details: "Not found in cache or online".to_string(),
                time_ms: step_time,
            });
        }
    }

    // Step 3: Direct RPC call to get account info
    let step_start = std::time::Instant::now();
    let rpc_client = get_rpc_client();

    // Use get_multiple_accounts for consistency with the decimal fetching logic
    match rpc_client.get_multiple_accounts(&[pubkey]).await {
        Ok(accounts) => {
            let step_time = step_start.elapsed().as_millis() as u64;
            if let Some(Some(account)) = accounts.first() {
                steps.push(DebugStep {
                    step_name: "3. RPC Account Fetch".to_string(),
                    success: true,
                    details: format!(
                        "Account exists, size: {} bytes, owner: {}",
                        account.data.len(),
                        account.owner
                    ),
                    time_ms: step_time,
                });

                // Step 4: Try to decode mint data
                let decode_start = std::time::Instant::now();
                match Mint::unpack(&account.data) {
                    Ok(mint_data) => {
                        let decode_time = decode_start.elapsed().as_millis() as u64;
                        steps.push(DebugStep {
                            step_name: "4. Mint Data Decode".to_string(),
                            success: true,
                            details: format!(
                                "Successfully decoded: {} decimals, mint_authority: {:?}, supply: {}",
                                mint_data.decimals,
                                mint_data.mint_authority,
                                mint_data.supply
                            ),
                            time_ms: decode_time,
                        });
                        final_decimals = Some(mint_data.decimals);
                    }
                    Err(e) => {
                        let decode_time = decode_start.elapsed().as_millis() as u64;
                        steps.push(DebugStep {
                            step_name: "4. Mint Data Decode".to_string(),
                            success: false,
                            details: format!("Failed to decode mint data: {}", e),
                            time_ms: decode_time,
                        });
                    }
                }
            } else {
                steps.push(DebugStep {
                    step_name: "3. RPC Account Fetch".to_string(),
                    success: false,
                    details: "Account does not exist".to_string(),
                    time_ms: step_time,
                });
            }
        }
        Err(e) => {
            let step_time = step_start.elapsed().as_millis() as u64;
            steps.push(DebugStep {
                step_name: "3. RPC Account Fetch".to_string(),
                success: false,
                details: format!("RPC error: {}", e),
                time_ms: step_time,
            });
        }
    }

    // Step 5: Test batch function
    let step_start = std::time::Instant::now();
    let batch_result = batch_fetch_token_decimals(&[mint_str.to_string()]).await;
    let step_time = step_start.elapsed().as_millis() as u64;

    if let Some((_, result)) = batch_result.first() {
        match result {
            Ok(decimals) => {
                steps.push(DebugStep {
                    step_name: "5. Batch Function Test".to_string(),
                    success: true,
                    details: format!("Batch function returned: {} decimals", decimals),
                    time_ms: step_time,
                });
                if final_decimals.is_none() {
                    final_decimals = Some(*decimals);
                }
            }
            Err(e) => {
                steps.push(DebugStep {
                    step_name: "5. Batch Function Test".to_string(),
                    success: false,
                    details: format!("Batch function error: {}", e),
                    time_ms: step_time,
                });
            }
        }
    }

    let total_time = start_time.elapsed().as_millis() as u64;

    log(
        LogTag::System,
        "DETAILED",
        &format!(
            "🔬 Debug complete for {}: {} steps, final result: {:?} ({} ms)",
            mint_str,
            steps.len(),
            final_decimals,
            total_time
        ),
    );

    DetailedDebugInfo {
        mint: mint_str.to_string(),
        steps,
        final_result: final_decimals,
        total_time_ms: total_time,
    }
}

/// Test some tokens from the failed cache to see if they're genuinely failing
async fn test_failed_tokens() -> Vec<DetailedDebugInfo> {
    let mut debug_results = Vec::new();

    // Test some hardcoded tokens that we know might fail to demonstrate the debugging
    let test_tokens = vec![
        (
            "EJwHi2ct3oNnDJzQpSFoayHgjLk5U8Gzj8P7Z3k4BjvT",
            "KNOWN_FAILED_1",
        ),
        (
            "MmaTxLbE6z4gEL5RdQz3u4TKL2k3f2bF8Y3nRnQ9t9bS",
            "KNOWN_FAILED_2",
        ),
        (
            "Eh4RFjAQ8u8LkP5n3mE9zN2vW8fK7qJ4A1cY6dR5x2uT",
            "KNOWN_FAILED_3",
        ),
    ];

    log(
        LogTag::System,
        "TEST_FAILED",
        &format!("Testing {} known problematic tokens", test_tokens.len()),
    );

    for (mint, symbol) in test_tokens {
        log(
            LogTag::System,
            "TEST_FAILED",
            &format!("Testing token {}: {}", mint, symbol),
        );
        let debug_info = debug_single_token_fetching(mint, symbol).await;
        debug_results.push(debug_info);
    }

    debug_results
}

/// Test blacklist integration functionality
async fn test_blacklist_integration() {
    println!("\n🛡️  Testing blacklist integration...");

    // First, migrate any existing failed tokens
    match migrate_failed_tokens_to_blacklist() {
        Ok(migrated) => {
            println!(
                "✅ Migration completed. {} tokens migrated to blacklist",
                migrated
            );
            log(
                LogTag::System,
                "BLACKLIST_MIGRATION",
                &format!("Migrated {} tokens", migrated),
            );
        }
        Err(e) => {
            println!("❌ Migration failed: {}", e);
            log(LogTag::System, "ERROR", &format!("Migration failed: {}", e));
            return;
        }
    }

    // Get some failed tokens to test blacklist status
    if let Ok((total, permanent, samples)) = get_failed_cache_stats() {
        println!("📊 Failed tokens statistics:");
        println!("   Total failed: {}", total);
        println!("   Permanent failures: {}", permanent);

        if total > 0 {
            println!("🔍 Testing blacklist status of failed tokens:");
            for sample in samples.iter().take(5) {
                if let Some(mint_part) = sample.split(':').next() {
                    let mint = mint_part.trim();
                    let is_blacklisted = blacklist::is_token_blacklisted(mint);
                    let status = if is_blacklisted {
                        "✅ BLACKLISTED"
                    } else {
                        "❌ NOT BLACKLISTED"
                    };
                    println!("   {} -> {}", mint, status);
                }
            }
        } else {
            println!("ℹ️  No failed tokens found to test blacklist status");
        }
    } else {
        println!("❌ Could not get failed token statistics");
    }

    // Test creating a new failed token and verify it gets blacklisted
    println!("\n🧪 Testing automatic blacklisting of new failures...");
    let test_mint = "TestMint1234567890abcdef1234567890abcdef12345678"; // Invalid mint for testing

    println!("Testing with invalid mint: {}", test_mint);

    // This should fail and automatically add to blacklist
    match get_token_decimals_from_chain(test_mint).await {
        Ok(_) => println!("⚠️  Unexpected success for invalid mint"),
        Err(e) => {
            println!("✅ Expected failure: {}", e);

            // Check if it was automatically blacklisted
            let is_blacklisted = blacklist::is_token_blacklisted(test_mint);
            let blacklist_status = if is_blacklisted {
                "✅ AUTO-BLACKLISTED"
            } else {
                "❌ NOT BLACKLISTED"
            };
            println!("   Blacklist status: {}", blacklist_status);
        }
    }

    println!("\n✅ Blacklist integration test completed");
    log(
        LogTag::System,
        "TEST_COMPLETE",
        "Blacklist integration test finished",
    );
}

/// Test retry limit functionality by repeatedly trying to fetch a problematic token
async fn test_retry_limit_functionality() {
    let test_mint = "BadTokenThatWillFailNetworkError123456789AbCdEf"; // Invalid mint that will cause network errors

    log(
        LogTag::System,
        "TEST_RETRY",
        &format!("Testing retry limit with problematic token: {}", test_mint),
    );

    // Try to fetch the same token multiple times to test retry limits
    for attempt in 1..=5 {
        log(
            LogTag::System,
            "TEST_RETRY",
            &format!(
                "Attempt {}: Trying to fetch decimals for test token",
                attempt
            ),
        );

        let results = batch_fetch_token_decimals(&[test_mint.to_string()]).await;

        if let Some((_mint, result)) = results.first() {
            match result {
                Ok(decimals) => {
                    log(
                        LogTag::System,
                        "TEST_RETRY",
                        &format!("✅ Attempt {}: SUCCESS - {} decimals", attempt, decimals),
                    );
                    break;
                }
                Err(error) => {
                    log(
                        LogTag::System,
                        "TEST_RETRY",
                        &format!("❌ Attempt {}: FAILED - {}", attempt, error),
                    );

                    // Check if it's now marked as permanently failed
                    if error.contains("max retries") || error.contains("permanent") {
                        log(
                            LogTag::System,
                            "TEST_RETRY",
                            &format!("🛑 Token reached retry limit and is now permanently failed"),
                        );
                        break;
                    }
                }
            }
        }

        // Small delay between attempts
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    log(LogTag::System, "TEST_RETRY", "✅ Retry limit test complete");
}

fn generate_recommendations(report: &mut DecimalAnalysisReport) {
    let mut recommendations = Vec::new();

    // Low success rate
    if report.summary.success_rate < 50.0 {
        recommendations.push(
            "CRITICAL: Very low decimal fetch success rate. Check RPC connectivity and rate limits.".to_string()
        );
    }

    // High permanent failures
    if report.summary.permanently_failed > 100 {
        recommendations.push(format!(
            "Consider cleaning {} permanently failed tokens from cache if they're outdated.",
            report.summary.permanently_failed
        ));
    }

    // Test results analysis
    if !report.test_results.is_empty() {
        let test_success_rate = ((report.test_results.iter().filter(|t| t.success).count() as f64)
            / (report.test_results.len() as f64))
            * 100.0;

        if test_success_rate < 30.0 {
            recommendations.push(
                "URGENT: Real-time decimal fetching is failing. Check RPC endpoints and network connectivity.".to_string()
            );
        } else if test_success_rate < 70.0 {
            recommendations.push(
                "WARNING: Moderate decimal fetching issues. Consider implementing retry logic or RPC fallbacks.".to_string()
            );
        }

        // Analyze common error patterns
        let mut error_patterns: HashMap<String, usize> = HashMap::new();
        for test in &report.test_results {
            if let Some(error) = &test.error {
                let pattern = if error.contains("rate limit") || error.contains("429") {
                    "Rate Limiting"
                } else if error.contains("timeout") || error.contains("connection") {
                    "Network Issues"
                } else if error.contains("account not found") || error.contains("invalid") {
                    "Invalid Tokens"
                } else {
                    "Other Errors"
                };
                *error_patterns.entry(pattern.to_string()).or_insert(0) += 1;
            }
        }

        for (pattern, count) in error_patterns {
            if count > 2 {
                recommendations.push(format!(
                    "Frequent error pattern: {} ({}x) - investigate and fix",
                    pattern, count
                ));
            }
        }
    }

    // High number of tokens without decimals
    if report.summary.tokens_without_decimals > 200 {
        recommendations.push(
            "Consider implementing proactive decimal fetching for high-liquidity tokens during database updates.".to_string()
        );
    }

    // Cache performance
    if report.database_stats.decimals_cached < 1000 {
        recommendations.push(
            "Low decimal cache size. Consider pre-fetching decimals for commonly traded tokens."
                .to_string(),
        );
    }

    report.recommendations = recommendations;
}

fn print_summary(report: &DecimalAnalysisReport) {
    println!("\n{}", "=".repeat(80));
    println!("📊 DECIMAL ANALYSIS SUMMARY");
    println!("{}", "=".repeat(80));
    println!(
        "Timestamp: {}",
        report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!();

    println!("🔢 TOKEN STATISTICS:");
    println!(
        "  Total tokens in DB:       {}",
        report.summary.total_tokens_in_db
    );
    println!(
        "  Tokens with decimals:     {} ({:.1}%)",
        report.summary.tokens_with_decimals, report.summary.success_rate
    );
    println!(
        "  Tokens without decimals:  {}",
        report.summary.tokens_without_decimals
    );
    println!(
        "  Zero liquidity tokens:    {}",
        report.summary.zero_liquidity_tokens
    );
    println!();

    println!("❌ FAILED TOKENS:");
    println!(
        "  Permanently failed:       {}",
        report.summary.permanently_failed
    );
    println!(
        "  Retryable failed:         {}",
        report.summary.retryable_failed
    );
    println!();

    println!("💾 CACHE STATISTICS:");
    println!(
        "  Cached decimals:          {}",
        report.database_stats.decimals_cached
    );
    println!(
        "  Failed cache entries:     {}",
        report.database_stats.failed_cached
    );
    println!();

    if !report.test_results.is_empty() {
        let successful_tests = report.test_results.iter().filter(|t| t.success).count();
        let test_success_rate =
            ((successful_tests as f64) / (report.test_results.len() as f64)) * 100.0;

        println!("🧪 LIVE TEST RESULTS:");
        println!("  Tests performed:          {}", report.test_results.len());
        println!(
            "  Successful:               {} ({:.1}%)",
            successful_tests, test_success_rate
        );
        println!(
            "  Failed:                   {}",
            report.test_results.len() - successful_tests
        );
        println!();
    }

    if !report.recommendations.is_empty() {
        println!("💡 RECOMMENDATIONS:");
        for (i, recommendation) in report.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, recommendation);
        }
        println!();
    }

    println!(
        "📁 Full report saved to: {}",
        "decimal_analysis_report.json"
    );
    println!("{}", "=".repeat(80));
}

fn print_help() {
    println!("Decimal Analysis Tool - Debug decimal fetching issues");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin debug_decimals [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --max-tokens <N>     Maximum tokens to analyze (default: 100)");
    println!("  --batch-size <N>     Number of tokens to test batch fetching (default: 10)");
    println!(
        "  --output <FILE>      Output JSON file name (default: decimal_analysis_report.json)"
    );
    println!("  --cleanup-failed     Clean up retryable failed tokens from cache before analysis");
    println!("  --detailed-debug     Enable step-by-step debugging of core fetching logic");
    println!("  --test-failed        Test tokens from failed cache to debug why they're failing");
    println!("  --test-retry-limit   Test retry limit functionality with problematic tokens");
    println!("  --test-blacklist     Test blacklist integration and migrate failed tokens");
    println!("  --help               Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  cargo run --bin debug_decimals");
    println!("  cargo run --bin debug_decimals --max-tokens 500 --batch-size 20");
    println!("  cargo run --bin debug_decimals --cleanup-failed --detailed-debug");
    println!("  cargo run --bin debug_decimals --test-blacklist");
    println!("  cargo run --bin debug_decimals --detailed-debug --output detailed_report.json");
}
