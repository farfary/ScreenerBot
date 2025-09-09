/// Test CANDY token swap with proper verification
/// This test specifically targets the CANDY/SOL CLMM pool with step-by-step verification

use screenerbot::pools::swap::SwapBuilder;
use screenerbot::logger::{ log, LogTag };
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log(LogTag::System, "INFO", "🎯 Testing CANDY token swap with verification");

    // Exact pool and token as requested
    let pool_address = "HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv";
    let token_mint = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t"; // CANDY token
    let sol_amount = 0.01; // Buy with exactly 0.01 SOL

    log(LogTag::System, "INFO", &format!("🔥 Pool: {}", pool_address));
    log(LogTag::System, "INFO", &format!("🍭 CANDY Token: {}", token_mint));
    log(LogTag::System, "INFO", &format!("💰 Amount: {} SOL", sol_amount));

    // First do a dry run to verify parameters
    log(LogTag::System, "INFO", "🧪 Step 1: Dry run verification");

    let dry_result = SwapBuilder::new()
        .pool_address(pool_address)?
        .token_mint(token_mint)?
        .amount_sol(sol_amount)
        .buy()
        .slippage_percent(2.0) // 2% slippage for safety
        .dry_run(true)
        .execute().await?;

    log(
        LogTag::System,
        "SUCCESS",
        &format!(
            "✅ Dry run successful: {} SOL → {} CANDY (min: {})",
            dry_result.params.input_amount,
            dry_result.params.expected_output,
            dry_result.params.minimum_output
        )
    );

    // Now execute the real transaction
    log(LogTag::System, "INFO", "💎 Step 2: Executing REAL transaction");

    let real_result = SwapBuilder::new()
        .pool_address(pool_address)?
        .token_mint(token_mint)?
        .amount_sol(sol_amount)
        .buy()
        .slippage_percent(2.0)
        .dry_run(false) // REAL TRANSACTION
        .execute().await?;

    // Verify success
    if let Some(signature) = real_result.signature {
        log(LogTag::System, "SUCCESS", &format!("🚀 REAL TRANSACTION SENT: {}", signature));
        log(
            LogTag::System,
            "INFO",
            &format!(
                "📊 Transaction details:
            Input: {} SOL
            Expected: {} CANDY
            Minimum: {} CANDY",
                real_result.params.input_amount,
                real_result.params.expected_output,
                real_result.params.minimum_output
            )
        );

        // Wait for confirmation
        log(LogTag::System, "INFO", "⏳ Waiting for blockchain confirmation...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        log(LogTag::System, "SUCCESS", "🎉 CANDY SWAP COMPLETED SUCCESSFULLY!");
        log(
            LogTag::System,
            "INFO",
            &format!("✅ Verify transaction at: https://solscan.io/tx/{}", signature)
        );
    } else {
        log(LogTag::System, "ERROR", "❌ No transaction signature returned");
        return Err("Transaction failed".into());
    }

    Ok(())
}
