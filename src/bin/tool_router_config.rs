/// Router Configuration Testing Tool
/// 
/// This tool demonstrates the swap router configuration system and allows testing
/// different router enable/disable scenarios.

use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    swaps::{
        init_router_config, get_enabled_routers, get_config_summary,
        validate_router_availability, has_available_routers, get_preferred_router_order,
        config::{is_gmgn_enabled, is_jupiter_enabled, is_raydium_enabled}
    },
    rpc::init_rpc_client,
    global::read_configs,
};

use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup command line arguments
    let matches = Command::new("Router Configuration Test")
        .version("1.0")
        .about("Test and demonstrate swap router configuration system")
        .arg(
            Arg::new("test-router")
                .long("test-router")
                .value_name("ROUTER")
                .help("Test availability of specific router (gmgn, jupiter, raydium)")
        )
        .arg(
            Arg::new("show-config")
                .long("show-config")
                .help("Show current router configuration")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Initialize systems
    init_file_logging();
    init_rpc_client()?;
    
    log(LogTag::Test, "STARTUP", "🔧 Router Configuration Testing Tool");

    // Initialize router configuration (this will log the current config)
    init_router_config();

    // Show detailed configuration if requested
    if matches.get_flag("show-config") {
        show_detailed_config();
    }

    // Test specific router if requested
    if let Some(router_name) = matches.get_one::<String>("test-router") {
        test_router_availability(router_name);
    }

    // Show summary
    show_configuration_summary();

    log(LogTag::Test, "COMPLETE", "✅ Router configuration testing completed");

    Ok(())
}

/// Display detailed router configuration information
fn show_detailed_config() {
    log(LogTag::Test, "CONFIG_DETAIL", "📋 Detailed Router Configuration:");
    
    // Individual router status
    log(
        LogTag::Test, 
        "CONFIG_GMGN",
        &format!("  🔵 GMGN Router: {}", if is_gmgn_enabled() { "✅ ENABLED" } else { "❌ DISABLED" })
    );
    
    log(
        LogTag::Test,
        "CONFIG_JUPITER", 
        &format!("  🟡 Jupiter Router: {}", if is_jupiter_enabled() { "✅ ENABLED" } else { "❌ DISABLED" })
    );
    
    log(
        LogTag::Test,
        "CONFIG_RAYDIUM",
        &format!("  🟣 Raydium Router: {} {}", 
            if is_raydium_enabled() { "✅ ENABLED" } else { "❌ DISABLED" },
            if !is_raydium_enabled() { "(API Deprecated)" } else { "" }
        )
    );

    // Enabled routers list
    let enabled_routers = get_enabled_routers();
    log(
        LogTag::Test,
        "CONFIG_ENABLED",
        &format!("  📊 Enabled Routers: {} (Total: {})", 
            if enabled_routers.is_empty() { "NONE".to_string() } else { enabled_routers.join(", ") },
            enabled_routers.len()
        )
    );

    // Preferred order
    let preferred_order = get_preferred_router_order();
    log(
        LogTag::Test,
        "CONFIG_ORDER",
        &format!("  🏆 Preferred Order: {}", 
            if preferred_order.is_empty() { "NONE".to_string() } else { preferred_order.join(" → ") }
        )
    );

    // System availability
    log(
        LogTag::Test,
        "CONFIG_AVAILABLE",
        &format!("  ⚡ System Ready: {}", if has_available_routers() { "✅ YES" } else { "❌ NO" })
    );
}

/// Test availability of a specific router
fn test_router_availability(router_name: &str) {
    log(
        LogTag::Test,
        "ROUTER_TEST",
        &format!("🧪 Testing router availability: {}", router_name.to_uppercase())
    );

    match validate_router_availability(router_name) {
        Ok(()) => {
            log(
                LogTag::Test,
                "ROUTER_OK",
                &format!("✅ {} router is AVAILABLE and ENABLED", router_name.to_uppercase())
            );
        }
        Err(error) => {
            log(
                LogTag::Test,
                "ROUTER_UNAVAILABLE",
                &format!("❌ {} router is UNAVAILABLE: {}", router_name.to_uppercase(), error)
            );
        }
    }

    // Show specific router status
    match router_name.to_lowercase().as_str() {
        "gmgn" => {
            log(
                LogTag::Test,
                "ROUTER_STATUS",
                &format!("  📊 GMGN Status: enabled={}", is_gmgn_enabled())
            );
        }
        "jupiter" => {
            log(
                LogTag::Test,
                "ROUTER_STATUS",
                &format!("  📊 Jupiter Status: enabled={}", is_jupiter_enabled())
            );
        }
        "raydium" => {
            log(
                LogTag::Test,
                "ROUTER_STATUS",
                &format!("  📊 Raydium Status: enabled={} (deprecated={})", 
                    is_raydium_enabled(), 
                    !is_raydium_enabled()
                )
            );
        }
        _ => {
            log(
                LogTag::Test,
                "ROUTER_UNKNOWN",
                &format!("❓ Unknown router: {} (valid options: gmgn, jupiter, raydium)", router_name)
            );
        }
    }
}

/// Show configuration summary
fn show_configuration_summary() {
    log(LogTag::Test, "SUMMARY", "📈 Configuration Summary:");
    
    let config_summary = get_config_summary();
    log(LogTag::Test, "SUMMARY_CONFIG", &format!("  🔧 Config: {}", config_summary));
    
    let enabled_count = get_enabled_routers().len();
    log(LogTag::Test, "SUMMARY_COUNT", &format!("  📊 Enabled: {}/3 routers", enabled_count));
    
    let system_ready = has_available_routers();
    log(
        LogTag::Test, 
        "SUMMARY_STATUS", 
        &format!("  ⚡ Status: {}", if system_ready { "READY" } else { "NOT READY" })
    );

    // Configuration recommendations
    if enabled_count == 0 {
        log(
            LogTag::Test,
            "RECOMMENDATION",
            "⚠️  CRITICAL: No routers enabled! Enable at least one router in src/swaps/config.rs"
        );
    } else if enabled_count == 1 {
        log(
            LogTag::Test,
            "RECOMMENDATION",
            "💡 SUGGESTION: Consider enabling multiple routers for better route optimization"
        );
    } else {
        log(
            LogTag::Test,
            "RECOMMENDATION",
            "✅ OPTIMAL: Multiple routers enabled for best route selection"
        );
    }

    // Show current configuration file location
    log(
        LogTag::Test,
        "CONFIG_FILE",
        "📁 Configuration file: src/swaps/config.rs"
    );
    
    log(
        LogTag::Test,
        "CONFIG_VARS",
        "🔧 Configuration variables: ENABLE_GMGN_ROUTER, ENABLE_JUPITER_ROUTER, ENABLE_RAYDIUM_ROUTER"
    );
}
