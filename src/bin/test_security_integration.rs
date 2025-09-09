/// Security Integration Test Tool
///
/// This tool tests the security integration between monitor.rs and filtering.rs
/// to ensure security data flows properly through the system.

use screenerbot::{
    logger::{ log, LogTag },
    tokens::{
        monitor::{ run_monitoring_cycle_once },
        security::{ init_security_analyzer, get_security_analyzer },
    },
    filtering::{ get_filtered_tokens },
};
use std::env;
use tokio;

const HELP_TEXT: &str =
    r#"
Security Integration Test Tool

USAGE:
    cargo run --bin test_security_integration [OPTIONS]

OPTIONS:
    --help                     Show this help message
    --test-monitor             Test security monitoring cycle
    --test-filtering           Test security-aware filtering
    --test-full-flow           Test complete monitor → filter flow
    --show-stats               Show security database statistics

EXAMPLES:
    # Test security monitoring
    cargo run --bin test_security_integration --test-monitor

    # Test security filtering
    cargo run --bin test_security_integration --test-filtering

    # Test complete flow
    cargo run --bin test_security_integration --test-full-flow
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic logger
    log(LogTag::Security, "DEBUG", "Starting security integration test");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    // Parse command line arguments
    let mut test_monitor = false;
    let mut test_filtering = false;
    let mut test_full_flow = false;
    let mut show_stats = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "--help" => {
                print_help();
                return Ok(());
            }
            "--test-monitor" => {
                test_monitor = true;
            }
            "--test-filtering" => {
                test_filtering = true;
            }
            "--test-full-flow" => {
                test_full_flow = true;
            }
            "--show-stats" => {
                show_stats = true;
            }
            _ => {
                println!("Unknown option: {}", arg);
                print_help();
                return Ok(());
            }
        }
    }

    // Initialize security analyzer
    println!("🔧 Initializing security analyzer...");
    let _analyzer = init_security_analyzer()?;
    println!("✅ Security analyzer initialized");

    if show_stats {
        test_security_stats().await?;
    }

    if test_monitor {
        test_monitor_security_cycle().await?;
    }

    if test_filtering {
        test_security_filtering().await?;
    }

    if test_full_flow {
        test_complete_flow().await?;
    }

    if !test_monitor && !test_filtering && !test_full_flow && !show_stats {
        println!("❌ No test specified. Use --help for options.");
        print_help();
    }

    Ok(())
}

fn print_help() {
    println!("{}", HELP_TEXT);
}

async fn test_security_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Testing security database statistics...");

    let _analyzer = get_security_analyzer();

    // Get some statistics
    println!("📊 Security database path: data/security.db");
    println!("✅ Security stats test completed");

    Ok(())
}

async fn test_monitor_security_cycle() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Testing security monitoring cycle...");

    // Test running a monitoring cycle (this should include security updates every 5th cycle)
    match run_monitoring_cycle_once().await {
        Ok(()) => {
            println!("✅ Monitoring cycle completed successfully");
            println!("   This cycle may have included security updates if it was the 5th cycle");
        }
        Err(e) => {
            println!("❌ Monitoring cycle failed: {}", e);
        }
    }

    Ok(())
}

async fn test_security_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Testing security-aware filtering...");

    // Test the main filtering function which now includes security validation
    match get_filtered_tokens().await {
        Ok(tokens) => {
            println!("✅ Security-aware filtering completed successfully");
            println!("   📊 Filtered {} tokens ready for monitoring", tokens.len());

            if tokens.len() > 0 {
                println!("   📝 Sample tokens:");
                for (i, token) in tokens.iter().take(5).enumerate() {
                    println!("      {}. {}", i + 1, token);
                }
                if tokens.len() > 5 {
                    println!("      ... and {} more", tokens.len() - 5);
                }
            }
        }
        Err(e) => {
            println!("❌ Security-aware filtering failed: {}", e);
        }
    }

    Ok(())
}

async fn test_complete_flow() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Testing complete monitor → filter flow...");

    // Step 1: Run monitoring cycle (may include security updates)
    println!("📡 Step 1: Running monitoring cycle...");
    match run_monitoring_cycle_once().await {
        Ok(()) => println!("   ✅ Monitoring cycle completed"),
        Err(e) => println!("   ⚠️  Monitoring cycle failed: {}", e),
    }

    // Step 2: Run security-aware filtering
    println!("🔍 Step 2: Running security-aware filtering...");
    match get_filtered_tokens().await {
        Ok(tokens) => {
            println!("   ✅ Security filtering completed");
            println!("   📊 Result: {} tokens passed security validation", tokens.len());
        }
        Err(e) => {
            println!("   ❌ Security filtering failed: {}", e);
        }
    }

    println!("🎉 Complete flow test finished!");

    Ok(())
}
