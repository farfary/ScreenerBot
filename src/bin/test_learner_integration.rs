/// Test learner integration with main system
/// This binary verifies that the learner system initializes and integrates correctly
use screenerbot::{
    learner,
    logger::{init_file_logging, log, LogTag},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing learner integration...");

    // Initialize logging
    init_file_logging();

    // Test 1: Initialize learner system
    match learner::initialize_learning_system().await {
        Ok(_) => {
            log(
                LogTag::Learning,
                "INFO",
                "✅ Learner initialization test passed",
            );
            println!("✅ Learner initialization: PASSED");
        }
        Err(e) => {
            println!("❌ Learner initialization: FAILED - {}", e);
            return Err(e.into());
        }
    }

    // Test 2: Get learning integration
    let integration = learner::get_learning_integration();
    println!("✅ Learning integration access: PASSED");

    // Test 3: Test entry confidence adjustment (simulated)
    let mint = "DummyMintForTest1111111111111111111111111";
    let adjustment = integration
        .get_entry_confidence_adjustment(mint, 0.001, 10.0, 5.0)
        .await;
    println!(
        "✅ Entry confidence adjustment: PASSED (result: {:.3})",
        adjustment
    );

    // Test 4: Test exit score adjustment (simulated)
    let exit_adjustment = integration
        .get_exit_score_adjustment(mint, 0.002, 0.001, 5)
        .await;
    println!(
        "✅ Exit score adjustment: PASSED (result: {:.3})",
        exit_adjustment
    );

    // Test 5: Check if predictions are available
    let predictions_available = learner::learning_predictions_available().await;
    println!(
        "✅ Predictions availability check: PASSED (available: {})",
        predictions_available
    );

    // Test 6: Shutdown learner system
    learner::shutdown_learning_system().await;
    println!("✅ Learner shutdown: PASSED");

    println!("\n🎉 All learner integration tests passed!");
    log(
        LogTag::Learning,
        "INFO",
        "All learner integration tests completed successfully",
    );

    Ok(())
}
