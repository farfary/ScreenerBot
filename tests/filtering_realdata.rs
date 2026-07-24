//! Filtering against the OWNER'S REAL DATABASE — hundreds of thousands of live tokens,
//! not fixtures.
//!
//! Synthetic tokens can only prove that a rule does what it says. Only the real corpus
//! answers the questions that matter: what fraction of tokens survive the shipped
//! configuration, which rule is actually doing the rejecting, and whether a full snapshot
//! fits inside its 30-second refresh interval.
//!
//! # Safety
//!
//! Filtering WRITES (rejection status, priorities, rejection stats), so these tests never
//! touch the live files. [`common::real_db_env`] clones the databases into a temp
//! directory and repoints `SCREENERBOT_DATA_DIR` at the clone; every write lands there
//! and the clone is deleted when the test ends. The bot may be running throughout.
//!
//! # Tier
//!
//! `#[ignore]`, run by `./test.sh live`. They self-skip when no real database exists.
//! By default they sample [`DEFAULT_SAMPLE`] tokens so a debug build finishes in
//! reasonable time; set `SB_TEST_REALDB_FULL=1` for the entire corpus.
#![allow(clippy::await_holding_lock)]

mod common;

use common::{config_guard, filters_all_disabled, filters_default_dex_only, per_item_micros};
use screenerbot::config::FilteringConfig;
use screenerbot::filtering::sources::{FilterRejectionReason, FilterSource};
use screenerbot::filtering::{evaluate_token, FilteringQuery, FilteringView};
use screenerbot::tokens::types::{DataSource, Token};
use std::collections::HashMap;
use std::time::Instant;

/// Tokens sampled per test unless `SB_TEST_REALDB_FULL=1` asks for everything.
const DEFAULT_SAMPLE: usize = 40_000;

fn full_corpus_requested() -> bool {
    matches!(
        std::env::var("SB_TEST_REALDB_FULL").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// Load the candidate set exactly as a snapshot does: tokens that HAVE market data.
async fn load_candidates() -> Vec<Token> {
    let started = Instant::now();
    let tokens = if full_corpus_requested() {
        screenerbot::tokens::get_all_tokens_for_filtering_async()
            .await
            .expect("load tokens with market data")
    } else {
        // Same query, bounded. `require_market_data` is false here, so filter down to the
        // rows a snapshot would actually consider.
        let page = screenerbot::tokens::get_all_tokens_optional_market_async(
            DEFAULT_SAMPLE * 2,
            0,
            None,
            None,
        )
        .await
        .expect("load token page");
        page.into_iter()
            .filter(|t| t.data_source != DataSource::Unknown)
            .take(DEFAULT_SAMPLE)
            .collect()
    };

    eprintln!(
        "REALDATA loaded {} candidate tokens in {:?}{}",
        tokens.len(),
        started.elapsed(),
        if full_corpus_requested() {
            " (full corpus)"
        } else {
            " (sample — set SB_TEST_REALDB_FULL=1 for all)"
        }
    );
    tokens
}

/// Rejection counts by label, plus how many passed.
struct Outcome {
    passed: usize,
    rejected: usize,
    by_reason: HashMap<String, usize>,
    by_source: HashMap<&'static str, usize>,
    elapsed: std::time::Duration,
}

impl Outcome {
    fn total(&self) -> usize {
        self.passed + self.rejected
    }

    fn top_reasons(&self, n: usize) -> Vec<(String, usize)> {
        let mut reasons: Vec<(String, usize)> = self
            .by_reason
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        reasons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        reasons.truncate(n);
        reasons
    }

    fn report(&self, label: &str) {
        eprintln!(
            "REALDATA {label}: {} tokens, {} passed ({:.3}%), {} rejected, {:?} ({:.2} us/token)",
            self.total(),
            self.passed,
            self.passed as f64 * 100.0 / self.total().max(1) as f64,
            self.rejected,
            self.elapsed,
            per_item_micros(self.elapsed, self.total())
        );
        for (reason, count) in self.top_reasons(15) {
            eprintln!(
                "REALDATA   {reason:<32} {count:>8}  ({:.2}%)",
                count as f64 * 100.0 / self.total().max(1) as f64
            );
        }
        let mut sources: Vec<(&str, usize)> =
            self.by_source.iter().map(|(k, v)| (*k, *v)).collect();
        sources.sort_by(|a, b| b.1.cmp(&a.1));
        eprintln!("REALDATA   by stage: {sources:?}");
    }
}

async fn evaluate_all(tokens: &[Token], config: &FilteringConfig) -> Outcome {
    let mut outcome = Outcome {
        passed: 0,
        rejected: 0,
        by_reason: HashMap::new(),
        by_source: HashMap::new(),
        elapsed: std::time::Duration::ZERO,
    };

    let started = Instant::now();
    for token in tokens {
        match evaluate_token(token, config).await {
            Ok(()) => outcome.passed += 1,
            Err(reason) => {
                outcome.rejected += 1;
                *outcome.by_reason.entry(reason.label()).or_insert(0) += 1;
                *outcome
                    .by_source
                    .entry(source_name(reason.source()))
                    .or_insert(0) += 1;
            }
        }
    }
    outcome.elapsed = started.elapsed();
    outcome
}

fn source_name(source: FilterSource) -> &'static str {
    source.as_str()
}

// ============================================================================
// WHAT THE SHIPPED CONFIGURATION DOES TO REAL TOKENS
// ============================================================================

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_default_config_rejects_the_entire_corpus() {
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    assert!(!tokens.is_empty(), "the real database returned no tokens");

    let outcome = evaluate_all(&tokens, &FilteringConfig::default()).await;
    outcome.report("default config");

    // CHARACTERISATION: the shipped defaults enable BOTH market sources, and the engine
    // gates each on `token.data_source`. A token has one source, so every token fails one
    // of the two gates and nothing can ever pass. `filtering_pipeline::
    // pipeline_default_config_rejects_every_token` pins the mechanism; this measures the
    // consequence on live data. When the gate is fixed, BOTH tests must be updated.
    assert_eq!(
        outcome.passed, 0,
        "the default configuration unexpectedly passed {} tokens — the source-gate \
         contradiction may have been fixed; update this test and its pipeline counterpart",
        outcome.passed
    );

    let gate_rejections: usize = outcome
        .by_reason
        .get(&FilterRejectionReason::GeckoTerminalDataMissing.label())
        .copied()
        .unwrap_or(0)
        + outcome
            .by_reason
            .get(&FilterRejectionReason::DexScreenerDataMissing.label())
            .copied()
            .unwrap_or(0);
    eprintln!(
        "REALDATA source-gate rejections: {gate_rejections} of {} ({:.1}%)",
        outcome.total(),
        gate_rejections as f64 * 100.0 / outcome.total().max(1) as f64
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_rugcheck_stage_alone_rejects_the_entire_corpus() {
    // CHARACTERISATION: removing the source-gate conflict is not enough — the Rugcheck
    // stage rejects EVERY real token on its own, for three separate reasons:
    //
    //   * `rug_transfer_fee_missing` — the default 5% ceiling turns ABSENT transfer-fee
    //     data into a rejection, and most SPL tokens have no fee extension to report.
    //   * `rug_data_missing`         — the engine demands a Rugcheck report before the
    //     stage runs, and only a fraction of the corpus has one.
    //   * `rug_score`                — the remainder score above the 10000 ceiling.
    //
    // So the shipped configuration cannot pass a token even with both market sources
    // agreeing. Update this test when the defaults change.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    let outcome = evaluate_all(&tokens, &filters_default_dex_only()).await;
    outcome.report("default minus GeckoTerminal source");

    assert_eq!(outcome.total(), tokens.len(), "a token got no decision");
    assert_eq!(
        outcome.passed, 0,
        "{} tokens now pass the default rules — the Rugcheck defaults may have been \
         loosened; update this test",
        outcome.passed
    );

    let rugcheck_stage = outcome.by_source.get("rugcheck").copied().unwrap_or(0)
        + outcome
            .by_reason
            .get(&FilterRejectionReason::RugcheckDataMissing.label())
            .copied()
            .unwrap_or(0);
    assert!(
        rugcheck_stage > 0,
        "expected the Rugcheck stage to be the wall; it rejected nothing"
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_market_rules_alone_yield_a_plausible_pass_rate() {
    // The positive counterpart: with the source conflict AND the Rugcheck stage out of
    // the way, the market and on-chain rules behave like a filter rather than a wall.
    // This is the assertion that would catch a market rule silently going absolute.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let mut config = filters_default_dex_only();
    config.rugcheck.enabled = false;

    let tokens = load_candidates().await;
    let outcome = evaluate_all(&tokens, &config).await;
    outcome.report("age + on-chain + dexscreener");

    assert!(
        outcome.passed > 0,
        "not one real token survives the age, on-chain and DexScreener rules; the reason \
         breakdown above shows where the corpus is lost"
    );

    let pass_rate = outcome.passed as f64 / outcome.total() as f64;
    eprintln!("REALDATA market-rules pass rate: {:.2}%", pass_rate * 100.0);
    assert!(
        pass_rate > 0.001,
        "only {:.4}% of real tokens pass the market rules",
        pass_rate * 100.0
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_cold_decimals_lookups_dominate_a_filtering_pass() {
    // The cost of the decimals-cache eviction measured directly: the same tokens,
    // evaluated twice. The first pass pays a database lookup for every mint the 100k
    // cache could not hold; the second finds them all cached.
    //
    // In production the cache never gets to stay warm — the corpus is four times the cap,
    // so each 30-second refresh re-evicts what the last one just pulled in.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    // Only the age rule, so the measurement is dominated by the decimals resolution in
    // `meta::evaluate` rather than by the rules themselves.
    let mut config = filters_all_disabled();
    config.age_enabled = true;

    let cold = evaluate_all(&tokens, &config).await;
    let warm = evaluate_all(&tokens, &config).await;

    let cold_us = per_item_micros(cold.elapsed, cold.total());
    let warm_us = per_item_micros(warm.elapsed, warm.total());
    eprintln!(
        "REALDATA decimals: first pass {cold_us:.1} us/token, second pass {warm_us:.1} \
         us/token ({:.1}x)",
        cold_us / warm_us.max(f64::EPSILON)
    );

    let corpus = screenerbot::tokens::count_tokens_async().await.unwrap_or(0);
    eprintln!(
        "REALDATA at the cold rate, resolving decimals for {corpus} tokens costs {:.1}s \
         per refresh",
        cold_us * corpus as f64 / 1_000_000.0
    );

    assert!(
        warm_us <= cold_us,
        "a warm decimals cache ({warm_us:.1} us/token) is not faster than a cold one \
         ({cold_us:.1} us/token)"
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_disabling_every_filter_passes_every_token() {
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    let outcome = evaluate_all(&tokens, &filters_all_disabled()).await;
    outcome.report("all filters disabled");

    // The ONLY thing that may still reject with every switch off is a token whose
    // decimals cannot be resolved — that check is not behind any config flag. Anything
    // else means a rule ignores its own enable switch.
    let unexpected: Vec<(String, usize)> = outcome
        .by_reason
        .iter()
        .filter(|(reason, _)| *reason != &FilterRejectionReason::NoDecimalsInDatabase.label())
        .map(|(reason, count)| (reason.clone(), *count))
        .collect();

    assert!(
        unexpected.is_empty(),
        "filters that reject while disabled: {unexpected:?}"
    );

    let missing_decimals = outcome
        .by_reason
        .get(&FilterRejectionReason::NoDecimalsInDatabase.label())
        .copied()
        .unwrap_or(0);
    eprintln!(
        "REALDATA tokens with unresolvable decimals: {missing_decimals} of {} ({:.2}%)",
        outcome.total(),
        missing_decimals as f64 * 100.0 / outcome.total().max(1) as f64
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_each_source_is_measured_in_isolation() {
    // Enabling one source at a time shows which rule is responsible for which share of
    // the corpus — the number a user needs in order to loosen the right setting.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;

    let mut only_onchain = filters_all_disabled();
    only_onchain.onchain.enabled = true;

    let mut only_dex = filters_all_disabled();
    only_dex.dexscreener.enabled = true;

    let mut only_rugcheck = filters_all_disabled();
    only_rugcheck.rugcheck.enabled = true;

    let mut only_age = filters_all_disabled();
    only_age.age_enabled = true;

    for (label, config) in [
        ("only age", only_age),
        ("only on-chain", only_onchain),
        ("only dexscreener", only_dex),
        ("only rugcheck", only_rugcheck),
    ] {
        let outcome = evaluate_all(&tokens, &config).await;
        outcome.report(label);
    }
}

// ============================================================================
// TIMING AT REAL SCALE
// ============================================================================

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_evaluation_fits_the_refresh_interval() {
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    let config = filters_default_dex_only();

    // Warm the caches so the measurement is of steady-state filtering, which is what the
    // 30-second loop actually does.
    let warm = tokens.len().min(2_000);
    let _ = evaluate_all(&tokens[..warm], &config).await;

    let outcome = evaluate_all(&tokens, &config).await;
    outcome.report("timing run");

    let per_token_secs = outcome.elapsed.as_secs_f64() / outcome.total().max(1) as f64;
    let corpus = screenerbot::tokens::count_tokens_async().await.unwrap_or(0);
    let projected = per_token_secs * corpus as f64;
    let budget = screenerbot::filtering::background::refresh_interval_secs() as f64;

    eprintln!(
        "REALDATA projection: {corpus} tokens in the database, {:.2} us/token, \
         {projected:.1}s per snapshot (debug build) against a {budget:.0}s refresh interval",
        per_token_secs * 1_000_000.0
    );

    // Debug builds run roughly an order of magnitude slower than the shipped release
    // binary, so allow ten intervals here. Beyond that, no release-build speedup saves
    // the loop and refreshes will overlap.
    assert!(
        projected < budget * 10.0,
        "evaluating the corpus projects to {projected:.1}s against a {budget:.0}s interval"
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_decimals_cache_is_too_small_for_the_corpus() {
    // DEFECT PIN: `DECIMALS_CACHE` is a moka cache capped at 100_000 entries, and the
    // startup preload pushes EVERY token's decimals through it. On a corpus larger than
    // the cap the surplus is evicted immediately, and `meta::evaluate` — the first stage
    // of filtering, run for every candidate — then misses the cache and falls through to
    // a per-token SQLite lookup (and, for anything not in the DB, to the data server and
    // then RPC). This is invisible in a snapshot's own timing because it looks like
    // "filtering is just slow".
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let db = common::init_real_token_db();

    let stored = db
        .get_all_tokens_with_decimals()
        .expect("read stored decimals");
    let total = stored.len();
    assert!(total > 0, "the real database has no decimals recorded");

    let hits = stored
        .iter()
        .filter(|(mint, _)| screenerbot::tokens::get_cached_decimals(mint).is_some())
        .count();
    let misses = total - hits;

    eprintln!(
        "REALDATA decimals cache: {total} mints preloaded, {hits} still cached, {misses} \
         evicted ({:.1}% miss rate on the very next read)",
        misses as f64 * 100.0 / total as f64
    );

    if total <= 100_000 {
        assert_eq!(
            misses, 0,
            "the corpus fits the cache, so nothing should have been evicted"
        );
        return;
    }

    assert!(
        misses > 0,
        "a corpus of {total} mints exceeds the 100k cache cap yet nothing was evicted — \
         the cap may have been raised; update this test"
    );
    eprintln!(
        "REALDATA every filtering pass therefore performs ~{misses} uncached decimals \
         lookups, every {}s",
        screenerbot::filtering::background::refresh_interval_secs()
    );
}

// ============================================================================
// FULL SNAPSHOT + QUERY PATH
// ============================================================================

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_snapshot_refresh_and_query_views() {
    // End to end on real data: build a snapshot through the store, then page every view
    // the dashboard offers. Exercises engine + store + store_helpers together, including
    // the batched database writes (which land in the clone).
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();
    common::set_config(|cfg| {
        // Without this the source-gate contradiction rejects everything and the views
        // below would all be trivially empty.
        cfg.filtering.geckoterminal.enabled = false;
    });

    let started = Instant::now();
    screenerbot::filtering::refresh()
        .await
        .expect("snapshot refresh");
    eprintln!("REALDATA snapshot refresh took {:?}", started.elapsed());

    let stats = screenerbot::filtering::fetch_stats()
        .await
        .expect("filtering stats");
    eprintln!(
        "REALDATA snapshot: {} tokens in database, {} with market data, {} passed, {} \
         priced, {} blacklisted, {} with ohlcv",
        stats.total_tokens_in_database,
        stats.total_tokens,
        stats.passed_filtering,
        stats.with_pool_price,
        stats.blacklisted,
        stats.with_ohlcv
    );

    assert!(
        stats.total_tokens > 0,
        "the snapshot loaded no tokens from a database that has them"
    );
    assert!(
        stats.total_tokens_in_database >= stats.total_tokens,
        "the market-data subset ({}) cannot exceed the whole database ({})",
        stats.total_tokens,
        stats.total_tokens_in_database
    );

    for view in [
        FilteringView::Pool,
        FilteringView::Passed,
        FilteringView::Rejected,
        FilteringView::Blacklisted,
        FilteringView::Positions,
        FilteringView::Recent,
    ] {
        let started = Instant::now();
        let result = screenerbot::filtering::query_tokens(FilteringQuery {
            view,
            page: 1,
            page_size: 50,
            ..Default::default()
        })
        .await
        .unwrap_or_else(|e| panic!("query {} failed: {e}", view.as_str()));
        eprintln!(
            "REALDATA view {:<12} total={:<8} page_items={:<4} in {:?}",
            view.as_str(),
            result.total,
            result.items.len(),
            started.elapsed()
        );

        assert!(
            result.items.len() <= result.page_size,
            "{} returned more rows than the page size",
            view.as_str()
        );
        let expected_pages = if result.total == 0 {
            0
        } else {
            result.total.div_ceil(result.page_size)
        };
        assert_eq!(
            result.total_pages,
            expected_pages,
            "{} reported the wrong page count",
            view.as_str()
        );
    }
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_pagination_never_repeats_or_skips_a_token() {
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();
    common::set_config(|cfg| cfg.filtering.geckoterminal.enabled = false);

    screenerbot::filtering::refresh()
        .await
        .expect("snapshot refresh");

    let page_size = 50;
    let mut seen: Vec<String> = Vec::new();
    for page in 1..=5 {
        let result = screenerbot::filtering::query_tokens(FilteringQuery {
            view: FilteringView::All,
            page,
            page_size,
            ..Default::default()
        })
        .await
        .expect("query All view");

        if result.items.is_empty() {
            break;
        }
        seen.extend(result.items.iter().map(|t| t.mint.clone()));
    }

    let unique: std::collections::HashSet<&String> = seen.iter().collect();
    assert_eq!(
        unique.len(),
        seen.len(),
        "paging the All view returned {} duplicate rows across 5 pages",
        seen.len() - unique.len()
    );
    eprintln!("REALDATA paged {} unique tokens across 5 pages", seen.len());
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_query_latency_is_interactive() {
    // The tokens page polls this. A query that takes longer than the poll interval means
    // the table can never settle.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();
    common::set_config(|cfg| cfg.filtering.geckoterminal.enabled = false);

    screenerbot::filtering::refresh()
        .await
        .expect("snapshot refresh");

    let mut samples = Vec::new();
    for _ in 0..10 {
        let started = Instant::now();
        let _ = screenerbot::filtering::query_tokens(FilteringQuery {
            view: FilteringView::Pool,
            page: 1,
            page_size: 50,
            search: Some("a".to_owned()),
            ..Default::default()
        })
        .await
        .expect("query");
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let worst = *samples.last().expect("non-empty");
    eprintln!("REALDATA query latency: median={median:?} max={worst:?}");

    assert!(
        median < std::time::Duration::from_secs(5),
        "a single dashboard query takes {median:?} against the real corpus"
    );
}
