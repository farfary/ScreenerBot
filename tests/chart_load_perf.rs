// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chart load cost of the token and position dialogs, measured against the OWNER'S REAL DATABASE.
//!
//! A dialog opens on its saved timeframe (5m by default) and every timeframe switch reads one
//! more series through `ohlcvs::get_chart_ohlcv`, the read behind `GET /api/tokens/:mint/ohlcv`.
//! A stored series must draw at once, whatever else the OHLCV runtime is doing:
//!
//! * [`a_charted_token_opens_every_timeframe_from_disk`] — the token holding the most candles
//!   with rows in all seven timeframes: the first open (which also builds the OHLCV runtime on
//!   the clone) and then every other timeframe, each answered from storage.
//! * [`a_signed_out_chart_never_waits_on_the_network`] — a token missing at least one coarse
//!   timeframe: without a Data Server session that timeframe is aggregated from the stored 1m
//!   rows at once, never fetched.
//!
//! The Data-Server-served leg (a cold timeframe answered `pending`, then served) needs a live
//! session and cannot run in-process; its scheduling floor is pinned by the read-through tests
//! in `src/ohlcvs/service.rs`.
//!
//! # Safety
//!
//! [`common::real_db_env`] clones `tokens.db`, `pools.db`, `ohlcvs.db` and `transactions.db`
//! into a temp directory and repoints `SCREENERBOT_DATA_DIR`, so the live files are never
//! touched and the bot may be running. The account store is never cloned, so the test process
//! is signed out and no read reaches the Data Server.
//!
//! # Tier
//!
//! `#[ignore]`, real-data, one test per process. They self-skip when no real database or no
//! suitable token exists. Each timeframe prints one
//! `chart-load-timing {"case":…,"timeframe":…,"ms":…}` line. Run with:
//! `cargo nextest run -E 'test(/^chart_load_perf::/)' --run-ignored all -j1 --success-output immediate`

use crate::common;

use std::path::Path;
use std::time::{Duration, Instant};

use screenerbot::chains::ChainId;
use screenerbot::ohlcvs::Timeframe;

/// The cloned real database is the Solana OHLCV database.
const CHAIN: ChainId = ChainId::Solana;

/// The dialog's first chart read, including building the OHLCV runtime on the clone. A stored
/// series reads in milliseconds; the margin is the runtime build in a debug binary.
const FIRST_OPEN_CEILING: Duration = Duration::from_millis(1_500);

/// One timeframe switch on a stored series, and any read of a signed-out chart.
const SWITCH_CEILING: Duration = Duration::from_millis(300);

/// The token holding the most candles with rows in all seven timeframes on its series pool.
const FULLY_CHARTED_TOKEN: &str = "\
    SELECT c.mint FROM ohlcv_candles c \
    JOIN ohlcv_pools p ON p.chain_id = c.chain_id AND p.mint = c.mint \
        AND p.pool_address = c.pool_address AND p.is_default = 1 \
    WHERE c.chain_id = 'solana' \
    GROUP BY c.mint HAVING COUNT(DISTINCT c.timeframe) = 7 \
    ORDER BY COUNT(*) DESC LIMIT 1";

/// The token with a series pool and no rows in at least one coarse timeframe that holds the
/// most 1m rows, so the signed-out aggregate has the most to read.
const PARTLY_CHARTED_TOKEN: &str = "\
    SELECT p.mint FROM ohlcv_pools p \
    WHERE p.chain_id = 'solana' AND p.is_default = 1 \
        AND (SELECT COUNT(DISTINCT c.timeframe) FROM ohlcv_candles c \
            WHERE c.chain_id = p.chain_id AND c.mint = p.mint \
                AND c.pool_address = p.pool_address AND c.timeframe <> '1m') < 6 \
    ORDER BY (SELECT COUNT(*) FROM ohlcv_candles c \
        WHERE c.chain_id = p.chain_id AND c.mint = p.mint \
            AND c.pool_address = p.pool_address AND c.timeframe = '1m') DESC \
    LIMIT 1";

/// The first mint `query` selects on the cloned OHLCV database, `None` (with a SKIP line)
/// when no token matches.
fn select_mint(data_dir: &Path, query: &str) -> Option<String> {
    let conn = rusqlite::Connection::open_with_flags(
        data_dir.join("data").join("ohlcvs.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open cloned ohlcvs.db");
    let mint = conn
        .query_row(query, [], |row| row.get::<_, String>(0))
        .ok();
    if mint.is_none() {
        eprintln!("SKIP chart-load: no token in the clone matches the case");
    }
    mint
}

/// Read one timeframe the way a chart dialog does, print its timing line and return the
/// candle count and the elapsed time.
async fn timed_read(case: &str, mint: &str, timeframe: Timeframe) -> (usize, Duration) {
    let started = Instant::now();
    let candles = screenerbot::ohlcvs::get_chart_ohlcv(CHAIN, mint, timeframe, 0)
        .await
        .unwrap_or_else(|e| panic!("chart read of {timeframe} failed: {e}"));
    let elapsed = started.elapsed();
    eprintln!(
        "chart-load-timing {{\"case\":\"{case}\",\"timeframe\":\"{}\",\"ms\":{:.1}}}",
        timeframe.as_str(),
        elapsed.as_secs_f64() * 1_000.0
    );
    (candles.len(), elapsed)
}

fn assert_signed_out() {
    assert!(
        !screenerbot::data_server::is_usable(screenerbot::data_server::Surface::Ohlcv),
        "the real-data tier must run signed out"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn a_charted_token_opens_every_timeframe_from_disk() {
    let Some(env) = common::real_db_env() else {
        return;
    };
    let Some(mint) = select_mint(env.path(), FULLY_CHARTED_TOKEN) else {
        return;
    };
    assert_signed_out();
    eprintln!("chart-load: mint={mint}");

    let (count, elapsed) = timed_read("stored", &mint, Timeframe::Minute5).await;
    assert!(count > 0, "5m answered no candles");
    assert!(
        elapsed < FIRST_OPEN_CEILING,
        "first open took {elapsed:?}, ceiling {FIRST_OPEN_CEILING:?}"
    );

    for timeframe in Timeframe::all()
        .into_iter()
        .filter(|timeframe| *timeframe != Timeframe::Minute5)
    {
        let (count, elapsed) = timed_read("stored", &mint, timeframe).await;
        assert!(count > 0, "{timeframe} answered no candles");
        assert!(
            elapsed < SWITCH_CEILING,
            "switch to {timeframe} took {elapsed:?}, ceiling {SWITCH_CEILING:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn a_signed_out_chart_never_waits_on_the_network() {
    let Some(env) = common::real_db_env() else {
        return;
    };
    let Some(mint) = select_mint(env.path(), PARTLY_CHARTED_TOKEN) else {
        return;
    };
    assert_signed_out();
    eprintln!("chart-load: mint={mint}");

    // The first read builds the OHLCV runtime on the clone; it is bounded like a dialog open.
    let (_, elapsed) = timed_read("signed-out", &mint, Timeframe::Minute1).await;
    assert!(
        elapsed < FIRST_OPEN_CEILING,
        "first open took {elapsed:?}, ceiling {FIRST_OPEN_CEILING:?}"
    );
    for timeframe in Timeframe::all()
        .into_iter()
        .filter(|timeframe| *timeframe != Timeframe::Minute1)
    {
        let (_, elapsed) = timed_read("signed-out", &mint, timeframe).await;
        assert!(
            elapsed < SWITCH_CEILING,
            "{timeframe} took {elapsed:?}, ceiling {SWITCH_CEILING:?}"
        );
    }
}
