/// Test script to verify debug summary functionality in summary functions
use screenerbot::{
    logger::{log, LogTag, init_file_logging},
    global::{set_cmd_args, is_debug_summary_enabled},
    summary::display_current_bot_summary,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize file logging
    init_file_logging();

    println!("🧪 Testing Debug Summary in Summary Functions\n");

    // Test without debug mode first
    println!("📊 Test 1: Summary display without debug mode");
    set_cmd_args(vec!["test_debug_summary".to_string()]);
    println!("Debug summary enabled: {}", is_debug_summary_enabled());
    
    // This should not show debug logs
    display_current_bot_summary().await;

    println!("\n{}\n", "=".repeat(60));

    // Test with debug mode enabled
    println!("📊 Test 2: Summary display WITH debug mode");
    set_cmd_args(vec!["test_debug_summary".to_string(), "--debug-summary".to_string()]);
    println!("Debug summary enabled: {}", is_debug_summary_enabled());
    
    // This should show debug logs
    display_current_bot_summary().await;

    println!("\n✅ Debug summary functionality test completed!");
    println!("🔍 Look for DEBUG logs in the output above when debug mode was enabled");
    
    Ok(())
}
