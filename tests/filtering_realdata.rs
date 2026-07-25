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
// WHAT THE FILTER ACTUALLY GETS TO SEE
// ============================================================================

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_batch_load_carries_the_fields_the_rules_read() {
    // A rule that is switched on in config must be evaluated against real data, or the
    // configuration is a lie. The batch query the snapshot uses fetched only `score`,
    // `rugged` and the two authorities from the Rugcheck table and hardcoded the REST to
    // `None`/empty — so holder concentration, insiders, creator balance, transfer fee, LP
    // providers, LP lock and the risk-level check were evaluated against absence.
    //
    // That is how `rug_transfer_fee_missing` and then `rug_lp_providers_missing` each came
    // to reject 46% of the corpus: not because the tokens lacked the data, but because the
    // query never asked for it.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;

    // A token has a Rugcheck report if the query returned the fields it always fills.
    let reported: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.security_score.is_some())
        .collect();
    assert!(
        !reported.is_empty(),
        "no candidate carries a Rugcheck score — the security join is broken outright"
    );

    let with = |predicate: fn(&&Token) -> bool| reported.iter().filter(|t| predicate(t)).count();

    let holders = with(|t| !t.top_holders.is_empty());
    let risks = with(|t| !t.security_risks.is_empty());
    let lp = with(|t| t.lp_provider_count.is_some());
    let fee = with(|t| t.transfer_fee_pct.is_some());
    let total_holders = with(|t| t.total_holders.is_some());
    let creator = with(|t| t.creator_balance_pct.is_some());

    eprintln!(
        "REALDATA of {} reported tokens: top_holders {holders}, risks {risks}, \
         lp_providers {lp}, transfer_fee {fee}, total_holders {total_holders}, \
         creator_balance {creator}",
        reported.len()
    );

    // Every one of these drives a rule that is ON by default. None may be universally
    // absent — that is the signature of a column the query forgot.
    for (name, populated) in [
        ("top_holders", holders),
        ("security_risks", risks),
        ("lp_provider_count", lp),
        ("transfer_fee_pct", fee),
        ("total_holders", total_holders),
    ] {
        assert!(
            populated > 0,
            "not one reported token carries {name} — the filtering batch load is dropping \
             the column the rule reads"
        );
    }
}

// ============================================================================
// WHAT THE SHIPPED CONFIGURATION DOES TO REAL TOKENS
// ============================================================================

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_default_config_passes_real_tokens() {
    // The end-to-end proof that the shipped configuration works on the owner's real data.
    // It used to pass 0 of 40,000: both market sources are enabled by default and each was
    // gated on the token's single `data_source`, so every token failed whichever gate did
    // not match. On top of that the Rugcheck stage rejected the whole corpus on its own,
    // 46% of it for not proving the absence of a transfer fee.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    assert!(!tokens.is_empty(), "the real database returned no tokens");

    let outcome = evaluate_all(&tokens, &FilteringConfig::default()).await;
    outcome.report("default config");

    let pass_rate = outcome.passed as f64 / outcome.total() as f64;
    eprintln!(
        "REALDATA default-config pass rate: {:.3}% ({} of {})",
        pass_rate * 100.0,
        outcome.passed,
        outcome.total()
    );

    // The defaults are deliberately strict, so the raw pass COUNT is a poor regression
    // signal — it is a handful of tokens and moves with the corpus. What must hold is that
    // no single rule is acting as a WALL. A rule that alone accounts for nearly the whole
    // corpus is not filtering, it is a contradiction (which is exactly what the source gate
    // and the transfer-fee rule each were).
    let (worst_reason, worst_count) = outcome
        .top_reasons(1)
        .into_iter()
        .next()
        .unwrap_or_default();
    let worst_share = worst_count as f64 / outcome.total().max(1) as f64;
    eprintln!(
        "REALDATA largest single rejection: {worst_reason} at {:.1}%",
        worst_share * 100.0
    );
    assert!(
        worst_share < 0.95,
        "{worst_reason} alone rejects {:.1}% of the corpus — that is a wall, not a filter",
        worst_share * 100.0
    );

    // And every stage must be reachable: tokens have to survive meta, on-chain and the
    // market rules in order to be judged on safety at all.
    let reached_rugcheck = outcome.passed
        + outcome.by_source.get("rugcheck").copied().unwrap_or(0)
        + outcome
            .by_reason
            .get(&FilterRejectionReason::RugcheckDataMissing.label())
            .copied()
            .unwrap_or(0);
    assert!(
        reached_rugcheck > 0,
        "no token reached the Rugcheck stage — an earlier stage is rejecting everything"
    );

    // The source gate must no longer be a mass rejector; it now only catches tokens whose
    // market data really is absent.
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
    let gate_rate = gate_rejections as f64 / outcome.total().max(1) as f64;
    eprintln!(
        "REALDATA source-gate rejections: {gate_rejections} of {} ({:.1}%)",
        outcome.total(),
        gate_rate * 100.0
    );
    assert!(
        gate_rate < 0.50,
        "{:.1}% of the corpus is still rejected for missing market data — the two sources \
         are being demanded together again",
        gate_rate * 100.0
    );
}

#[tokio::test]
#[ignore = "reads the owner's real database (cloned); run with ./test.sh live"]
async fn realdata_rugcheck_stage_is_selective_not_absolute() {
    // The Rugcheck stage used to reject EVERY real token by itself, 46% of them with
    // `rug_transfer_fee_missing` — the default 5% ceiling read absent fee data as a failure
    // to prove compliance, and an ordinary SPL token has no Token-2022 fee extension to
    // report. Absence now means "no fee", which is what it actually means.
    //
    // `rug_data_missing` remains, and remains correct: the engine will not judge a token's
    // safety from a report it does not have. It is a coverage limit, not a rule defect.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let tokens = load_candidates().await;
    let outcome = evaluate_all(&tokens, &filters_default_dex_only()).await;
    outcome.report("default minus GeckoTerminal source");

    assert_eq!(outcome.total(), tokens.len(), "a token got no decision");
    assert!(
        outcome.passed > 0,
        "the Rugcheck stage is still rejecting the entire corpus"
    );

    // Of the tokens that HAVE a Rugcheck report, the transfer-fee rule must be a rarity.
    let with_report = tokens
        .iter()
        .filter(|t| t.security_score.is_some() || !t.security_risks.is_empty())
        .count();
    let fee_rejections = outcome
        .by_reason
        .get(&FilterRejectionReason::RugcheckTransferFeeTooHigh.label())
        .copied()
        .unwrap_or(0);
    eprintln!(
        "REALDATA transfer-fee rejections: {fee_rejections} of {with_report} tokens with a \
         Rugcheck report"
    );
    assert!(
        fee_rejections * 10 < with_report.max(1),
        "{fee_rejections} of {with_report} reported tokens rejected on transfer fee — real \
         fee-bearing mints are rare, so absence is being treated as a failure again"
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
async fn realdata_the_meta_stage_never_pays_for_a_decimals_lookup() {
    // This measured the single most expensive thing filtering did. `meta::evaluate` used
    // to resolve decimals through the cache for every candidate; the cache holds far fewer
    // entries than the corpus has tokens, so the majority missed and fell through to a
    // per-token SQLite lookup — 216.8 us/token cold against 4.3 us warm, about 95 SECONDS
    // of lookups per 30-second refresh interval. And the cache could never stay warm,
    // because each pass re-evicted what the last one pulled in.
    //
    // The batch load already carries each token's decimals, from the same row the resolver
    // would have read, so a token whose decimals are KNOWN must now cost nothing on the
    // very first pass — no warm-up, no cache to miss.
    //
    // The measurement is split by that property on purpose. A token whose decimals we have
    // never resolved still goes to the resolver, and should: that is a one-time, self-
    // healing cost (the resolver persists what it finds), not a per-refresh tax. Averaging
    // the two groups together hides the fix behind the warm-up of the other group.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let _db = common::init_real_token_db();
    let _cfg = config_guard();

    let all = load_candidates().await;
    let (known, unknown): (Vec<Token>, Vec<Token>) = all.into_iter().partition(|t| {
        t.decimals
            .is_some_and(screenerbot::tokens::decimals_are_valid)
    });
    eprintln!(
        "REALDATA candidates: {} with stored decimals, {} without ({:.2}% unresolved)",
        known.len(),
        unknown.len(),
        unknown.len() as f64 * 100.0 / (known.len() + unknown.len()).max(1) as f64
    );
    assert!(!known.is_empty(), "no candidate carries stored decimals");

    // Only the age rule, so the measurement is dominated by the meta stage rather than by
    // the rules themselves.
    let mut config = filters_all_disabled();
    config.age_enabled = true;

    let first = evaluate_all(&known, &config).await;
    let second = evaluate_all(&known, &config).await;

    let first_us = per_item_micros(first.elapsed, first.total());
    let second_us = per_item_micros(second.elapsed, second.total());
    let ratio = first_us / second_us.max(f64::EPSILON);
    eprintln!(
        "REALDATA meta stage, decimals known: first pass {first_us:.1} us/token, second \
         pass {second_us:.1} us/token ({ratio:.1}x)"
    );

    let corpus = screenerbot::tokens::count_tokens_async().await.unwrap_or(0);
    eprintln!(
        "REALDATA meta stage over {corpus} tokens at the first-pass rate: {:.2}s",
        first_us * corpus as f64 / 1_000_000.0
    );

    // A first pass that costs the same as a warmed one is the whole point: nothing was
    // consulted, so there was nothing to warm. It used to be 50x.
    assert!(
        ratio < 3.0,
        "the first pass cost {ratio:.1}x the second — the meta stage is resolving decimals \
         again instead of reading what the batch load already carries"
    );

    // And in absolute terms it must be cheap enough to disappear into the refresh interval.
    let budget = screenerbot::filtering::background::refresh_interval_secs() as f64;
    let projected = first_us * corpus as f64 / 1_000_000.0;
    assert!(
        projected < budget,
        "resolving decimals for the corpus projects to {projected:.1}s against a \
         {budget:.0}s refresh interval"
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
async fn realdata_decimals_preload_survives_its_own_cache() {
    // The decimals cache is a bounded LRU whose consumers — the SYNCHRONOUS pool decoders —
    // have no fallback: a miss makes the decoder skip the pool, so the token loses its live
    // price. The preload used to push EVERY token's decimals through it (405k rows into a
    // 100k cache), which evicted three quarters of what it had just loaded, including most
    // pool-backed mints. Loading no more than the cache can hold is what makes the preload
    // mean anything.
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let db = common::init_real_token_db();

    let preloaded = db
        .get_tokens_with_decimals_for_preload(screenerbot::tokens::decimals::PRELOAD_CAPACITY)
        .expect("read stored decimals");
    let total = preloaded.len();
    assert!(total > 0, "the real database has no decimals recorded");

    let evicted = preloaded
        .iter()
        .filter(|(mint, _)| screenerbot::tokens::get_cached_decimals(mint).is_none())
        .count();

    eprintln!(
        "REALDATA decimals cache: {total} mints preloaded (cap {}), {evicted} evicted",
        screenerbot::tokens::decimals::PRELOAD_CAPACITY
    );

    assert_eq!(
        evicted, 0,
        "the preload must not exceed the cache it is filling"
    );

    // The ordering is load-bearing, not cosmetic: pool-backed mints come LAST so that once
    // runtime upserts start pushing the cache over its capacity, the entries with no
    // fallback are the most recently used and the last to be evicted. Sample the tail and
    // confirm those really are the pooled ones.
    let sample = 100.min(preloaded.len());
    let tail_pooled = preloaded[preloaded.len() - sample..]
        .iter()
        .filter(|(mint, _)| {
            db.get_token_pools(mint)
                .ok()
                .flatten()
                .is_some_and(|snapshot| !snapshot.pools.is_empty())
        })
        .count();

    eprintln!(
        "REALDATA preload tail: {tail_pooled}/{sample} of the last-inserted mints have pools"
    );
    assert_eq!(
        tail_pooled, sample,
        "pool-backed mints must be inserted last, where eviction reaches them last"
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
