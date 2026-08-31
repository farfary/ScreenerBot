//! Combined filtering — the whole pipeline for one token, in order, with every source
//! on, every source off, and the source-availability gates in between.
//!
//! `filtering::evaluate_token` is what a snapshot runs per candidate. These tests seed
//! the decimals cache (its only I/O dependency) and hold the config guard, so the tier
//! stays pure: no network, no database, no wallet.
#![allow(clippy::await_holding_lock)]

mod common;

use common::{config_guard, filter_token, filters_all_disabled, filters_default_dex_only, holder};
use screenerbot::config::FilteringConfig;
use screenerbot::filtering::evaluate_token;
use screenerbot::filtering::sources::{FilterRejectionReason, FilterSource};
use screenerbot::tokens::types::{DataSource, Token};

const MINT: &str = "PipelineMint11111111111111111111111111111111";

/// A token whose decimals are cached, so `meta::evaluate` resolves them without I/O.
fn seeded_token() -> Token {
    common::seed_decimals(MINT, 9);
    filter_token(MINT)
}

fn rejection(result: Result<(), FilterRejectionReason>) -> FilterRejectionReason {
    result.expect_err("expected a rejection, token passed")
}

// ============================================================================
// THE DEFAULT CONFIGURATION
// ============================================================================

#[tokio::test]
async fn pipeline_default_config_accepts_a_healthy_token_from_either_source() {
    // THE headline regression. DexScreener and GeckoTerminal are two providers of the same
    // market data and a token carries exactly one `data_source`, so gating each source on
    // its own `data_source` made the shipped defaults — both sources enabled —
    // unsatisfiable: whichever gate did not match reported "data missing" and the pipeline
    // passed NOTHING. Measured against the production database at the time: 0 of 40,000.
    let _cfg = config_guard();
    let config = FilteringConfig::default();

    for source in [DataSource::DexScreener, DataSource::GeckoTerminal] {
        let mut token = seeded_token();
        token.data_source = source;
        assert!(
            evaluate_token(&token, &config).await.is_ok(),
            "a healthy {source:?} token must pass the shipped defaults"
        );
    }
}

#[tokio::test]
async fn pipeline_still_applies_the_source_that_matches_the_token() {
    // Accepting both sources must not mean skipping both. Each enabled source is applied
    // to the tokens it actually covers, and only those.
    let _cfg = config_guard();
    let mut config = FilteringConfig::default();
    config.dexscreener.min_liquidity_usd = 10_000_000.0;
    config.geckoterminal.min_liquidity_usd = 10_000_000.0;

    let mut dex = seeded_token();
    dex.data_source = DataSource::DexScreener;
    assert_eq!(
        rejection(evaluate_token(&dex, &config).await),
        FilterRejectionReason::DexScreenerInsufficientLiquidity,
        "a DexScreener token is judged by the DexScreener rules"
    );

    let mut gecko = seeded_token();
    gecko.data_source = DataSource::GeckoTerminal;
    assert_eq!(
        rejection(evaluate_token(&gecko, &config).await),
        FilterRejectionReason::GeckoTerminalLiquidityTooLow,
        "and a GeckoTerminal token by the GeckoTerminal rules"
    );
}

#[tokio::test]
async fn pipeline_market_gate_matrix() {
    // Every combination of (which sources are enabled) x (which source the token carries).
    // `None` means the token must pass; `Some(reason)` names the rejection.
    let _cfg = config_guard();

    let cases: [(bool, bool, DataSource, Option<FilterRejectionReason>); 9] = [
        (true, true, DataSource::DexScreener, None),
        (true, true, DataSource::GeckoTerminal, None),
        (
            true,
            true,
            DataSource::Unknown,
            Some(FilterRejectionReason::DexScreenerDataMissing),
        ),
        (true, false, DataSource::DexScreener, None),
        (
            true,
            false,
            DataSource::GeckoTerminal,
            // The user asked for DexScreener rules and this token has no DexScreener data.
            Some(FilterRejectionReason::DexScreenerDataMissing),
        ),
        (
            false,
            true,
            DataSource::DexScreener,
            Some(FilterRejectionReason::GeckoTerminalDataMissing),
        ),
        (false, true, DataSource::GeckoTerminal, None),
        // With no market source enabled, market data is not required at all.
        (false, false, DataSource::Unknown, None),
        (false, false, DataSource::DexScreener, None),
    ];

    for (dex_on, gecko_on, source, expected) in cases {
        let mut config = FilteringConfig::default();
        config.dexscreener.enabled = dex_on;
        config.geckoterminal.enabled = gecko_on;

        let mut token = seeded_token();
        token.data_source = source;

        let verdict = evaluate_token(&token, &config).await;
        match expected {
            None => assert!(
                verdict.is_ok(),
                "dex={dex_on} gecko={gecko_on} source={source:?} should pass, got {verdict:?}"
            ),
            Some(reason) => assert_eq!(
                rejection(verdict),
                reason,
                "dex={dex_on} gecko={gecko_on} source={source:?}"
            ),
        }
    }
}

#[tokio::test]
async fn pipeline_passes_a_healthy_token_with_a_single_market_source() {
    let _cfg = config_guard();
    let token = seeded_token();

    assert!(
        evaluate_token(&token, &filters_default_dex_only())
            .await
            .is_ok(),
        "default rules with only DexScreener enabled must accept the baseline token"
    );
}

#[tokio::test]
async fn pipeline_with_no_filters_enabled_accepts_a_hostile_token() {
    let _cfg = config_guard();
    let config = filters_all_disabled();

    let mut token = seeded_token();
    token.symbol = "0000".to_owned();
    token.name = String::new();
    token.data_source = DataSource::Unknown;
    token.liquidity_usd = Some(0.0);
    token.market_cap = None;
    token.is_rugged = true;
    token.mint_authority = Some("Mint111111111111111111111111111111111111111".to_owned());
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    token.is_mutable = Some(false);
    token.total_holders = Some(1);
    token.top_holders = vec![holder(
        "Whale11111111111111111111111111111111111111",
        99.0,
        true,
    )];
    token.transfer_fee_pct = None;
    token.lp_provider_count = None;
    token.first_discovered_at = chrono::Utc::now();

    assert!(
        evaluate_token(&token, &config).await.is_ok(),
        "with every switch off the pipeline must be a pass-through — no hidden mandatory rule"
    );
}

// ============================================================================
// DECIMALS — the first thing every candidate is checked for
// ============================================================================

#[tokio::test]
async fn pipeline_uses_the_decimals_the_token_already_carries() {
    // The batch load reads `decimals` from the same row the resolver would consult, so a
    // token that carries them must never pay for a lookup. This is the whole cost of the
    // filtering pass: the snapshot evaluates every token with market data (328k on a
    // mature database) while the decimals cache holds far fewer, so routing every token
    // through the resolver meant a DB round-trip for the majority of them, every refresh.
    //
    // A mint that is in NO cache and NO database proves it: if the pipeline consulted the
    // resolver at all, this would fall through to the network. It returns immediately.
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut token = filter_token("UncachedMint111111111111111111111111111111");
    token.decimals = Some(6);

    let started = std::time::Instant::now();
    let verdict = evaluate_token(&token, &config).await;
    let elapsed = started.elapsed();

    assert!(verdict.is_ok(), "got {verdict:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "resolving decimals the token already carries took {elapsed:?} — the fast path is gone"
    );
}

#[tokio::test]
async fn pipeline_rejects_decimals_it_cannot_trust() {
    // Zero and out-of-range values both live in the production database (2,881 rows at 0,
    // hundreds above 18). `0` is the placeholder every persistence path here writes for
    // "never resolved", and >18 is impossible for a Solana mint, so neither may be taken at
    // face value — treating a junk value as known decimals is how token math goes wrong by
    // orders of magnitude.
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    // Keeping this tier PURE takes two things, because an unresolved token is exactly the
    // case that falls through to the resolver: switch off the shared data server, and use a
    // mint that is not valid base58 ('0' is not in the alphabet) so the chain fallback fails
    // at parse time instead of opening a socket. There is no global database in this tier,
    // so the DB and Rugcheck fallbacks miss without any I/O either.
    common::set_config(|cfg| cfg.tokens.sources.screenerbot_server.enabled = false);
    const UNRESOLVED: &str = "0nresolvedMint111111111111111111111111111";

    for junk in [Some(0), Some(19), Some(123), None] {
        let mut token = filter_token(UNRESOLVED);
        token.decimals = junk;

        assert_eq!(
            rejection(evaluate_token(&token, &config).await),
            FilterRejectionReason::NoDecimalsInDatabase,
            "decimals {junk:?} must not count as resolved"
        );
    }

    // Every legal value is accepted.
    for good in 1..=18u8 {
        let mut token = seeded_token();
        token.decimals = Some(good);
        assert!(evaluate_token(&token, &config).await.is_ok());
    }
}

// ============================================================================
// ORDER OF EVALUATION
// ============================================================================

#[tokio::test]
async fn pipeline_reports_the_first_failing_stage_meta_before_onchain() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut token = seeded_token();
    token.first_discovered_at = chrono::Utc::now(); // too new
    token.symbol = "0000".to_owned(); // and a scam symbol
    token.liquidity_usd = Some(0.0); // and no liquidity

    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::TokenTooNew,
        "meta runs first, so age must win over the cheaper on-chain and market rules"
    );
}

#[tokio::test]
async fn pipeline_runs_onchain_before_any_market_source() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut token = seeded_token();
    token.symbol = "0000".to_owned();
    token.liquidity_usd = Some(0.0);

    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::OnChainNumericSymbol,
        "the zero-cost scam check must pre-empt market filtering"
    );
}

#[tokio::test]
async fn pipeline_runs_market_sources_before_rugcheck() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut token = seeded_token();
    token.liquidity_usd = Some(0.0); // DexScreener rejects
    token.is_rugged = true; // Rugcheck would too

    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::DexScreenerZeroLiquidity,
        "the cheaper market check must not be preceded by the security one"
    );
}

#[tokio::test]
async fn pipeline_rejection_sources_match_the_stage_that_produced_them() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut too_new = seeded_token();
    too_new.first_discovered_at = chrono::Utc::now();

    let mut scam_symbol = seeded_token();
    scam_symbol.symbol = "0000".to_owned();

    let mut no_liquidity = seeded_token();
    no_liquidity.liquidity_usd = Some(0.0);

    let mut rugged = seeded_token();
    rugged.is_rugged = true;

    let cases = [
        (&too_new, FilterSource::Core),
        (&scam_symbol, FilterSource::OnChain),
        (&no_liquidity, FilterSource::DexScreener),
        (&rugged, FilterSource::Rugcheck),
    ];

    for (token, expected_source) in cases {
        let reason = rejection(evaluate_token(token, &config).await);
        assert_eq!(
            reason.source(),
            expected_source,
            "{} was attributed to the wrong stage",
            reason.label()
        );
    }
}

// ============================================================================
// AGE
// ============================================================================

#[tokio::test]
async fn pipeline_age_boundary_is_inclusive() {
    let _cfg = config_guard();
    let mut config = filters_default_dex_only();
    config.age_enabled = true;
    config.min_token_age_minutes = 60;

    let mut token = seeded_token();
    // A whole-minute boundary truncates downwards, so add a second of slack to keep the
    // assertion about the RULE rather than about clock resolution.
    token.first_discovered_at = chrono::Utc::now() - chrono::Duration::seconds(60 * 60 + 1);
    assert!(
        evaluate_token(&token, &config).await.is_ok(),
        "exactly the minimum age is old enough (`<` rejects)"
    );

    token.first_discovered_at = chrono::Utc::now() - chrono::Duration::minutes(59);
    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::TokenTooNew
    );
}

#[tokio::test]
async fn pipeline_age_measures_discovery_not_creation() {
    // DEFECT PIN: the age rule reads `first_discovered_at` — when the BOT first saw the
    // token — while the snapshot's "Recent" view reads `blockchain_created_at`. A token
    // minted a year ago is "too new" for a minute after discovery, which quietly delays
    // every token found after a restart or a cold start.
    let _cfg = config_guard();
    let mut config = filters_default_dex_only();
    config.age_enabled = true;
    config.min_token_age_minutes = 60;

    let mut token = seeded_token();
    token.blockchain_created_at = Some(chrono::Utc::now() - chrono::Duration::days(365));
    token.first_discovered_at = chrono::Utc::now() - chrono::Duration::minutes(1);

    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::TokenTooNew,
        "a year-old token counts as new because WE only just found it"
    );
}

#[tokio::test]
async fn pipeline_age_check_tolerates_a_future_discovery_timestamp() {
    // A clock skew or a bad API timestamp can put discovery in the future; the negative
    // age is clamped to 0 rather than wrapping into a huge positive number.
    let _cfg = config_guard();
    let mut config = filters_default_dex_only();
    config.age_enabled = true;
    config.min_token_age_minutes = 60;

    let mut token = seeded_token();
    token.first_discovered_at = chrono::Utc::now() + chrono::Duration::days(7);

    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::TokenTooNew,
        "a future timestamp must read as age 0, not as an ancient token"
    );
}

// ============================================================================
// SOURCE-AVAILABILITY GATES
// ============================================================================

#[tokio::test]
async fn pipeline_dexscreener_gate_rejects_tokens_sourced_elsewhere() {
    let _cfg = config_guard();
    let mut config = FilteringConfig::default();
    config.geckoterminal.enabled = false;

    let mut token = seeded_token();
    token.data_source = DataSource::Unknown;
    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::DexScreenerDataMissing
    );

    // Even a token that DOES have market data is rejected when it came from the other
    // provider — the gate is about provenance, not about whether the data exists.
    token.data_source = DataSource::GeckoTerminal;
    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::DexScreenerDataMissing
    );
}

#[tokio::test]
async fn pipeline_geckoterminal_gate_mirrors_the_dexscreener_one() {
    let _cfg = config_guard();
    let mut config = FilteringConfig::default();
    config.dexscreener.enabled = false;

    let mut token = seeded_token();
    token.data_source = DataSource::GeckoTerminal;
    assert!(evaluate_token(&token, &config).await.is_ok());

    token.data_source = DataSource::DexScreener;
    assert_eq!(
        rejection(evaluate_token(&token, &config).await),
        FilterRejectionReason::GeckoTerminalDataMissing
    );
}

#[tokio::test]
async fn pipeline_rugcheck_gate_needs_at_least_one_security_field() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut bare = seeded_token();
    bare.security_score = None;
    bare.token_type = None;
    bare.mint_authority = None;
    bare.freeze_authority = None;
    bare.graph_insiders_detected = None;
    bare.lp_provider_count = None;
    bare.total_holders = None;
    bare.security_risks.clear();
    bare.top_holders.clear();
    bare.creator_balance_pct = None;
    bare.transfer_fee_pct = None;
    bare.transfer_fee_max_amount = None;
    bare.transfer_fee_authority = None;

    assert_eq!(
        rejection(evaluate_token(&bare, &config).await),
        FilterRejectionReason::RugcheckDataMissing
    );

    // Any ONE of the tracked fields satisfies the gate — even a field that carries no
    // safety information by itself.
    let mut only_type = bare.clone();
    only_type.token_type = Some("spl".to_owned());
    assert_ne!(
        rejection(evaluate_token(&only_type, &config).await),
        FilterRejectionReason::RugcheckDataMissing,
        "a bare token_type is enough to look like a fetched Rugcheck report"
    );

    let mut only_holders = bare.clone();
    only_holders.total_holders = Some(500);
    only_holders.transfer_fee_pct = Some(0.0);
    only_holders.lp_provider_count = Some(8);
    assert!(evaluate_token(&only_holders, &config).await.is_ok());
}

#[tokio::test]
async fn pipeline_disabling_rugcheck_also_disables_its_data_requirement() {
    let _cfg = config_guard();
    let mut config = filters_default_dex_only();
    config.rugcheck.enabled = false;

    let mut token = seeded_token();
    token.security_score = None;
    token.token_type = None;
    token.lp_provider_count = None;
    token.total_holders = None;
    token.transfer_fee_pct = None;
    token.security_risks.clear();
    token.top_holders.clear();
    token.creator_balance_pct = None;
    token.transfer_fee_max_amount = None;

    assert!(evaluate_token(&token, &config).await.is_ok());
}

// ============================================================================
// LLM ANALYSIS STAGE
// ============================================================================

#[tokio::test]
async fn pipeline_llm_analysis_stage_is_inert_when_disabled() {
    let _cfg = config_guard(); // resets the global config, where llm.enabled defaults to false
    let token = seeded_token();

    assert!(evaluate_token(&token, &filters_default_dex_only())
        .await
        .is_ok());
}

#[tokio::test]
async fn pipeline_llm_analysis_stage_fails_open_when_the_engine_is_missing() {
    // LLM analysis is configured on, but no engine was initialised. The stage must let the token
    // through rather than rejecting every candidate — filtering must never depend on an
    // optional subsystem being up.
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.llm.enabled = true;
        cfg.llm_analysis.filtering_enabled = true;
    });

    let token = seeded_token();
    assert!(evaluate_token(&token, &filters_default_dex_only())
        .await
        .is_ok());
}

// ============================================================================
// CORRUPTED / ADVERSARIAL TOKENS
// ============================================================================

#[tokio::test]
async fn pipeline_survives_structurally_corrupt_tokens() {
    // None of these may panic — a single bad row must not take down a snapshot of
    // hundreds of thousands of tokens. Whether each passes or fails is a separate
    // question; the contract here is "returns a decision".
    let _cfg = config_guard();
    let config = filters_default_dex_only();

    let mut cases: Vec<(&str, Token)> = Vec::new();

    let mut nan_everywhere = seeded_token();
    nan_everywhere.liquidity_usd = Some(f64::NAN);
    nan_everywhere.market_cap = Some(f64::NAN);
    nan_everywhere.fdv = Some(f64::NAN);
    nan_everywhere.volume_h24 = Some(f64::NAN);
    nan_everywhere.price_change_h24 = Some(f64::NAN);
    nan_everywhere.creator_balance_pct = Some(f64::NAN);
    nan_everywhere.transfer_fee_pct = Some(f64::NAN);
    cases.push(("nan", nan_everywhere));

    let mut infinities = seeded_token();
    infinities.liquidity_usd = Some(f64::INFINITY);
    infinities.market_cap = Some(f64::NEG_INFINITY);
    infinities.top_holders = vec![holder(
        "Inf1111111111111111111111111111111111111111",
        f64::INFINITY,
        true,
    )];
    cases.push(("infinity", infinities));

    let mut extremes = seeded_token();
    extremes.total_holders = Some(i64::MAX);
    extremes.lp_provider_count = Some(i64::MIN);
    extremes.graph_insiders_detected = Some(i64::MAX);
    extremes.txns_m5_buys = Some(i64::MAX);
    extremes.txns_m5_sells = Some(i64::MAX); // saturating_add must not overflow
    cases.push(("integer extremes", extremes));

    let mut unicode = seeded_token();
    unicode.symbol = "🚀🚀🚀".to_owned();
    unicode.name = "\u{202e}gnisrever".to_owned();
    unicode.description = Some("\0\0\0".to_owned());
    cases.push(("unicode", unicode));

    let mut empty_strings = seeded_token();
    empty_strings.name = String::new();
    empty_strings.symbol = String::new();
    empty_strings.price_native = String::new();
    empty_strings.image_url = Some(String::new());
    cases.push(("empty strings", empty_strings));

    let mut huge_holders = seeded_token();
    huge_holders.top_holders = (0..5_000)
        .map(|i| holder(&format!("H{i:042}"), 0.01, i % 7 == 0))
        .collect();
    cases.push(("5k holders", huge_holders));

    let mut risky = seeded_token();
    risky.security_risks = (0..500)
        .map(|i| common::security_risk(&format!("risk {i}"), "%%%", "lp lock ???", "unknown"))
        .collect();
    cases.push(("500 unparseable risks", risky));

    for (label, token) in cases {
        // The assertion is that this returns at all, and produces a labelled decision.
        match evaluate_token(&token, &config).await {
            Ok(()) => {}
            Err(reason) => assert!(
                !reason.label().is_empty(),
                "{label}: rejection carried no label"
            ),
        }
    }
}

#[tokio::test]
async fn pipeline_saturates_instead_of_overflowing_transaction_counts() {
    let _cfg = config_guard();
    let mut config = filters_default_dex_only();
    config.dexscreener.min_transactions_5min = 1;
    config.dexscreener.min_transactions_1h = 1;

    let mut token = seeded_token();
    token.txns_m5_buys = Some(i64::MAX);
    token.txns_m5_sells = Some(i64::MAX);
    token.txns_h1_buys = Some(i64::MAX);
    token.txns_h1_sells = Some(i64::MAX);

    assert!(
        evaluate_token(&token, &config).await.is_ok(),
        "i64::MAX + i64::MAX must saturate, not wrap negative and read as no activity"
    );
}

#[tokio::test]
async fn pipeline_is_deterministic_for_the_same_token() {
    let _cfg = config_guard();
    let config = filters_default_dex_only();
    let mut token = seeded_token();
    token.liquidity_usd = Some(0.0);

    let first = rejection(evaluate_token(&token, &config).await);
    for _ in 0..25 {
        assert_eq!(
            rejection(evaluate_token(&token, &config).await),
            first,
            "the same token and config must always yield the same decision"
        );
    }
}
