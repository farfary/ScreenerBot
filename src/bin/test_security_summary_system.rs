use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    tokens::security::{
        get_security_analyzer, initialize_security_analyzer, start_security_summary_task,
    },
};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    log(
        LogTag::System,
        "TEST",
        "Starting security summary system test",
    );

    // Initialize security analyzer
    if let Err(e) = initialize_security_analyzer() {
        log(
            LogTag::System,
            "ERROR",
            &format!("Failed to initialize security analyzer: {}", e),
        );
        return Ok(());
    }

    log(LogTag::System, "SUCCESS", "Security analyzer initialized");

    // Start the summary task
    start_security_summary_task();
    log(LogTag::System, "TEST", "Summary task started");

    // Test some token analysis to generate metrics
    if let Some(analyzer) = get_security_analyzer() {
        log(
            LogTag::System,
            "TEST",
            "Testing token analysis to generate metrics...",
        );

        // Analyze a few tokens to create some metrics
        let test_tokens = [
            "B9Apdx78ZubBXy6evLKYeJpAfxGcsT9JVueGm3ND7AmD", // Should be in DB
            "3bfQ8XNzMbwhndPWRCW2ovoENe4CQw4As178bZ4eQvnA", // Should be in DB
            "InvalidTokenMint12345678901234567890123456",   // Should fail
        ];

        for token in &test_tokens {
            log(
                LogTag::System,
                "TEST",
                &format!("Analyzing token: {}", token),
            );
            let analysis = analyzer.analyze_token(token).await;
            log(
                LogTag::System,
                "TEST",
                &format!(
                    "Analysis result: authorities_safe={}, risk={:?}",
                    analysis.authorities_safe, analysis.risk_level
                ),
            );
        }

        // Wait for some summary reports
        log(
            LogTag::System,
            "TEST",
            "Waiting for summary reports (90 seconds to see 3 reports)...",
        );
        sleep(Duration::from_secs(90)).await;

        // Get final summary
        let summary = analyzer.get_security_summary().await;
        log(
            LogTag::System,
            "TEST",
            &format!(
                "Final summary - API calls: {}, Cache hits: {}, DB hits: {}, Tokens analyzed: {} (safe: {}, unsafe: {}, unknown: {})",
                summary.api_calls_total,
                summary.cache_hits,
                summary.db_hits,
                summary.tokens_analyzed,
                summary.tokens_safe,
                summary.tokens_unsafe,
                summary.tokens_unknown
            )
        );
    } else {
        log(LogTag::System, "ERROR", "Could not get security analyzer");
    }

    log(
        LogTag::System,
        "TEST",
        "Security summary system test completed",
    );
    Ok(())
}
