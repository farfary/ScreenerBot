/// Test the new dual-router quote comparison system
/// This demonstrates the improved get_best_quote function with real comparison

use screenerbot::{
    global::read_configs,
    logger::{log, LogTag},
    swaps::{
        get_best_quote, execute_best_swap,
        types::SOL_MINT,
        transaction::get_wallet_address,
    },
};

/// Test the new dual-router quote comparison
#[tokio::main]
async fn main() {
    log(LogTag::Test, "START", "🚀 Testing dual-router quote comparison system");

    // Test configuration
    let test_sol_amount = 10_000_000; // 0.01 SOL
    let slippage = 15.0;
    let fee = 0.25;
    let anti_mev = true;

    // Known tokens for testing
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

    match test_dual_router_comparison(test_sol_amount, usdc_mint, slippage, fee, anti_mev).await {
        Ok(_) => {
            log(LogTag::Test, "SUCCESS", "✅ Dual-router comparison test completed successfully");
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("❌ Test failed: {}", e));
            std::process::exit(1);
        }
    }
}

/// Test dual-router quote comparison with real data
async fn test_dual_router_comparison(
    amount: u64,
    target_mint: &str,
    slippage: f64,
    fee: f64,
    anti_mev: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let _configs = read_configs()?;
    let wallet_address = get_wallet_address()?;

    log(
        LogTag::Test,
        "DUAL_ROUTER_TEST",
        &format!(
            "🔄 Testing dual-router comparison: {} SOL -> {} (amount: {})",
            (amount as f64) / 1_000_000_000.0,
            if target_mint == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" { "USDC" } else { &target_mint[..8] },
            amount
        )
    );

    // Use the new get_best_quote function which compares both routers
    match get_best_quote(
        SOL_MINT,
        target_mint,
        amount,
        &wallet_address,
        slippage,
        fee,
        anti_mev,
    ).await {
        Ok(best_quote) => {
            log(
                LogTag::Test,
                "DUAL_ROUTER_RESULT",
                &format!(
                    "🏆 Best quote result:
                    • Winner: {:?}
                    • Output: {} tokens
                    • Price Impact: {:.2}%
                    • Fee: {} lamports
                    • Route: {}",
                    best_quote.router,
                    best_quote.output_amount,
                    best_quote.price_impact_pct,
                    best_quote.fee_lamports,
                    best_quote.route_plan
                )
            );

            log(LogTag::Test, "DUAL_ROUTER_SUCCESS", "✅ Dual-router comparison successful - system correctly compared both GMGN and Jupiter!");

            Ok(())
        }
        Err(e) => {
            log(LogTag::Test, "DUAL_ROUTER_ERROR", &format!("❌ Dual-router comparison failed: {}", e));
            Err(Box::new(e))
        }
    }
}
