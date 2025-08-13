/// Test binary to verify the centralized argument system works correctly
/// 
/// Usage: cargo run --bin test_arguments -- --debug-trader --debug-profit --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v

use screenerbot::arguments::{
    set_cmd_args, get_cmd_args, has_arg, get_arg_value,
    is_debug_trader_enabled, is_debug_profit_enabled, is_debug_filtering_enabled,
    is_any_debug_enabled, get_enabled_debug_modes, print_debug_info,
    patterns
};
use std::env;

#[tokio::main]
async fn main() {
    println!("🧪 Testing Centralized Argument System");
    println!("=====================================");

    // Initialize arguments from command line
    let args: Vec<String> = env::args().collect();
    set_cmd_args(args.clone());

    // Test basic argument access
    println!("\n📋 Command-line arguments:");
    let retrieved_args = get_cmd_args();
    for (i, arg) in retrieved_args.iter().enumerate() {
        println!("  [{}]: {}", i, arg);
    }

    // Test specific argument checking
    println!("\n🔍 Argument checking:");
    println!("  has_arg('--debug-trader'): {}", has_arg("--debug-trader"));
    println!("  has_arg('--debug-profit'): {}", has_arg("--debug-profit"));
    println!("  has_arg('--debug-nonexistent'): {}", has_arg("--debug-nonexistent"));

    // Test argument value extraction
    println!("\n📖 Argument value extraction:");
    println!("  get_arg_value('--mint'): {:?}", get_arg_value("--mint"));
    println!("  get_arg_value('--symbol'): {:?}", get_arg_value("--symbol"));

    // Test debug flag functions
    println!("\n🐛 Debug flag functions:");
    println!("  is_debug_trader_enabled(): {}", is_debug_trader_enabled());
    println!("  is_debug_profit_enabled(): {}", is_debug_profit_enabled());
    println!("  is_debug_filtering_enabled(): {}", is_debug_filtering_enabled());
    println!("  is_any_debug_enabled(): {}", is_any_debug_enabled());

    // Test enabled debug modes
    println!("\n📊 Enabled debug modes:");
    let enabled_modes = get_enabled_debug_modes();
    if enabled_modes.is_empty() {
        println!("  No debug modes enabled");
    } else {
        for mode in enabled_modes {
            println!("  ✓ {}", mode);
        }
    }

    // Test common patterns
    println!("\n🔧 Common patterns:");
    println!("  patterns::is_help_requested(): {}", patterns::is_help_requested());
    println!("  patterns::is_version_requested(): {}", patterns::is_version_requested());
    println!("  patterns::get_duration_seconds(): {:?}", patterns::get_duration_seconds());
    println!("  patterns::get_mint_address(): {:?}", patterns::get_mint_address());
    println!("  patterns::is_quiet_mode(): {}", patterns::is_quiet_mode());
    println!("  patterns::is_verbose_mode(): {}", patterns::is_verbose_mode());

    // Test the print_debug_info function
    println!("\n📝 Debug info dump:");
    print_debug_info();

    println!("\n✅ All argument system tests completed!");
}
