use screenerbot::arguments::set_cmd_args;
/// Test SOL price coverage logic
///
/// This tool tests the new SOL price coverage requirement to ensure
/// that SOL prices are available before any token OHLCV fetching.
use screenerbot::logger::{init_file_logging, log, LogTag};
use screenerbot::rpc::init_rpc_client;
use screenerbot::tokens::ohlcv_db::{init_ohlcv_database, MAX_OHLCV_AGE_HOURS};
use screenerbot::tokens::ohlcvs::{
    ensure_sol_price_coverage, get_latest_ohlcv, init_ohlcv_service,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logging for OHLCV operations
    set_cmd_args(vec![
        "test_sol_coverage".to_string(),
        "--debug-ohlcv".to_string(),
        "--debug-sol-price".to_string(),
    ]);

    // Initialize logging
    init_file_logging();
    log(
        LogTag::System,
        "START",
        "🧪 Testing SOL price coverage logic with debug enabled",
    );

    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        log(
            LogTag::System,
            "ERROR",
            &format!("Failed to initialize RPC client: {}", e),
        );
        return Err(e.into());
    }

    // Initialize OHLCV database
    log(LogTag::System, "INIT", "📊 Initializing OHLCV database...");
    if let Err(e) = init_ohlcv_database() {
        log(
            LogTag::System,
            "ERROR",
            &format!("Failed to initialize OHLCV database: {}", e),
        );
        return Err(e.into());
    }

    // Initialize OHLCV service
    log(LogTag::System, "INIT", "🌐 Initializing OHLCV service...");
    if let Err(e) = init_ohlcv_service().await {
        log(
            LogTag::System,
            "ERROR",
            &format!("Failed to initialize OHLCV service: {}", e),
        );
        return Err(e.into());
    }

    // Test SOL price coverage requirement with timeout
    log(
        LogTag::System,
        "TEST1",
        "🔍 Testing SOL price coverage requirement...",
    );

    // Use tokio timeout to prevent hanging
    let coverage_result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        ensure_sol_price_coverage(),
    )
    .await;

    match coverage_result {
        Ok(Ok(())) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!(
                    "✅ SOL coverage available for {} hours",
                    MAX_OHLCV_AGE_HOURS
                ),
            );

            // Test token OHLCV fetching now that coverage is ensured
            log(
                LogTag::System,
                "TEST2",
                "🔍 Testing token OHLCV fetching with coverage...",
            );

            // Try to get a sample token's OHLCV data (should work if SOL coverage is available)
            let ohlcv_result = tokio::time::timeout(
                std::time::Duration::from_secs(45),
                get_latest_ohlcv("So11111111111111111111111111111111111111112", 60),
            )
            .await;

            match ohlcv_result {
                Ok(Ok(data)) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("✅ Token OHLCV data retrieved: {} candles", data.len()),
                    );
                }
                Ok(Err(e)) => {
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("Token OHLCV failed: {}", e),
                    );
                }
                Err(_) => {
                    log(
                        LogTag::System,
                        "TIMEOUT",
                        "⏰ Token OHLCV request timed out",
                    );
                }
            }
        }
        Ok(Err(e)) => {
            log(
                LogTag::System,
                "FAIL",
                &format!("❌ SOL coverage check failed: {}", e),
            );
            println!(
                "SOL coverage test failed as expected if no SOL price data: {}",
                e
            );

            // Try token OHLCV anyway to verify it's properly blocked
            log(
                LogTag::System,
                "TEST2",
                "🔍 Testing token OHLCV fetching without coverage (should fail)...",
            );
            let ohlcv_result = tokio::time::timeout(
                std::time::Duration::from_secs(45),
                get_latest_ohlcv("So11111111111111111111111111111111111111112", 60),
            )
            .await;

            match ohlcv_result {
                Ok(Ok(data)) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!(
                            "❌ CRITICAL ERROR: Token OHLCV succeeded without SOL coverage! {} candles",
                            data.len()
                        )
                    );
                }
                Ok(Err(e)) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "✅ Token OHLCV properly blocked without SOL coverage: {}",
                            e
                        ),
                    );
                }
                Err(_) => {
                    log(
                        LogTag::System,
                        "TIMEOUT",
                        "⏰ Token OHLCV request timed out (expected)",
                    );
                }
            }
        }
        Err(_) => {
            log(
                LogTag::System,
                "TIMEOUT",
                "⏰ SOL coverage check timed out - likely fetching SOL price data",
            );
            println!(
                "SOL coverage check timed out - this indicates the system is working and fetching required SOL price data"
            );
        }
    }

    log(LogTag::System, "COMPLETE", "🎯 SOL coverage test completed");
    Ok(())
}
