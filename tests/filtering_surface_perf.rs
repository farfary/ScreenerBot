//! Cost of OPENING the filtering tab, measured against the OWNER'S REAL DATABASE.
//!
//! `dashboard_startup_perf` covers the launch stall — the snapshot build and the endpoints
//! that used to wait on it. This tier covers what is left once nothing waits: the reads the
//! filtering tab itself issues the moment it is opened. Its three sub-tabs all read
//! `update_tracking` through the rejection columns, and every one of those reads is
//! a full scan plus a temp b-tree sort of the whole table unless the partial rejection
//! indexes exist:
//!
//! * [`status_tab_rejection_stats_are_index_driven`] — the GROUP BY behind the Status tab's
//!   rejection breakdown, which the page also re-polls every 5 seconds.
//! * [`analytics_tab_reads_are_index_driven`] — the three database reads the Analytics
//!   endpoint makes on one request, including the "most recently rejected" list whose
//!   `ORDER BY last_rejection_at DESC` sorted 430k rows to return 20.
//! * [`explore_tab_pages_are_index_driven`] — Explore always pages WITHIN one rejection
//!   reason, and its "last page" control issues a binary search of these same requests, so
//!   a slow page is paid five to seven times over.
//! * [`opening_the_filtering_tab_never_waits_for_a_snapshot`] — every read the tab issues on
//!   activation, run concurrently with NO snapshot built. This is the front delay itself:
//!   the wall-clock time between the click and the tab having its numbers.
//!
//! # Safety
//!
//! [`common::real_db_env`] clones the databases into a temp directory and repoints
//! `SCREENERBOT_DATA_DIR`, so the live files are never touched and the bot may be running.
//! The clone pays a one-time index build on first open (~3 s against the owner's database),
//! which is startup cost, not query cost — every measurement below takes a warm pass first.
//!
//! # Tier
//!
//! `#[ignore]`, real-data, one test per process. They self-skip when no real database
//! exists. Run with:
//! `cargo nextest run -E 'binary(filtering_surface_perf)' --run-ignored all -j1 --success-output immediate`

mod common;

use std::time::{Duration, Instant};

/// The Status tab's rejection breakdown, re-read every 5 seconds while the tab is open.
/// Covered by `idx_tracking_rejection_reason` it groups ~26 reasons out of an index; without
/// it, it scans all 453k `update_tracking` rows (measured 366 ms against 42 ms warm).
const REJECTION_STATS_CEILING: Duration = Duration::from_millis(700);

/// One Analytics request: the snapshot counters, the rejection breakdown and the recent
/// rejections list. The recent list alone took 1.1 s scanning-and-sorting the whole table;
/// entering through `idx_tracking_rejection_at` it stops after 20 index entries.
const ANALYTICS_READS_CEILING: Duration = Duration::from_millis(1_500);

/// One Explore page. It is a 50-row window inside a single rejection reason, so with
/// `idx_tracking_rejection_reason_at` it is a seek plus 50 steps regardless of how deep the
/// page is; without it, every page re-sorts every rejected row (measured ~45 ms each, and
/// the "last page" control fires up to seven of them in series).
const EXPLORE_PAGE_CEILING: Duration = Duration::from_millis(400);

/// Everything the tab asks for when it is opened, issued concurrently, with no snapshot.
/// This is what the user experiences as "the filtering tab takes a while to come up".
const TAB_OPEN_CEILING: Duration = Duration::from_secs(2);

/// Shared setup: clone the real databases, register the token DB, warm the decimals cache.
/// Returns `None` (already having printed a SKIP line) when there is no real database.
fn setup() -> Option<tempfile::TempDir> {
    let env = common::real_db_env()?;
    common::init_real_token_db();
    Some(env)
}

/// The rejection reason holding the most tokens — the one Explore opens on, and the worst
/// case for a query that has to sort within a reason.
async fn largest_rejection_reason() -> Option<(String, i64)> {
    let stats = screenerbot::tokens::get_rejection_stats_async()
        .await
        .expect("rejection stats");

    let mut by_reason: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for (reason, _source, count) in stats {
        *by_reason.entry(reason).or_default() += count;
    }

    by_reason.into_iter().max_by_key(|(_, count)| *count)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn status_tab_rejection_stats_are_index_driven() {
    let Some(_env) = setup() else { return };

    // Warm pass: the first touch of a freshly cloned file pays page-cache faults that have
    // nothing to do with the query plan.
    let _ = screenerbot::tokens::get_rejection_stats_async()
        .await
        .expect("warm rejection stats");

    let started = Instant::now();
    let stats = screenerbot::tokens::get_rejection_stats_async()
        .await
        .expect("rejection stats");
    let elapsed = started.elapsed();

    let rejected: i64 = stats.iter().map(|(_, _, count)| count).sum();
    eprintln!(
        "status tab rejection stats: {} reason/source groups over {rejected} rejected tokens in {elapsed:?}",
        stats.len(),
    );

    assert!(
        elapsed < REJECTION_STATS_CEILING,
        "grouping rejection reasons took {elapsed:?}, ceiling {REJECTION_STATS_CEILING:?} — \
         the query is scanning every update_tracking row instead of reading \
         idx_tracking_rejection_reason, and the Status tab re-runs it every 5 seconds",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn analytics_tab_reads_are_index_driven() {
    let Some(_env) = setup() else { return };

    // Warm pass over both reads the analytics endpoint makes.
    let _ = screenerbot::tokens::get_rejection_stats_with_time_filter_async(None, None).await;
    let _ = screenerbot::tokens::get_recent_rejections_async(20).await;

    let started = Instant::now();
    let breakdown = screenerbot::tokens::get_rejection_stats_with_time_filter_async(None, None)
        .await
        .expect("rejection breakdown");
    let breakdown_elapsed = started.elapsed();

    let started = Instant::now();
    let recent = screenerbot::tokens::get_recent_rejections_async(20)
        .await
        .expect("recent rejections");
    let recent_elapsed = started.elapsed();

    eprintln!(
        "analytics reads: breakdown {} groups in {breakdown_elapsed:?}, {} recent rejections in {recent_elapsed:?}",
        breakdown.len(),
        recent.len(),
    );

    // Called out separately because this is the one that used to dominate: 20 rows returned
    // after sorting every rejected token in the database.
    assert!(
        recent_elapsed < REJECTION_STATS_CEILING,
        "listing the 20 most recent rejections took {recent_elapsed:?}, ceiling \
         {REJECTION_STATS_CEILING:?} — ORDER BY last_rejection_at is sorting the whole \
         table instead of walking idx_tracking_rejection_at",
    );

    let total = breakdown_elapsed + recent_elapsed;
    assert!(
        total < ANALYTICS_READS_CEILING,
        "one analytics request spends {total:?} in the database, ceiling \
         {ANALYTICS_READS_CEILING:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn explore_tab_pages_are_index_driven() {
    let Some(_env) = setup() else { return };

    let Some((reason, count)) = largest_rejection_reason().await else {
        eprintln!("SKIP explore paging: no rejected tokens in the real database");
        return;
    };

    let page_size = 50;
    let deep_offset = ((count as usize).saturating_sub(page_size)).min(20_000);

    // Warm pass, so the measurement is of the plan rather than the filesystem.
    let _ = screenerbot::tokens::get_rejected_tokens_async(
        Some(reason.clone()),
        None,
        None,
        page_size,
        0,
    )
    .await;

    let started = Instant::now();
    let first = screenerbot::tokens::get_rejected_tokens_async(
        Some(reason.clone()),
        None,
        None,
        page_size,
        0,
    )
    .await
    .expect("first explore page");
    let first_elapsed = started.elapsed();

    let started = Instant::now();
    let deep = screenerbot::tokens::get_rejected_tokens_async(
        Some(reason.clone()),
        None,
        None,
        page_size,
        deep_offset,
    )
    .await
    .expect("deep explore page");
    let deep_elapsed = started.elapsed();

    eprintln!(
        "explore '{reason}' ({count} tokens): page 1 {} rows in {first_elapsed:?}, offset {deep_offset} {} rows in {deep_elapsed:?}",
        first.len(),
        deep.len(),
    );

    assert!(
        first_elapsed < EXPLORE_PAGE_CEILING,
        "the first Explore page took {first_elapsed:?}, ceiling {EXPLORE_PAGE_CEILING:?} — \
         paging within a rejection reason is re-sorting every rejected row instead of \
         seeking into idx_tracking_rejection_reason_at",
    );
    assert!(
        deep_elapsed < EXPLORE_PAGE_CEILING,
        "Explore page at offset {deep_offset} took {deep_elapsed:?}, ceiling \
         {EXPLORE_PAGE_CEILING:?} — a deep page must cost the same as a shallow one",
    );
}

/// The reproduction of the remaining front delay: open the tab, with nothing warmed and no
/// snapshot built, and time everything it asks for.
///
/// The requests go out CONCURRENTLY because that is what the page does — it no longer
/// awaits one loader before starting the next, and no panel waits for another panel's data.
/// So the tab's cost is the slowest single read, not their sum, and a regression that
/// re-serialises them shows up here as an immediate multiple.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real-data tier"]
async fn opening_the_filtering_tab_never_waits_for_a_snapshot() {
    let Some(_env) = setup() else { return };

    let reason = largest_rejection_reason().await.map(|(reason, _)| reason);

    // Deliberately NO snapshot: this is the state of a freshly launched app, and the state
    // in which the tab used to sit blank for tens of seconds.
    let started = Instant::now();
    let (stats, rejection_stats, analytics_breakdown, recent, explore_page) = tokio::join!(
        screenerbot::filtering::try_fetch_stats(),
        screenerbot::tokens::get_rejection_stats_async(),
        screenerbot::tokens::get_rejection_stats_with_time_filter_async(None, None),
        screenerbot::tokens::get_recent_rejections_async(20),
        async {
            match reason.clone() {
                Some(reason) => {
                    screenerbot::tokens::get_rejected_tokens_async(Some(reason), None, None, 50, 0)
                        .await
                }
                None => Ok(Vec::new()),
            }
        },
    );
    let elapsed = started.elapsed();

    let rejection_stats = rejection_stats.expect("rejection stats");
    let analytics_breakdown = analytics_breakdown.expect("analytics breakdown");
    let recent = recent.expect("recent rejections");
    let explore_page = explore_page.expect("explore page");

    eprintln!(
        "filtering tab open (cold, no snapshot): {elapsed:?} — snapshot present: {}, \
         {} rejection groups, {} analytics groups, {} recent, {} explore rows",
        stats.is_some(),
        rejection_stats.len(),
        analytics_breakdown.len(),
        recent.len(),
        explore_page.len(),
    );

    // The snapshot read is the one that must not block: it reports "not built yet" rather
    // than building one, and the tab renders that state instead of zeros.
    assert!(
        stats.is_none(),
        "a snapshot existed before the tab opened — either another test built one in this \
         process, or this call built one synchronously, which is the stall this tier exists \
         to catch",
    );

    assert!(
        elapsed < TAB_OPEN_CEILING,
        "opening the filtering tab cost {elapsed:?} of database work, ceiling \
         {TAB_OPEN_CEILING:?} — the tab is back to waiting on full table scans (or the \
         requests have been re-serialised behind one another)",
    );
}
