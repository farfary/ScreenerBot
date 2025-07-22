use anyhow::Result;
use screenerbot::pool_price::{ PoolDiscoveryAndPricing, PoolType };

#[tokio::main]
async fn main() -> Result<()> {
    // Test pool address for Raydium LaunchLab
    let pool_address = "oxw6tRHFzerBtcsnnWpaAidAUDTphWoc5XH5wjXox7i";

    println!("Testing Raydium LaunchLab pool discovery and price calculation");
    println!("Pool address: {}", pool_address);

    // Initialize the pool price calculator
    let pool_price = PoolDiscoveryAndPricing::new("https://api.mainnet-beta.solana.com");

    // Attempt to detect the pool type
    let detected_pool_type = pool_price.detect_pool_type(pool_address).await?;
    println!("Detected pool type: {:?}", detected_pool_type);

    // Verify it's detected as RaydiumLaunchLab
    if detected_pool_type != PoolType::RaydiumLaunchLab {
        println!("❌ Pool not detected as RaydiumLaunchLab!");
        return Err(anyhow::anyhow!("Pool not detected as RaydiumLaunchLab"));
    } else {
        println!("✅ Pool correctly detected as RaydiumLaunchLab");
    }

    // Get pool data
    println!("Attempting to parse pool data...");
    let pool_data = match pool_price.parse_pool_data(pool_address, detected_pool_type).await {
        Ok(data) => {
            println!("✅ Successfully parsed pool data");
            data
        }
        Err(e) => {
            println!("❌ Failed to parse pool data: {}", e);
            return Err(e);
        }
    };

    // Print pool information
    println!("Token A: {} ({} decimals)", pool_data.token_a.mint, pool_data.token_a.decimals);
    println!("Token B: {} ({} decimals)", pool_data.token_b.mint, pool_data.token_b.decimals);
    println!("Reserve A: {} tokens", pool_data.reserve_a.balance);
    println!("Reserve B: {} tokens", pool_data.reserve_b.balance);

    // Calculate price
    let price = pool_price.calculate_price_from_pool_data(&pool_data).await?;
    println!("Calculated price: {} SOL per token", price);

    // Test the from_dex_id_and_labels method
    let pool_type_from_dex = PoolType::from_dex_id_and_labels(
        "raydium",
        &vec!["launchlab".to_string()]
    );
    println!("Pool type from dex_id and labels: {:?}", pool_type_from_dex);

    if pool_type_from_dex == PoolType::RaydiumLaunchLab {
        println!("✅ from_dex_id_and_labels correctly identified RaydiumLaunchLab");
    } else {
        println!("❌ from_dex_id_and_labels failed to identify RaydiumLaunchLab");
    }

    println!("All tests completed successfully!");
    Ok(())
}
