/// Test Enhanced Price Change Logging
/// Demonstrates the improved log_price_change function with comprehensive details

use screenerbot::{
    logger::{ log_price_change, log, LogTag, init_file_logging },
    global::read_configs,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize file logging
    init_file_logging();

    println!("🧪 Testing Enhanced Price Change Logging\n");

    log(LogTag::System, "INFO", "Starting enhanced price change logging demonstration");

    // Test Case 1: Pool price with full details and positive P&L
    println!("📈 Test Case 1: Pool price update with profit position");
    log_price_change(
        "2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv",
        "PENGU",
        0.0002,
        0.00023,
        "pool",
        Some("ORCA WHIRLPOOL"),
        Some("FAqh648xeeaTqL7du49sztp9nfj5PjRQrfvaMccyd9cz"),
        Some(0.000225), // API price for comparison
        Some((0.00015, 15.0)) // P&L: +0.000150 SOL, +15.0%
    );

    println!();

    // Test Case 2: Pool price with negative P&L
    println!("📉 Test Case 2: Pool price update with loss position");
    log_price_change(
        "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump",
        "Fartcoin",
        0.006,
        0.0055,
        "pool",
        Some("RAYDIUM LEGACY AMM"),
        Some("Bzc9NZfMqkXR6fz1DBph7BDf9BroyEf6pnzESP7v5iiw"),
        Some(0.0056), // API price higher than pool
        Some((-0.0005, -8.33)) // P&L: -0.000500 SOL, -8.33%
    );

    println!();

    // Test Case 3: API price with no position
    println!("🌐 Test Case 3: API price update with no position");
    log_price_change(
        "ArkmLDH65ao6NWbQdP58CiE7zzrCyp5DzDpTkZ3aDPdS",
        "ARK",
        0.000004,
        0.0000042,
        "api",
        None,
        None,
        None, // No API comparison needed
        None // No position
    );

    println!();

    // Test Case 4: Long symbol with complex pool information
    println!("🔥 Test Case 4: Long symbol with complex details");
    log_price_change(
        "HBsbZz9hvxzi3EnCYWLLwvMPWgV4aeC74mEdvTg9bonk",
        "SuperLongTokenName",
        0.0000015,
        0.0000018,
        "pool",
        Some("METEORA DLMM"),
        Some("C6ELogyx2aAd4FfMS9YcVQ284sPvD454hNaFGj7WFYYh"),
        Some(0.00000175), // API slightly different
        Some((0.0003, 25.5)) // Great profit
    );

    println!();

    // Test Case 5: Perfect match between pool and API
    println!("✨ Test Case 5: Perfect price match");
    log_price_change(
        "GmaDaRhj31EqBn4SowT3KPDsWwS64P3qHJfD6YdK7vcp",
        "BONK",
        0.00001,
        0.0000105,
        "pool",
        Some("raydium-cpmm"), // Test legacy formatting
        Some("HpFbTEcMXKFw66xr1RbiTv1x8LUhvNYLigTamvyxBSp2"),
        Some(0.0000105), // Perfect match
        Some((0.000025, 2.5)) // Small profit
    );

    println!();

    log(LogTag::System, "SUCCESS", "Enhanced price change logging demonstration completed");
    println!("✅ Enhanced logging test completed! Check the log output above.");

    Ok(())
}
