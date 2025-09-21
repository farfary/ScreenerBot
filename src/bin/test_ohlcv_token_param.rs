use screenerbot::tokens::geckoterminal::get_ohlcv_data_from_geckoterminal;
use screenerbot::logger::{ init_file_logging, log, LogTag };
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(LogTag::Api, "TEST_START", "🧪 Testing OHLCV API with token parameter fix");

    // Get test parameters from command line
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: test_ohlcv_token_param <pool_address> <token_mint>");
        println!(
            "Example: test_ohlcv_token_param 3Lu4g43LVCWYj37LfU54qpkoX5obkvYajnxKWfs1gUoW 2oGLxYuNBJRcepT1mEV6KnETaLD7Bf6qq3CM6skasBfe"
        );
        return Ok(());
    }

    let pool_address = &args[1];
    let token_mint = &args[2];

    log(
        LogTag::Api,
        "TEST_PARAMS",
        &format!("Testing pool: {}, token: {}", pool_address, token_mint)
    );

    // Test the OHLCV API call with our token parameter fix
    match get_ohlcv_data_from_geckoterminal(pool_address, token_mint, 5).await {
        Ok(ohlcv_data) => {
            log(
                LogTag::Api,
                "TEST_SUCCESS",
                &format!("✅ Successfully fetched {} OHLCV data points", ohlcv_data.len())
            );

            if !ohlcv_data.is_empty() {
                let first = &ohlcv_data[0];
                log(
                    LogTag::Api,
                    "TEST_DATA",
                    &format!(
                        "📊 First OHLCV point: timestamp={}, open={}, high={}, low={}, close={}, volume={}",
                        first.timestamp,
                        first.open,
                        first.high,
                        first.low,
                        first.close,
                        first.volume
                    )
                );
            }

            println!("🎉 Token parameter fix working correctly!");
            println!("📊 Fetched {} OHLCV data points", ohlcv_data.len());
            for (i, point) in ohlcv_data.iter().enumerate() {
                println!(
                    "   [{}] timestamp={}, O={}, H={}, L={}, C={}, V={}",
                    i + 1,
                    point.timestamp,
                    point.open,
                    point.high,
                    point.low,
                    point.close,
                    point.volume
                );
            }
        }
        Err(e) => {
            log(LogTag::Api, "TEST_ERROR", &format!("❌ OHLCV API call failed: {}", e));
            println!("❌ Test failed: {}", e);
        }
    }

    log(LogTag::Api, "TEST_END", "🧪 Test completed");
    Ok(())
}
