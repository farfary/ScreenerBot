//! Dashboard first-paint cost, measured against the OWNER'S REAL DATABASE.
//!
//! The launch symptom this tier exists to catch: the app window opens, the loading screen
//! clears, and then every panel — header, home hero, featured row — sits in its skeleton
//! for tens of seconds while the tabs refuse to switch. That is not a rendering problem,
//! it is the API handlers behind those panels blocking on the first filtering snapshot,
//! and every token-database read in the process queuing behind the one connection mutex
//! that snapshot holds.
//!
//! So the measurements here are deliberately the ones a user actually waits on:
//!
//! * [`candidate_load_is_index_driven`] — the batch load a snapshot opens with. It must be
//!   driven by the market-data timestamp indexes, not a full scan of every token row.
//! * [`first_snapshot_fits_refresh_interval`] — the whole `compute_snapshot`, which the
//!   refresh loop repeats every 30s. Overrunning that means the bot is permanently
//!   rebuilding.
//! * [`header_stats_do_not_block_on_first_snapshot`] — the header endpoint's stats read,
//!   taken with NO snapshot built. This is the one that produced the 30-second hang.
//! * [`token_count_is_cheap_under_snapshot_load`] — a count served while a snapshot build
//!   holds the token database, i.e. exactly the contention the launch sequence creates.
//!
//! # Safety
//!
//! Filtering WRITES (rejection status, priorities, stats), so this never touches the live
//! files: [`common::real_db_env`] clones the databases into a temp directory and repoints
//! `SCREENERBOT_DATA_DIR` at the clone. The bot may be running throughout.
//!
//! # Tier
//!
//! `#[ignore]`, real-data. They self-skip when no real database exists. Run with:
//! `cargo nextest run -E 'binary(dashboard_startup_perf)' --run-ignored all --success-output immediate`
#![allow(clippy::await_holding_lock)]

mod common;

use std::time::{Duration, Instant};

/// Ceiling for the candidate batch load. The query returns single-digit thousands of rows
/// on the owner's database; anything approaching a second means it went back to scanning
/// the whole `tokens` table instead of entering through a market-data index. Measured at
/// ~0.37s index-driven, against ~3.1s for the scan it replaced.
const CANDIDATE_LOAD_CEILING: Duration = Duration::from_millis(1_500);

/// The refresh loop rebuilds every 30s. A build that does not comfortably fit inside that
/// leaves the bot rebuilding continuously, holding the token database the whole time.
const SNAPSHOT_BUILD_CEILING: Duration = Duration::from_secs(25);

/// The header polls roughly once a second and is the first thing a launched app paints.
/// With no snapshot yet built it must answer immediately rather than wait for one.
const HEADER_STATS_CEILING: Duration = Duration::from_millis(500);

/// A COUNT served while a snapshot build is running. Deliberately set BELOW the duration
/// of the build's own batch load (~2.1s): a count that takes longer than this is not
/// answering, it is queuing. Measured at ~0.16s pooled, against ~9.0s when every token
/// read shared one connection.
const CONTENDED_COUNT_CEILING: Duration = Duration::from_secs(2);

/// Shared setup: clone the real databases, register the token DB, warm the decimals cache.
/// Returns `None` (already having printed a SKIP line) when there is no real database.
fn setup() -> Option<tempfile::TempDir> {
    let env = common::real_db_env()?;
    common::init_real_token_db();
    Some(env)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn candidate_load_is_index_driven() {
    let Some(_env) = setup() else { return };

    // Warm pass first: the first touch of a freshly cloned file pays page-cache faults
    // that have nothing to do with the query plan.
    let _ = screenerbot::tokens::get_all_tokens_for_filtering_async()
        .await
        .expect("warm candidate load");

    let started = Instant::now();
    let tokens = screenerbot::tokens::get_all_tokens_for_filtering_async()
        .await
        .expect("candidate load");
    let elapsed = started.elapsed();

    let total = screenerbot::tokens::count_tokens_async()
        .await
        .expect("count tokens");

    eprintln!(
        "candidate load: {} of {} tokens in {:?} ({:.1} us/token, {:.1}% of corpus)",
        tokens.len(),
        total,
        elapsed,
        common::per_item_micros(elapsed, tokens.len()),
        if total == 0 {
            0.0
        } else {
            tokens.len() as f64 / total as f64 * 100.0
        },
    );

    assert!(
        elapsed < CANDIDATE_LOAD_CEILING,
        "candidate load took {elapsed:?}, ceiling {CANDIDATE_LOAD_CEILING:?} — \
         the query is almost certainly scanning all {total} token rows instead of \
         entering through market_data_last_fetched_at",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn first_snapshot_fits_refresh_interval() {
    let Some(_env) = setup() else { return };

    let started = Instant::now();
    screenerbot::filtering::refresh()
        .await
        .expect("build first snapshot");
    let elapsed = started.elapsed();

    let stats = screenerbot::filtering::try_fetch_stats()
        .await
        .expect("stats present after an explicit refresh");

    eprintln!(
        "first snapshot: {:?} for {} tokens ({} passed, {} blacklisted)",
        elapsed, stats.total_tokens, stats.passed_filtering, stats.blacklisted,
    );

    assert!(
        elapsed < SNAPSHOT_BUILD_CEILING,
        "snapshot build took {elapsed:?}, ceiling {SNAPSHOT_BUILD_CEILING:?} — \
         the 30s refresh loop cannot keep up, so the token database is held continuously",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn header_stats_do_not_block_on_first_snapshot() {
    let Some(_env) = setup() else { return };

    // Deliberately do NOT build a snapshot first. This is the state a freshly launched
    // app is in, and the state in which the header used to hang for its full 30s timeout.
    let started = Instant::now();
    let stats = screenerbot::filtering::try_fetch_stats().await;
    let elapsed = started.elapsed();

    eprintln!(
        "header stats with no snapshot: {:?} (stats present: {})",
        elapsed,
        stats.is_some(),
    );

    assert!(
        elapsed < HEADER_STATS_CEILING,
        "the header's stats read took {elapsed:?}, ceiling {HEADER_STATS_CEILING:?} — \
         a first-paint endpoint is waiting for the first snapshot to be built",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn token_count_is_cheap_under_snapshot_load() {
    let Some(_env) = setup() else { return };

    // Kick off a real snapshot build and, while it is running, ask for the count the home
    // dashboard's token panel needs. On launch these two genuinely do overlap.
    let build = tokio::spawn(async { screenerbot::filtering::refresh().await });

    // Give the build long enough to be inside its batch load, holding the connection.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let started = Instant::now();
    let count = screenerbot::tokens::count_tokens_async()
        .await
        .expect("count tokens under load");
    let elapsed = started.elapsed();

    eprintln!("contended count: {count} tokens in {elapsed:?}");

    let _ = build.await;

    assert!(
        elapsed < CONTENDED_COUNT_CEILING,
        "counting tokens during a snapshot build took {elapsed:?}, ceiling \
         {CONTENDED_COUNT_CEILING:?} — reads are serialising behind the build on the \
         single token-database connection",
    );
}
