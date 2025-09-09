/// Test tool for pool discovery source configuration
///
/// This tool demonstrates how to check and configure which pool discovery sources are enabled.
/// It shows the current configuration and logs discovery activity.

use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::pools::PoolDiscovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(LogTag::PoolDiscovery, "INFO", "🔧 Pool Discovery Configuration Test");

    // Display current source configuration
    let (dex_enabled, gecko_enabled, raydium_enabled) = PoolDiscovery::get_source_config();

    log(
        LogTag::PoolDiscovery,
        "CONFIG",
        &format!(
            "Current configuration - DexScreener: {}, GeckoTerminal: {}, Raydium: {}",
            if dex_enabled {
                "✅ ENABLED"
            } else {
                "❌ DISABLED"
            },
            if gecko_enabled {
                "✅ ENABLED"
            } else {
                "❌ DISABLED"
            },
            if raydium_enabled {
                "✅ ENABLED"
            } else {
                "❌ DISABLED"
            }
        )
    );

    // Log the full source configuration as the discovery service would
    PoolDiscovery::log_source_config();

    // Check if any sources are enabled
    if !dex_enabled && !gecko_enabled && !raydium_enabled {
        log(
            LogTag::PoolDiscovery,
            "WARN",
            "⚠️ All pool discovery sources are disabled! No pools will be discovered."
        );
    } else {
        let enabled_count = [dex_enabled, gecko_enabled, raydium_enabled]
            .iter()
            .filter(|&&x| x)
            .count();
        log(
            LogTag::PoolDiscovery,
            "INFO",
            &format!("✅ {} out of 3 discovery sources are enabled", enabled_count)
        );
    }

    log(
        LogTag::PoolDiscovery,
        "INFO",
        "💡 To modify configuration, edit the constants in src/pools/discovery.rs:"
    );
    log(LogTag::PoolDiscovery, "INFO", "   - ENABLE_DEXSCREENER_DISCOVERY");
    log(LogTag::PoolDiscovery, "INFO", "   - ENABLE_GECKOTERMINAL_DISCOVERY");
    log(LogTag::PoolDiscovery, "INFO", "   - ENABLE_RAYDIUM_DISCOVERY");

    log(LogTag::PoolDiscovery, "SUCCESS", "Configuration test completed");

    Ok(())
}
