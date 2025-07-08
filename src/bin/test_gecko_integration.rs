use screenerbot::trader::MarketDataFrame;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing GeckoTerminal API Integration");
    
    // Test with a known pool address (SOL/USDC)
    let test_pool_address = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";
    let test_mint = "So11111111111111111111111111111111111111112"; // SOL mint
    
    println!("📊 Testing historical data loading for pool: {}", test_pool_address);
    
    let mut market_data = MarketDataFrame::new_with_pool_info(
        test_pool_address.to_string(),
        "SOL".to_string(),
        "USDC".to_string()
    );
    
    // Load historical data
    match market_data.load_historical_data(test_pool_address, test_mint).await {
        Ok(()) => {
            println!("✅ Successfully loaded historical data!");
            println!("📈 Minute data points: {}", market_data.minute_data.timestamps.len());
            println!("📈 Hour data points: {}", market_data.hour_data.timestamps.len());
            println!("📈 Day data points: {}", market_data.day_data.timestamps.len());
            
            // Show some sample data
            if !market_data.minute_data.timestamps.is_empty() {
                let latest_idx = market_data.minute_data.timestamps.len() - 1;
                println!("📊 Latest minute data: timestamp={}, open={}, high={}, low={}, close={}, volume={}", 
                    market_data.minute_data.timestamps[latest_idx],
                    market_data.minute_data.opens[latest_idx],
                    market_data.minute_data.highs[latest_idx],
                    market_data.minute_data.lows[latest_idx],
                    market_data.minute_data.closes[latest_idx],
                    market_data.minute_data.volumes[latest_idx]
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to load historical data: {}", e);
        }
    }
    
    // Test cache functionality
    println!("\n🗂️  Testing cache functionality...");
    match market_data.load_historical_data(test_pool_address, test_mint).await {
        Ok(()) => {
            println!("✅ Cache test successful - second load should be faster");
        }
        Err(e) => {
            println!("❌ Cache test failed: {}", e);
        }
    }
    
    println!("\n✅ GeckoTerminal API integration test completed");
    
    Ok(())
}
