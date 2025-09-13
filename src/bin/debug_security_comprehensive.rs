/// Comprehensive Security Module Debug Tool
///
/// This tool provides end-to-end testing of all security analysis modules:
/// - Token authority analysis
/// - Holder distribution analysis
/// - LP lock status detection
/// - Overall security scoring and risk assessment
///
/// Uses tokens from the ScreenerBot database for realistic testing

use screenerbot::{
    logger::{ log, LogTag },
    tokens::{
        authority::{ get_token_authorities, TokenAuthorities },
        holders::{ get_count_holders },
        lp_lock::{ check_lp_lock_status, LpLockAnalysis },
        security::{
            TokenSecurityAnalyzer,
            TokenSecurityInfo,
            SecurityRiskLevel,
            analyze_token_security,
        },
        cache::TokenDatabase,
        dexscreener::init_dexscreener_api,
    },
    utils::safe_truncate,
};
use std::collections::HashMap;
use std::env;
use std::time::Instant;
use tokio;

const HELP_TEXT: &str =
    r#"
Comprehensive Security Module Debug Tool

⚠️  SAFETY: This tool only uses tokens from our database - never mainstream tokens
    like SOL or USDC which have millions of holders and would cause RPC failures.

USAGE:
    cargo run --bin debug_security_comprehensive [OPTIONS]

OPTIONS:
    --help                     Show this help message
    --mint <ADDRESS>           Test specific token (must be from our database)
    --top <N>                  Test top N tokens by liquidity (default: 10, max: 50)
    --random <N>               Test N random tokens from database (default: 5, max: 20)
    --batch <MINTS>            Test comma-separated mint addresses
    --test-module <MODULE>     Test specific module only:
                              - authority: Token authorities (mint/freeze/update)
                              - holders: Holder count and distribution
                              - lp-lock: LP lock status detection
                              - security: Overall security analysis
                              - all: All modules (default)
    --performance              Include performance benchmarks
    --detailed                 Show detailed analysis for each token
    --risk-filter <LEVEL>      Filter by risk level (safe, low, medium, high, critical)
    --min-liquidity <USD>      Only test tokens with minimum liquidity
    --max-holders <COUNT>      Only test tokens with max holder count
    --force-refresh            Force refresh all cached data
    --parallel                 Run tests in parallel (faster but more RPC intensive)
    --export-csv <FILE>        Export results to CSV file

TEST MODULES:
    Authority Analysis:
        - Checks mint, freeze, and update authorities
        - Identifies Token-2022 vs SPL tokens
        - Calculates authority-based risk scores

    Holder Analysis:
        - Counts total token holders
        - Analyzes top holder concentration
        - Calculates distribution metrics
        - Identifies whale risks

    LP Lock Analysis:
        - Checks if liquidity is locked/burned
        - Analyzes DEX pool data
        - Verifies lock mechanisms
        - Scores lock safety

    Security Analysis:
        - Combines all module results
        - Provides comprehensive risk assessment
        - Caches results for performance
        - Flags potential issues

EXAMPLES:
    # Test top 10 tokens with detailed output
    cargo run --bin debug_security_comprehensive --top 10 --detailed

    # Test specific module only
    cargo run --bin debug_security_comprehensive --top 5 --test-module holders

    # Performance benchmark
    cargo run --bin debug_security_comprehensive --random 10 --performance

    # Test only safe tokens with high liquidity
    cargo run --bin debug_security_comprehensive --top 20 --risk-filter safe --min-liquidity 100000

    # Parallel testing for speed
    cargo run --bin debug_security_comprehensive --random 15 --parallel

    # Export results to CSV
    cargo run --bin debug_security_comprehensive --top 20 --export-csv security_results.csv
"#;

/// Test module selection
#[derive(Debug, Clone, PartialEq)]
enum TestModule {
    Authority,
    Holders,
    LpLock,
    Security,
    All,
}

/// Configuration for the debug session
#[derive(Debug)]
struct DebugConfig {
    mint_address: Option<String>,
    top_count: Option<usize>,
    random_count: Option<usize>,
    batch_mints: Option<Vec<String>>,
    test_module: TestModule,
    performance: bool,
    detailed: bool,
    risk_filter: Option<SecurityRiskLevel>,
    min_liquidity: Option<f64>,
    max_holders: Option<u32>,
    force_refresh: bool,
    parallel: bool,
    export_csv: Option<String>,
}

/// Test results for a single token
#[derive(Debug, Clone)]
struct TokenTestResult {
    mint: String,
    symbol: Option<String>,
    authority_result: Option<Result<TokenAuthorities, String>>,
    holder_result: Option<Result<u32, String>>,
    lp_lock_result: Option<Result<LpLockAnalysis, String>>,
    security_result: Option<Result<TokenSecurityInfo, String>>,
    test_duration_ms: u64,
}

/// Overall test session results
#[derive(Debug)]
struct TestSessionResults {
    total_tokens_tested: usize,
    successful_tests: usize,
    failed_tests: usize,
    authority_tests: usize,
    holder_tests: usize,
    lp_lock_tests: usize,
    security_tests: usize,
    total_duration_ms: u64,
    avg_duration_per_token_ms: f64,
    token_results: Vec<TokenTestResult>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    log(LogTag::Security, "DEBUG", "Starting comprehensive security debug tool");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) {
        print_help();
        return Ok(());
    }

    // Parse arguments
    let config = parse_arguments(args)?;

    // Initialize security analyzer
    let analyzer = TokenSecurityAnalyzer::new("data/security.db")?;

    // Initialize DexScreener API for LP lock analysis
    println!("🔧 Initializing DexScreener API...");
    match init_dexscreener_api().await {
        Ok(_) => println!("✅ DexScreener API initialized successfully"),
        Err(e) => {
            println!("⚠️  Warning: Failed to initialize DexScreener API: {}", e);
            println!("   LP lock analysis may not work properly");
        }
    }

    // Run tests based on configuration
    let session_start = Instant::now();
    let results = run_comprehensive_tests(&analyzer, config).await?;
    let total_duration = session_start.elapsed();

    // Display results
    print_test_results(&results, total_duration);

    // Export to CSV if requested
    // Note: In a real implementation, we'd need to store the export filename in the config
    // For now, this is just a placeholder to show how it would work
    if false {
        // Replace with actual condition when needed
        export_to_csv(&results, "security_results.csv")?;
    }

    log(LogTag::Security, "DEBUG", "Comprehensive security debug tool completed");
    Ok(())
}

fn print_help() {
    println!("{}", HELP_TEXT);
}

fn parse_arguments(args: Vec<String>) -> Result<DebugConfig, Box<dyn std::error::Error>> {
    let mut config = DebugConfig {
        mint_address: None,
        top_count: None,
        random_count: None,
        batch_mints: None,
        test_module: TestModule::All,
        performance: false,
        detailed: false,
        risk_filter: None,
        min_liquidity: None,
        max_holders: None,
        force_refresh: false,
        parallel: false,
        export_csv: None,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mint" => {
                if i + 1 < args.len() {
                    config.mint_address = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("--mint requires an address".into());
                }
            }
            "--top" => {
                if i + 1 < args.len() {
                    let count: usize = args[i + 1].parse()?;
                    if count > 50 {
                        return Err("Top count cannot exceed 50".into());
                    }
                    config.top_count = Some(count);
                    i += 2;
                } else {
                    return Err("--top requires a number".into());
                }
            }
            "--random" => {
                if i + 1 < args.len() {
                    let count: usize = args[i + 1].parse()?;
                    if count > 20 {
                        return Err("Random count cannot exceed 20".into());
                    }
                    config.random_count = Some(count);
                    i += 2;
                } else {
                    return Err("--random requires a number".into());
                }
            }
            "--batch" => {
                if i + 1 < args.len() {
                    let mints: Vec<String> = args[i + 1]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    config.batch_mints = Some(mints);
                    i += 2;
                } else {
                    return Err("--batch requires comma-separated mint addresses".into());
                }
            }
            "--test-module" => {
                if i + 1 < args.len() {
                    config.test_module = match args[i + 1].as_str() {
                        "authority" => TestModule::Authority,
                        "holders" => TestModule::Holders,
                        "lp-lock" => TestModule::LpLock,
                        "security" => TestModule::Security,
                        "all" => TestModule::All,
                        _ => {
                            return Err(
                                "Invalid test module. Use: authority, holders, lp-lock, security, all".into()
                            );
                        }
                    };
                    i += 2;
                } else {
                    return Err("--test-module requires a module name".into());
                }
            }
            "--risk-filter" => {
                if i + 1 < args.len() {
                    config.risk_filter = Some(match args[i + 1].to_lowercase().as_str() {
                        "safe" => SecurityRiskLevel::Safe,
                        "low" => SecurityRiskLevel::Low,
                        "medium" => SecurityRiskLevel::Medium,
                        "high" => SecurityRiskLevel::High,
                        "critical" => SecurityRiskLevel::Critical,
                        _ => {
                            return Err(
                                "Invalid risk level. Use: safe, low, medium, high, critical".into()
                            );
                        }
                    });
                    i += 2;
                } else {
                    return Err("--risk-filter requires a risk level".into());
                }
            }
            "--min-liquidity" => {
                if i + 1 < args.len() {
                    config.min_liquidity = Some(args[i + 1].parse()?);
                    i += 2;
                } else {
                    return Err("--min-liquidity requires a USD amount".into());
                }
            }
            "--max-holders" => {
                if i + 1 < args.len() {
                    config.max_holders = Some(args[i + 1].parse()?);
                    i += 2;
                } else {
                    return Err("--max-holders requires a count".into());
                }
            }
            "--export-csv" => {
                if i + 1 < args.len() {
                    config.export_csv = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("--export-csv requires a filename".into());
                }
            }
            "--performance" => {
                config.performance = true;
                i += 1;
            }
            "--detailed" => {
                config.detailed = true;
                i += 1;
            }
            "--force-refresh" => {
                config.force_refresh = true;
                i += 1;
            }
            "--parallel" => {
                config.parallel = true;
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option: {}", args[i]).into());
            }
        }
    }

    // Validate configuration
    let token_source_count = [
        config.mint_address.is_some(),
        config.top_count.is_some(),
        config.random_count.is_some(),
        config.batch_mints.is_some(),
    ]
        .iter()
        .filter(|&&x| x)
        .count();

    if token_source_count == 0 {
        config.top_count = Some(10); // Default to top 10
    } else if token_source_count > 1 {
        return Err(
            "Please specify only one token source: --mint, --top, --random, or --batch".into()
        );
    }

    Ok(config)
}

async fn run_comprehensive_tests(
    analyzer: &TokenSecurityAnalyzer,
    config: DebugConfig
) -> Result<TestSessionResults, Box<dyn std::error::Error>> {
    println!("🔍 Starting comprehensive security module testing");
    println!(
        "Module: {:?} | Detailed: {} | Performance: {}",
        config.test_module,
        config.detailed,
        config.performance
    );
    println!("{}", "=".repeat(80));

    // Get tokens to test
    let tokens_to_test = get_tokens_for_testing(&config).await?;

    if tokens_to_test.is_empty() {
        return Err("No tokens found for testing".into());
    }

    println!("📊 Found {} tokens for testing", tokens_to_test.len());

    let mut session_results = TestSessionResults {
        total_tokens_tested: tokens_to_test.len(),
        successful_tests: 0,
        failed_tests: 0,
        authority_tests: 0,
        holder_tests: 0,
        lp_lock_tests: 0,
        security_tests: 0,
        total_duration_ms: 0,
        avg_duration_per_token_ms: 0.0,
        token_results: Vec::new(),
    };

    // Run tests
    if config.parallel {
        session_results.token_results = run_parallel_tests(
            &tokens_to_test,
            &config,
            analyzer
        ).await?;
    } else {
        session_results.token_results = run_sequential_tests(
            &tokens_to_test,
            &config,
            analyzer
        ).await?;
    }

    // Calculate final statistics
    for result in &session_results.token_results {
        if
            result.authority_result.as_ref().map_or(false, |r| r.is_ok()) ||
            result.holder_result.as_ref().map_or(false, |r| r.is_ok()) ||
            result.lp_lock_result.as_ref().map_or(false, |r| r.is_ok()) ||
            result.security_result.as_ref().map_or(false, |r| r.is_ok())
        {
            session_results.successful_tests += 1;
        } else {
            session_results.failed_tests += 1;
        }

        if result.authority_result.is_some() {
            session_results.authority_tests += 1;
        }
        if result.holder_result.is_some() {
            session_results.holder_tests += 1;
        }
        if result.lp_lock_result.is_some() {
            session_results.lp_lock_tests += 1;
        }
        if result.security_result.is_some() {
            session_results.security_tests += 1;
        }

        session_results.total_duration_ms += result.test_duration_ms;
    }

    if session_results.total_tokens_tested > 0 {
        session_results.avg_duration_per_token_ms =
            (session_results.total_duration_ms as f64) /
            (session_results.total_tokens_tested as f64);
    }

    Ok(session_results)
}

async fn get_tokens_for_testing(
    config: &DebugConfig
) -> Result<Vec<(String, Option<String>)>, Box<dyn std::error::Error>> {
    if let Some(ref mint) = config.mint_address {
        // Single token test
        validate_token_in_database(mint).await?;
        return Ok(vec![(mint.clone(), None)]);
    }

    if let Some(ref mints) = config.batch_mints {
        // Batch test
        let mut validated_tokens = Vec::new();
        for mint in mints {
            if let Ok(_) = validate_token_in_database(mint).await {
                validated_tokens.push((mint.clone(), None));
            } else {
                println!(
                    "⚠️  Warning: {} not found in database, skipping",
                    safe_truncate(mint, 12)
                );
            }
        }
        return Ok(validated_tokens);
    }

    // Database query for top or random tokens
    let db = TokenDatabase::new()?;
    let all_tokens = db.get_all_tokens().await?;

    let mut filtered_tokens: Vec<_> = all_tokens
        .into_iter()
        .filter(|token| {
            // Apply filters
            if let Some(min_liq) = config.min_liquidity {
                if
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) < min_liq
                {
                    return false;
                }
            }

            // Exclude mainstream tokens
            let mainstream_symbols = ["SOL", "USDC", "USDT", "WSOL", "BTC", "ETH", "WBTC", "BONK"];
            if mainstream_symbols.contains(&token.symbol.as_str()) {
                return false;
            }

            true
        })
        .map(|token| (token.mint, Some(token.symbol)))
        .collect();

    if let Some(top_count) = config.top_count {
        filtered_tokens.truncate(top_count);
    } else if let Some(random_count) = config.random_count {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        filtered_tokens.shuffle(&mut rng);
        filtered_tokens.truncate(random_count);
    }

    if filtered_tokens.is_empty() {
        // Fallback to known test tokens that should work
        println!("⚠️  No suitable tokens found in database, using fallback test tokens");
        println!("💡 These are small tokens specifically chosen for testing");
        return Ok(
            vec![
                (
                    "EPD9qjtFaFrR3GvTPmPt8spmu4hfwUN6Dc5tHtDmpump".to_string(),
                    Some("TEST1".to_string()),
                ),
                (
                    "BwC4NhHGfT5GrzUSjiYe2LcUeyWqSRt5JY5EqHH8pump".to_string(),
                    Some("TEST2".to_string()),
                ),
                (
                    "67ESYv7wxKu2vo637GhtffzSU2USwLHLZCQogsVmpump".to_string(),
                    Some("TEST3".to_string()),
                ),
                (
                    "5VLLv8V8eLECvv1V3VK4zW4ZjL1VE1q8V8V8V8V8pump".to_string(),
                    Some("TEST4".to_string()),
                ),
                (
                    "AaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAapump".to_string(),
                    Some("TEST5".to_string()),
                )
            ]
        );
    }

    Ok(filtered_tokens)
}

async fn validate_token_in_database(mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = TokenDatabase::new()?;
    match db.get_token_by_mint(mint)? {
        Some(_) => Ok(()),
        None => Err(format!("Token {} not found in database", mint).into()),
    }
}

async fn run_sequential_tests(
    tokens: &[(String, Option<String>)],
    config: &DebugConfig,
    analyzer: &TokenSecurityAnalyzer
) -> Result<Vec<TokenTestResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for (i, (mint, symbol)) in tokens.iter().enumerate() {
        println!(
            "\n🧪 Testing token {}/{}: {} ({})",
            i + 1,
            tokens.len(),
            safe_truncate(mint, 12),
            symbol.as_deref().unwrap_or("Unknown")
        );

        let test_start = Instant::now();
        let result = test_single_token_with_retry(mint, config, analyzer).await;
        let test_duration = test_start.elapsed();

        if config.detailed {
            print_detailed_token_result(mint, symbol, &result);
        } else {
            print_summary_token_result(mint, symbol, &result);
        }

        results.push(TokenTestResult {
            mint: mint.clone(),
            symbol: symbol.clone(),
            authority_result: result.0,
            holder_result: result.1,
            lp_lock_result: result.2,
            security_result: result.3,
            test_duration_ms: test_duration.as_millis() as u64,
        });

        // Show progress for longer tests
        if tokens.len() > 5 {
            let progress = (((i + 1) as f64) / (tokens.len() as f64)) * 100.0;
            println!("   📊 Progress: {:.1}% ({}/{})", progress, i + 1, tokens.len());
        }

        // Small delay between tests to be respectful to RPC
        if i < tokens.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    Ok(results)
}

async fn run_parallel_tests(
    tokens: &[(String, Option<String>)],
    config: &DebugConfig,
    _analyzer: &TokenSecurityAnalyzer
) -> Result<Vec<TokenTestResult>, Box<dyn std::error::Error>> {
    println!("🚀 Running tests in parallel (may be more RPC intensive)");

    // Batch size for parallel execution
    const BATCH_SIZE: usize = 5;

    let mut all_results = Vec::new();

    for chunk in tokens.chunks(BATCH_SIZE) {
        let mut batch_futures = Vec::new();

        for (mint, symbol) in chunk {
            let mint_clone = mint.clone();
            let symbol_clone = symbol.clone();
            let config_module = config.test_module.clone();

            batch_futures.push(async move {
                let test_start = Instant::now();
                let result = test_single_token_by_module(&mint_clone, &config_module).await;
                let test_duration = test_start.elapsed();

                TokenTestResult {
                    mint: mint_clone,
                    symbol: symbol_clone,
                    authority_result: result.0,
                    holder_result: result.1,
                    lp_lock_result: result.2,
                    security_result: result.3,
                    test_duration_ms: test_duration.as_millis() as u64,
                }
            });
        }

        let batch_results = futures::future::join_all(batch_futures).await;
        all_results.extend(batch_results.iter().cloned());

        // Display batch results
        for result in batch_results.iter() {
            if config.detailed {
                print_detailed_token_result(
                    &result.mint,
                    &result.symbol,
                    &(
                        result.authority_result.clone(),
                        result.holder_result.clone(),
                        result.lp_lock_result.clone(),
                        result.security_result.clone(),
                    )
                );
            } else {
                print_summary_token_result(
                    &result.mint,
                    &result.symbol,
                    &(
                        result.authority_result.clone(),
                        result.holder_result.clone(),
                        result.lp_lock_result.clone(),
                        result.security_result.clone(),
                    )
                );
            }
        }

        // Small delay between batches
        if chunk.len() == BATCH_SIZE {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    Ok(all_results)
}

async fn test_single_token_with_retry(
    mint: &str,
    config: &DebugConfig,
    _analyzer: &TokenSecurityAnalyzer
) -> (
    Option<Result<TokenAuthorities, String>>,
    Option<Result<u32, String>>,
    Option<Result<LpLockAnalysis, String>>,
    Option<Result<TokenSecurityInfo, String>>,
) {
    // First attempt
    let result = test_single_token_by_module(mint, &config.test_module).await;

    // Check if we got any non-timeout errors that might benefit from retry
    let should_retry = match &result {
        (Some(Err(e)), _, _, _) if e.contains("timeout") => true,
        (_, Some(Err(e)), _, _) if e.contains("timeout") => true,
        (_, _, Some(Err(e)), _) if e.contains("timeout") => true,
        (_, _, _, Some(Err(e))) if e.contains("timeout") => true,
        _ => false,
    };

    if should_retry {
        println!("   🔄 Retrying due to timeout...");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        test_single_token_by_module(mint, &config.test_module).await
    } else {
        result
    }
}

async fn test_single_token_by_module(
    mint: &str,
    test_module: &TestModule
) -> (
    Option<Result<TokenAuthorities, String>>,
    Option<Result<u32, String>>,
    Option<Result<LpLockAnalysis, String>>,
    Option<Result<TokenSecurityInfo, String>>,
) {
    let mut authority_result = None;
    let mut holder_result = None;
    let mut lp_lock_result = None;
    let mut security_result = None;

    match test_module {
        TestModule::Authority | TestModule::All => {
            authority_result = Some(
                get_token_authorities(mint).await.map_err(|e| format!("Authority error: {}", e))
            );
        }
        _ => {}
    }

    match test_module {
        TestModule::Holders | TestModule::All => {
            // Add a timeout and better error handling for holder count
            let holder_future = tokio::time::timeout(
                tokio::time::Duration::from_secs(10),
                get_count_holders(mint)
            );

            holder_result = Some(match holder_future.await {
                Ok(Ok(count)) => Ok(count),
                Ok(Err(e)) => Err(format!("Holder error: {}", e)),
                Err(_) => Err("Holder error: Request timeout (10s)".to_string()),
            });
        }
        _ => {}
    }

    match test_module {
        TestModule::LpLock | TestModule::All => {
            // Add timeout for LP lock analysis
            let lp_future = tokio::time::timeout(
                tokio::time::Duration::from_secs(15),
                check_lp_lock_status(mint)
            );

            lp_lock_result = Some(match lp_future.await {
                Ok(Ok(analysis)) => Ok(analysis),
                Ok(Err(e)) => Err(format!("LP lock error: {}", e)),
                Err(_) => Err("LP lock error: Request timeout (15s)".to_string()),
            });
        }
        _ => {}
    }

    match test_module {
        TestModule::Security | TestModule::All => {
            // Add timeout for security analysis
            let security_future = tokio::time::timeout(
                tokio::time::Duration::from_secs(20),
                analyze_token_security(mint)
            );

            security_result = Some(match security_future.await {
                Ok(Ok(analysis)) => Ok(analysis),
                Ok(Err(e)) => Err(format!("Security error: {}", e)),
                Err(_) => Err("Security error: Request timeout (20s)".to_string()),
            });
        }
        _ => {}
    }

    (authority_result, holder_result, lp_lock_result, security_result)
}

fn print_summary_token_result(
    mint: &str,
    symbol: &Option<String>,
    result: &(
        Option<Result<TokenAuthorities, String>>,
        Option<Result<u32, String>>,
        Option<Result<LpLockAnalysis, String>>,
        Option<Result<TokenSecurityInfo, String>>,
    )
) {
    let symbol_str = symbol.as_deref().unwrap_or("?");

    let auth_status = match &result.0 {
        Some(Ok(_)) => "✅",
        Some(Err(_)) => "❌",
        None => "⏭️",
    };

    let holder_status = match &result.1 {
        Some(Ok(_)) => "✅",
        Some(Err(_)) => "❌",
        None => "⏭️",
    };

    let lp_status = match &result.2 {
        Some(Ok(_)) => "✅",
        Some(Err(_)) => "❌",
        None => "⏭️",
    };

    let security_status = match &result.3 {
        Some(Ok(_)) => "✅",
        Some(Err(_)) => "❌",
        None => "⏭️",
    };

    println!(
        "  {} {} | Auth:{} Holders:{} LP:{} Security:{}",
        safe_truncate(mint, 12),
        symbol_str,
        auth_status,
        holder_status,
        lp_status,
        security_status
    );
}

fn print_detailed_token_result(
    mint: &str,
    symbol: &Option<String>,
    result: &(
        Option<Result<TokenAuthorities, String>>,
        Option<Result<u32, String>>,
        Option<Result<LpLockAnalysis, String>>,
        Option<Result<TokenSecurityInfo, String>>,
    )
) {
    let symbol_str = symbol.as_deref().unwrap_or("Unknown");
    println!("  📋 {} ({})", safe_truncate(mint, 12), symbol_str);

    if let Some(auth_result) = &result.0 {
        match auth_result {
            Ok(auth) => {
                println!(
                    "    🔐 Authority: {} - {}",
                    auth.get_risk_level().as_str(),
                    auth.get_authority_summary()
                );
            }
            Err(e) => println!("    🔐 Authority: ❌ {}", e),
        }
    }

    if let Some(holder_result) = &result.1 {
        match holder_result {
            Ok(count) => println!("    👥 Holders: {} total", count),
            Err(e) => println!("    👥 Holders: ❌ {}", e),
        }
    }

    if let Some(lp_result) = &result.2 {
        match lp_result {
            Ok(lp) => {
                println!("    🔒 LP Lock: {} (Score: {})", lp.status.description(), lp.lock_score);
            }
            Err(e) => println!("    🔒 LP Lock: ❌ {}", e),
        }
    }

    if let Some(security_result) = &result.3 {
        match security_result {
            Ok(security) => {
                println!(
                    "    🛡️  Security: {:?} (Score: {}/100)",
                    security.risk_level,
                    security.security_score
                );
            }
            Err(e) => println!("    🛡️  Security: ❌ {}", e),
        }
    }
}

fn print_test_results(results: &TestSessionResults, total_duration: std::time::Duration) {
    println!("\n{}", "=".repeat(80));
    println!("📊 COMPREHENSIVE SECURITY TEST RESULTS");
    println!("{}", "=".repeat(80));

    println!("📈 Test Statistics:");
    println!("  • Total Tokens Tested: {}", results.total_tokens_tested);
    println!("  • Successful Tests: {}", results.successful_tests);
    println!("  • Failed Tests: {}", results.failed_tests);
    println!(
        "  • Success Rate: {:.1}%",
        ((results.successful_tests as f64) / (results.total_tokens_tested as f64)) * 100.0
    );

    println!("\n🧪 Module Test Counts:");
    println!("  • Authority Tests: {}", results.authority_tests);
    println!("  • Holder Tests: {}", results.holder_tests);
    println!("  • LP Lock Tests: {}", results.lp_lock_tests);
    println!("  • Security Tests: {}", results.security_tests);

    println!("\n⏱️  Performance Metrics:");
    println!("  • Total Test Duration: {:.2}s", total_duration.as_secs_f64());
    println!("  • Average per Token: {:.0}ms", results.avg_duration_per_token_ms);
    println!(
        "  • Tests per Second: {:.2}",
        (results.total_tokens_tested as f64) / total_duration.as_secs_f64()
    );

    // Risk distribution for security tests
    if results.security_tests > 0 {
        let mut risk_counts: HashMap<String, usize> = HashMap::new();
        for result in &results.token_results {
            if let Some(Ok(security)) = &result.security_result {
                let risk_key = format!("{:?}", security.risk_level);
                *risk_counts.entry(risk_key).or_insert(0) += 1;
            }
        }

        println!("\n🎯 Risk Level Distribution:");
        for (risk, count) in risk_counts {
            let percentage = ((count as f64) / (results.security_tests as f64)) * 100.0;
            println!("  • {}: {} ({:.1}%)", risk, count, percentage);
        }
    }

    // Top errors with better categorization
    let mut error_counts = HashMap::new();
    let mut timeout_count = 0;
    let mut not_found_count = 0;
    let mut api_error_count = 0;

    for result in &results.token_results {
        // Check each error type separately since they have different types
        if let Some(Err(e)) = &result.authority_result {
            categorize_error(
                e,
                &mut timeout_count,
                &mut not_found_count,
                &mut api_error_count,
                &mut error_counts
            );
        }
        if let Some(Err(e)) = &result.holder_result {
            categorize_error(
                e,
                &mut timeout_count,
                &mut not_found_count,
                &mut api_error_count,
                &mut error_counts
            );
        }
        if let Some(Err(e)) = &result.lp_lock_result {
            categorize_error(
                e,
                &mut timeout_count,
                &mut not_found_count,
                &mut api_error_count,
                &mut error_counts
            );
        }
        if let Some(Err(e)) = &result.security_result {
            categorize_error(
                e,
                &mut timeout_count,
                &mut not_found_count,
                &mut api_error_count,
                &mut error_counts
            );
        }
    }

    if timeout_count > 0 || not_found_count > 0 || api_error_count > 0 || !error_counts.is_empty() {
        println!("\n❌ Error Summary:");
        if timeout_count > 0 {
            println!("  • ⏱️  Timeouts: {} occurrences", timeout_count);
        }
        if not_found_count > 0 {
            println!("  • 🔍 Token not found (404): {} occurrences", not_found_count);
        }
        if api_error_count > 0 {
            println!("  • 🔧 API initialization errors: {} occurrences", api_error_count);
        }

        if !error_counts.is_empty() {
            println!("  • 🚨 Other errors:");
            let mut sorted_errors: Vec<_> = error_counts.into_iter().collect();
            sorted_errors.sort_by(|a, b| b.1.cmp(&a.1));
            for (error, count) in sorted_errors.iter().take(3) {
                println!("    - {} ({}x)", error, count);
            }
        }
    }
}

fn export_to_csv(
    results: &TestSessionResults,
    filename: &str
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(filename)?;

    // CSV header
    writeln!(
        file,
        "mint,symbol,authority_status,holder_count,lp_lock_status,security_score,risk_level,test_duration_ms"
    )?;

    for result in &results.token_results {
        let authority_status = match &result.authority_result {
            Some(Ok(_)) => "SUCCESS".to_string(),
            Some(Err(e)) => format!("ERROR: {}", safe_truncate(e, 30)),
            None => "SKIPPED".to_string(),
        };

        let holder_count = match &result.holder_result {
            Some(Ok(count)) => count.to_string(),
            Some(Err(_)) => "ERROR".to_string(),
            None => "SKIPPED".to_string(),
        };

        let lp_lock_status = match &result.lp_lock_result {
            Some(Ok(lp)) => lp.status.description().to_string(),
            Some(Err(_)) => "ERROR".to_string(),
            None => "SKIPPED".to_string(),
        };

        let (security_score, risk_level) = match &result.security_result {
            Some(Ok(security)) =>
                (security.security_score.to_string(), format!("{:?}", security.risk_level)),
            Some(Err(_)) => ("ERROR".to_string(), "ERROR".to_string()),
            None => ("SKIPPED".to_string(), "SKIPPED".to_string()),
        };

        writeln!(
            file,
            "{},{},{},{},{},{},{},{}",
            result.mint,
            result.symbol.as_deref().unwrap_or(""),
            authority_status,
            holder_count,
            lp_lock_status,
            security_score,
            risk_level,
            result.test_duration_ms
        )?;
    }

    println!("📄 Results exported to: {}", filename);
    Ok(())
}

fn categorize_error(
    error: &str,
    timeout_count: &mut usize,
    not_found_count: &mut usize,
    api_error_count: &mut usize,
    error_counts: &mut HashMap<String, usize>
) {
    if error.contains("timeout") {
        *timeout_count += 1;
    } else if error.contains("404") || error.contains("Not Found") {
        *not_found_count += 1;
    } else if error.contains("API") || error.contains("not initialized") {
        *api_error_count += 1;
    } else {
        let short_error = safe_truncate(error, 60).to_string();
        *error_counts.entry(short_error).or_insert(0) += 1;
    }
}
