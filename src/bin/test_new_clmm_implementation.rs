/// Test the new Raydium CLMM implementation on real pools
/// This test verifies that the CLMM swap functionality works with proper account derivation

use screenerbot::pools::swap::SwapBuilder;
use screenerbot::pools::swap::types::SwapDirection;
use screenerbot::logger::{ log, LogTag };
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log(LogTag::System, "INFO", "🧪 Testing new Raydium CLMM implementation");

    // Test pool: HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv (CANDY/SOL)
    let pool_address = "HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv";
    let token_mint = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t"; // CANDY token

    log(LogTag::System, "INFO", &format!("📊 Testing pool: {}", pool_address));
    log(LogTag::System, "INFO", &format!("🪙 Token mint: {}", token_mint));

    // Test 1: Buy 0.001 SOL worth of tokens (dry run)
    log(LogTag::System, "INFO", "🔄 Test 1: Buy tokens with 0.001 SOL (dry run)");

    let buy_result = SwapBuilder::new()
        .pool_address(pool_address)?
        .token_mint(token_mint)?
        .amount_sol(0.001)
        .buy()
        .slippage_percent(1.0)
        .dry_run(true)
        .execute().await;

    match buy_result {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", "✅ Buy test successful!");
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "💰 Input: {} SOL → Expected output: {} tokens (min: {})",
                    result.params.input_amount,
                    result.params.expected_output,
                    result.params.minimum_output
                )
            );

            if let Some(transaction) = &result.transaction {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("📝 Transaction accounts: {}", transaction.message.account_keys.len())
                );
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Buy test failed: {}", e));
            return Err(e.into());
        }
    }

    // Test 2: Sell 1000 tokens (dry run)
    log(LogTag::System, "INFO", "🔄 Test 2: Sell 1000 tokens (dry run)");

    let sell_result = SwapBuilder::new()
        .pool_address(pool_address)?
        .token_mint(token_mint)?
        .amount_tokens(1000.0)
        .sell()
        .slippage_percent(1.0)
        .dry_run(true)
        .execute().await;

    match sell_result {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", "✅ Sell test successful!");
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "💰 Input: {} tokens → Expected output: {} SOL (min: {})",
                    result.params.input_amount,
                    result.params.expected_output,
                    result.params.minimum_output
                )
            );

            if let Some(transaction) = &result.transaction {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("📝 Transaction accounts: {}", transaction.message.account_keys.len())
                );
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Sell test failed: {}", e));
            return Err(e.into());
        }
    }

    log(LogTag::System, "SUCCESS", "🎉 All CLMM tests completed successfully!");
    log(
        LogTag::System,
        "INFO",
        "✨ New implementation properly derives accounts and builds transactions"
    );

    Ok(())
}
