/// Test program for route failure blacklist functionality
use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    tokens::blacklist::{
        get_blacklist_stats_db, initialize_blacklist_system, is_token_blacklisted,
        track_route_failure_db, MAX_NO_ROUTE_FAILURES,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    log(
        LogTag::System,
        "TEST_START",
        "🧪 Testing route failure blacklist functionality",
    );

    // Initialize blacklist system
    initialize_blacklist_system()?;

    // Test token details (using a real token from database)
    let test_mint = "EPD9qjtFaFrR3GvTPmPt8spmu4hfwUN6Dc5tHtDmpump";
    let test_symbol = "meme/coin";

    log(
        LogTag::System,
        "TEST_SETUP",
        &format!("Testing with token: {} ({})", test_symbol, test_mint),
    );

    // Check initial blacklist status
    let initially_blacklisted = is_token_blacklisted(test_mint);
    log(
        LogTag::System,
        "INITIAL_STATUS",
        &format!("Initially blacklisted: {}", initially_blacklisted),
    );

    // Simulate multiple route failures
    for i in 1..=MAX_NO_ROUTE_FAILURES + 1 {
        log(
            LogTag::System,
            "SIMULATE_FAILURE",
            &format!("Simulating route failure #{} for {}", i, test_symbol),
        );

        let still_allowed = track_route_failure_db(test_mint, test_symbol, "no_route");

        log(
            LogTag::System,
            "TRACK_RESULT",
            &format!("After failure #{}: still_allowed = {}", i, still_allowed),
        );

        // Check if token is now blacklisted
        let is_blacklisted = is_token_blacklisted(test_mint);
        log(
            LogTag::System,
            "BLACKLIST_CHECK",
            &format!("After failure #{}: blacklisted = {}", i, is_blacklisted),
        );

        if i >= MAX_NO_ROUTE_FAILURES {
            if is_blacklisted && !still_allowed {
                log(
                    LogTag::System,
                    "TEST_SUCCESS",
                    "✅ Token correctly blacklisted after 5 failures",
                );
            } else {
                log(
                    LogTag::System,
                    "TEST_FAILURE",
                    "❌ Token should be blacklisted but is not",
                );
            }
        }
    }

    // Show final blacklist statistics
    if let Some(stats) = get_blacklist_stats_db() {
        log(
            LogTag::System,
            "FINAL_STATS",
            &format!(
                "Final blacklist stats: {} total blacklisted, {} tracked",
                stats.total_blacklisted, stats.total_tracked
            ),
        );

        for (reason, count) in stats.reason_breakdown {
            log(
                LogTag::System,
                "REASON_BREAKDOWN",
                &format!("  {}: {}", reason, count),
            );
        }
    }

    log(
        LogTag::System,
        "TEST_COMPLETE",
        "🧪 Route failure blacklist test completed",
    );

    Ok(())
}
