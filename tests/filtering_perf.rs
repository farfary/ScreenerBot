//! Filtering timing and throughput — how the pipeline behaves at scale, and which parts
//! of it cost more than they should.
//!
//! These run in a DEBUG build, where absolute numbers mean little, so the assertions are
//! deliberately of two kinds:
//!
//! * **Shape** — doubling the input must not more than quadruple the time. This is what
//!   catches an accidental quadratic; it holds regardless of machine or build profile.
//! * **Ceilings** — an order of magnitude above the observed debug cost. They only fire
//!   on a catastrophic regression, never on a slow laptop.
//!
//! Every test also prints its measurement, so `cargo nextest run --no-capture` (or the
//! captured output of a failure) is a usable profile of the filtering pipeline.
#![allow(clippy::await_holding_lock)]

mod common;

use common::{config_guard, filter_token, filters_default_dex_only, holder, per_item_micros};
use screenerbot::config::schemas::{DexScreenerFilters, OnChainFilters, RugCheckFilters};
use screenerbot::config::FilteringConfig;
use screenerbot::filtering::evaluate_token;
use screenerbot::filtering::sources::{dexscreener, onchain, rugcheck};
use screenerbot::tokens::types::Token;
use std::time::{Duration, Instant};

/// Big enough to average out scheduler noise, small enough to stay well inside the
/// decimals cache (100k entries) and to keep a debug run near a second.
const CORPUS: usize = 8_000;

/// A distinct, decimals-seeded token per index.
fn corpus(size: usize) -> Vec<Token> {
    (0..size)
        .map(|i| {
            let mint = format!("PerfMint{i:035}");
            common::seed_decimals(&mint, 9);
            let mut token = filter_token(&mint);
            // Vary the numbers so no branch predictor or cache trick makes this
            // unrealistically uniform.
            token.symbol = format!("PERF{i}");
            token.liquidity_usd = Some(10_000.0 + (i % 1_000) as f64);
            token.market_cap = Some(250_000.0 + (i % 5_000) as f64);
            token.volume_h24 = Some(50_000.0 + (i % 900) as f64);
            token
        })
        .collect()
}

async fn time_pipeline(tokens: &[Token], config: &FilteringConfig) -> Duration {
    let started = Instant::now();
    for token in tokens {
        let _ = evaluate_token(token, config).await;
    }
    started.elapsed()
}

fn report(label: &str, elapsed: Duration, items: usize) {
    eprintln!(
        "PERF {label}: {items} items in {elapsed:?} ({:.2} us/item, {:.0} items/s)",
        per_item_micros(elapsed, items),
        items as f64 / elapsed.as_secs_f64().max(f64::EPSILON)
    );
}

// ============================================================================
// PIPELINE THROUGHPUT
// ============================================================================

#[tokio::test]
async fn perf_pipeline_throughput_for_passing_tokens() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();
    let tokens = corpus(CORPUS);

    // Warm the caches and the branch layout before measuring.
    let _ = time_pipeline(&tokens[..500], &config).await;

    let elapsed = time_pipeline(&tokens, &config).await;
    report("pipeline/pass", elapsed, tokens.len());

    let per_token = per_item_micros(elapsed, tokens.len());
    assert!(
        per_token < 250.0,
        "a full pipeline pass costs {per_token:.2} us/token; a 300k-token snapshot would \
         take {:.0}s of pure evaluation",
        per_token * 300_000.0 / 1_000_000.0
    );
}

#[tokio::test]
async fn perf_pipeline_scales_linearly_with_corpus_size() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let small = corpus(2_000);
    let large = corpus(8_000); // 4x

    // Warm, then measure each size twice and keep the better run — a single sample can
    // be poisoned by an unrelated thread on the machine.
    let _ = time_pipeline(&small, &config).await;
    let small_time = std::cmp::min(
        time_pipeline(&small, &config).await,
        time_pipeline(&small, &config).await,
    );
    let large_time = std::cmp::min(
        time_pipeline(&large, &config).await,
        time_pipeline(&large, &config).await,
    );

    report("pipeline/2k", small_time, small.len());
    report("pipeline/8k", large_time, large.len());

    let growth = large_time.as_secs_f64() / small_time.as_secs_f64().max(f64::EPSILON);
    eprintln!("PERF pipeline growth for 4x input: {growth:.2}x");
    assert!(
        growth < 8.0,
        "4x the tokens cost {growth:.2}x the time — per-token work is growing with corpus \
         size, which is quadratic in a snapshot"
    );
}

#[tokio::test]
async fn perf_early_rejection_is_cheaper_than_a_full_pass() {
    // The pipeline is ordered cheapest-first on purpose: on-chain scam detection exists
    // so the expensive market and security rules never run for junk. If a rejection at
    // stage two cost as much as a full pass, that ordering would be buying nothing.
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let passing = corpus(CORPUS);
    let mut rejected_early = passing.clone();
    for token in rejected_early.iter_mut() {
        token.symbol = "0000".to_owned(); // on-chain stage
    }

    let _ = time_pipeline(&passing[..500], &config).await;
    let pass_time = time_pipeline(&passing, &config).await;
    let reject_time = time_pipeline(&rejected_early, &config).await;

    report("pipeline/pass", pass_time, passing.len());
    report("pipeline/reject-early", reject_time, rejected_early.len());

    assert!(
        reject_time <= pass_time,
        "rejecting at the on-chain stage ({reject_time:?}) costs more than running every \
         later stage ({pass_time:?}) — the cheap-first ordering is inverted"
    );
}

#[tokio::test]
async fn perf_pipeline_latency_tail_is_bounded() {
    // A snapshot is a serial loop, so one pathological token stalls every token behind
    // it. Measure the distribution, not just the mean.
    let _cfg = config_guard();
    let config = filters_default_dex_only();
    let tokens = corpus(4_000);

    let _ = time_pipeline(&tokens[..500], &config).await;

    let mut samples: Vec<Duration> = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let started = Instant::now();
        let _ = evaluate_token(token, &config).await;
        samples.push(started.elapsed());
    }
    samples.sort_unstable();

    let p50 = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];
    let worst = *samples.last().expect("non-empty");
    eprintln!("PERF pipeline latency: p50={p50:?} p99={p99:?} max={worst:?}");

    assert!(
        p99 < Duration::from_millis(5),
        "p99 per-token latency is {p99:?}; at 300k tokens the slow 1% alone would add \
         {:.0}s to a snapshot",
        p99.as_secs_f64() * 3_000.0
    );
}

// ============================================================================
// WHERE THE TIME GOES
// ============================================================================

#[tokio::test]
async fn perf_cost_per_source_is_reported() {
    // Not an assertion about any one source — a printed breakdown so a regression can be
    // attributed to a stage instead of guessed at.
    let _cfg = config_guard();
    let tokens = corpus(CORPUS);

    let onchain_config = OnChainFilters::default();
    let started = Instant::now();
    for token in &tokens {
        let _ = onchain::evaluate(token, &onchain_config);
    }
    report("source/onchain", started.elapsed(), tokens.len());

    let dex_config = DexScreenerFilters {
        volume_enabled: true,
        price_change_enabled: true,
        fdv_enabled: true,
        ..Default::default()
    };
    let started = Instant::now();
    for token in &tokens {
        let _ = dexscreener::evaluate(token, &dex_config);
    }
    report("source/dexscreener", started.elapsed(), tokens.len());

    let rug_config = RugCheckFilters::default();
    let started = Instant::now();
    for token in &tokens {
        let _ = rugcheck::evaluate(token, &rug_config);
    }
    report("source/rugcheck", started.elapsed(), tokens.len());

    let config = filters_default_dex_only();
    let combined = time_pipeline(&tokens, &config).await;
    report("source/combined", combined, tokens.len());
}

#[tokio::test]
async fn perf_rugcheck_cost_grows_with_the_holder_list() {
    // DEFECT PIN: `check_holder_distribution` CLONES the whole `top_holders` vector and
    // sorts the copy on every single evaluation, only to read the largest entry and the
    // top three. That is an allocation plus an O(n log n) sort per token per snapshot,
    // for information a single linear scan would give. Rugcheck reports commonly carry
    // large holder lists, so this is paid for real tokens, every 30 seconds.
    let config = RugCheckFilters {
        max_top_holder_pct: 100.0,
        max_top_3_holders_pct: 100.0,
        max_insider_total_pct: 100.0,
        max_insider_holders_in_top_10: 10,
        ..Default::default()
    };

    fn measure(holders: usize, config: &RugCheckFilters, rounds: usize) -> Duration {
        let mut token = filter_token("HolderMint111111111111111111111111111111111");
        token.top_holders = (0..holders)
            .map(|i| holder(&format!("H{i:042}"), 0.001, false))
            .collect();

        let started = Instant::now();
        for _ in 0..rounds {
            let _ = rugcheck::evaluate(&token, config);
        }
        started.elapsed()
    }

    let rounds = 2_000;
    let _ = measure(10, &config, 200); // warm
    let small = measure(10, &config, rounds);
    let large = measure(1_000, &config, rounds);

    report("rugcheck/10-holders", small, rounds);
    report("rugcheck/1000-holders", large, rounds);

    let growth = large.as_secs_f64() / small.as_secs_f64().max(f64::EPSILON);
    eprintln!("PERF rugcheck growth for 100x holders: {growth:.1}x");

    assert!(
        growth < 400.0,
        "a 100x larger holder list costs {growth:.1}x more per token — worse than the \
         O(n log n) clone-and-sort already implies"
    );
}

#[tokio::test]
async fn perf_degenerate_values_do_not_slow_the_pipeline_down() {
    // NaN and infinity take different branches through the comparisons; make sure they
    // are not a slow path an attacker (or a broken provider) could use to stall a
    // snapshot.
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let clean = corpus(CORPUS);
    let mut degenerate = clean.clone();
    for token in degenerate.iter_mut() {
        token.liquidity_usd = Some(f64::NAN);
        token.market_cap = Some(f64::INFINITY);
        token.volume_h24 = Some(f64::NEG_INFINITY);
        token.price_change_h24 = Some(f64::NAN);
    }

    let _ = time_pipeline(&clean[..500], &config).await;
    let clean_time = time_pipeline(&clean, &config).await;
    let degenerate_time = time_pipeline(&degenerate, &config).await;

    report("pipeline/clean", clean_time, clean.len());
    report("pipeline/degenerate", degenerate_time, degenerate.len());

    let ratio = degenerate_time.as_secs_f64() / clean_time.as_secs_f64().max(f64::EPSILON);
    assert!(
        ratio < 4.0,
        "degenerate float data costs {ratio:.2}x normal data"
    );
}

#[tokio::test]
async fn perf_disabled_filtering_is_the_cheapest_configuration() {
    // Turning a source off must cost less than leaving it on. It sounds obvious, but the
    // engine checks `config.<source>.enabled` in two different places per source, and a
    // gate placed after the work would be invisible except in timing.
    let _cfg = config_guard();
    let tokens = corpus(CORPUS);

    let full = filters_default_dex_only();
    let none = common::filters_all_disabled();

    let _ = time_pipeline(&tokens[..500], &full).await;
    let full_time = time_pipeline(&tokens, &full).await;
    let none_time = time_pipeline(&tokens, &none).await;

    report("pipeline/all-filters", full_time, tokens.len());
    report("pipeline/no-filters", none_time, tokens.len());

    assert!(
        none_time <= full_time,
        "evaluating with every filter disabled ({none_time:?}) is not cheaper than \
         evaluating with all of them enabled ({full_time:?})"
    );
}

// ============================================================================
// SNAPSHOT-SCALE PROJECTION
// ============================================================================

#[tokio::test]
async fn perf_projects_a_full_snapshot_within_the_refresh_interval() {
    // The background loop refreshes every 30s (`background::refresh_interval_secs`).
    // Evaluation is only one phase of a snapshot — the token load, the blacklist joins
    // and the batched DB writes are on top — so pure evaluation has to finish in a
    // fraction of that budget or the loop can never keep up.
    let _cfg = config_guard();
    let config = filters_default_dex_only();
    let tokens = corpus(CORPUS);

    let _ = time_pipeline(&tokens[..500], &config).await;
    let elapsed = time_pipeline(&tokens, &config).await;

    let per_token_secs = elapsed.as_secs_f64() / tokens.len() as f64;
    let projected = per_token_secs * 330_000.0; // the owner's live corpus with market data
    let budget = screenerbot::filtering::background::refresh_interval_secs() as f64;
    eprintln!(
        "PERF projected evaluation of 330k tokens: {projected:.1}s (debug build) against a \
         {budget:.0}s refresh interval"
    );

    // A debug build is roughly an order of magnitude slower than the shipped release
    // build, so the debug projection is allowed several times the interval; anything
    // beyond that cannot possibly fit once the other snapshot phases are added.
    assert!(
        projected < budget * 10.0,
        "evaluation alone projects to {projected:.1}s for 330k tokens against a {budget:.0}s \
         refresh interval"
    );
}
