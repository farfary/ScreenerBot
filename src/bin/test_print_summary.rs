/// Test to verify print_summary analyzes all closed positions
use screenerbot::{ prelude::*, helpers, persistence };
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing print_summary with all closed positions analysis...\n");

    // Load closed positions to see how many we have
    if let Ok(data) = std::fs::read_to_string("closed_positions.json") {
        if let Ok(closed_positions) = serde_json::from_str::<HashMap<String, Position>>(&data) {
            println!(
                "📊 Found {} total closed positions in closed_positions.json",
                closed_positions.len()
            );
        }
    }

    // Load the positions into the global state
    println!("🔄 Loading positions into global state...");
    if let Err(e) = persistence::load_cache().await {
        println!("⚠️ Failed to load positions: {:?}", e);
    } else {
        println!("✅ Positions loaded successfully");
    }

    println!("📋 Now running print_summary to verify it analyzes ALL closed positions...\n");

    // Run the print summary function
    helpers::print_summary().await;

    println!("\n✅ Test completed! Check the output above:");
    println!("   - The recent table should show maximum 15 positions");
    println!("   - The analysis metrics should be calculated from ALL closed positions");
    println!("   - Look for 'Total Closed P/L' and win rate numbers");

    Ok(())
}
