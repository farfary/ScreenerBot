//! Unit tests for the filtering store helpers — view membership, query filtering,
//! sorting and staleness.
//!
//! These live inside the crate because `collect_entries` / `apply_filters` /
//! `sort_tokens` are `pub(super)`: they are what the tokens dashboard actually asks for
//! on every poll, but no integration test can reach them.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};

use super::store_helpers::{apply_filters, collect_entries, is_snapshot_stale, sort_tokens};
use super::types::{
    FilteringQuery, FilteringSnapshot, FilteringView, SortDirection, TokenEntry, TokenSortKey,
};
use crate::tokens::types::{DataSource, Token};
use crate::tokens::Priority;

// ============================================================================
// FIXTURES
// ============================================================================

fn token(mint: &str) -> Token {
    let now = Utc::now();
    Token {
        mint: mint.to_owned(),
        symbol: "TEST".to_owned(),
        name: "Test Token".to_owned(),
        decimals: 9,
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,
        data_source: DataSource::DexScreener,
        first_discovered_at: now,
        blockchain_created_at: None,
        metadata_last_fetched_at: now,
        decimals_last_fetched_at: now,
        market_data_last_fetched_at: now,
        security_data_last_fetched_at: None,
        pool_price_last_calculated_at: now,
        pool_price_last_used_pool: None,
        price_usd: 1.0,
        price_sol: 0.005,
        price_native: "0.005".to_owned(),
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
        is_mutable: Some(true),
        security_score: None,
        security_score_normalised: None,
        is_rugged: false,
        token_type: None,
        graph_insiders_detected: None,
        lp_provider_count: None,
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

/// Builder for the snapshot entries the views select over.
struct EntryBuilder {
    token: Token,
    has_pool_price: bool,
    has_open_position: bool,
    has_ohlcv: bool,
    pair_created_at: Option<i64>,
}

impl EntryBuilder {
    fn new(mint: &str) -> Self {
        Self {
            token: token(mint),
            has_pool_price: false,
            has_open_position: false,
            has_ohlcv: false,
            pair_created_at: Some(Utc::now().timestamp()),
        }
    }

    fn priced(mut self) -> Self {
        self.has_pool_price = true;
        self
    }

    fn in_position(mut self) -> Self {
        self.has_open_position = true;
        self
    }

    fn with_ohlcv(mut self) -> Self {
        self.has_ohlcv = true;
        self
    }

    fn blacklisted(mut self) -> Self {
        self.token.is_blacklisted = true;
        self
    }

    fn created_at(mut self, timestamp: Option<i64>) -> Self {
        self.pair_created_at = timestamp;
        self
    }

    fn edit<F: FnOnce(&mut Token)>(mut self, f: F) -> Self {
        f(&mut self.token);
        self
    }

    fn build(self) -> (String, TokenEntry) {
        let mint = self.token.mint.clone();
        (
            mint,
            TokenEntry {
                token: Arc::new(self.token),
                has_pool_price: self.has_pool_price,
                has_open_position: self.has_open_position,
                has_ohlcv: self.has_ohlcv,
                pair_created_at: self.pair_created_at,
                last_updated: Utc::now(),
            },
        )
    }
}

fn snapshot_of(entries: Vec<(String, TokenEntry)>) -> FilteringSnapshot {
    let tokens: HashMap<String, TokenEntry> = entries.into_iter().collect();
    FilteringSnapshot {
        updated_at: Utc::now(),
        filtered_mints: Vec::new(),
        passed_tokens: Vec::new(),
        rejected_mints: Vec::new(),
        rejected_tokens: Vec::new(),
        tokens,
        blacklist_reasons: HashMap::new(),
    }
}

/// Sorted mints of a collected view, for order-independent membership assertions.
fn mints(entries: &[&TokenEntry]) -> Vec<String> {
    let mut result: Vec<String> = entries.iter().map(|e| e.token.mint.clone()).collect();
    result.sort();
    result
}

/// Sorted mints of a filtered token list.
fn mints_of(items: &[&Token]) -> Vec<String> {
    let mut result: Vec<String> = items.iter().map(|t| t.mint.clone()).collect();
    result.sort();
    result
}

fn token_refs(snapshot: &FilteringSnapshot, view: FilteringView) -> Vec<&Token> {
    collect_entries(snapshot, view, None)
        .into_iter()
        .map(|entry| entry.token.as_ref())
        .collect()
}

fn query() -> FilteringQuery {
    FilteringQuery::default()
}

// ============================================================================
// VIEW MEMBERSHIP
// ============================================================================

#[test]
fn pool_and_no_market_views_are_exact_complements() {
    // The "No Market Data" view is defined as `!has_pool_price`, i.e. the exact inverse
    // of the Pool view — it says nothing about whether market data exists. Every token is
    // in one or the other, never both, never neither.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A").priced().build(),
        EntryBuilder::new("B").priced().build(),
        EntryBuilder::new("C").build(),
    ]);

    let pool = collect_entries(&snapshot, FilteringView::Pool, None);
    let no_market = collect_entries(&snapshot, FilteringView::NoMarketData, None);

    assert_eq!(mints(&pool), vec!["A", "B"]);
    assert_eq!(mints(&no_market), vec!["C"]);
    assert_eq!(pool.len() + no_market.len(), snapshot.tokens.len());
}

#[test]
fn all_view_returns_every_entry() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A").priced().build(),
        EntryBuilder::new("B").blacklisted().build(),
        EntryBuilder::new("C").in_position().build(),
    ]);

    assert_eq!(
        collect_entries(&snapshot, FilteringView::All, None).len(),
        3
    );
}

#[test]
fn passed_view_follows_the_filtered_list_and_deduplicates_it() {
    let mut snapshot = snapshot_of(vec![
        EntryBuilder::new("A").build(),
        EntryBuilder::new("B").build(),
    ]);
    // A duplicated mint (two pools resolving to the same token, a double push) must not
    // show the same row twice.
    snapshot.filtered_mints = vec!["A".to_owned(), "B".to_owned(), "A".to_owned()];

    let passed = collect_entries(&snapshot, FilteringView::Passed, None);
    assert_eq!(mints(&passed), vec!["A", "B"]);
}

#[test]
fn passed_view_skips_mints_with_no_snapshot_entry() {
    let mut snapshot = snapshot_of(vec![EntryBuilder::new("A").build()]);
    snapshot.filtered_mints = vec!["A".to_owned(), "GONE".to_owned()];

    assert_eq!(
        collect_entries(&snapshot, FilteringView::Passed, None).len(),
        1
    );
}

#[test]
fn rejected_view_excludes_blacklisted_tokens_and_does_not_deduplicate() {
    // Blacklisted tokens get their own view, so they are removed here. Note the
    // asymmetry with Passed: this list is NOT deduplicated.
    let mut snapshot = snapshot_of(vec![
        EntryBuilder::new("A").build(),
        EntryBuilder::new("B").blacklisted().build(),
    ]);
    snapshot.rejected_mints = vec!["A".to_owned(), "B".to_owned(), "A".to_owned()];

    let rejected = collect_entries(&snapshot, FilteringView::Rejected, None);
    assert_eq!(
        mints(&rejected),
        vec!["A", "A"],
        "blacklisted rows are excluded; duplicates in rejected_mints are not"
    );
}

#[test]
fn blacklisted_and_positions_views_select_on_their_flag() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A").blacklisted().build(),
        EntryBuilder::new("B").in_position().build(),
        EntryBuilder::new("C").build(),
    ]);

    assert_eq!(
        mints(&collect_entries(
            &snapshot,
            FilteringView::Blacklisted,
            None
        )),
        vec!["A"]
    );
    assert_eq!(
        mints(&collect_entries(&snapshot, FilteringView::Positions, None)),
        vec!["B"]
    );
}

#[test]
fn recent_view_is_strictly_newer_than_the_cutoff() {
    let now = Utc::now();
    let cutoff = now - Duration::hours(24);
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("NEW")
            .created_at(Some((now - Duration::hours(1)).timestamp()))
            .build(),
        EntryBuilder::new("OLD")
            .created_at(Some((now - Duration::hours(48)).timestamp()))
            .build(),
        EntryBuilder::new("EXACT")
            .created_at(Some(cutoff.timestamp()))
            .build(),
        EntryBuilder::new("UNKNOWN").created_at(None).build(),
    ]);

    let recent = collect_entries(&snapshot, FilteringView::Recent, Some(cutoff));
    assert_eq!(
        mints(&recent),
        vec!["NEW"],
        "the comparison is `>`, so a token created exactly at the cutoff is not recent, \
         and one with no creation timestamp is never recent"
    );
}

#[test]
fn recent_view_drops_entries_with_an_unrepresentable_timestamp() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("BAD").created_at(Some(i64::MAX)).build(),
        EntryBuilder::new("ZERO").created_at(Some(0)).build(),
    ]);

    // i64::MAX is not a valid UTC second, so it must be discarded rather than treated as
    // the newest token in existence.
    let recent = collect_entries(&snapshot, FilteringView::Recent, Some(Utc::now()));
    assert!(recent.is_empty());
}

// ============================================================================
// QUERY FILTERS
// ============================================================================

#[test]
fn search_matches_symbol_name_and_mint_case_insensitively() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("AAA")
            .edit(|t| {
                t.symbol = "BONK".to_owned();
                t.name = "Bonk Inu".to_owned();
            })
            .build(),
        EntryBuilder::new("BBB")
            .edit(|t| {
                t.symbol = "WIF".to_owned();
                t.name = "dogwifhat".to_owned();
            })
            .build(),
    ]);

    for term in ["bonk", "BONK", " Bonk ", "onk inu", "aaa"] {
        let mut items = token_refs(&snapshot, FilteringView::All);
        let mut q = query();
        q.search = Some(term.to_owned());
        apply_filters(&mut items, &q, &snapshot);
        assert_eq!(items.len(), 1, "search term {term:?} must match one token");
        assert_eq!(items[0].mint, "AAA");
    }

    // A blank search is not a filter.
    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.search = Some("   ".to_owned());
    apply_filters(&mut items, &q, &snapshot);
    assert_eq!(items.len(), 2);
}

#[test]
fn numeric_range_filters_treat_missing_values_asymmetrically() {
    // A token with no liquidity reading is treated as 0 by the minimum filter and as
    // f64::MAX by the maximum filter, so it is excluded by BOTH — it can never appear in
    // a bounded range even though the value is simply unknown.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("HAS")
            .edit(|t| t.liquidity_usd = Some(5_000.0))
            .build(),
        EntryBuilder::new("NONE").build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.min_liquidity = Some(1_000.0);
    apply_filters(&mut items, &q, &snapshot);
    assert_eq!(mints_of(&items), vec!["HAS"]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.max_liquidity = Some(10_000.0);
    apply_filters(&mut items, &q, &snapshot);
    assert_eq!(mints_of(&items), vec!["HAS"]);

    // Only an unbounded query shows it at all.
    let mut items = token_refs(&snapshot, FilteringView::All);
    apply_filters(&mut items, &query(), &snapshot);
    assert_eq!(items.len(), 2);
}

#[test]
fn risk_score_filter_excludes_unscored_tokens() {
    // An unscored token is treated as maximally risky, so "max risk 50" hides every
    // token Rugcheck has not covered yet.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("SAFE")
            .edit(|t| t.security_score = Some(10))
            .build(),
        EntryBuilder::new("RISKY")
            .edit(|t| t.security_score = Some(9_000))
            .build(),
        EntryBuilder::new("UNSCORED").build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.max_risk_score = Some(50);
    apply_filters(&mut items, &q, &snapshot);

    assert_eq!(mints_of(&items), vec!["SAFE"]);
}

#[test]
fn holder_and_volume_filters_use_zero_for_missing_values() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("BIG")
            .edit(|t| {
                t.total_holders = Some(1_000);
                t.volume_h24 = Some(100_000.0);
            })
            .build(),
        EntryBuilder::new("UNKNOWN").build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.min_unique_holders = Some(100);
    q.min_volume_24h = Some(1.0);
    apply_filters(&mut items, &q, &snapshot);

    assert_eq!(mints_of(&items), vec!["BIG"]);
}

#[test]
fn derived_flag_filters_match_both_polarities() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("PRICED").priced().build(),
        EntryBuilder::new("POSITION").in_position().build(),
        EntryBuilder::new("OHLCV").with_ohlcv().build(),
        EntryBuilder::new("PLAIN").build(),
    ]);

    let cases: [(fn(&mut FilteringQuery), Vec<&str>); 6] = [
        (|q| q.has_pool_price = Some(true), vec!["PRICED"]),
        (
            |q| q.has_pool_price = Some(false),
            vec!["OHLCV", "PLAIN", "POSITION"],
        ),
        (|q| q.has_open_position = Some(true), vec!["POSITION"]),
        (
            |q| q.has_open_position = Some(false),
            vec!["OHLCV", "PLAIN", "PRICED"],
        ),
        (|q| q.has_ohlcv = Some(true), vec!["OHLCV"]),
        (
            |q| q.has_ohlcv = Some(false),
            vec!["PLAIN", "POSITION", "PRICED"],
        ),
    ];

    for (mutate, expected) in cases {
        let mut items = token_refs(&snapshot, FilteringView::All);
        let mut q = query();
        mutate(&mut q);
        apply_filters(&mut items, &q, &snapshot);
        assert_eq!(mints_of(&items), expected);
    }
}

#[test]
fn rejection_reason_filter_is_case_insensitive_and_exact() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A")
            .edit(|t| t.last_rejection_reason = Some("dex_liq_low".to_owned()))
            .build(),
        EntryBuilder::new("B")
            .edit(|t| t.last_rejection_reason = Some("rug_score".to_owned()))
            .build(),
        EntryBuilder::new("C").build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.rejection_reason = Some("DEX_LIQ_LOW".to_owned());
    apply_filters(&mut items, &q, &snapshot);
    assert_eq!(mints_of(&items), vec!["A"]);

    // A prefix must not match — the reasons are codes, not free text.
    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.rejection_reason = Some("dex_liq".to_owned());
    apply_filters(&mut items, &q, &snapshot);
    assert!(items.is_empty());
}

#[test]
fn filters_compose_as_a_conjunction() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("MATCH")
            .priced()
            .edit(|t| {
                t.symbol = "GOOD".to_owned();
                t.liquidity_usd = Some(50_000.0);
                t.volume_h24 = Some(100_000.0);
                t.security_score = Some(500);
            })
            .build(),
        EntryBuilder::new("WRONGSYMBOL")
            .priced()
            .edit(|t| {
                t.symbol = "BAD".to_owned();
                t.liquidity_usd = Some(50_000.0);
                t.volume_h24 = Some(100_000.0);
                t.security_score = Some(500);
            })
            .build(),
        EntryBuilder::new("UNPRICED")
            .edit(|t| {
                t.symbol = "GOOD".to_owned();
                t.liquidity_usd = Some(50_000.0);
                t.volume_h24 = Some(100_000.0);
                t.security_score = Some(500);
            })
            .build(),
        EntryBuilder::new("TOOSMALL")
            .priced()
            .edit(|t| {
                t.symbol = "GOOD".to_owned();
                t.liquidity_usd = Some(10.0);
                t.volume_h24 = Some(100_000.0);
                t.security_score = Some(500);
            })
            .build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    let mut q = query();
    q.search = Some("good".to_owned());
    q.min_liquidity = Some(1_000.0);
    q.max_liquidity = Some(1_000_000.0);
    q.min_volume_24h = Some(1_000.0);
    q.max_risk_score = Some(1_000);
    q.has_pool_price = Some(true);
    apply_filters(&mut items, &q, &snapshot);

    assert_eq!(mints_of(&items), vec!["MATCH"]);
}

#[test]
fn an_empty_query_filters_nothing() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A").build(),
        EntryBuilder::new("B").blacklisted().build(),
    ]);

    let mut items = token_refs(&snapshot, FilteringView::All);
    apply_filters(&mut items, &query(), &snapshot);
    assert_eq!(items.len(), 2);
}

// ============================================================================
// SORTING
// ============================================================================

fn sorted_mints(
    snapshot: &FilteringSnapshot,
    key: TokenSortKey,
    dir: SortDirection,
) -> Vec<String> {
    let mut items = token_refs(snapshot, FilteringView::All);
    sort_tokens(&mut items, key, dir);
    items.iter().map(|t| t.mint.clone()).collect()
}

#[test]
fn numeric_sorts_order_both_ways() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("LOW")
            .edit(|t| t.liquidity_usd = Some(10.0))
            .build(),
        EntryBuilder::new("MID")
            .edit(|t| t.liquidity_usd = Some(500.0))
            .build(),
        EntryBuilder::new("HIGH")
            .edit(|t| t.liquidity_usd = Some(9_000.0))
            .build(),
    ]);

    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::LiquidityUsd, SortDirection::Desc),
        vec!["HIGH", "MID", "LOW"]
    );
    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::LiquidityUsd, SortDirection::Asc),
        vec!["LOW", "MID", "HIGH"]
    );
}

#[test]
fn missing_numeric_values_sort_as_zero_not_as_absent() {
    // `cmp_f64` maps None to 0.0, so an unknown liquidity ranks between negative and
    // positive values rather than sinking to the bottom of a descending list.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("KNOWN")
            .edit(|t| t.liquidity_usd = Some(100.0))
            .build(),
        EntryBuilder::new("UNKNOWN").build(),
        EntryBuilder::new("NEGATIVE")
            .edit(|t| t.liquidity_usd = Some(-50.0))
            .build(),
    ]);

    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::LiquidityUsd, SortDirection::Asc),
        vec!["NEGATIVE", "UNKNOWN", "KNOWN"]
    );
}

#[test]
fn nan_values_do_not_corrupt_the_sort_order() {
    // `partial_cmp` returns None for NaN and the comparator falls back to Equal. The sort
    // must still terminate and return every row exactly once.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("A")
            .edit(|t| t.liquidity_usd = Some(f64::NAN))
            .build(),
        EntryBuilder::new("B")
            .edit(|t| t.liquidity_usd = Some(1.0))
            .build(),
        EntryBuilder::new("C")
            .edit(|t| t.liquidity_usd = Some(f64::NAN))
            .build(),
        EntryBuilder::new("D")
            .edit(|t| t.liquidity_usd = Some(2.0))
            .build(),
    ]);

    let mut result = sorted_mints(&snapshot, TokenSortKey::LiquidityUsd, SortDirection::Desc);
    assert_eq!(result.len(), 4);
    result.sort();
    assert_eq!(result, vec!["A", "B", "C", "D"]);
}

#[test]
fn risk_score_sort_sinks_unscored_tokens_in_ascending_order() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("SAFE")
            .edit(|t| t.security_score = Some(1))
            .build(),
        EntryBuilder::new("RISKY")
            .edit(|t| t.security_score = Some(90_000))
            .build(),
        EntryBuilder::new("UNSCORED").build(),
    ]);

    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::RiskScore, SortDirection::Asc),
        vec!["SAFE", "RISKY", "UNSCORED"],
        "unscored sorts as i32::MAX — last when ascending by risk"
    );
}

#[test]
fn transaction_sorts_use_the_buy_plus_sell_total() {
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("QUIET")
            .edit(|t| {
                t.txns_h24_buys = Some(1);
                t.txns_h24_sells = Some(1);
            })
            .build(),
        EntryBuilder::new("BUSY")
            .edit(|t| {
                t.txns_h24_buys = Some(500);
                t.txns_h24_sells = Some(400);
            })
            .build(),
        EntryBuilder::new("NONE").build(),
    ]);

    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::Txns24h, SortDirection::Desc),
        vec!["BUSY", "QUIET", "NONE"],
        "both sides are summed, and a token with no counts at all sorts last"
    );
}

#[test]
fn transaction_sorts_count_a_one_sided_reading_as_the_whole_total() {
    // `Token::txns_24h_total` returns whichever side it has when the other is absent, so
    // a token with 9_999 buys and no recorded sells outranks one with 900 real trades.
    // Half a reading is presented as a complete one — worth knowing when reading the
    // Txns column, and the same helper backs all four windows.
    //
    // Note also that this helper adds with a plain `+` while the DexScreener activity
    // FILTER uses `saturating_add` for the same pair of fields; only one of the two is
    // protected against a corrupted provider value near `i64::MAX`.
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("BUSY")
            .edit(|t| {
                t.txns_h24_buys = Some(500);
                t.txns_h24_sells = Some(400);
            })
            .build(),
        EntryBuilder::new("HALF")
            .edit(|t| t.txns_h24_buys = Some(9_999))
            .build(),
    ]);

    assert_eq!(
        sorted_mints(&snapshot, TokenSortKey::Txns24h, SortDirection::Desc),
        vec!["HALF", "BUSY"]
    );
}

#[test]
fn blockchain_created_sort_falls_back_to_discovery_time() {
    let now = Utc::now();
    let snapshot = snapshot_of(vec![
        EntryBuilder::new("OLDCHAIN")
            .edit(|t| {
                t.blockchain_created_at = Some(now - Duration::days(100));
                t.first_discovered_at = now;
            })
            .build(),
        EntryBuilder::new("NOCHAIN")
            .edit(|t| {
                t.blockchain_created_at = None;
                t.first_discovered_at = now - Duration::days(1);
            })
            .build(),
    ]);

    assert_eq!(
        sorted_mints(
            &snapshot,
            TokenSortKey::BlockchainCreatedAt,
            SortDirection::Asc
        ),
        vec!["OLDCHAIN", "NOCHAIN"]
    );
}

#[test]
fn sorting_is_a_permutation_of_the_input() {
    let entries: Vec<(String, TokenEntry)> = (0..200)
        .map(|i| {
            EntryBuilder::new(&format!("M{i:03}"))
                .edit(|t| {
                    // Deliberate ties every ten rows.
                    t.liquidity_usd = Some((i % 10) as f64);
                    t.symbol = format!("S{}", i % 7);
                })
                .build()
        })
        .collect();
    let snapshot = snapshot_of(entries);

    for key in [
        TokenSortKey::LiquidityUsd,
        TokenSortKey::Symbol,
        TokenSortKey::Mint,
        TokenSortKey::RiskScore,
        TokenSortKey::MarketCap,
    ] {
        for dir in [SortDirection::Asc, SortDirection::Desc] {
            let mut result = sorted_mints(&snapshot, key, dir);
            assert_eq!(result.len(), 200);
            result.sort();
            result.dedup();
            assert_eq!(result.len(), 200, "sorting dropped or duplicated rows");
        }
    }
}

// ============================================================================
// PAGINATION INPUT
// ============================================================================

#[test]
fn page_bounds_reject_zero() {
    let query = FilteringQuery {
        page: 0,
        page_size: 0,
        ..Default::default()
    }
    .with_page_bounds();

    assert_eq!(query.page, 1);
    assert_eq!(query.page_size, 50);
}

#[test]
fn page_size_is_clamped_into_range() {
    let mut query = FilteringQuery {
        page_size: 100_000,
        ..Default::default()
    };
    query.clamp_page_size(200);
    assert_eq!(query.page_size, 200);

    let mut query = FilteringQuery {
        page_size: 0,
        ..Default::default()
    };
    query.clamp_page_size(200);
    assert_eq!(query.page_size, 1, "a zero page size would divide by zero");

    // A nonsensical maximum still yields a usable page size.
    let mut query = FilteringQuery {
        page_size: 50,
        ..Default::default()
    };
    query.clamp_page_size(0);
    assert_eq!(query.page_size, 1);
}

// ============================================================================
// STALENESS
// ============================================================================

#[test]
fn snapshot_staleness_threshold() {
    let mut snapshot = snapshot_of(Vec::new());

    snapshot.updated_at = Utc::now();
    assert!(!is_snapshot_stale(&snapshot));

    snapshot.updated_at = Utc::now() - Duration::seconds(179);
    assert!(!is_snapshot_stale(&snapshot));

    snapshot.updated_at = Utc::now() - Duration::seconds(240);
    assert!(is_snapshot_stale(&snapshot));
}

#[test]
fn a_snapshot_timestamped_in_the_future_is_not_stale() {
    // Clock skew must clamp the age to zero rather than wrap into a huge unsigned value
    // and mark every snapshot stale forever.
    let mut snapshot = snapshot_of(Vec::new());
    snapshot.updated_at = Utc::now() + Duration::hours(1);

    assert!(!is_snapshot_stale(&snapshot));
}

// ============================================================================
// QUERY COST AT SNAPSHOT SCALE
// ============================================================================

#[test]
fn apply_filters_rebuilds_a_flag_map_over_the_whole_snapshot_every_call() {
    // DEFECT PIN: `apply_filters` builds a HashMap over EVERY token in the snapshot
    // before it looks at the query, even when no derived-flag filter is set and the view
    // being paged holds a handful of rows. The dashboard polls this once a second, so the
    // cost of showing 50 rows scales with the size of the whole corpus.
    fn measure(snapshot_size: usize) -> std::time::Duration {
        let entries: Vec<(String, TokenEntry)> = (0..snapshot_size)
            .map(|i| EntryBuilder::new(&format!("M{i:039}")).build())
            .collect();
        let snapshot = snapshot_of(entries);

        // One row selected out of the whole snapshot — the work the query itself needs
        // is constant.
        let mut q = query();
        q.search = Some(format!("M{:039}", 0));

        let started = std::time::Instant::now();
        for _ in 0..20 {
            let mut items = token_refs(&snapshot, FilteringView::All);
            apply_filters(&mut items, &q, &snapshot);
        }
        started.elapsed()
    }

    let small = measure(2_000);
    let large = measure(20_000); // 10x
    let growth = large.as_secs_f64() / small.as_secs_f64().max(f64::EPSILON);
    eprintln!(
        "PERF apply_filters: 2k={small:?} 20k={large:?} growth={growth:.1}x for a 10x \
         larger snapshot returning the same single row"
    );

    assert!(
        growth > 2.0,
        "expected the per-query cost to track snapshot size (it builds a map of every \
         token); if this now holds constant the optimisation landed and this pin can go"
    );
    assert!(
        growth < 40.0,
        "per-query cost is growing faster than the snapshot itself ({growth:.1}x for 10x)"
    );
}

#[test]
fn sorting_a_large_view_stays_within_n_log_n() {
    fn measure(size: usize) -> std::time::Duration {
        let entries: Vec<(String, TokenEntry)> = (0..size)
            .map(|i| {
                EntryBuilder::new(&format!("M{i:039}"))
                    .edit(|t| t.liquidity_usd = Some(((i * 7919) % size) as f64))
                    .build()
            })
            .collect();
        let snapshot = snapshot_of(entries);

        let started = std::time::Instant::now();
        for _ in 0..5 {
            let mut items = token_refs(&snapshot, FilteringView::All);
            sort_tokens(&mut items, TokenSortKey::LiquidityUsd, SortDirection::Desc);
        }
        started.elapsed()
    }

    let small = measure(5_000);
    let large = measure(50_000); // 10x
    let growth = large.as_secs_f64() / small.as_secs_f64().max(f64::EPSILON);
    eprintln!("PERF sort_tokens: 5k={small:?} 50k={large:?} growth={growth:.1}x");

    assert!(
        growth < 40.0,
        "sorting 10x the rows costs {growth:.1}x — worse than the n log n the comparator \
         implies"
    );
}
