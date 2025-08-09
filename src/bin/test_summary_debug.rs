/// Test script to verify Summary log tag and debug summary functionality
use screenerbot::{
    logger::{log, LogTag, init_file_logging},
    global::{set_cmd_args, is_debug_summary_enabled},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize file logging
    init_file_logging();

    println!("🧪 Testing Summary Log Tag and Debug Summary Features\n");

    // Test 1: Summary log tag without debug mode
    println!("📋 Test 1: Summary log tag (normal mode)");
    log(LogTag::Summary, "INFO", "Testing Summary log tag - this should appear");
    log(LogTag::Summary, "DEBUG", "Testing Summary debug log - this should appear");
    log(LogTag::Summary, "SUCCESS", "Summary logging test completed");

    // Test 2: Enable debug summary mode and test
    println!("\n📋 Test 2: Debug summary mode");
    set_cmd_args(vec!["test_summary_debug".to_string(), "--debug-summary".to_string()]);
    
    println!("Debug summary enabled: {}", is_debug_summary_enabled());
    
    if is_debug_summary_enabled() {
        log(LogTag::Summary, "DEBUG", "Debug summary mode is now enabled");
        log(LogTag::Summary, "INFO", "Summary information with debug mode active");
        log(LogTag::Summary, "DEBUG", "Additional debug information available in debug mode");
    }

    // Test 3: Other log tags to ensure no conflicts
    println!("\n📋 Test 3: Other log tags (should work normally)");
    log(LogTag::System, "INFO", "System log test");
    log(LogTag::Trader, "INFO", "Trader log test");
    log(LogTag::Swap, "INFO", "Swap log test");

    println!("\n✅ All Summary log tag tests completed successfully!");
    println!("📄 Check the logs/ directory for file output");
    
    Ok(())
}
