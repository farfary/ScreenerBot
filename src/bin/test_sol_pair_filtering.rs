/// Test SOL pair filtering in DexScreener API
use screenerbot::{
    tokens::api::{ init_dexscreener_api, get_global_dexscreener_api },
    logger::{ init_file_logging, log, LogTag },
};
use std::collections::HashSet;

const TEST_TOKENS: &[&str] = &[
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC (should be filtered)
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", // JUP (should be SOL pair)
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // BONK (should be SOL pair)
    "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs", // ETH (might be USDC pair)
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    log(LogTag::System, "START", "Testing SOL pair filtering");

    // Initialize the global API
    init_dexscreener_api().await?;

    // Test with mixed tokens (some SOL pairs, some not)
    let test_mints: Vec<String> = TEST_TOKENS.iter()
        .map(|s| s.to_string())
        .collect();

    log(
        LogTag::System,
        "TEST",
        &format!("Testing {} tokens for SOL pair filtering", test_mints.len())
    );

    let result = {
        let api = get_global_dexscreener_api().await?;
        let mut api_instance = api.lock().await;
        api_instance.get_tokens_info(&test_mints).await
    };

    match result {
        Ok(tokens) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Successfully parsed {} SOL-paired tokens", tokens.len())
            );

            let parsed_mints: HashSet<String> = tokens
                .iter()
                .map(|t| t.mint.clone())
                .collect();
            let requested_mints: HashSet<String> = test_mints.into_iter().collect();
            let filtered_out: Vec<String> = requested_mints
                .difference(&parsed_mints)
                .cloned()
                .collect();

            if !filtered_out.is_empty() {
                log(
                    LogTag::System,
                    "FILTERED",
                    &format!(
                        "Filtered out {} non-SOL pairs: {:?}",
                        filtered_out.len(),
                        filtered_out
                    )
                );
            }

            for token in &tokens {
                log(
                    LogTag::System,
                    "SOL_PAIR",
                    &format!(
                        "✓ {} ({}) - SOL price: {:.10}",
                        token.symbol,
                        token.mint,
                        token.price_sol.unwrap_or(0.0)
                    )
                );
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token info: {}", e));
        }
    }

    log(LogTag::System, "COMPLETE", "SOL pair filtering test completed");
    Ok(())
}
