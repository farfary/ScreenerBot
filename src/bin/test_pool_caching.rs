use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::logger::{ log, LogTag };
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    log(LogTag::System, "INFO", "🧪 Testing Pool Caching System");
    log(LogTag::System, "INFO", "=====================================");

    // Initialize pool service
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let pool_service = PoolDiscoveryAndPricing::new(rpc_url);

    // Test token mint (using BONK as an example)
    let test_token = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK

    log(LogTag::System, "INFO", &format!("Testing with token: {}", test_token));

    // Test 1: First call - should fetch from API and cache
    log(LogTag::System, "INFO", "\n🔍 Test 1: First call (should fetch and cache)");
    let start = Instant::now();

    match pool_service.get_biggest_pool_cached(test_token).await {
        Ok(Some(pool)) => {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "✅ Found biggest pool: {} ({}) with liquidity ${:.2}",
                    pool.pool_address,
                    pool.dex_id,
                    pool.liquidity_usd
                )
            );
        }
        Ok(None) => {
            log(LogTag::System, "WARN", "❌ No pools found for token");
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Error fetching pools: {}", e));
        }
    }

    let duration = start.elapsed();
    log(LogTag::System, "INFO", &format!("⏱️ First call took: {:.2}s", duration.as_secs_f64()));

    // Test 2: Second call - should use cache
    log(LogTag::System, "INFO", "\n🚀 Test 2: Second call (should use cache)");
    let start = Instant::now();

    match pool_service.get_biggest_pool_cached(test_token).await {
        Ok(Some(pool)) => {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "✅ Found biggest pool from cache: {} ({}) with liquidity ${:.2}",
                    pool.pool_address,
                    pool.dex_id,
                    pool.liquidity_usd
                )
            );
        }
        Ok(None) => {
            log(LogTag::System, "WARN", "❌ No pools found for token");
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Error fetching pools: {}", e));
        }
    }

    let duration = start.elapsed();
    log(
        LogTag::System,
        "INFO",
        &format!("⏱️ Second call took: {:.2}s (should be much faster)", duration.as_secs_f64())
    );

    // Test 3: Program IDs caching
    log(LogTag::System, "INFO", "\n🔧 Test 3: Program IDs caching");
    let start = Instant::now();

    match pool_service.get_program_ids_cached(test_token).await {
        Ok(program_ids) => {
            log(LogTag::System, "INFO", &format!("✅ Found {} program IDs:", program_ids.len()));
            for (i, program_id) in program_ids.iter().enumerate() {
                log(LogTag::System, "INFO", &format!("   {}. {}", i + 1, program_id));
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Error fetching program IDs: {}", e));
        }
    }

    let duration = start.elapsed();
    log(
        LogTag::System,
        "INFO",
        &format!("⏱️ Program IDs call took: {:.2}s", duration.as_secs_f64())
    );

    // Test 4: Program IDs second call (should be cached)
    log(LogTag::System, "INFO", "\n⚡ Test 4: Program IDs second call (should use cache)");
    let start = Instant::now();

    match pool_service.get_program_ids_cached(test_token).await {
        Ok(program_ids) => {
            log(
                LogTag::System,
                "INFO",
                &format!("✅ Found {} program IDs from cache", program_ids.len())
            );
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Error fetching program IDs: {}", e));
        }
    }

    let duration = start.elapsed();
    log(
        LogTag::System,
        "INFO",
        &format!(
            "⏱️ Program IDs cached call took: {:.2}s (should be instant)",
            duration.as_secs_f64()
        )
    );

    // Test 5: Cache cleanup
    log(LogTag::System, "INFO", "\n🧹 Test 5: Cache cleanup");
    pool_service.cleanup_expired_cache();
    log(LogTag::System, "INFO", "✅ Cache cleanup completed");

    log(LogTag::System, "INFO", "\n✨ Pool caching test completed!");
    log(LogTag::System, "INFO", "Cache expires after 2 minutes (120 seconds)");

    Ok(())
}
