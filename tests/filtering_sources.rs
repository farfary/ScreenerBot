//! Per-source filtering logic — every rule of every filter source, its boundaries, its
//! disabled path, and what it does with missing or corrupted values.
//!
//! Pure tier: the four source evaluators are synchronous and read nothing but the token
//! and the config passed in. Where a rule's behaviour looks wrong, the test asserts what
//! the code ACTUALLY does and says so in a comment — a test that encodes the intended
//! behaviour would just fail, and a test that stays silent hides the defect.

mod common;

use common::{filter_token, holder, security_risk};
use screenerbot::config::schemas::{
    DexScreenerFilters, GeckoTerminalFilters, OnChainFilters, RugCheckFilters,
};
use screenerbot::filtering::sources::{
    dexscreener, geckoterminal, onchain, rugcheck, FilterRejectionReason, FilterSource,
};
use screenerbot::tokens::types::{DataSource, Token};

const MINT: &str = "FilterMint111111111111111111111111111111111";

/// One table-driven case: mutate the baseline token, expect this rejection.
type Case = (fn(&mut Token), FilterRejectionReason);

/// The rejection reason, or a panic naming what unexpectedly passed.
fn rejection(result: Result<(), FilterRejectionReason>) -> FilterRejectionReason {
    result.expect_err("expected a rejection, token passed")
}

fn dex_token() -> Token {
    filter_token(MINT)
}

fn gecko_token() -> Token {
    let mut token = filter_token(MINT);
    token.data_source = DataSource::GeckoTerminal;
    token
}

// ============================================================================
// ON-CHAIN  (no external data, runs before every API source)
// ============================================================================

#[test]
fn onchain_disabled_passes_an_obvious_scam() {
    let config = OnChainFilters {
        enabled: false,
        ..Default::default()
    };
    let mut token = dex_token();
    token.symbol = "0000".to_owned();
    token.name = "0000".to_owned();
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    token.is_mutable = Some(false);

    assert!(onchain::evaluate(&token, &config).is_ok());
}

#[test]
fn onchain_rejects_numeric_only_symbol() {
    let config = OnChainFilters::default();
    let mut token = dex_token();
    token.symbol = "0000".to_owned();

    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainNumericSymbol
    );
}

#[test]
fn onchain_numeric_check_trims_and_needs_all_digits() {
    let config = OnChainFilters::default();
    let mut token = dex_token();

    token.symbol = "  123  ".to_owned();
    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainNumericSymbol,
        "surrounding whitespace must not hide a numeric symbol"
    );

    // One non-digit is enough to make it a normal symbol.
    token.symbol = "123X".to_owned();
    assert!(onchain::evaluate(&token, &config).is_ok());
}

#[test]
fn onchain_rejects_empty_and_null_padded_symbols() {
    let config = OnChainFilters::default();
    let mut token = dex_token();

    for symbol in ["", "   ", "\0\0\0", "\0", "\t\n"] {
        token.symbol = symbol.to_owned();
        assert_eq!(
            rejection(onchain::evaluate(&token, &config)),
            FilterRejectionReason::OnChainEmptySymbol,
            "symbol {symbol:?} must be treated as empty"
        );
    }
}

#[test]
fn onchain_misses_a_null_symbol_padded_with_whitespace() {
    // DEFECT PIN: `is_empty_or_whitespace` tries three tests in sequence — `is_empty`,
    // `trim().is_empty()`, and `trim_matches('\0').trim().is_empty()` — but never trims
    // BOTH kinds of padding in the right order. Rust's `trim` does not treat NUL as
    // whitespace, and `trim_matches('\0')` only strips NULs at the very ends, so a symbol
    // whose NULs sit INSIDE the whitespace survives every check and is treated as a real
    // symbol. One `trim_matches(|c: char| c.is_whitespace() || c == '\0')` would cover
    // all of it.
    let config = OnChainFilters {
        // Isolate the empty-symbol rule from the combined score, which would also fire.
        combined_risk_enabled: false,
        ..Default::default()
    };
    let mut token = dex_token();
    token.symbol = " \0 ".to_owned();

    assert!(
        onchain::evaluate(&token, &config).is_ok(),
        "a NUL padded with spaces is currently accepted as a valid symbol"
    );
}

#[test]
fn onchain_single_char_rule_only_rejects_non_alphabetic() {
    // The config field is called "Reject Single-Char Symbols", but the implementation
    // keeps single ASCII LETTERS and rejects only other single characters.
    let config = OnChainFilters {
        reject_single_char_symbols: true,
        ..Default::default()
    };
    let mut token = dex_token();

    token.symbol = "$".to_owned();
    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainSuspiciousSymbol
    );

    token.symbol = "A".to_owned();
    assert!(
        onchain::evaluate(&token, &config).is_ok(),
        "single ASCII letters are deliberately kept despite the filter's name"
    );
}

#[test]
fn onchain_rejects_immutable_metadata_with_freeze_authority() {
    let config = OnChainFilters::default();
    let mut token = dex_token();
    token.is_mutable = Some(false);
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());

    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainImmutableWithFreeze
    );

    // Either signal alone is not the pattern.
    token.freeze_authority = None;
    assert!(onchain::evaluate(&token, &config).is_ok());
    token.is_mutable = Some(true);
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    assert!(onchain::evaluate(&token, &config).is_ok());
}

#[test]
fn onchain_combined_risk_accumulates_weak_signals() {
    // Only the combined score is live, so the individual rules cannot pre-empt it.
    let config = OnChainFilters {
        reject_numeric_symbols: false,
        reject_empty_symbols: false,
        reject_immutable_with_freeze: false,
        max_combined_risk_score: 60,
        ..Default::default()
    };
    let mut token = dex_token();
    // numeric 30 + freeze 10 + immutable 10 + name==symbol 15 = 65.
    token.symbol = "123".to_owned();
    token.name = "123".to_owned();
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    token.is_mutable = Some(false);

    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainHighRiskScore
    );
}

#[test]
fn onchain_combined_risk_rejects_exactly_at_the_threshold() {
    // The comparison is `>=`, so the configured score is itself rejected.
    let config = OnChainFilters {
        reject_numeric_symbols: false,
        max_combined_risk_score: 30,
        ..Default::default()
    };
    let mut token = dex_token();
    token.symbol = "123".to_owned(); // exactly 30
    token.name = "Something Else".to_owned();

    assert_eq!(
        rejection(onchain::evaluate(&token, &config)),
        FilterRejectionReason::OnChainHighRiskScore
    );
}

#[test]
fn onchain_immutable_bonus_depends_on_signal_order() {
    // DEFECT PIN: `compute_risk_score` adds the +10 immutable bonus only when the score
    // is already non-zero AT THAT POINT, and the +15 "name == symbol" signal is added
    // AFTERWARDS. So an immutable token whose only other signal is name == symbol scores
    // 15, not 25 — the bonus silently depends on the order the signals are written in.
    let config = OnChainFilters {
        reject_numeric_symbols: false,
        reject_empty_symbols: false,
        reject_immutable_with_freeze: false,
        max_combined_risk_score: 20,
        ..Default::default()
    };
    let mut token = dex_token();
    token.symbol = "SAME".to_owned();
    token.name = "same".to_owned(); // case-insensitive match, +15
    token.is_mutable = Some(false); // +10 ONLY if score > 0 already — it is not yet
    token.freeze_authority = None;

    assert!(
        onchain::evaluate(&token, &config).is_ok(),
        "scores 15 (not 25) because the immutable bonus is evaluated before name==symbol"
    );
}

// ============================================================================
// DEXSCREENER
// ============================================================================

#[test]
fn dexscreener_disabled_skips_every_check() {
    let config = DexScreenerFilters {
        enabled: false,
        ..Default::default()
    };
    let mut token = dex_token();
    token.name = String::new();
    token.symbol = String::new();
    token.liquidity_usd = Some(0.0);
    token.market_cap = Some(0.0);

    assert!(dexscreener::evaluate(&token, &config).is_ok());
}

#[test]
fn dexscreener_baseline_token_passes() {
    assert!(dexscreener::evaluate(&dex_token(), &DexScreenerFilters::default()).is_ok());
}

#[test]
fn dexscreener_requires_name_and_symbol() {
    let config = DexScreenerFilters::default();

    let mut token = dex_token();
    token.name = "   ".to_owned();
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerEmptyName
    );

    let mut token = dex_token();
    token.symbol = String::new();
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerEmptySymbol
    );
}

#[test]
fn dexscreener_optional_logo_and_website_requirements() {
    let config = DexScreenerFilters {
        require_logo_url: true,
        require_website_url: true,
        ..Default::default()
    };

    let mut token = dex_token();
    token.image_url = Some("  ".to_owned());
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerEmptyLogoUrl,
        "a blank logo string counts as missing, not present"
    );

    let mut token = dex_token();
    token.websites.clear();
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerEmptyWebsiteUrl
    );

    // Both requirements are off by default.
    let mut token = dex_token();
    token.image_url = None;
    token.websites.clear();
    assert!(dexscreener::evaluate(&token, &DexScreenerFilters::default()).is_ok());
}

#[test]
fn dexscreener_transaction_minimums() {
    let config = DexScreenerFilters {
        min_transactions_5min: 10,
        min_transactions_1h: 100,
        ..Default::default()
    };

    let mut token = dex_token();
    token.txns_m5_buys = Some(4);
    token.txns_m5_sells = Some(5); // 9 < 10
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerInsufficientTransactions5Min
    );

    let mut token = dex_token();
    token.txns_m5_buys = Some(5);
    token.txns_m5_sells = Some(5); // exactly 10 — the minimum passes
    token.txns_h1_buys = Some(50);
    token.txns_h1_sells = Some(49); // 99 < 100
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerInsufficientTransactions1H
    );
}

#[test]
fn dexscreener_missing_5m_transactions_skips_the_1h_minimum_too() {
    // DEFECT PIN: `check_transaction_activity` returns None (= pass) as soon as either
    // 5m counter is absent, so it never reaches the 1h check. A token with no 5m data
    // and ZERO hourly trades satisfies the activity filter.
    let config = DexScreenerFilters {
        min_transactions_5min: 10,
        min_transactions_1h: 500,
        ..Default::default()
    };
    let mut token = dex_token();
    token.txns_m5_buys = None;
    token.txns_m5_sells = None;
    token.txns_h1_buys = Some(0);
    token.txns_h1_sells = Some(0);

    assert!(
        dexscreener::evaluate(&token, &config).is_ok(),
        "missing 5m counters must not silently waive the 1h activity minimum"
    );
}

#[test]
fn dexscreener_transaction_check_only_applies_to_dexscreener_sourced_tokens() {
    let config = DexScreenerFilters {
        min_transactions_1h: 1_000_000,
        ..Default::default()
    };
    let mut token = dex_token();
    token.data_source = DataSource::GeckoTerminal;

    assert!(dexscreener::evaluate(&token, &config).is_ok());
}

#[test]
fn dexscreener_liquidity_bounds_and_boundaries() {
    let config = DexScreenerFilters {
        min_liquidity_usd: 1_000.0,
        max_liquidity_usd: 100_000.0,
        ..Default::default()
    };

    let mut token = dex_token();
    token.liquidity_usd = Some(0.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerZeroLiquidity
    );

    token.liquidity_usd = Some(-5.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerZeroLiquidity,
        "negative liquidity is a corrupted feed, not a low one"
    );

    token.liquidity_usd = Some(999.99);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerInsufficientLiquidity
    );

    token.liquidity_usd = Some(100_000.01);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerLiquidityTooHigh
    );

    // Both bounds are inclusive.
    token.liquidity_usd = Some(1_000.0);
    assert!(dexscreener::evaluate(&token, &config).is_ok());
    token.liquidity_usd = Some(100_000.0);
    assert!(dexscreener::evaluate(&token, &config).is_ok());
}

#[test]
fn dexscreener_missing_liquidity_and_market_cap_pass_but_missing_fdv_rejects() {
    // DEFECT PIN: three money filters, three different answers to "no data". Liquidity
    // and market cap treat absence as acceptable; FDV rejects it. GeckoTerminal rejects
    // absence for all of them. Nothing in the config expresses this difference.
    let config = DexScreenerFilters {
        fdv_enabled: true,
        ..Default::default()
    };

    let mut token = dex_token();
    token.liquidity_usd = None;
    token.market_cap = None;
    assert!(
        dexscreener::evaluate(&token, &config).is_ok(),
        "absent liquidity/market cap currently waive their own checks"
    );

    let mut token = dex_token();
    token.fdv = None;
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerFdvMissing
    );
}

#[test]
fn dexscreener_market_cap_and_fdv_bounds() {
    let config = DexScreenerFilters {
        min_market_cap_usd: 10_000.0,
        max_market_cap_usd: 1_000_000.0,
        fdv_enabled: true,
        min_fdv_usd: 20_000.0,
        max_fdv_usd: 2_000_000.0,
        ..Default::default()
    };

    let mut token = dex_token();
    token.market_cap = Some(9_999.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerMarketCapTooLow
    );

    token.market_cap = Some(1_000_001.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerMarketCapTooHigh
    );

    token.market_cap = Some(500_000.0);
    token.fdv = Some(19_999.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerFdvTooLow
    );

    token.fdv = Some(2_000_001.0);
    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerFdvTooHigh
    );
}

#[test]
fn dexscreener_volume_threshold_of_zero_disables_that_window() {
    let config = DexScreenerFilters {
        volume_enabled: true,
        ..Default::default() // every min_volume_* is 0.0
    };
    let mut token = dex_token();
    token.volume_m5 = None;
    token.volume_h1 = None;
    token.volume_h6 = None;
    token.volume_h24 = None;

    assert!(
        dexscreener::evaluate(&token, &config).is_ok(),
        "a zero threshold must not demand the data it does not use"
    );
}

#[test]
fn dexscreener_volume_windows_reject_low_and_missing_values() {
    let config = DexScreenerFilters {
        volume_enabled: true,
        min_volume_5m: 100.0,
        min_volume_1h: 1_000.0,
        min_volume_6h: 5_000.0,
        min_volume_24h: 20_000.0,
        ..Default::default()
    };

    let cases: [Case; 8] = [
        (
            |t| t.volume_m5 = Some(99.0),
            FilterRejectionReason::DexScreenerVolume5mTooLow,
        ),
        (
            |t| t.volume_m5 = None,
            FilterRejectionReason::DexScreenerVolume5mMissing,
        ),
        (
            |t| t.volume_h1 = Some(999.0),
            FilterRejectionReason::DexScreenerVolume1hTooLow,
        ),
        (
            |t| t.volume_h1 = None,
            FilterRejectionReason::DexScreenerVolume1hMissing,
        ),
        (
            |t| t.volume_h6 = Some(4_999.0),
            FilterRejectionReason::DexScreenerVolume6hTooLow,
        ),
        (
            |t| t.volume_h6 = None,
            FilterRejectionReason::DexScreenerVolume6hMissing,
        ),
        (
            |t| t.volume_h24 = Some(19_999.0),
            FilterRejectionReason::DexScreenerVolumeTooLow,
        ),
        (
            |t| t.volume_h24 = None,
            FilterRejectionReason::DexScreenerVolumeMissing,
        ),
    ];

    for (mutate, expected) in cases {
        let mut token = dex_token();
        mutate(&mut token);
        assert_eq!(rejection(dexscreener::evaluate(&token, &config)), expected);
    }

    // Exactly at the threshold passes (`<` rejects).
    let mut token = dex_token();
    token.volume_m5 = Some(100.0);
    token.volume_h1 = Some(1_000.0);
    token.volume_h6 = Some(5_000.0);
    token.volume_h24 = Some(20_000.0);
    assert!(dexscreener::evaluate(&token, &config).is_ok());
}

#[test]
fn dexscreener_price_change_rejects_missing_data_even_with_an_open_range() {
    // DEFECT PIN: the volume helper skips a window whose threshold is 0, but the price
    // change helper has no such escape — enabling price-change checks with the DEFAULT
    // (-100%, +10000%) range still rejects every token missing any one window.
    let config = DexScreenerFilters {
        price_change_enabled: true,
        ..Default::default()
    };
    let mut token = dex_token();
    token.price_change_m5 = None;

    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerPriceChange5mMissing,
        "an unbounded range still demands the value"
    );
}

#[test]
fn dexscreener_price_change_bounds_per_window() {
    let config = DexScreenerFilters {
        price_change_enabled: true,
        min_price_change_m5: -10.0,
        max_price_change_m5: 10.0,
        min_price_change_h1: -20.0,
        max_price_change_h1: 20.0,
        min_price_change_h6: -30.0,
        max_price_change_h6: 30.0,
        min_price_change_h24: -40.0,
        max_price_change_h24: 40.0,
        ..Default::default()
    };

    let cases: [Case; 8] = [
        (
            |t| t.price_change_m5 = Some(-10.1),
            FilterRejectionReason::DexScreenerPriceChange5mTooLow,
        ),
        (
            |t| t.price_change_m5 = Some(10.1),
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh,
        ),
        (
            |t| t.price_change_h1 = Some(-20.1),
            FilterRejectionReason::DexScreenerPriceChangeTooLow,
        ),
        (
            |t| t.price_change_h1 = Some(20.1),
            FilterRejectionReason::DexScreenerPriceChangeTooHigh,
        ),
        (
            |t| t.price_change_h6 = Some(-30.1),
            FilterRejectionReason::DexScreenerPriceChange6hTooLow,
        ),
        (
            |t| t.price_change_h6 = Some(30.1),
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh,
        ),
        (
            |t| t.price_change_h24 = Some(-40.1),
            FilterRejectionReason::DexScreenerPriceChange24hTooLow,
        ),
        (
            |t| t.price_change_h24 = Some(40.1),
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh,
        ),
    ];

    for (mutate, expected) in cases {
        let mut token = dex_token();
        mutate(&mut token);
        assert_eq!(rejection(dexscreener::evaluate(&token, &config)), expected);
    }
}

#[test]
fn dexscreener_nan_values_slip_through_every_numeric_bound() {
    // DEFECT PIN: NaN compares false against `<`, `>` and `<=` alike, so a corrupted feed
    // value satisfies min AND max at once. Only an explicit `is_finite` guard would catch
    // it, and no source has one.
    let config = DexScreenerFilters {
        min_liquidity_usd: 1_000.0,
        max_liquidity_usd: 10_000.0,
        min_market_cap_usd: 1_000.0,
        max_market_cap_usd: 10_000.0,
        volume_enabled: true,
        min_volume_24h: 5_000.0,
        price_change_enabled: true,
        min_price_change_h24: -50.0,
        max_price_change_h24: 50.0,
        ..Default::default()
    };
    let mut token = dex_token();
    token.liquidity_usd = Some(f64::NAN);
    token.market_cap = Some(f64::NAN);
    token.volume_h24 = Some(f64::NAN);
    token.price_change_h24 = Some(f64::NAN);

    assert!(
        dexscreener::evaluate(&token, &config).is_ok(),
        "NaN market data currently passes bounds it should fail"
    );
}

#[test]
fn dexscreener_infinite_values_are_caught_by_the_upper_bounds() {
    let config = DexScreenerFilters {
        max_liquidity_usd: 10_000.0,
        ..Default::default()
    };
    let mut token = dex_token();
    token.liquidity_usd = Some(f64::INFINITY);

    assert_eq!(
        rejection(dexscreener::evaluate(&token, &config)),
        FilterRejectionReason::DexScreenerLiquidityTooHigh
    );
}

// ============================================================================
// GECKOTERMINAL
// ============================================================================

#[test]
fn geckoterminal_disabled_skips_every_check() {
    let config = GeckoTerminalFilters {
        enabled: false,
        ..Default::default()
    };
    let mut token = gecko_token();
    token.liquidity_usd = None;
    token.market_cap = None;

    assert!(geckoterminal::evaluate(&token, &config).is_ok());
}

#[test]
fn geckoterminal_ignores_tokens_from_another_source() {
    let config = GeckoTerminalFilters {
        min_liquidity_usd: 1_000_000_000.0,
        ..Default::default()
    };
    let token = dex_token(); // DexScreener-sourced

    assert!(
        geckoterminal::evaluate(&token, &config).is_ok(),
        "the source gate short-circuits before any threshold is read"
    );
}

#[test]
fn geckoterminal_missing_liquidity_and_market_cap_reject() {
    let config = GeckoTerminalFilters {
        market_cap_enabled: true,
        ..Default::default()
    };

    let mut token = gecko_token();
    token.liquidity_usd = None;
    assert_eq!(
        rejection(geckoterminal::evaluate(&token, &config)),
        FilterRejectionReason::GeckoTerminalLiquidityMissing,
        "opposite of DexScreener, which lets absent liquidity through"
    );

    let mut token = gecko_token();
    token.market_cap = None;
    assert_eq!(
        rejection(geckoterminal::evaluate(&token, &config)),
        FilterRejectionReason::GeckoTerminalMarketCapMissing
    );
}

#[test]
fn geckoterminal_zero_maximum_means_no_upper_bound() {
    let config = GeckoTerminalFilters {
        max_liquidity_usd: 0.0,
        market_cap_enabled: true,
        max_market_cap_usd: 0.0,
        ..Default::default()
    };
    let mut token = gecko_token();
    token.liquidity_usd = Some(9.9e12);
    token.market_cap = Some(9.9e12);

    assert!(
        geckoterminal::evaluate(&token, &config).is_ok(),
        "0 is the sentinel for 'unbounded', not for 'reject everything'"
    );
}

#[test]
fn geckoterminal_liquidity_and_market_cap_bounds() {
    let config = GeckoTerminalFilters {
        min_liquidity_usd: 1_000.0,
        max_liquidity_usd: 100_000.0,
        market_cap_enabled: true,
        min_market_cap_usd: 10_000.0,
        max_market_cap_usd: 1_000_000.0,
        ..Default::default()
    };

    let cases: [Case; 4] = [
        (
            |t| t.liquidity_usd = Some(999.0),
            FilterRejectionReason::GeckoTerminalLiquidityTooLow,
        ),
        (
            |t| t.liquidity_usd = Some(100_001.0),
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh,
        ),
        (
            |t| t.market_cap = Some(9_999.0),
            FilterRejectionReason::GeckoTerminalMarketCapTooLow,
        ),
        (
            |t| t.market_cap = Some(1_000_001.0),
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh,
        ),
    ];

    for (mutate, expected) in cases {
        let mut token = gecko_token();
        mutate(&mut token);
        assert_eq!(
            rejection(geckoterminal::evaluate(&token, &config)),
            expected
        );
    }
}

#[test]
fn geckoterminal_volume_and_price_change_windows() {
    let config = GeckoTerminalFilters {
        volume_enabled: true,
        min_volume_5m: 100.0,
        min_volume_1h: 1_000.0,
        min_volume_24h: 20_000.0,
        price_change_enabled: true,
        min_price_change_h24: -25.0,
        max_price_change_h24: 25.0,
        ..Default::default()
    };

    let cases: [Case; 7] = [
        (
            |t| t.volume_m5 = Some(99.0),
            FilterRejectionReason::GeckoTerminalVolume5mTooLow,
        ),
        (
            |t| t.volume_m5 = None,
            FilterRejectionReason::GeckoTerminalVolume5mMissing,
        ),
        (
            |t| t.volume_h1 = Some(1.0),
            FilterRejectionReason::GeckoTerminalVolume1hTooLow,
        ),
        (
            |t| t.volume_h24 = None,
            FilterRejectionReason::GeckoTerminalVolume24hMissing,
        ),
        (
            |t| t.price_change_h24 = Some(-25.1),
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow,
        ),
        (
            |t| t.price_change_h24 = Some(25.1),
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh,
        ),
        (
            |t| t.price_change_h24 = None,
            FilterRejectionReason::GeckoTerminalPriceChange24hMissing,
        ),
    ];

    for (mutate, expected) in cases {
        let mut token = gecko_token();
        mutate(&mut token);
        assert_eq!(
            rejection(geckoterminal::evaluate(&token, &config)),
            expected
        );
    }
}

#[test]
fn geckoterminal_pool_metrics_zero_thresholds_are_inert() {
    let config = GeckoTerminalFilters {
        pool_metrics_enabled: true,
        min_pool_count: 0,
        max_pool_count: 0,
        min_reserve_usd: 0.0,
        ..Default::default()
    };
    let mut token = gecko_token();
    token.pool_count = None;
    token.reserve_in_usd = None;

    assert!(geckoterminal::evaluate(&token, &config).is_ok());
}

#[test]
fn geckoterminal_pool_metrics_bounds() {
    let config = GeckoTerminalFilters {
        pool_metrics_enabled: true,
        min_pool_count: 2,
        max_pool_count: 10,
        min_reserve_usd: 5_000.0,
        ..Default::default()
    };

    let cases: [Case; 5] = [
        (
            |t| t.pool_count = Some(1),
            FilterRejectionReason::GeckoTerminalPoolCountTooLow,
        ),
        (
            |t| t.pool_count = None,
            FilterRejectionReason::GeckoTerminalPoolCountMissing,
        ),
        (
            |t| t.pool_count = Some(11),
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh,
        ),
        (
            |t| t.reserve_in_usd = Some(4_999.0),
            FilterRejectionReason::GeckoTerminalReserveTooLow,
        ),
        (
            |t| t.reserve_in_usd = None,
            FilterRejectionReason::GeckoTerminalReserveMissing,
        ),
    ];

    for (mutate, expected) in cases {
        let mut token = gecko_token();
        mutate(&mut token);
        assert_eq!(
            rejection(geckoterminal::evaluate(&token, &config)),
            expected
        );
    }
}

// ============================================================================
// RUGCHECK
// ============================================================================

#[test]
fn rugcheck_disabled_skips_every_check() {
    let config = RugCheckFilters {
        enabled: false,
        ..Default::default()
    };
    let mut token = dex_token();
    token.is_rugged = true;
    token.mint_authority = Some("Mint111111111111111111111111111111111111111".to_owned());

    assert!(rugcheck::evaluate(&token, &config).is_ok());
}

#[test]
fn rugcheck_baseline_token_passes() {
    assert!(rugcheck::evaluate(&dex_token(), &RugCheckFilters::default()).is_ok());
}

#[test]
fn rugcheck_blocks_rugged_tokens() {
    let mut token = dex_token();
    token.is_rugged = true;

    assert_eq!(
        rejection(rugcheck::evaluate(&token, &RugCheckFilters::default())),
        FilterRejectionReason::RugcheckRuggedToken
    );

    let permissive = RugCheckFilters {
        block_rugged_tokens: false,
        ..Default::default()
    };
    assert!(rugcheck::evaluate(&token, &permissive).is_ok());
}

#[test]
fn rugcheck_risk_score_boundary_and_absence() {
    let config = RugCheckFilters {
        max_risk_score: 10_000,
        ..Default::default()
    };

    let mut token = dex_token();
    token.security_score = Some(10_001);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckRiskScoreTooHigh
    );

    token.security_score = Some(10_000);
    assert!(rugcheck::evaluate(&token, &config).is_ok(), "`>` is strict");

    token.security_score = None;
    assert!(
        rugcheck::evaluate(&token, &config).is_ok(),
        "an unscored token is not rejected by the score rule"
    );
}

#[test]
fn rugcheck_danger_level_is_case_insensitive() {
    let config = RugCheckFilters::default();

    for level in ["danger", "Danger", "DANGER"] {
        let mut token = dex_token();
        token.security_risks = vec![security_risk("Some Risk", "", "", level)];
        assert_eq!(
            rejection(rugcheck::evaluate(&token, &config)),
            FilterRejectionReason::RugcheckRiskLevelDanger,
            "level {level:?} must match"
        );
    }

    let mut token = dex_token();
    token.security_risks = vec![security_risk("Some Risk", "", "", "warn")];
    assert!(rugcheck::evaluate(&token, &config).is_ok());
}

#[test]
fn rugcheck_authority_rules() {
    let config = RugCheckFilters::default();

    let mut token = dex_token();
    token.mint_authority = Some("Mint111111111111111111111111111111111111111".to_owned());
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckMintAuthorityBlocked
    );

    let mut token = dex_token();
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckFreezeAuthorityBlocked
    );

    // Explicit allowances.
    let permissive = RugCheckFilters {
        allow_mint_authority: true,
        allow_freeze_authority: true,
        ..Default::default()
    };
    let mut token = dex_token();
    token.mint_authority = Some("Mint111111111111111111111111111111111111111".to_owned());
    token.freeze_authority = Some("Freeze1111111111111111111111111111111111111".to_owned());
    assert!(rugcheck::evaluate(&token, &permissive).is_ok());

    // The master switch also waives both.
    let unchecked = RugCheckFilters {
        require_authorities_safe: false,
        ..Default::default()
    };
    assert!(rugcheck::evaluate(&token, &unchecked).is_ok());
}

#[test]
fn rugcheck_holder_distribution_rules() {
    let config = RugCheckFilters {
        min_unique_holders: 50,
        max_top_holder_pct: 40.0,
        max_top_3_holders_pct: 60.0,
        ..Default::default()
    };

    let mut token = dex_token();
    token.total_holders = Some(49);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckNotEnoughHolders
    );

    let mut token = dex_token();
    token.total_holders = None;
    assert!(
        rugcheck::evaluate(&token, &config).is_ok(),
        "an unknown holder count is not treated as too few"
    );

    let mut token = dex_token();
    token.top_holders = vec![
        holder("A11111111111111111111111111111111111111111", 41.0, false),
        holder("B11111111111111111111111111111111111111111", 1.0, false),
    ];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckTopHolderTooHigh
    );

    let mut token = dex_token();
    token.top_holders = vec![
        holder("A11111111111111111111111111111111111111111", 25.0, false),
        holder("B11111111111111111111111111111111111111111", 20.0, false),
        holder("C11111111111111111111111111111111111111111", 16.0, false),
        holder("D11111111111111111111111111111111111111111", 10.0, false),
    ];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckTop3HoldersTooHigh,
        "61% across the top three exceeds the 60% ceiling"
    );
}

#[test]
fn rugcheck_finds_the_largest_holder_regardless_of_input_order() {
    let config = RugCheckFilters {
        max_top_holder_pct: 40.0,
        max_top_3_holders_pct: 100.0,
        ..Default::default()
    };
    let mut token = dex_token();
    // The dangerous holder is last in the provider's list.
    token.top_holders = vec![
        holder("A11111111111111111111111111111111111111111", 1.0, false),
        holder("B11111111111111111111111111111111111111111", 2.0, false),
        holder("C11111111111111111111111111111111111111111", 90.0, false),
    ];

    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckTopHolderTooHigh
    );
}

#[test]
fn rugcheck_insider_count_looks_at_ten_holders_but_the_percentage_looks_at_all() {
    // The two insider rules disagree on scope: the COUNT rule takes the first 10 holders
    // as given by the provider, while the PERCENTAGE rule sums every insider in the list.
    let config = RugCheckFilters {
        max_insider_holders_in_top_10: 1,
        max_insider_total_pct: 20.0,
        max_top_holder_pct: 100.0,
        max_top_3_holders_pct: 100.0,
        ..Default::default()
    };

    let mut token = dex_token();
    token.top_holders = (0..12)
        .map(|i| {
            holder(
                &format!("Holder{i:037}"),
                1.0,
                // Insiders sit at positions 10 and 11 — outside the count window.
                i >= 10,
            )
        })
        .collect();
    assert!(
        rugcheck::evaluate(&token, &config).is_ok(),
        "two insiders beyond the tenth holder do not trip the count rule"
    );

    let mut token = dex_token();
    token.top_holders = vec![
        holder("A11111111111111111111111111111111111111111", 2.0, true),
        holder("B11111111111111111111111111111111111111111", 2.0, true),
    ];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckInsiderHolderCount
    );

    let mut token = dex_token();
    token.top_holders = vec![holder(
        "A11111111111111111111111111111111111111111",
        20.1,
        true,
    )];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckInsiderTotalPct
    );
}

#[test]
fn rugcheck_graph_insider_and_creator_balance_bounds() {
    let config = RugCheckFilters {
        max_graph_insiders: 3,
        max_creator_balance_pct: 10.0,
        ..Default::default()
    };

    let mut token = dex_token();
    token.graph_insiders_detected = Some(4);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckGraphInsidersTooHigh
    );

    token.graph_insiders_detected = Some(3);
    assert!(rugcheck::evaluate(&token, &config).is_ok());

    let mut token = dex_token();
    token.creator_balance_pct = Some(10.1);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckCreatorBalanceTooHigh
    );

    token.creator_balance_pct = None;
    assert!(rugcheck::evaluate(&token, &config).is_ok());
}

#[test]
fn rugcheck_missing_transfer_fee_data_rejects_under_the_default_config() {
    // DEFECT PIN: the code comment says missing transfer-fee data is acceptable, but any
    // `max_transfer_fee_pct` below 100 turns absence into a rejection — and the default
    // is 5. Most SPL tokens have no transfer-fee extension at all, so this rule rejects
    // them on a technicality unless the field is explicitly populated with 0.
    let config = RugCheckFilters::default();
    let mut token = dex_token();
    token.transfer_fee_pct = None;

    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckTransferFeeMissing
    );

    let opt_out = RugCheckFilters {
        max_transfer_fee_pct: 100.0,
        ..Default::default()
    };
    assert!(
        rugcheck::evaluate(&token, &opt_out).is_ok(),
        "only a 100% ceiling waives the requirement"
    );
}

#[test]
fn rugcheck_transfer_fee_present_and_too_high() {
    let block_any = RugCheckFilters {
        block_transfer_fee_tokens: true,
        ..Default::default()
    };
    let mut token = dex_token();
    token.transfer_fee_pct = Some(0.1);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &block_any)),
        FilterRejectionReason::RugcheckTransferFeePresent
    );

    // Exactly zero is not "present".
    token.transfer_fee_pct = Some(0.0);
    assert!(rugcheck::evaluate(&token, &block_any).is_ok());

    let config = RugCheckFilters::default(); // max 5%
    token.transfer_fee_pct = Some(5.1);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckTransferFeeTooHigh
    );
    token.transfer_fee_pct = Some(5.0);
    assert!(rugcheck::evaluate(&token, &config).is_ok());
}

#[test]
fn rugcheck_lp_provider_rules() {
    let config = RugCheckFilters {
        min_lp_providers: 3,
        ..Default::default()
    };

    let mut token = dex_token();
    token.lp_provider_count = Some(2);
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpProvidersTooLow
    );

    token.lp_provider_count = Some(3);
    assert!(rugcheck::evaluate(&token, &config).is_ok());

    token.lp_provider_count = None;
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpProvidersMissing
    );
}

#[test]
fn rugcheck_lp_lock_is_only_mandatory_for_pumpfun_tokens() {
    let config = RugCheckFilters::default();

    let mut token = dex_token();
    token.token_type = Some("spl".to_owned());
    token.security_risks.clear();
    assert!(
        rugcheck::evaluate(&token, &config).is_ok(),
        "a regular token with no lock data is allowed through"
    );

    token.token_type = Some("pumpfun".to_owned());
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpLockMissing,
        "a PumpFun token must prove its lock"
    );
}

#[test]
fn rugcheck_lp_lock_percentage_parsing() {
    let config = RugCheckFilters {
        min_pumpfun_lp_lock_pct: 50.0,
        min_regular_lp_lock_pct: 50.0,
        ..Default::default()
    };

    // Percentage read out of the risk `value`.
    let mut token = dex_token();
    token.security_risks = vec![security_risk("LP Locked", "45%", "", "warn")];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpLockTooLow
    );

    // Exactly the minimum passes.
    token.security_risks = vec![security_risk("LP Locked", "50%", "", "warn")];
    assert!(rugcheck::evaluate(&token, &config).is_ok());

    // Read out of the description when the value carries no number.
    token.security_risks = vec![security_risk(
        "LP Lock",
        "n/a",
        "Liquidity locked 30% of supply",
        "warn",
    )];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpLockTooLow
    );

    // Keyword inference: "unlocked" means zero.
    token.security_risks = vec![security_risk("LP Lock", "unlocked", "", "warn")];
    assert_eq!(
        rejection(rugcheck::evaluate(&token, &config)),
        FilterRejectionReason::RugcheckLpLockTooLow
    );

    // Keyword inference: plain "locked" means fully locked.
    token.security_risks = vec![security_risk("LP Lock", "locked", "", "warn")];
    assert!(rugcheck::evaluate(&token, &config).is_ok());

    // A risk row that is not about LP locking is ignored entirely.
    token.security_risks = vec![security_risk("Mutable metadata", "10%", "", "warn")];
    token.token_type = Some("spl".to_owned());
    assert!(rugcheck::evaluate(&token, &config).is_ok());
}

// ============================================================================
// REJECTION TAXONOMY
// ============================================================================

/// Every reason a source can produce, paired with the source it must be attributed to.
fn reason_source_pairs() -> Vec<(FilterRejectionReason, FilterSource)> {
    vec![
        (
            FilterRejectionReason::NoDecimalsInDatabase,
            FilterSource::Core,
        ),
        (FilterRejectionReason::TokenTooNew, FilterSource::Core),
        (FilterRejectionReason::CooldownFiltered, FilterSource::Core),
        (
            FilterRejectionReason::DexScreenerDataMissing,
            FilterSource::Core,
        ),
        (
            FilterRejectionReason::GeckoTerminalDataMissing,
            FilterSource::Core,
        ),
        (
            FilterRejectionReason::RugcheckDataMissing,
            FilterSource::Core,
        ),
        (
            FilterRejectionReason::OnChainNumericSymbol,
            FilterSource::OnChain,
        ),
        (
            FilterRejectionReason::OnChainHighRiskScore,
            FilterSource::OnChain,
        ),
        (
            FilterRejectionReason::DexScreenerZeroLiquidity,
            FilterSource::DexScreener,
        ),
        (
            FilterRejectionReason::DexScreenerFdvMissing,
            FilterSource::DexScreener,
        ),
        (
            FilterRejectionReason::GeckoTerminalReserveTooLow,
            FilterSource::GeckoTerminal,
        ),
        (
            FilterRejectionReason::RugcheckLpLockMissing,
            FilterSource::Rugcheck,
        ),
        (
            FilterRejectionReason::AiRejected {
                reason: "looks like a scam".to_owned(),
                confidence: 90,
                provider: "test".to_owned(),
            },
            FilterSource::Ai,
        ),
    ]
}

#[test]
fn rejection_reasons_are_attributed_to_the_right_source() {
    for (reason, expected) in reason_source_pairs() {
        assert_eq!(
            reason.source(),
            expected,
            "{} is attributed to the wrong source",
            reason.label()
        );
    }
}

#[test]
fn rejection_labels_are_machine_readable_and_displayable() {
    for (reason, _) in reason_source_pairs() {
        let label = reason.label();
        assert!(!label.is_empty(), "empty label");
        assert_eq!(
            label,
            label.to_lowercase(),
            "{label} must stay lowercase — the actions-history filter matches on it"
        );
        assert!(
            !label.contains(' '),
            "{label} must not contain spaces (it is persisted as a code)"
        );
        assert_eq!(reason.to_string(), label, "Display must mirror label()");
        assert!(!reason.display_label().is_empty(), "empty display label");
    }
}

#[test]
fn ai_rejection_carries_its_reasoning_into_the_display_label() {
    let reason = FilterRejectionReason::AiRejected {
        reason: "unverifiable team".to_owned(),
        confidence: 72,
        provider: "anthropic".to_owned(),
    };

    assert_eq!(reason.label(), "ai_rejected", "the code stays constant");
    let display = reason.display_label();
    assert!(display.contains("unverifiable team"));
    assert!(display.contains("72"));
    assert!(display.contains("anthropic"));
}

#[test]
fn ai_rejections_with_different_details_are_distinct_values() {
    // The engine counts rejections in a HashMap keyed by reason, so two AI rejections
    // with different reasoning must not collapse into one another.
    let first = FilterRejectionReason::AiRejected {
        reason: "a".to_owned(),
        confidence: 90,
        provider: "p".to_owned(),
    };
    let second = FilterRejectionReason::AiRejected {
        reason: "b".to_owned(),
        confidence: 90,
        provider: "p".to_owned(),
    };

    assert_ne!(first, second);
    assert_eq!(first.label(), second.label());
}
