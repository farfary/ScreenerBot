//! Catalog coverage for event producers: every producer id renders its
//! source-locale sentence for a sample argument set.

use crate::i18n::{ids, MessageId, UiArg, UiText};

fn render(id: MessageId, args: &[(&'static str, &str)]) -> String {
    args.iter()
        .fold(UiText::new(id), |text, (name, value)| {
            text.arg(*name, UiArg::Text((*value).to_owned()))
        })
        .render_source_plain()
}

fn assert_table(table: &[(MessageId, &[(&'static str, &str)], &str)]) {
    for (id, args, expected) in table {
        assert_eq!(&render(*id, args), expected, "{id}");
    }
}

#[test]
fn ohlcv_producers_render_their_sentences() {
    let mint: &[(&str, &str)] = &[("mint", "MintA")];
    let mint_pool: &[(&str, &str)] = &[("mint", "MintA"), ("pool", "PoolB")];
    assert_table(&[
        (
            ids::EVENTS_OHLCV_TOKEN_MISSING,
            mint,
            "Token MintA was missing during processing",
        ),
        (
            ids::EVENTS_OHLCV_POOL_UNAVAILABLE,
            mint,
            "No healthy pools available for MintA; deferring",
        ),
        (
            ids::EVENTS_OHLCV_RATE_LIMIT_HIT,
            mint,
            "Rate limit triggered while processing MintA",
        ),
        (
            ids::EVENTS_OHLCV_PROCESS_TOKEN_ERROR,
            &[("mint", "MintA"), ("error", "boom")],
            "Error processing MintA: boom",
        ),
        (
            ids::EVENTS_OHLCV_POOL_DISCOVERY_SUCCESS,
            mint,
            "Discovered pools for MintA",
        ),
        (
            ids::EVENTS_OHLCV_POOL_DISCOVERY_FAILED,
            mint,
            "Pool discovery failed for MintA",
        ),
        (
            ids::EVENTS_OHLCV_EMPTY_FETCH,
            mint_pool,
            "Empty OHLCV fetch for MintA via PoolB",
        ),
        (
            ids::EVENTS_OHLCV_RETENTION_BACKFILL_FAILED,
            mint_pool,
            "Retention backfill failed for MintA via PoolB",
        ),
        (
            ids::EVENTS_OHLCV_FETCH_SUCCESS,
            &[("count", "12345"), ("mint", "MintA")],
            "Stored 12345 OHLCV points for MintA",
        ),
        (
            ids::EVENTS_OHLCV_GAP_DETECTION_FAILED,
            mint_pool,
            "Gap detection failed for MintA via PoolB",
        ),
        (
            ids::EVENTS_OHLCV_FETCH_FAILED,
            &[("mint", "MintA"), ("pool", "PoolB"), ("error", "boom")],
            "Failed to fetch OHLCV for MintA via PoolB: boom",
        ),
        (
            ids::EVENTS_OHLCV_BACKFILL_SCHEDULED,
            mint_pool,
            "Scheduled multi-timeframe backfill for MintA via PoolB",
        ),
        (
            ids::EVENTS_OHLCV_GAP_FILL_FAILED,
            mint,
            "Gap fill error for MintA",
        ),
        (
            ids::EVENTS_OHLCV_GAP_CLEANUP_FAILED,
            &[],
            "Failed to cleanup filled gap records",
        ),
        (
            ids::EVENTS_OHLCV_CACHE_CLEANUP_FAILED,
            &[],
            "Failed to cleanup OHLCV cache",
        ),
    ]);
}

#[test]
fn trader_producers_render_their_sentences() {
    assert_table(&[
        (
            ids::EVENTS_MONITORS_STARTING,
            &[],
            "Automated trading monitors starting up",
        ),
        (
            ids::EVENTS_MONITORS_STOPPED,
            &[],
            "Automated trading monitors stopped",
        ),
        (
            ids::EVENTS_ENTRY_MONITOR_STARTED,
            &[],
            "Entry opportunity monitor started",
        ),
        (
            ids::EVENTS_EXIT_MONITOR_STARTED,
            &[],
            "Exit/position monitor started",
        ),
        (
            ids::EVENTS_TRADER_SERVICE_INITIALIZING,
            &[],
            "Trader service initialization beginning",
        ),
        (
            ids::EVENTS_TRADER_TRADING_DISABLED,
            &[],
            "Trading is disabled in configuration",
        ),
        (
            ids::EVENTS_TRADER_TRADING_ENABLED,
            &[],
            "Trading is enabled and active",
        ),
        (
            ids::EVENTS_TRADER_AUTO_TRADING_ERROR,
            &[],
            "Auto trading encountered an error",
        ),
        (
            ids::EVENTS_TRADER_SERVICE_STARTED,
            &[],
            "Trader service fully initialized and running",
        ),
        (
            ids::EVENTS_TRADER_SERVICE_STOPPING,
            &[],
            "Trader service shutdown initiated",
        ),
        (
            ids::EVENTS_TRADER_SERVICE_STOPPED,
            &[],
            "Trader service gracefully stopped",
        ),
    ]);
}

#[test]
fn connectivity_producers_render_their_sentences() {
    assert_table(&[
        (
            ids::EVENTS_CONNECTIVITY_SERVICE_INITIALIZED,
            &[("count", "12345")],
            "Connectivity service initialized with 12345 monitors",
        ),
        (
            ids::EVENTS_CONNECTIVITY_MONITORING_STARTED,
            &[("seconds", "30")],
            "Connectivity monitoring started (interval=30s)",
        ),
        (
            ids::EVENTS_CONNECTIVITY_MONITORING_STOPPED,
            &[],
            "Connectivity monitoring stopped",
        ),
        (
            ids::EVENTS_CONNECTIVITY_ENDPOINT_RECOVERED,
            &[("from", "degraded")],
            "Endpoint recovered from degraded to healthy",
        ),
        (
            ids::EVENTS_CONNECTIVITY_CRITICAL_UNHEALTHY,
            &[("count", "2")],
            "2 critical endpoint(s) unhealthy - System should pause operations",
        ),
    ]);
}
