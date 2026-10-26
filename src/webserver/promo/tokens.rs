// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generator for the Tokens page — the token universe behind
//! `/api/tokens/list`, `/api/tokens/stats` and `/api/tokens/favorites`.
//!
//! The Tokens page is the only surface whose emptiness is not a fixture gap but a
//! fact about the machine it was captured on: a local database that has been
//! running for an hour holds a handful of tokens, so Pool Service photographs as
//! "No results found" and Positions as two rows. The promo session substitutes a
//! universe at the scale the rest of the promo already claims —
//! `PROMO_TOKENS_TRACKED` monitored, `PROMO_BLACKLISTED` blacklisted — so the
//! Tokens tabs and the status bar above them stop contradicting each other.
//!
//! **Two populations, deliberately.** The head is curated: every token the promo
//! holds or has closed, with its real mint, real logo and hand-written market data,
//! because those are the rows a screenshot actually shows. The tail is generated —
//! the thousands of low-liquidity launches that make up any real Solana token
//! database and that exist here only so the counters are honest about what a page
//! of results was drawn from. The default sort is liquidity descending, so the
//! curated head is what lands on screen and the tail stays where it belongs.
//!
//! Generated identities are deterministic (a fixed-seed LCG, no clock, no
//! randomness) so two capture runs produce the same table.

use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{Duration, Utc};

use crate::config::{try_with_config, FilteringConfig};
use crate::filtering::sources::{rejection_source, FilterSource};
use crate::filtering::{
    BlacklistReasonInfo, FilteringQuery, FilteringView, SortDirection, TokenSortKey,
};
use crate::tokens::types::{DataSource, Priority, SecurityRisk, Token};
use crate::webserver::routes::tokens::types::{TokenListItem, TokenListResponse};

use super::data::{
    PROMO_BLACKLISTED, PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS, PROMO_TOKENS_TRACKED,
};

/// Passed tokens in the promo universe — the count the header already reports.
const PROMO_PASSED: usize = 347;

/// How many of the newest discoveries the Recent view shows.
///
/// The real view is time-boxed; at promo scale a fixed slice of the newest
/// discoveries is the same thing said in a way that cannot depend on how long the
/// session has been running.
const PROMO_RECENT: usize = 120;

/// Tokens with no market data at all — discovered, never priced by any venue.
const PROMO_NO_MARKET: usize = 412;

/// Rejected tokens the pool service still prices on-chain (see `has_pool_price`).
const PROMO_POOL_TRACKED_REJECTED: usize = 300;

/// Which curated tokens are pinned as favorites.
const PROMO_FAVORITE_SYMBOLS: &[&str] = &["WIF", "Fartcoin", "GOAT", "MOODENG", "GIGA"];

/// SOL/USD used to convert the curated SOL prices into the USD fields the market
/// columns render. The same constant the rest of the promo prices against.
const PROMO_SOL_USD: f64 = super::PROMO_SOL_PRICE_FALLBACK;

/// Rejection reasons of priced tokens, as the filtering pipeline codes them, each
/// with the weight it is drawn by.
///
/// These are the real codes — a promo screenshot of the Rejected view is showing
/// the product's own vocabulary, and inventing reason strings for it would teach a
/// reader something untrue about how filtering explains itself. The weights give
/// the breakdowns the shape a live corpus has: market activity and liquidity reject
/// most priced tokens, safety checks a smaller share, and every source the
/// Filtering page has a settings tab for raises at least one reason. Each token's
/// source is the one the pipeline assigns its code (`rejection_source`). A code is
/// drawn only while the session's filtering config runs its rule (`rule_enabled`),
/// so no tab counts rejections from a rule its settings show switched off.
const PROMO_REJECTIONS: &[(&str, usize)] = &[
    ("dex_txn_5m", 420),
    ("dex_liq_low", 300),
    ("dex_empty_logo", 170),
    ("dex_mcap_low", 150),
    ("token_too_new", 120),
    ("dex_vol_low", 110),
    ("dex_txn_1h", 90),
    ("gecko_vol5m_low", 70),
    ("rug_score", 70),
    ("rug_top_holder", 60),
    ("onchain_numeric_symbol", 55),
    ("gecko_liq_low", 50),
    ("rug_lp_lock_low", 50),
    ("rug_mint_authority", 45),
    ("onchain_suspicious_symbol", 40),
    ("rug_data_missing", 40),
    ("rug_min_holders", 40),
    ("gecko_pool_count_low", 35),
    ("onchain_high_risk_score", 35),
    ("rug_freeze_authority", 30),
    ("onchain_immutable_with_freeze", 25),
    ("rug_insider_pct", 25),
    ("rug_transfer_fee_high", 15),
    ("rug_transfer_fee_present", 15),
];

/// The reasons of a token no venue has priced yet, in the order the pipeline
/// attributes it (`filtering::sources::market`): DexScreener while that source is
/// on, otherwise GeckoTerminal. An unpriced token is rejected for exactly that,
/// whatever else is true of it — the pipeline never reaches a market rule it has no
/// data for.
const PROMO_NO_MARKET_REJECTIONS: [&str; 2] = ["dex_data_missing", "gecko_data_missing"];

/// The filtering config the sample corpus is rejected under, read once when the
/// universe is built; the shipped defaults when no config is loaded.
static FILTERING_CONFIG: LazyLock<FilteringConfig> =
    LazyLock::new(|| try_with_config(|cfg| cfg.filtering.clone()).unwrap_or_default());

/// Whether `config` runs the rule behind a sample rejection code, by the same switches
/// the pipeline checks before it can produce that code; `None` for a code this table
/// does not know.
fn rule_enabled(code: &str, config: &FilteringConfig) -> Option<bool> {
    let dex = &config.dexscreener;
    let gecko = &config.geckoterminal;
    let rug = &config.rugcheck;
    let onchain = &config.onchain;
    let rug_authorities =
        rug.enabled && rug.authority_checks_enabled && rug.require_authorities_safe;
    Some(match code {
        "token_too_new" => config.age_enabled,
        "dex_data_missing" => dex.enabled,
        "gecko_data_missing" => gecko.enabled && !dex.enabled,
        "dex_txn_5m" | "dex_txn_1h" => dex.enabled && dex.transactions_enabled,
        "dex_liq_low" => dex.enabled && dex.liquidity_enabled,
        "dex_mcap_low" => dex.enabled && dex.market_cap_enabled,
        "dex_vol_low" => dex.enabled && dex.volume_enabled,
        "dex_empty_logo" => dex.enabled && dex.token_info_enabled && dex.require_logo_url,
        "gecko_liq_low" => gecko.enabled && gecko.liquidity_enabled,
        "gecko_vol5m_low" => gecko.enabled && gecko.volume_enabled,
        "gecko_pool_count_low" => {
            gecko.enabled && gecko.pool_metrics_enabled && gecko.min_pool_count > 0
        }
        "rug_data_missing" => rug.enabled,
        "rug_score" => rug.enabled && rug.risk_score_enabled,
        "rug_mint_authority" => rug_authorities && !rug.allow_mint_authority,
        "rug_freeze_authority" => rug_authorities && !rug.allow_freeze_authority,
        "rug_top_holder" | "rug_min_holders" => rug.enabled && rug.holder_distribution_enabled,
        "rug_lp_lock_low" => rug.enabled && rug.lp_lock_enabled,
        "rug_insider_pct" => rug.enabled && rug.insider_holder_checks_enabled,
        "rug_transfer_fee_high" => rug.enabled && rug.transfer_fee_enabled,
        "rug_transfer_fee_present" => {
            rug.enabled && rug.transfer_fee_enabled && rug.block_transfer_fee_tokens
        }
        "onchain_numeric_symbol" => onchain.enabled && onchain.reject_numeric_symbols,
        "onchain_suspicious_symbol" => onchain.enabled && onchain.reject_single_char_symbols,
        "onchain_immutable_with_freeze" => onchain.enabled && onchain.reject_immutable_with_freeze,
        "onchain_high_risk_score" => onchain.enabled && onchain.combined_risk_enabled,
        _ => return None,
    })
}

/// The weighted priced-token reasons `config` can produce; the whole table when it
/// switches every one of them off, since the corpus still has its rejected count.
fn active_rejections(config: &FilteringConfig) -> Vec<(&'static str, usize)> {
    let active: Vec<(&'static str, usize)> = PROMO_REJECTIONS
        .iter()
        .copied()
        .filter(|(code, _)| rule_enabled(code, config) == Some(true))
        .collect();
    if active.is_empty() {
        PROMO_REJECTIONS.to_vec()
    } else {
        active
    }
}

/// Youngest generated token, in minutes: past its first filter check, which runs at
/// most `PROMO_FIRST_CHECK_MAX_SECS` after discovery, so no rejection time lies ahead
/// of the session clock.
const PROMO_MIN_TOKEN_AGE_MINUTES: i64 = 4;

/// Latest first filter check after a token's discovery, in seconds.
const PROMO_FIRST_CHECK_MAX_SECS: usize = 200;

/// How far back a market rejection may date. Market thresholds are re-evaluated on
/// every build, so a token rejected for its activity or depth is rejected "since" a
/// recent build, not since its discovery.
const PROMO_MARKET_REJECTION_WINDOW_MINUTES: usize = 360;

/// Blacklist categories, as the blacklist records them: (category, reason).
const PROMO_BLACKLIST_REASONS: &[(&str, &str)] = &[
    ("security", "Mint authority retained after launch"),
    ("security", "Freeze authority retained"),
    ("liquidity", "Liquidity pulled within the first hour"),
    ("distribution", "Single wallet holds over 50% of supply"),
    (
        "behaviour",
        "Sell transactions revert for non-deployer wallets",
    ),
    ("manual", "Blacklisted by hand after review"),
];

// =============================================================================
// DETERMINISTIC GENERATION
// =============================================================================

/// A tiny linear congruential generator.
///
/// Seeded per token index rather than carried as a stream, so any one token's
/// values depend only on its index — the universe can be built in any order and
/// still come out identical.
struct Lcg(u64);

impl Lcg {
    fn seeded(index: usize) -> Self {
        // Knuth's multiplier over the index, then one step, so adjacent indices do
        // not produce adjacent first values.
        Lcg(6_364_136_223_846_793_005u64
            .wrapping_mul(index as u64 + 1)
            .wrapping_add(1_442_695_040_888_963_407))
    }

    /// Advance, then run the state through splitmix64's finaliser.
    ///
    /// The raw LCG state is NOT usable here. Its consecutive outputs stay
    /// correlated, and this generator draws several fields per token in a row —
    /// so the tokens with the largest liquidity draw also came out with nearly the
    /// same price draw, and the table rendered ten rows reading `0.00055…` against
    /// `$23.9K`. The finaliser decorrelates successive values; the sequence stays
    /// exactly as reproducible.
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        let mut mixed = self.0;
        mixed ^= mixed >> 30;
        mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed ^= mixed >> 27;
        mixed = mixed.wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// A value in `[low, high)`.
    fn range(&mut self, low: f64, high: f64) -> f64 {
        let unit = (self.next_u64() % 1_000_000) as f64 / 1_000_000.0;
        low + unit * (high - low)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Keeps a generated pool address from colliding with the token's own mint.
const POOL_SEED: usize = 0x9E37;

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Syllables for generated symbols. Deliberately nonsense — a generated row must
/// never carry the name of a token that exists, because a reader who looked it up
/// would find real numbers behind fabricated ones.
const HEAD: &[&str] = &[
    "ZOR", "MUN", "KEP", "VAL", "TRI", "GNO", "PYX", "DRA", "SOL", "LUX", "NEB", "ORB", "QUA",
    "RIV", "TAL", "VEX", "WYN", "XAN", "YRA", "ZEP",
];
const TAIL: &[&str] = &["BI", "KA", "DO", "MI", "TA", "NU", "RO", "SI", "VA", "ZO"];

fn generated_mint(index: usize, pump: bool) -> String {
    let mut rng = Lcg::seeded(index ^ 0x5EED);
    let body: String = (0..if pump { 40 } else { 44 })
        .map(|_| BASE58[rng.below(BASE58.len())] as char)
        .collect();
    if pump {
        format!("{body}pump")
    } else {
        body
    }
}

fn generated_symbol(index: usize) -> String {
    let mut rng = Lcg::seeded(index ^ 0xC0FFEE);
    format!(
        "{}{}",
        HEAD[rng.below(HEAD.len())],
        TAIL[rng.below(TAIL.len())]
    )
}

// =============================================================================
// TOKEN CONSTRUCTION
// =============================================================================

/// Build a token with every field set, so callers only state what makes their token
/// different. There is no `Default` on `Token` and there should not be: a token
/// with no mint is not a thing the domain has.
#[allow(clippy::too_many_arguments)]
fn base_token(mint: &str, symbol: &str, name: &str, discovered_minutes: i64) -> Token {
    let now = Utc::now();
    let discovered = now - Duration::minutes(discovered_minutes);

    Token {
        mint: mint.to_owned(),
        symbol: symbol.to_owned(),
        name: name.to_owned(),
        decimals: Some(6),
        description: None,
        image_url: None,
        header_image_url: None,
        supply: Some("1000000000".to_owned()),
        data_source: DataSource::DexScreener,
        first_discovered_at: discovered,
        blockchain_created_at: Some(discovered - Duration::minutes(4)),
        pool_created_at: None,
        metadata_last_fetched_at: discovered,
        decimals_last_fetched_at: discovered,
        market_data_last_fetched_at: now - Duration::seconds(28),
        security_data_last_fetched_at: Some(now - Duration::minutes(6)),
        pool_price_last_calculated_at: now - Duration::milliseconds(500),
        pool_price_last_used_pool: None,
        price_usd: 0.0,
        price_sol: 0.0,
        price_native: "0".to_owned(),
        price_change_m5: None,
        price_change_h1: None,
        price_change_h6: None,
        price_change_h24: None,
        market_cap: None,
        fdv: None,
        liquidity_usd: None,
        volume_m5: None,
        volume_h1: None,
        volume_h6: None,
        volume_h24: None,
        pool_count: None,
        reserve_in_usd: None,
        txns_m5_buys: None,
        txns_m5_sells: None,
        txns_h1_buys: None,
        txns_h1_sells: None,
        txns_h6_buys: None,
        txns_h6_sells: None,
        txns_h24_buys: None,
        txns_h24_sells: None,
        websites: Vec::new(),
        socials: Vec::new(),
        mint_authority: None,
        freeze_authority: None,
        update_authority: None,
        creator: None,
        is_mutable: Some(false),
        security_score: None,
        security_score_normalised: None,
        is_rugged: false,
        token_type: Some("spl".to_owned()),
        graph_insiders_detected: Some(0),
        lp_provider_count: Some(3),
        security_risks: Vec::new(),
        total_holders: None,
        top_holders: Vec::new(),
        creator_balance_pct: None,
        top_10_holders_pct: None,
        transfer_fee_pct: None,
        transfer_fee_max_amount: None,
        transfer_fee_authority: None,
        is_blacklisted: false,
        priority: Priority::Standard,
        last_rejection_reason: None,
        last_rejection_source: None,
        last_rejection_at: None,
    }
}

/// Fill in the market columns from a price and a liquidity depth.
///
/// Volume, transaction counts and valuation are all derived from those two, because
/// on a real token they are not independent: a pool with 1.2M of liquidity does not
/// trade eight times in a day, and a table where they disagree reads as fake to
/// anyone who trades.
fn with_market(token: &mut Token, seed: usize, price_sol: f64, liquidity_usd: f64) {
    let mut rng = Lcg::seeded(seed);

    let price_usd = price_sol * PROMO_SOL_USD;
    // Turnover: thin pools churn their depth several times a day, deep pools do not.
    let turnover = rng.range(0.6, 4.5) * (1_000_000.0 / (liquidity_usd + 50_000.0)).clamp(0.4, 6.0);
    let volume_h24 = liquidity_usd * turnover;

    // Valuation is anchored to pool depth, and SUPPLY is what gives way. Assuming a
    // flat billion-token supply instead made FDV a pure function of price, which
    // put a $97M valuation on a token with a $24K pool — a ratio no real pair has,
    // and the first thing a trader reading the table would disbelieve.
    let fdv = liquidity_usd * rng.range(6.0, 45.0);
    let supply = fdv / price_usd;

    token.price_sol = price_sol;
    token.price_usd = price_usd;
    token.price_native = format!("{price_sol:.12}");
    token.supply = Some(format!("{:.0}", supply));
    token.liquidity_usd = Some(liquidity_usd);
    token.reserve_in_usd = Some(liquidity_usd);
    token.fdv = Some(fdv);
    token.market_cap = Some(fdv * rng.range(0.55, 0.95));
    token.volume_h24 = Some(volume_h24);
    token.volume_h6 = Some(volume_h24 * rng.range(0.18, 0.34));
    token.volume_h1 = Some(volume_h24 * rng.range(0.03, 0.07));
    token.volume_m5 = Some(volume_h24 * rng.range(0.002, 0.006));
    token.price_change_h24 = Some(rng.range(-38.0, 62.0));
    token.price_change_h6 = Some(rng.range(-22.0, 31.0));
    token.price_change_h1 = Some(rng.range(-11.0, 14.0));
    token.price_change_m5 = Some(rng.range(-3.5, 4.0));
    token.pool_count = Some(rng.below(4) as u32 + 1);

    // One average trade size sets every window's count, so the four windows stay in
    // proportion to each other and to volume.
    let avg_trade_usd = rng.range(90.0, 640.0);
    let daily_trades = (volume_h24 / avg_trade_usd).round().max(4.0);
    let buy_share = rng.range(0.42, 0.58);
    let split = |total: f64, share: f64| {
        let buys = (total * share).round() as i64;
        (buys, total as i64 - buys)
    };
    let (b24, s24) = split(daily_trades, buy_share);
    let (b6, s6) = split(daily_trades * 0.26, buy_share);
    let (b1, s1) = split(daily_trades * 0.05, buy_share);
    let (b5, s5) = split((daily_trades * 0.004).max(1.0), buy_share);
    token.txns_h24_buys = Some(b24);
    token.txns_h24_sells = Some(s24);
    token.txns_h6_buys = Some(b6);
    token.txns_h6_sells = Some(s6);
    token.txns_h1_buys = Some(b1);
    token.txns_h1_sells = Some(s1);
    token.txns_m5_buys = Some(b5);
    token.txns_m5_sells = Some(s5);

    token.total_holders = Some((liquidity_usd / rng.range(140.0, 900.0)).round() as i64 + 60);
    token.top_10_holders_pct = Some(rng.range(9.0, 31.0));
    token.creator_balance_pct = Some(rng.range(0.0, 4.5));
}

/// Apply a security posture consistent with whether the token passed.
fn with_security(token: &mut Token, seed: usize, passed: bool) {
    let mut rng = Lcg::seeded(seed ^ 0xBEEF);

    if passed {
        let normalised = rng.below(28) as i32 + 4;
        token.security_score = Some(normalised * 1_400);
        token.security_score_normalised = Some(normalised);
        return;
    }

    let normalised = rng.below(38) as i32 + 58;
    token.security_score = Some(normalised * 1_400);
    token.security_score_normalised = Some(normalised);
    token.mint_authority = Some(generated_mint(seed ^ 0xA11CE, false));
    token.security_risks = vec![SecurityRisk {
        name: "Mint Authority".to_owned(),
        value: "enabled".to_owned(),
        description: "The deployer can still mint new supply.".to_owned(),
        score: normalised * 1_400,
        level: "danger".to_owned(),
    }];
}

/// The reason a priced token was rejected, drawn by the weights of the reasons
/// `config` can produce.
fn rejection_code(seed: usize, config: &FilteringConfig) -> &'static str {
    let active = active_rejections(config);
    let total: usize = active.iter().map(|(_, weight)| weight).sum();
    let mut pick = Lcg::seeded(seed ^ 0xDEAD).below(total);
    for (code, weight) in &active {
        if pick < *weight {
            return code;
        }
        pick -= weight;
    }
    active[0].0
}

/// The reason an unpriced token was rejected: the missing-data reason the pipeline
/// attributes under `config`, or a drawn reason when neither market source is on.
fn no_market_code(seed: usize, config: &FilteringConfig) -> &'static str {
    PROMO_NO_MARKET_REJECTIONS
        .into_iter()
        .find(|code| rule_enabled(code, config) == Some(true))
        .unwrap_or_else(|| rejection_code(seed, config))
}

/// Record why a token was rejected, by which source, and since when: a market
/// threshold since a recent build, anything else since the check that followed the
/// token's discovery.
fn with_rejection(token: &mut Token, seed: usize, code: &str) {
    let mut rng = Lcg::seeded(seed ^ 0xD1CE);
    let source = rejection_source(code);
    let market = matches!(
        source,
        Some(FilterSource::DexScreener | FilterSource::GeckoTerminal)
    );
    let discovered = token.first_discovered_at;
    let age_minutes = (Utc::now() - discovered).num_minutes().max(1) as usize;
    let since = if market {
        Utc::now()
            - Duration::minutes(
                rng.below(age_minutes.min(PROMO_MARKET_REJECTION_WINDOW_MINUTES)) as i64,
            )
            - Duration::seconds(rng.below(60) as i64)
    } else {
        discovered + Duration::seconds(rng.below(PROMO_FIRST_CHECK_MAX_SECS - 20) as i64 + 20)
    };
    token.last_rejection_reason = Some(code.to_owned());
    token.last_rejection_source = source.map(|source| source.as_str().to_owned());
    token.last_rejection_at = Some(since);
}

// =============================================================================
// THE UNIVERSE
// =============================================================================

/// What each token is, for view selection. Derived once with the token rather than
/// re-inferred per request, so two views can never disagree about the same row.
struct PromoToken {
    token: Token,
    passed: bool,
    has_position: bool,
    has_pool_price: bool,
    has_market_data: bool,
}

static UNIVERSE: LazyLock<Vec<PromoToken>> = LazyLock::new(build_universe);

fn build_universe() -> Vec<PromoToken> {
    let mut universe: Vec<PromoToken> = Vec::with_capacity(PROMO_TOKENS_TRACKED);

    // --- The head: every token the promo session actually trades. ---------------
    for (index, (symbol, name, mint, logo, _entry, current, _size, hold_minutes)) in
        PROMO_OPEN_TOKENS.iter().enumerate()
    {
        let mut token = base_token(mint, symbol, name, hold_minutes + 30);
        token.image_url = Some((*logo).to_owned());
        // Depth descends with the array's own order, which runs from the largest
        // name to the smallest. The default sort is liquidity, so this decides the
        // order every token view shows them in — and the biggest token has to be
        // the one carrying the deepest pool.
        with_market(
            &mut token,
            index,
            *current,
            2_400_000.0 - index as f64 * 210_000.0,
        );
        with_security(&mut token, index, true);
        token.priority = Priority::OpenPosition;
        token.pool_price_last_used_pool = Some(generated_mint(index ^ POOL_SEED, false));
        universe.push(PromoToken {
            token,
            passed: true,
            has_position: true,
            has_pool_price: true,
            has_market_data: true,
        });
    }

    // Closed trades are still monitored tokens — the bot does not forget a token
    // because it sold it, and the Passed view is where they belong.
    for (offset, (symbol, name, mint, logo, _entry, exit, _size, _reason)) in
        PROMO_CLOSED_TOKENS.iter().enumerate()
    {
        if universe.iter().any(|entry| entry.token.mint == *mint) {
            continue;
        }
        let index = 100 + offset;
        let mut token = base_token(mint, symbol, name, 240 + offset as i64 * 37);
        token.image_url = Some((*logo).to_owned());
        with_market(&mut token, index, *exit, 38_000.0 + offset as f64 * 9_500.0);
        with_security(&mut token, index, true);
        token.priority = Priority::FilterPassed;
        universe.push(PromoToken {
            token,
            passed: true,
            has_position: false,
            has_pool_price: true,
            has_market_data: true,
        });
    }

    // --- The tail: the rest of the monitored database. --------------------------
    let curated = universe.len();
    let passed_tail = PROMO_PASSED.saturating_sub(curated);
    let rejected_tail = PROMO_TOKENS_TRACKED - PROMO_PASSED;

    for offset in 0..passed_tail {
        let index = 1_000 + offset;
        let symbol = generated_symbol(index);
        let mut rng = Lcg::seeded(index);
        let mut token = base_token(
            &generated_mint(index, true),
            &symbol,
            &symbol,
            rng.below(60 * 72) as i64 + 45,
        );
        with_market(
            &mut token,
            index,
            rng.range(0.000_002, 0.004),
            rng.range(12_000.0, 90_000.0),
        );
        with_security(&mut token, index, true);
        token.priority = Priority::FilterPassed;
        universe.push(PromoToken {
            token,
            passed: true,
            has_position: false,
            has_pool_price: true,
            has_market_data: true,
        });
    }

    for offset in 0..rejected_tail {
        let index = 100_000 + offset;
        let symbol = generated_symbol(index);
        let mut rng = Lcg::seeded(index);
        // The unpriced ones are the newest: a token with no market data is usually
        // one no venue has listed yet, not an old one that lost its listing.
        let unpriced = offset < PROMO_NO_MARKET;
        let config = &*FILTERING_CONFIG;
        let code = if unpriced {
            no_market_code(index, config)
        } else {
            rejection_code(index, config)
        };
        // A token rejected as too new is younger than the configured minimum age.
        let too_new_span = (config.min_token_age_minutes - PROMO_MIN_TOKEN_AGE_MINUTES).max(1);
        let discovered_minutes = if unpriced {
            rng.below(180) as i64 + PROMO_MIN_TOKEN_AGE_MINUTES
        } else if code == "token_too_new" {
            rng.below(too_new_span as usize) as i64 + PROMO_MIN_TOKEN_AGE_MINUTES
        } else {
            rng.below(60 * 24 * 30) as i64 + 60
        };

        let mut token = base_token(
            &generated_mint(index, true),
            &symbol,
            &symbol,
            discovered_minutes,
        );
        if !unpriced {
            with_market(
                &mut token,
                index,
                rng.range(0.000_000_4, 0.000_8),
                rng.range(600.0, 24_000.0),
            );
        }
        with_security(&mut token, index, false);
        with_rejection(&mut token, index, code);
        // A token with no market data is still being seeded, not parked in the
        // background refresh queue — the priority has to say which of the two.
        token.priority = if unpriced {
            Priority::Uninitialized
        } else {
            Priority::Background
        };
        // Blacklisting starts past the unpriced block. A token nobody has priced yet
        // has shown no behaviour to blacklist it for, and marking the whole No
        // Market Data view blacklisted says the bot condemns tokens for being new.
        let token_is_blacklisted =
            !unpriced && offset < PROMO_NO_MARKET.saturating_add(PROMO_BLACKLISTED);
        token.is_blacklisted = token_is_blacklisted;

        universe.push(PromoToken {
            token,
            passed: false,
            has_position: false,
            // Pool Service does not price the whole database: it tracks what the bot
            // has a reason to hold a live on-chain price for. Everything that passed
            // qualifies, plus the rejected-but-not-blacklisted tokens still inside
            // the pool service's working set — failing a filter is not a reason to
            // stop pricing a token that is still trading, but being blacklisted is.
            has_pool_price: !unpriced
                && !token_is_blacklisted
                && offset < PROMO_NO_MARKET + PROMO_BLACKLISTED + PROMO_POOL_TRACKED_REJECTED,
            has_market_data: !unpriced,
        });
    }

    universe
}

// =============================================================================
// QUERYING
// =============================================================================

/// Every code a promo token can be rejected for under the session's filtering config.
fn promo_rejection_codes() -> impl Iterator<Item = &'static str> {
    PROMO_REJECTIONS
        .iter()
        .map(|(code, _)| *code)
        .chain(PROMO_NO_MARKET_REJECTIONS)
        .filter(|code| rule_enabled(code, &FILTERING_CONFIG) == Some(true))
}

/// Every token of the universe, and whether it passed filtering.
pub(super) fn promo_universe() -> impl Iterator<Item = (&'static Token, bool)> {
    UNIVERSE.iter().map(|entry| (&entry.token, entry.passed))
}

fn matches_view(entry: &PromoToken, view: FilteringView) -> bool {
    match view {
        FilteringView::All => true,
        FilteringView::Pool => entry.has_pool_price,
        FilteringView::Passed => entry.passed,
        FilteringView::Rejected => !entry.passed,
        FilteringView::Blacklisted => entry.token.is_blacklisted,
        FilteringView::Positions => entry.has_position,
        FilteringView::NoMarketData => !entry.has_market_data,
        FilteringView::Recent => false, // handled by the caller: it is a slice, not a predicate
    }
}

fn sort_value(token: &Token, key: TokenSortKey) -> f64 {
    match key {
        TokenSortKey::PriceSol => token.price_sol,
        TokenSortKey::Volume24h => token.volume_h24.unwrap_or(0.0),
        TokenSortKey::Fdv => token.fdv.unwrap_or(0.0),
        TokenSortKey::MarketCap => token.market_cap.unwrap_or(0.0),
        TokenSortKey::PriceChangeH1 => token.price_change_h1.unwrap_or(0.0),
        TokenSortKey::PriceChangeH24 => token.price_change_h24.unwrap_or(0.0),
        TokenSortKey::RiskScore => token.security_score_normalised.unwrap_or(0) as f64,
        TokenSortKey::Txns5m => token.txns_5m_total().unwrap_or(0) as f64,
        TokenSortKey::Txns1h => token.txns_1h_total().unwrap_or(0) as f64,
        TokenSortKey::Txns6h => token.txns_6h_total().unwrap_or(0) as f64,
        TokenSortKey::Txns24h => token.txns_24h_total().unwrap_or(0) as f64,
        TokenSortKey::FirstDiscoveredAt => token.first_discovered_at.timestamp() as f64,
        TokenSortKey::BlockchainCreatedAt => token.created_at().at.timestamp() as f64,
        TokenSortKey::MarketDataLastFetchedAt => {
            token.market_data_last_fetched_at.timestamp() as f64
        }
        TokenSortKey::MetadataLastFetchedAt => token.metadata_last_fetched_at.timestamp() as f64,
        TokenSortKey::PoolPriceLastCalculatedAt => {
            token.pool_price_last_calculated_at.timestamp() as f64
        }
        // Symbol and Mint sort as text; the caller handles them separately.
        TokenSortKey::Symbol | TokenSortKey::Mint => 0.0,
        TokenSortKey::LiquidityUsd => token.liquidity_usd.unwrap_or(0.0),
    }
}

/// Serve one page of the promo token universe.
pub fn get_promo_tokens_list(query: &FilteringQuery) -> TokenListResponse {
    let view = query.view;

    let mut selected: Vec<&PromoToken> = if view == FilteringView::Recent {
        // Recent is the newest slice of everything monitored, ordered by discovery
        // before any user sort is applied — otherwise "recent" would mean whatever
        // the current sort column happens to rank first.
        let mut all: Vec<&PromoToken> = UNIVERSE.iter().collect();
        all.sort_by(|a, b| {
            b.token
                .first_discovered_at
                .cmp(&a.token.first_discovered_at)
        });
        all.into_iter().take(PROMO_RECENT).collect()
    } else {
        UNIVERSE
            .iter()
            .filter(|entry| matches_view(entry, view))
            .collect()
    };

    if let Some(search) = query.search.as_deref() {
        let needle = search.to_lowercase();
        selected.retain(|entry| {
            entry.token.symbol.to_lowercase().contains(&needle)
                || entry.token.name.to_lowercase().contains(&needle)
                || entry.token.mint.to_lowercase().contains(&needle)
        });
    }

    if let Some(reason) = query.rejection_reason.as_deref() {
        selected.retain(|entry| entry.token.last_rejection_reason.as_deref() == Some(reason));
    }

    // Every key sorts descending here and the ascending case is one reversal at the
    // end, so text and numeric columns cannot end up honouring the direction
    // differently — which is exactly what a per-branch comparator got wrong.
    match query.sort_key {
        TokenSortKey::Symbol => selected.sort_by(|a, b| {
            b.token
                .symbol
                .to_lowercase()
                .cmp(&a.token.symbol.to_lowercase())
        }),
        TokenSortKey::Mint => selected.sort_by(|a, b| b.token.mint.cmp(&a.token.mint)),
        key => selected.sort_by(|a, b| {
            sort_value(&b.token, key)
                .partial_cmp(&sort_value(&a.token, key))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
    if query.sort_direction == SortDirection::Asc {
        selected.reverse();
    }

    let total = selected.len();
    let page_size = query.page_size.max(1);
    let page = query.page.max(1);
    let total_pages = total.div_ceil(page_size);
    let start = (page - 1) * page_size;

    let priced_total = selected.iter().filter(|entry| entry.has_pool_price).count();
    let positions_total = selected.iter().filter(|entry| entry.has_position).count();
    let blacklisted_total = selected
        .iter()
        .filter(|entry| entry.token.is_blacklisted)
        .count();

    let items: Vec<Token> = selected
        .iter()
        .skip(start)
        .take(page_size)
        .map(|entry| entry.token.clone())
        .collect();

    let blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = items
        .iter()
        .filter(|token| token.is_blacklisted)
        .map(|token| {
            let mut rng = Lcg::seeded(token.mint.len() ^ token.symbol.len() ^ 0x81AC);
            let (category, reason) =
                PROMO_BLACKLIST_REASONS[rng.below(PROMO_BLACKLIST_REASONS.len())];
            (
                token.mint.clone(),
                vec![BlacklistReasonInfo {
                    category: category.to_owned(),
                    reason: reason.to_owned(),
                    detail: None,
                }],
            )
        })
        .collect();

    // Same shape as the production list: mint -> code for the rejected view only.
    let (rejection_reasons, available_rejection_reasons) = if view == FilteringView::Rejected {
        (
            items
                .iter()
                .filter_map(|token| {
                    token
                        .last_rejection_reason
                        .as_ref()
                        .map(|code| (token.mint.clone(), code.clone()))
                })
                .collect::<HashMap<String, String>>(),
            promo_rejection_codes().map(str::to_owned).collect(),
        )
    } else {
        (HashMap::new(), Vec::new())
    };

    let next_cursor = if start + items.len() < total {
        Some(start + items.len())
    } else {
        None
    };

    TokenListResponse {
        items: items.into_iter().map(TokenListItem::from).collect(),
        page,
        page_size,
        total,
        total_pages,
        timestamp: Utc::now().to_rfc3339(),
        cursor: Some(start),
        next_cursor,
        prev_cursor: if start == 0 {
            None
        } else {
            Some(start.saturating_sub(page_size))
        },
        priced_total,
        positions_total,
        blacklisted_total,
        rejection_texts: crate::webserver::routes::tokens::types::rejection_texts(
            rejection_reasons
                .values()
                .chain(&available_rejection_reasons),
        ),
        rejection_reasons,
        available_rejection_reasons,
        blacklist_reasons,
    }
}

/// The counters above the Tokens table.
///
/// Counted from the universe rather than restated, so the chips and the table they
/// sit over cannot disagree.
pub fn get_promo_tokens_stats() -> (usize, usize, usize, usize, usize) {
    let total = UNIVERSE.len();
    let priced = UNIVERSE.iter().filter(|entry| entry.has_pool_price).count();
    let positions = UNIVERSE.iter().filter(|entry| entry.has_position).count();
    let blacklisted = UNIVERSE
        .iter()
        .filter(|entry| entry.token.is_blacklisted)
        .count();
    // Candles are kept for what the bot actually charts: the tokens that passed.
    let with_ohlcv = UNIVERSE.iter().filter(|entry| entry.passed).count();
    (total, priced, positions, blacklisted, with_ohlcv)
}

/// The Favorites subtab.
///
/// Favorites are rendered from the same row shape as every other token view, with
/// the favorite extras merged in — the subtab is the token table with a different
/// selection, not a different table.
pub fn get_promo_favorites() -> Vec<serde_json::Value> {
    UNIVERSE
        .iter()
        .filter(|entry| PROMO_FAVORITE_SYMBOLS.contains(&entry.token.symbol.as_str()))
        .enumerate()
        .map(|(index, entry)| {
            let mut row =
                serde_json::to_value(&entry.token).unwrap_or_else(|_| serde_json::json!({}));
            if let serde_json::Value::Object(map) = &mut row {
                map.insert("is_favorite".to_owned(), serde_json::json!(true));
                map.insert("notes".to_owned(), serde_json::json!(null));
                map.insert(
                    "favorite_created_at".to_owned(),
                    serde_json::json!((Utc::now() - Duration::days(index as i64 + 2)).to_rfc3339()),
                );
                map.insert(
                    "has_open_position".to_owned(),
                    serde_json::json!(entry.has_position),
                );
                map.insert(
                    "blacklisted".to_owned(),
                    serde_json::json!(entry.token.is_blacklisted),
                );
            }
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filtering::sources::rejection_text;
    use crate::i18n::ids;
    use crate::webserver::promo::data::promo_curated_tokens;

    #[test]
    fn promo_rejection_codes_are_production_codes() {
        let table = PROMO_REJECTIONS.iter().map(|(code, _)| *code);
        for code in table.chain(PROMO_NO_MARKET_REJECTIONS) {
            let text = rejection_text(code);
            assert_ne!(text.id, ids::FILTERING_REJECT_UNKNOWN.as_str(), "{code}");
            assert!(rejection_source(code).is_some(), "{code}");
            assert!(
                rule_enabled(code, &FilteringConfig::default()).is_some(),
                "{code}"
            );
        }
    }

    /// A rule switched off draws no rejections, its weight goes to the rules still on,
    /// and an unpriced token is attributed like the pipeline does: to DexScreener while
    /// it is on, to GeckoTerminal when only that source is.
    #[test]
    fn sample_reasons_follow_the_rules_the_config_enables() {
        let defaults = FilteringConfig::default();
        let sources: std::collections::HashSet<FilterSource> = active_rejections(&defaults)
            .iter()
            .filter_map(|(code, _)| rejection_source(code))
            .collect();
        assert_eq!(
            sources.len(),
            5,
            "the shipped rules leave a source silent: {sources:?}"
        );

        let mut config = FilteringConfig::default();
        config.geckoterminal.enabled = false;
        config.dexscreener.market_cap_enabled = false;
        let drawn: std::collections::HashSet<&str> = (0..5_000)
            .map(|seed| rejection_code(seed, &config))
            .collect();
        assert!(drawn
            .iter()
            .all(|code| rule_enabled(code, &config) == Some(true)));
        assert!(!drawn.contains("dex_mcap_low"));
        assert!(drawn.iter().all(|code| !code.starts_with("gecko_")));
        assert_eq!(no_market_code(0, &config), "dex_data_missing");

        config.geckoterminal.enabled = true;
        config.dexscreener.enabled = false;
        assert_eq!(no_market_code(0, &config), "gecko_data_missing");
    }

    /// A rejected token carries the source the pipeline assigns its code, and a
    /// token rejected as too new is younger than the shipped minimum age.
    #[test]
    fn rejected_tokens_carry_their_code_source_and_a_consistent_age() {
        for (token, passed) in promo_universe() {
            let Some(code) = token.last_rejection_reason.as_deref() else {
                assert!(passed, "{} is rejected without a reason", token.mint);
                continue;
            };
            assert!(!passed, "{} passed with a rejection", token.mint);
            assert_eq!(
                token.last_rejection_source.as_deref(),
                rejection_source(code).map(|source| source.as_str()),
                "{code}"
            );
            let since = token.last_rejection_at.expect("a rejection has a time");
            assert!(
                since >= token.first_discovered_at,
                "{code} predates discovery"
            );
            assert!(since <= Utc::now(), "{code} rejected in the future");
            assert_eq!(
                rule_enabled(code, &FILTERING_CONFIG),
                Some(true),
                "{code} rejects under a rule the config switches off"
            );
            if code == "token_too_new" {
                assert!(
                    (Utc::now() - token.first_discovered_at).num_minutes()
                        < FILTERING_CONFIG.min_token_age_minutes
                );
            }
        }
    }

    #[test]
    fn favorite_symbols_name_curated_tokens() {
        let curated = promo_curated_tokens();
        for favorite in PROMO_FAVORITE_SYMBOLS {
            assert!(
                curated.iter().any(|(symbol, _)| symbol == favorite),
                "{favorite}"
            );
        }
    }

    #[test]
    fn rejected_list_maps_promo_mints_to_promo_codes() {
        let query = FilteringQuery {
            view: FilteringView::Rejected,
            page_size: 100,
            ..FilteringQuery::default()
        };
        let response = get_promo_tokens_list(&query);
        assert!(!response.rejection_reasons.is_empty());
        for (mint, code) in &response.rejection_reasons {
            assert!(
                UNIVERSE.iter().any(|entry| &entry.token.mint == mint),
                "{mint}"
            );
            assert!(promo_rejection_codes().any(|known| known == code), "{code}");
            assert!(response.rejection_texts.contains_key(code), "{code}");
        }
    }
}
