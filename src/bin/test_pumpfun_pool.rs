use screenerbot::pool_price::{ PoolDiscoveryAndPricing, PoolType };
use screenerbot::global::read_configs;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Pump.fun AMM Pool Support");

    let configs = read_configs("configs.json").expect("Failed to load configs");
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Test token mint and pool address from user request
    let test_token = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";
    let test_pool = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

    println!("🔍 Testing with Pump.fun token: {}", test_token);
    println!("🏊 Testing with pool address: {}", test_pool);

    // Test 1: Pool Discovery
    println!("\n📊 Step 1: Discovering pools for token...");
    match pool_service.discover_pools(test_token).await {
        Ok(pools) => {
            println!("✅ Found {} pools", pools.len());
            for (i, pool) in pools.iter().enumerate() {
                println!(
                    "   Pool {}: {} on {} (pair: {})",
                    i + 1,
                    pool.dex_id,
                    pool.labels.join(", "),
                    pool.pair_address
                );
            }
        }
        Err(e) => {
            println!("❌ Pool discovery failed: {}", e);
        }
    }

    // Test 2: Pool Type Detection
    println!("\n🔎 Step 2: Detecting pool type for test pool...");
    match pool_service.detect_pool_type(test_pool).await {
        Ok(pool_type) => {
            println!("✅ Detected pool type: {:?}", pool_type);
            if pool_type == PoolType::PumpfunAmm {
                println!("🎉 Pump.fun AMM type detected correctly!");
            } else {
                println!("⚠️  Pool type is not Pump.fun AMM: {:?}", pool_type);
            }
        }
        Err(e) => {
            println!("❌ Pool type detection failed: {}", e);
        }
    }

    // Test 3: Pool Data Parsing
    println!("\n🔍 Step 3: Parsing pool data...");
    match pool_service.parse_pool_data(test_pool, PoolType::PumpfunAmm).await {
        Ok(pool_data) => {
            println!("✅ Successfully parsed Pump.fun AMM pool data!");
            println!(
                "   Token A: {} (decimals: {})",
                pool_data.token_a.mint,
                pool_data.token_a.decimals
            );
            println!(
                "   Token B: {} (decimals: {})",
                pool_data.token_b.mint,
                pool_data.token_b.decimals
            );
            println!("   Reserve A Balance: {}", pool_data.reserve_a.balance);
            println!("   Reserve B Balance: {}", pool_data.reserve_b.balance);

            if
                let screenerbot::pool_price::PoolSpecificData::PumpfunAmm {
                    pool_bump,
                    index,
                    lp_supply,
                    ..
                } = &pool_data.specific_data
            {
                println!("   Pool Bump: {}", pool_bump);
                println!("   Index: {}", index);
                println!("   LP Supply: {}", lp_supply);
            }
        }
        Err(e) => {
            println!("❌ Pool data parsing failed: {}", e);
        }
    }

    // Test 4: Price Calculation
    println!("\n💰 Step 4: Calculating price...");
    match pool_service.calculate_pool_price_with_type(test_pool, PoolType::PumpfunAmm).await {
        Ok((price, token_symbol, sol_symbol, pool_type)) => {
            println!("✅ Successfully calculated Pump.fun AMM price!");
            println!("   Price: {} {} per {}", price, sol_symbol, token_symbol);
            println!("   Pool Type: {:?}", pool_type);
        }
        Err(e) => {
            println!("❌ Price calculation failed: {}", e);
        }
    }

    println!("\n🏁 Pump.fun AMM test completed!");
    Ok(())
}
