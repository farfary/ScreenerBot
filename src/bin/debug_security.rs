/// Token Security Analysis Debug Tool
///
/// This tool provides comprehensive testing and debugging capabilities for the token security
/// analysis module. It can test individual tokens, batch operations, caching behavior,
/// and database operations.

use screenerbot::{
    errors::ScreenerBotError,
    global::init_global_config,
    logger::{init_logger, log, LogTag},
    rpc::init_rpc_client,
    tokens::security::{
        analyze_multiple_tokens_security, analyze_token_security, get_security_analyzer, 
        get_security_summary, get_token_risk_level, init_security_analyzer, SecurityRiskLevel,
        TokenSecurityInfo, UpdateStrategy
    },
};

use chrono::Utc;
use std::collections::HashMap;
use std::env;
use tokio;

/// Test configuration
struct TestConfig {
    /// Test individual token analysis
    test_individual: bool,
    /// Test batch analysis
    test_batch: bool,
    /// Test caching behavior
    test_cache: bool,
    /// Test database operations
    test_database: bool,
    /// Test performance
    test_performance: bool,
    /// Tokens to test with
    test_tokens: Vec<String>,
    /// Enable detailed output
    verbose: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_individual: true,
            test_batch: true,
            test_cache: true,
            test_database: true,
            test_performance: false,
            test_tokens: vec![
                // Known tokens for testing (replace with actual Solana token mints)
                "So11111111111111111111111111111111111111112".to_string(), // Wrapped SOL
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
            ],
            verbose: false,
        }
    }
}

#[tokio::main]
async fn main() {
    println!("🔒 Token Security Analysis Debug Tool");
    println!("=====================================");

    // Parse command line arguments
    let config = parse_args();
    
    // Initialize system
    if let Err(e) = init_system().await {
        eprintln!("❌ Failed to initialize system: {}", e);
        std::process::exit(1);
    }

    // Initialize security analyzer
    if let Err(e) = init_security_analyzer() {
        eprintln!("❌ Failed to initialize security analyzer: {}", e);
        std::process::exit(1);
    }

    println!("✅ System initialized successfully");
    println!();

    // Run tests based on configuration
    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;

    if config.test_individual {
        println!("🧪 Testing Individual Token Analysis");
        println!("====================================");
        let (total, passed, failed) = test_individual_analysis(&config).await;
        total_tests += total;
        passed_tests += passed;
        failed_tests += failed;
        println!();
    }

    if config.test_batch {
        println!("🧪 Testing Batch Token Analysis");
        println!("===============================");
        let (total, passed, failed) = test_batch_analysis(&config).await;
        total_tests += total;
        passed_tests += passed;
        failed_tests += failed;
        println!();
    }

    if config.test_cache {
        println!("🧪 Testing Cache Behavior");
        println!("=========================");
        let (total, passed, failed) = test_cache_behavior(&config).await;
        total_tests += total;
        passed_tests += passed;
        failed_tests += failed;
        println!();
    }

    if config.test_database {
        println!("🧪 Testing Database Operations");
        println!("==============================");
        let (total, passed, failed) = test_database_operations(&config).await;
        total_tests += total;
        passed_tests += passed;
        failed_tests += failed;
        println!();
    }

    if config.test_performance {
        println!("🧪 Testing Performance");
        println!("======================");
        let (total, passed, failed) = test_performance(&config).await;
        total_tests += total;
        passed_tests += passed;
        failed_tests += failed;
        println!();
    }

    // Summary
    println!("📊 Test Summary");
    println!("===============");
    println!("Total Tests: {}", total_tests);
    println!("✅ Passed: {}", passed_tests);
    println!("❌ Failed: {}", failed_tests);
    println!("Success Rate: {:.1}%", 
        if total_tests > 0 { (passed_tests as f64 / total_tests as f64) * 100.0 } else { 0.0 });

    if failed_tests > 0 {
        std::process::exit(1);
    }
}

/// Parse command line arguments
fn parse_args() -> TestConfig {
    let args: Vec<String> = env::args().collect();
    let mut config = TestConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--individual" => config.test_individual = true,
            "--batch" => config.test_batch = true,
            "--cache" => config.test_cache = true,
            "--database" => config.test_database = true,
            "--performance" => config.test_performance = true,
            "--all" => {
                config.test_individual = true;
                config.test_batch = true;
                config.test_cache = true;
                config.test_database = true;
                config.test_performance = true;
            },
            "--verbose" | "-v" => config.verbose = true,
            "--token" => {
                if i + 1 < args.len() {
                    config.test_tokens.push(args[i + 1].clone());
                    i += 1;
                }
            },
            "--tokens" => {
                if i + 1 < args.len() {
                    let tokens: Vec<String> = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                    config.test_tokens.extend(tokens);
                    i += 1;
                }
            },
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            },
            _ => {
                if args[i].starts_with("--") {
                    eprintln!("Unknown option: {}", args[i]);
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    // If no test types specified, enable all
    if !config.test_individual && !config.test_batch && !config.test_cache && 
       !config.test_database && !config.test_performance {
        config.test_individual = true;
        config.test_batch = true;
        config.test_cache = true;
        config.test_database = true;
    }

    config
}

/// Print help message
fn print_help() {
    println!("Token Security Analysis Debug Tool");
    println!();
    println!("Usage: debug_security [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --individual       Test individual token analysis");
    println!("  --batch           Test batch token analysis");
    println!("  --cache           Test caching behavior");
    println!("  --database        Test database operations");
    println!("  --performance     Test performance metrics");
    println!("  --all             Run all tests");
    println!("  --verbose, -v     Enable verbose output");
    println!("  --token <MINT>    Add specific token to test");
    println!("  --tokens <LIST>   Add comma-separated list of tokens");
    println!("  --help, -h        Show this help message");
    println!();
    println!("Examples:");
    println!("  debug_security --all --verbose");
    println!("  debug_security --individual --token So11111111111111111111111111111111111111112");
    println!("  debug_security --batch --tokens \"MINT1,MINT2,MINT3\"");
}

/// Initialize system components
async fn init_system() -> Result<(), ScreenerBotError> {
    // Initialize logger
    init_logger()?;
    
    // Initialize global config
    init_global_config()?;
    
    // Initialize RPC client
    init_rpc_client()?;
    
    Ok(())
}

/// Test individual token analysis
async fn test_individual_analysis(config: &TestConfig) -> (u32, u32, u32) {
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    for token in &config.test_tokens {
        total += 1;
        println!("📄 Testing individual analysis for {}", token);
        
        let start_time = std::time::Instant::now();
        
        match analyze_token_security(token).await {
            Ok(security_info) => {
                passed += 1;
                let duration = start_time.elapsed();
                
                println!("  ✅ Analysis completed in {:?}", duration);
                print_security_info(&security_info, config.verbose);
                
                // Test convenience functions
                if let Ok(risk_level) = get_token_risk_level(token).await {
                    println!("  🎯 Risk Level: {} {}", risk_level.color_emoji(), risk_level.as_str());
                }
                
                if let Ok(summary) = get_security_summary(token).await {
                    println!("  📋 Summary: {}", summary);
                }
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ Analysis failed: {}", e);
            }
        }
        println!();
    }

    (total, passed, failed)
}

/// Test batch token analysis
async fn test_batch_analysis(config: &TestConfig) -> (u32, u32, u32) {
    let mut total = 1;
    let mut passed = 0;
    let mut failed = 0;

    println!("📦 Testing batch analysis for {} tokens", config.test_tokens.len());
    
    let start_time = std::time::Instant::now();
    
    match analyze_multiple_tokens_security(&config.test_tokens).await {
        Ok(results) => {
            passed += 1;
            let duration = start_time.elapsed();
            
            println!("  ✅ Batch analysis completed in {:?}", duration);
            println!("  📊 Results: {}/{} tokens analyzed", results.len(), config.test_tokens.len());
            
            if config.verbose {
                for (mint, security_info) in &results {
                    println!("    🔗 {}: {} {} (Score: {})", 
                        mint, 
                        security_info.risk_level.color_emoji(),
                        security_info.risk_level.as_str(),
                        security_info.security_score
                    );
                }
            }
            
            // Test update strategies
            let strategies: HashMap<String, u32> = results.values()
                .map(|info| match info.update_strategy {
                    UpdateStrategy::Full => "Full",
                    UpdateStrategy::DynamicOnly => "Dynamic",
                    UpdateStrategy::StaticOnly => "Static",
                    UpdateStrategy::Cached => "Cached",
                })
                .fold(HashMap::new(), |mut acc, strategy| {
                    *acc.entry(strategy.to_string()).or_insert(0) += 1;
                    acc
                });
            
            println!("  📈 Update Strategies: {:?}", strategies);
        },
        Err(e) => {
            failed += 1;
            println!("  ❌ Batch analysis failed: {}", e);
        }
    }

    (total, passed, failed)
}

/// Test cache behavior
async fn test_cache_behavior(config: &TestConfig) -> (u32, u32, u32) {
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    if let Some(token) = config.test_tokens.first() {
        // Test 1: First call (should miss cache)
        total += 1;
        println!("🗄️  Test 1: First call (cache miss expected)");
        
        let start_time = std::time::Instant::now();
        match analyze_token_security(token).await {
            Ok(_) => {
                passed += 1;
                let duration = start_time.elapsed();
                println!("  ✅ First call completed in {:?}", duration);
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ First call failed: {}", e);
            }
        }

        // Test 2: Second call (should hit cache)
        total += 1;
        println!("🗄️  Test 2: Second call (cache hit expected)");
        
        let start_time = std::time::Instant::now();
        match analyze_token_security(token).await {
            Ok(_) => {
                passed += 1;
                let duration = start_time.elapsed();
                println!("  ✅ Second call completed in {:?} (should be faster)", duration);
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ Second call failed: {}", e);
            }
        }

        // Test 3: Cache statistics
        total += 1;
        let analyzer = get_security_analyzer();
        let (total_cached, static_count, dynamic_count) = analyzer.cache.stats();
        
        if total_cached > 0 {
            passed += 1;
            println!("  ✅ Cache contains {} items ({} static, {} dynamic)", 
                total_cached, static_count, dynamic_count);
        } else {
            failed += 1;
            println!("  ❌ Cache is empty (unexpected)");
        }
    }

    (total, passed, failed)
}

/// Test database operations
async fn test_database_operations(config: &TestConfig) -> (u32, u32, u32) {
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    if let Some(token) = config.test_tokens.first() {
        let analyzer = get_security_analyzer();

        // Test 1: Database read/write
        total += 1;
        println!("💾 Test 1: Database read/write operations");
        
        match analyzer.analyze_token_security(token).await {
            Ok(security_info) => {
                // Check if it was saved to database
                match analyzer.database.get_security_info(token) {
                    Ok(Some(db_info)) => {
                        passed += 1;
                        println!("  ✅ Data successfully saved and retrieved from database");
                        
                        if config.verbose {
                            println!("    📅 First analyzed: {}", db_info.timestamps.first_analyzed);
                            println!("    📅 Last updated: {}", db_info.timestamps.last_updated);
                        }
                    },
                    Ok(None) => {
                        failed += 1;
                        println!("  ❌ Data not found in database");
                    },
                    Err(e) => {
                        failed += 1;
                        println!("  ❌ Database read failed: {}", e);
                    }
                }
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ Analysis failed: {}", e);
            }
        }

        // Test 2: Batch database operations
        total += 1;
        println!("💾 Test 2: Batch database operations");
        
        match analyzer.database.get_multiple_security_infos(&config.test_tokens) {
            Ok(results) => {
                passed += 1;
                println!("  ✅ Batch database read successful: {} records", results.len());
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ Batch database read failed: {}", e);
            }
        }

        // Test 3: Database cleanup (dry run)
        total += 1;
        println!("💾 Test 3: Database cleanup simulation");
        
        match analyzer.database.cleanup_old_data(365) { // 1 year
            Ok(deleted) => {
                passed += 1;
                println!("  ✅ Cleanup simulation: {} records would be deleted", deleted);
            },
            Err(e) => {
                failed += 1;
                println!("  ❌ Cleanup simulation failed: {}", e);
            }
        }
    }

    (total, passed, failed)
}

/// Test performance metrics
async fn test_performance(config: &TestConfig) -> (u32, u32, u32) {
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    // Test 1: Individual analysis performance
    total += 1;
    println!("⚡ Test 1: Individual analysis performance");
    
    let mut individual_times = Vec::new();
    for token in &config.test_tokens {
        let start_time = std::time::Instant::now();
        if let Ok(_) = analyze_token_security(token).await {
            individual_times.push(start_time.elapsed());
        }
    }
    
    if !individual_times.is_empty() {
        passed += 1;
        let avg_time = individual_times.iter().sum::<std::time::Duration>() / individual_times.len() as u32;
        let min_time = individual_times.iter().min().unwrap();
        let max_time = individual_times.iter().max().unwrap();
        
        println!("  ✅ Individual analysis performance:");
        println!("    📊 Average: {:?}", avg_time);
        println!("    ⚡ Fastest: {:?}", min_time);
        println!("    🐌 Slowest: {:?}", max_time);
    } else {
        failed += 1;
        println!("  ❌ No successful individual analyses");
    }

    // Test 2: Batch analysis performance
    total += 1;
    println!("⚡ Test 2: Batch analysis performance");
    
    let start_time = std::time::Instant::now();
    match analyze_multiple_tokens_security(&config.test_tokens).await {
        Ok(results) => {
            passed += 1;
            let duration = start_time.elapsed();
            let tokens_per_second = results.len() as f64 / duration.as_secs_f64();
            
            println!("  ✅ Batch analysis performance:");
            println!("    ⏱️  Total time: {:?}", duration);
            println!("    📊 Tokens/second: {:.2}", tokens_per_second);
            println!("    🎯 Results: {}/{} tokens", results.len(), config.test_tokens.len());
        },
        Err(e) => {
            failed += 1;
            println!("  ❌ Batch analysis failed: {}", e);
        }
    }

    // Test 3: Cache performance
    total += 1;
    println!("⚡ Test 3: Cache performance");
    
    if let Some(token) = config.test_tokens.first() {
        // Warm up cache
        let _ = analyze_token_security(token).await;
        
        // Measure cache hit performance
        let mut cache_times = Vec::new();
        for _ in 0..5 {
            let start_time = std::time::Instant::now();
            if let Ok(_) = analyze_token_security(token).await {
                cache_times.push(start_time.elapsed());
            }
        }
        
        if !cache_times.is_empty() {
            passed += 1;
            let avg_cache_time = cache_times.iter().sum::<std::time::Duration>() / cache_times.len() as u32;
            
            println!("  ✅ Cache performance:");
            println!("    ⚡ Average cache hit: {:?}", avg_cache_time);
            
            if let Some(individual_avg) = individual_times.first() {
                let speedup = individual_avg.as_nanos() as f64 / avg_cache_time.as_nanos() as f64;
                println!("    🚀 Cache speedup: {:.1}x", speedup);
            }
        } else {
            failed += 1;
            println!("  ❌ Cache performance test failed");
        }
    } else {
        failed += 1;
        println!("  ❌ No tokens available for cache test");
    }

    (total, passed, failed)
}

/// Print detailed security information
fn print_security_info(info: &TokenSecurityInfo, verbose: bool) {
    println!("  🔒 Security Analysis for {}", info.mint);
    println!("    🎯 Risk Level: {} {} (Score: {})", 
        info.risk_level.color_emoji(), info.risk_level.as_str(), info.security_score);
    
    // Authority information
    println!("    👑 Authority Info:");
    println!("      🔸 Mint: {}", if info.authority_info.is_mint_disabled() { "🔒 DISABLED" } else { "⚠️  ENABLED" });
    println!("      🔸 Freeze: {}", if info.authority_info.is_freeze_disabled() { "🔒 DISABLED" } else { "⚠️  ENABLED" });
    println!("      🔸 Update: {}", if info.authority_info.is_update_disabled() { "🔒 DISABLED" } else { "⚠️  ENABLED" });
    
    // LP lock information
    if let Some(ref lp_info) = info.lp_lock_info {
        println!("    💧 LP Lock: {} {}", 
            if lp_info.status.is_safe() { "🔒" } else { "⚠️" },
            lp_info.status.description());
    } else {
        println!("    💧 LP Lock: ❓ Unknown");
    }
    
    // Holder information
    if let Some(ref holder_info) = info.holder_info {
        println!("    👥 Holders: {} total", holder_info.total_holders);
        println!("      🔸 Top 10 concentration: {:.1}%", holder_info.top_10_concentration);
        println!("      🔸 Top 5 concentration: {:.1}%", holder_info.top_5_concentration);
        println!("      🔸 Largest holder: {:.1}%", holder_info.largest_holder_percentage);
        println!("      🔸 Whales (>5%): {}", holder_info.whale_count);
        println!("      🔸 Distribution score: {}/100", holder_info.distribution_score);
    } else {
        println!("    👥 Holders: ❓ Unknown");
    }
    
    // Security flags
    println!("    🚩 Security Flags:");
    println!("      🔸 Can mint: {}", if info.security_flags.can_mint { "⚠️  YES" } else { "✅ NO" });
    println!("      🔸 Can freeze: {}", if info.security_flags.can_freeze { "⚠️  YES" } else { "✅ NO" });
    println!("      🔸 High concentration: {}", if info.security_flags.high_concentration { "⚠️  YES" } else { "✅ NO" });
    println!("      🔸 Few holders: {}", if info.security_flags.few_holders { "⚠️  YES" } else { "✅ NO" });
    println!("      🔸 Whale risk: {}", if info.security_flags.whale_risk { "⚠️  YES" } else { "✅ NO" });
    
    if verbose {
        println!("    📅 Timestamps:");
        println!("      🔸 First analyzed: {}", info.timestamps.first_analyzed.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("      🔸 Last updated: {}", info.timestamps.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("      🔸 Authority checked: {}", info.timestamps.authority_last_checked.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(holder_checked) = info.timestamps.holder_last_checked {
            println!("      🔸 Holders checked: {}", holder_checked.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        if let Some(lp_checked) = info.timestamps.lp_lock_last_checked {
            println!("      🔸 LP lock checked: {}", lp_checked.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }
}
