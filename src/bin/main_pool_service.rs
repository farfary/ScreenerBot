use screenerbot::{
    arguments::{ get_cmd_args, set_cmd_args, is_debug_pool_monitor_enabled },
    logger::{ init_file_logging, log, LogTag },
    pool_service::{ init_pool_service },
    pool_monitor::{ get_pool_monitor_stats },
    pool_tokens::{ get_pool_tokens_stats },
    pool_interface::PoolInterface,
};
use std::time::Duration;
use tokio::time::sleep;

/// Pool Service Management Tool
/// 
/// This tool provides a command-line interface for managing the pool service:
/// - Initialize and start the pool service
/// - Monitor service health and statistics
/// - Display task statuses and performance metrics
/// - Test pool service functionality
/// 
/// Usage:
///   cargo run --bin main_pool_service -- --help
///   cargo run --bin main_pool_service -- --start
///   cargo run --bin main_pool_service -- --monitor
///   cargo run --bin main_pool_service -- --stats
///   cargo run --bin main_pool_service -- --test
///   cargo run --bin main_pool_service -- --debug-pool-service
///   cargo run --bin main_pool_service -- --debug-pool-monitor

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize file logging first
    init_file_logging();
    
    // Get command line arguments
    let args = get_cmd_args();
    
    // Set command line arguments for global access
    set_cmd_args(args.clone());
    
    log(LogTag::Pool, "TOOL_START", "🚀 Starting Pool Service Management Tool");
    
    // Check for help flag
    if args.contains(&"--help".to_string()) {
        print_help();
        return Ok(());
    }
    
    // Initialize pool service
    let pool_service = init_pool_service();
    log(LogTag::Pool, "SERVICE_INIT", "✅ Pool service initialized");
    
    // Handle different command modes
    if args.contains(&"--start".to_string()) {
        start_pool_service(pool_service).await?;
    } else if args.contains(&"--monitor".to_string()) {
        monitor_pool_service(pool_service).await?;
    } else if args.contains(&"--stats".to_string()) {
        show_pool_service_stats(pool_service).await?;
    } else if args.contains(&"--test".to_string()) {
        test_pool_service(pool_service).await?;
    } else if args.contains(&"--status".to_string()) {
        show_task_statuses(pool_service).await?;
    } else if args.contains(&"--debug-tokens".to_string()) {
        debug_pool_tokens(pool_service).await?;
    } else {
        // Default: start and monitor
        start_and_monitor_pool_service(pool_service).await?;
    }
    
    log(LogTag::Pool, "TOOL_END", "🛑 Pool Service Management Tool finished");
    Ok(())
}

/// Print help information
fn print_help() {
    println!("Pool Service Management Tool");
    println!();
    println!("Usage: cargo run --bin main_pool_service -- [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --help              Show this help message");
    println!("  --start             Start the pool service and exit");
    println!("  --monitor           Monitor pool service health continuously");
    println!("  --stats             Show pool service statistics and exit");
    println!("  --test              Test pool service functionality");
    println!("  --status            Show task statuses and exit");
    println!("  --debug-tokens      Debug pool tokens loading and database access");
    println!("  --debug-pool-service    Enable pool service debug logging");
    println!("  --debug-pool-monitor    Enable pool monitor debug logging");
    println!("  --debug-pool-tokens     Enable pool tokens debug logging");
    println!();
    println!("Examples:");
    println!("  cargo run --bin main_pool_service -- --start");
    println!("  cargo run --bin main_pool_service -- --monitor");
    println!("  cargo run --bin main_pool_service -- --stats --debug-pool-service");
    println!();
    println!("Default behavior: Start service and monitor for 60 seconds");
}

/// Start the pool service
async fn start_pool_service(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "SERVICE_START", "🚀 Starting pool service...");
    
    pool_service.start().await?;
    
    // Wait a moment for service to initialize
    sleep(Duration::from_secs(2)).await;
    
    // Check if service is running
    if pool_service.is_running().await {
        log(LogTag::Pool, "SERVICE_RUNNING", "✅ Pool service is running successfully");
        
        // Show initial statistics
        let stats = pool_service.get_stats().await;
        log(
            LogTag::Pool,
            "SERVICE_STATS",
            &format!(
                "Service stats - Available tokens: {}, Cache hits: {}, Price fetches: {}",
                stats.total_tokens_available,
                stats.cache_hits,
                stats.successful_price_fetches
            )
        );
    } else {
        log(LogTag::Pool, "SERVICE_ERROR", "❌ Pool service failed to start");
        return Err("Pool service failed to start".into());
    }
    
    Ok(())
}

/// Monitor pool service health continuously
async fn monitor_pool_service(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "MONITOR_START", "🔍 Starting continuous monitoring...");
    
    // Start the service first
    pool_service.start().await?;
    sleep(Duration::from_secs(3)).await;
    
    let mut cycle_count = 0;
    let max_cycles = 120; // Monitor for 10 minutes (120 * 5 seconds)
    
    loop {
        cycle_count += 1;
        
        // Get service statistics
        let stats = pool_service.get_stats().await;
        let task_statuses = pool_service.get_task_statuses().await;
        
        // Get monitor statistics
        let monitor_stats = get_pool_monitor_stats().await;
        
        // Log service status
        log(
            LogTag::Pool,
            "MONITOR_CYCLE",
            &format!(
                "Cycle {} - Service running: {}, Available tokens: {}, Cache hits: {}, Health checks: {}",
                cycle_count,
                pool_service.is_running().await,
                stats.total_tokens_available,
                stats.cache_hits,
                monitor_stats.health_checks_performed
            )
        );
        
        // Log task statuses
        let mut running_tasks = 0;
        let mut error_tasks = 0;
        
        for (name, status) in &task_statuses {
            match &status.state {
                screenerbot::pool_monitor::TaskState::Running => {
                    running_tasks += 1;
                    if is_debug_pool_monitor_enabled() {
                        log(
                            LogTag::Pool,
                            "TASK_STATUS",
                            &format!("Task {}: Running (runs: {}, errors: {})", name, status.run_count, status.error_count)
                        );
                    }
                }
                screenerbot::pool_monitor::TaskState::Error(e) => {
                    error_tasks += 1;
                    log(
                        LogTag::Pool,
                        "TASK_ERROR",
                        &format!("Task {}: Error - {}", name, e)
                    );
                }
                _ => {
                    if is_debug_pool_monitor_enabled() {
                        log(
                            LogTag::Pool,
                            "TASK_STATUS",
                            &format!("Task {}: {:?}", name, status.state)
                        );
                    }
                }
            }
        }
        
        log(
            LogTag::Pool,
            "TASK_SUMMARY",
            &format!("Tasks - Running: {}, Errors: {}, Total: {}", running_tasks, error_tasks, task_statuses.len())
        );
        
        // Check if we should stop monitoring
        if cycle_count >= max_cycles {
            log(LogTag::Pool, "MONITOR_COMPLETE", "🔍 Monitoring completed (reached max cycles)");
            break;
        }
        
        // Wait before next cycle
        sleep(Duration::from_secs(5)).await;
    }
    
    // Stop the service
    pool_service.stop().await;
    log(LogTag::Pool, "SERVICE_STOP", "🛑 Pool service stopped");
    
    Ok(())
}

/// Show pool service statistics
async fn show_pool_service_stats(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "STATS_START", "📊 Gathering pool service statistics...");
    
    // Start service briefly to get stats
    pool_service.start().await?;
    sleep(Duration::from_secs(2)).await;
    
    // Get service statistics
    let stats = pool_service.get_stats().await;
    let task_statuses = pool_service.get_task_statuses().await;
    
    // Get monitor statistics
    let monitor_stats = get_pool_monitor_stats().await;
    
    // Display statistics
    println!("\n=== Pool Service Statistics ===");
    println!("Service Status: {}", if pool_service.is_running().await { "Running" } else { "Stopped" });
    println!("Available Tokens: {}", stats.total_tokens_available);
    println!("Cache Hits: {}", stats.cache_hits);
    println!("Successful Price Fetches: {}", stats.successful_price_fetches);
    println!("Failed Price Fetches: {}", stats.failed_price_fetches);
    println!("Last Update: {:?}", stats.last_update);
    
    println!("\n=== Monitor Statistics ===");
    println!("Total Monitoring Cycles: {}", monitor_stats.total_monitoring_cycles);
    println!("Successful Cycles: {}", monitor_stats.successful_cycles);
    println!("Failed Cycles: {}", monitor_stats.failed_cycles);
    println!("Tasks Restarted: {}", monitor_stats.tasks_restarted);
    println!("Health Checks Performed: {}", monitor_stats.health_checks_performed);
    println!("Average Health Percentage: {:.1}%", monitor_stats.average_health_percentage);
    println!("Success Rate: {:.1}%", monitor_stats.get_success_rate());
    println!("Last Health Check: {:?}", monitor_stats.last_health_check);
    println!("Last Monitoring Cycle: {:?}", monitor_stats.last_monitoring_cycle);
    
    println!("\n=== Task Statuses ===");
    for (name, status) in &task_statuses {
        println!("Task: {}", name);
        println!("  State: {:?}", status.state);
        println!("  Run Count: {}", status.run_count);
        println!("  Error Count: {}", status.error_count);
        println!("  Last Run: {:?}", status.last_run);
        if let Some(error) = &status.last_error {
            println!("  Last Error: {}", error);
        }
        println!();
    }
    
    // Stop the service
    pool_service.stop().await;
    
    log(LogTag::Pool, "STATS_COMPLETE", "📊 Statistics gathering completed");
    Ok(())
}

/// Test pool service functionality
async fn test_pool_service(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "TEST_START", "🧪 Starting pool service tests...");
    
    // Start the service
    pool_service.start().await?;
    sleep(Duration::from_secs(3)).await;
    
    // Test 1: Check if service is running
    log(LogTag::Pool, "TEST_1", "Testing service startup...");
    if pool_service.is_running().await {
        log(LogTag::Pool, "TEST_1_PASS", "✅ Service startup test passed");
    } else {
        log(LogTag::Pool, "TEST_1_FAIL", "❌ Service startup test failed");
        return Err("Service startup test failed".into());
    }
    
    // Test 2: Check available tokens
    log(LogTag::Pool, "TEST_2", "Testing available tokens...");
    let available_tokens = pool_service.get_available_tokens().await;
    log(
        LogTag::Pool,
        "TEST_2_RESULT",
        &format!("Available tokens count: {}", available_tokens.len())
    );
    
    // Test 3: Check task statuses
    log(LogTag::Pool, "TEST_3", "Testing task statuses...");
    let task_statuses = pool_service.get_task_statuses().await;
    let running_tasks = task_statuses.values()
        .filter(|status| matches!(status.state, screenerbot::pool_monitor::TaskState::Running))
        .count();
    
    log(
        LogTag::Pool,
        "TEST_3_RESULT",
        &format!("Running tasks: {}/{}", running_tasks, task_statuses.len())
    );
    
    // Test 4: Check monitor service
    log(LogTag::Pool, "TEST_4", "Testing monitor service...");
    let monitor_stats = get_pool_monitor_stats().await;
    log(
        LogTag::Pool,
        "TEST_4_RESULT",
        &format!("Monitor health checks: {}", monitor_stats.health_checks_performed)
    );
    
    // Test 5: Test price fetching (if tokens available)
    if !available_tokens.is_empty() {
        log(LogTag::Pool, "TEST_5", "Testing price fetching...");
        let test_token = &available_tokens[0];
        let price_info = pool_service.get_price(test_token).await;
        
        if let Some(price) = price_info {
            log(
                LogTag::Pool,
                "TEST_5_PASS",
                &format!("✅ Price fetch test passed for token {}: {:.6} SOL", test_token, 
                    price.pool_price_sol.unwrap_or(0.0))
            );
        } else {
            log(LogTag::Pool, "TEST_5_SKIP", "⏭️ Price fetch test skipped (no price data available)");
        }
    } else {
        log(LogTag::Pool, "TEST_5_SKIP", "⏭️ Price fetch test skipped (no tokens available)");
    }
    
    // Stop the service
    pool_service.stop().await;
    
    log(LogTag::Pool, "TEST_COMPLETE", "🧪 All tests completed successfully");
    Ok(())
}

/// Show task statuses
async fn show_task_statuses(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "STATUS_START", "📋 Gathering task statuses...");
    
    // Start service briefly
    pool_service.start().await?;
    sleep(Duration::from_secs(2)).await;
    
    let task_statuses = pool_service.get_task_statuses().await;
    
    println!("\n=== Task Statuses ===");
    for (name, status) in &task_statuses {
        println!("Task: {}", name);
        println!("  State: {:?}", status.state);
        println!("  Run Count: {}", status.run_count);
        println!("  Error Count: {}", status.error_count);
        println!("  Last Run: {:?}", status.last_run);
        if let Some(error) = &status.last_error {
            println!("  Last Error: {}", error);
        }
        println!();
    }
    
    // Stop the service
    pool_service.stop().await;
    
    log(LogTag::Pool, "STATUS_COMPLETE", "📋 Task status gathering completed");
    Ok(())
}

/// Debug pool tokens loading and database access
async fn debug_pool_tokens(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "DEBUG_TOKENS_START", "🔍 Starting pool tokens debugging...");
    
    // Start service briefly
    pool_service.start().await?;
    sleep(Duration::from_secs(3)).await;
    
    // Get pool tokens statistics
    let tokens_stats = get_pool_tokens_stats().await;
    
    println!("\n=== Pool Tokens Debug Information ===");
    println!("Total Tokens Loaded: {}", tokens_stats.total_tokens_loaded);
    println!("Active Tokens: {}", tokens_stats.active_tokens);
    println!("Tokens with Liquidity: {}", tokens_stats.tokens_with_liquidity);
    println!("Average Liquidity USD: ${:.2}", tokens_stats.average_liquidity_usd);
    println!("Database Query Time: {:.1}ms", tokens_stats.database_query_time_ms);
    println!("Last Database Query: {:?}", tokens_stats.last_database_query);
    println!("Last Cache Update: {:?}", tokens_stats.last_cache_update);
    
    // Test database access directly
    log(LogTag::Pool, "DEBUG_DB_TEST", "Testing database access...");
    
    use screenerbot::tokens::cache::TokenDatabase;
    match TokenDatabase::new() {
        Ok(db) => {
            match db.get_all_tokens().await {
                Ok(tokens) => {
                    println!("\n=== Database Direct Access ===");
                    println!("Total tokens in database: {}", tokens.len());
                    
                    let tokens_with_liquidity = tokens.iter()
                        .filter(|t| t.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0) > 1000.0)
                        .count();
                    println!("Tokens with liquidity > $1000: {}", tokens_with_liquidity);
                    
                    // Show top 10 tokens by liquidity
                    let mut sorted_tokens = tokens;
                    sorted_tokens.sort_by(|a, b| {
                        let a_liq = a.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                        let b_liq = b.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                        b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    
                    println!("\n=== Top 10 Tokens by Liquidity ===");
                    for (i, token) in sorted_tokens.iter().take(10).enumerate() {
                        let liquidity = token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                        println!("{}. {} ({}) - ${:.0} liquidity", 
                            i + 1, 
                            token.symbol, 
                            &token.mint[0..8], 
                            liquidity
                        );
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get tokens from database: {}", e);
                    log(LogTag::Pool, "DEBUG_DB_ERROR", &format!("Database error: {}", e));
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create database connection: {}", e);
            log(LogTag::Pool, "DEBUG_DB_ERROR", &format!("Database connection error: {}", e));
        }
    }
    
    // Test pool tokens service directly
    log(LogTag::Pool, "DEBUG_TOKENS_SERVICE", "Testing pool tokens service...");
    
    match screenerbot::pool_tokens::load_tokens_from_database().await {
        Ok(count) => {
            println!("\n=== Pool Tokens Service Test ===");
            println!("✅ Successfully loaded {} tokens", count);
            
            let tracked_tokens = screenerbot::pool_tokens::get_tracked_tokens().await;
            println!("Tracked tokens count: {}", tracked_tokens.len());
            
            if !tracked_tokens.is_empty() {
                println!("\n=== Sample Tracked Tokens ===");
                for (i, mint) in tracked_tokens.iter().take(5).enumerate() {
                    println!("{}. {}", i + 1, mint);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to load tokens via pool tokens service: {}", e);
            log(LogTag::Pool, "DEBUG_TOKENS_ERROR", &format!("Pool tokens service error: {}", e));
        }
    }
    
    // Stop the service
    pool_service.stop().await;
    
    log(LogTag::Pool, "DEBUG_TOKENS_COMPLETE", "🔍 Pool tokens debugging completed");
    Ok(())
}

/// Start and monitor pool service (default behavior)
async fn start_and_monitor_pool_service(pool_service: &screenerbot::pool_service::PoolService) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "DEFAULT_START", "🚀 Starting pool service with default monitoring...");
    
    // Start the service
    pool_service.start().await?;
    sleep(Duration::from_secs(3)).await;
    
    if !pool_service.is_running().await {
        log(LogTag::Pool, "SERVICE_ERROR", "❌ Pool service failed to start");
        return Err("Pool service failed to start".into());
    }
    
    log(LogTag::Pool, "SERVICE_RUNNING", "✅ Pool service is running");
    
    // Monitor for 60 seconds
    let mut cycle_count = 0;
    let max_cycles = 12; // 60 seconds / 5 seconds per cycle
    
    while cycle_count < max_cycles {
        cycle_count += 1;
        
        let stats = pool_service.get_stats().await;
        let task_statuses = pool_service.get_task_statuses().await;
        let running_tasks = task_statuses.values()
            .filter(|status| matches!(status.state, screenerbot::pool_monitor::TaskState::Running))
            .count();
        
        log(
            LogTag::Pool,
            "MONITOR_CYCLE",
            &format!(
                "Cycle {}/{} - Running tasks: {}/{}, Available tokens: {}, Cache hits: {}",
                cycle_count,
                max_cycles,
                running_tasks,
                task_statuses.len(),
                stats.total_tokens_available,
                stats.cache_hits
            )
        );
        
        sleep(Duration::from_secs(5)).await;
    }
    
    // Stop the service
    pool_service.stop().await;
    log(LogTag::Pool, "SERVICE_STOP", "🛑 Pool service stopped");
    
    Ok(())
}
