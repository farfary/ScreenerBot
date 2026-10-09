// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The wallet's live worth — the single source of truth for "what is my wallet worth".
//!
//! Everything the user sees as a wallet figure (header card, home hero, dashboard
//! overview) reads `get_wallet_worth()`. It is deliberately SYNC and lock-free: it
//! reads the live snapshot published by the balance monitor and prices the held
//! tokens at call time, so a poll costs no database work and no RPC.
//!
//! Two clocks feed it, and both have to be fast or the number lies:
//!
//! - PRICES move continuously. They are resolved per call from the pool price cache
//!   (~500ms), so worth tracks the market between balance refreshes.
//! - BALANCES move on-chain. `request_balance_refresh()` wakes the monitor to take a
//!   fresh snapshot immediately instead of waiting out its interval. It is called
//!   from the wallet's `logsSubscribe` stream (which sees EVERY confirmed transaction
//!   mentioning the wallet — the bot's trades AND anything the owner does from another
//!   app) and from every verified position transition. Bursts are collapsed by a short
//!   debounce, so a multi-swap sequence costs one refresh, not one per signature.
//!
//! Holdings are valued live-pool-price first, falling back to the token database's
//! market price for a token the pool service does not track. A token we cannot price
//! contributes 0 and is counted in `unpriced_token_count` — the worth is never padded
//! with a fabricated value.
//!
//! The market fallback is read from the token database once per snapshot and published
//! WITH the snapshot. It must never come from the token snapshot store: that store
//! holds an entry only for its TTL after a market updater last refreshed it, so a
//! fallback read from it prices a held token for part of each minute and drops it for
//! the rest, and the headline alternates between two values.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use futures::future::join_all;
use tokio::sync::Notify;

use crate::logger::{self, LogTag};

use super::types::{WalletSnapshot, WalletWorth};

/// Token-database market price in SOL per held mint, read when the snapshot was taken.
/// Mints without a positive market price are absent.
pub(super) type MarketPrices = HashMap<String, f64>;

/// Collapse a burst of on-chain activity (a swap emits several notifications, and a
/// DCA/partial sequence emits several swaps) into ONE refresh. Also lets the balance
/// settle: `logsSubscribe` fires at `confirmed`, and reading the account a beat later
/// avoids racing the very transaction that woke us.
const REFRESH_DEBOUNCE: Duration = Duration::from_millis(800);

/// A published snapshot together with the market prices of its holdings, swapped as
/// one value so a reader never pairs new balances with another snapshot's prices.
struct LiveHoldings {
    snapshot: Arc<WalletSnapshot>,
    market_prices: MarketPrices,
}

/// The latest fully-populated snapshot, published by the balance monitor. Read
/// lock-free by every worth query.
static LIVE_SNAPSHOT: ArcSwapOption<LiveHoldings> = ArcSwapOption::const_empty();

static REFRESH_NOTIFY: std::sync::LazyLock<Notify> = std::sync::LazyLock::new(Notify::new);

/// Publish a freshly collected snapshot, with its holdings' market prices, as the live one.
pub(super) fn publish_snapshot(snapshot: Arc<WalletSnapshot>, market_prices: MarketPrices) {
    LIVE_SNAPSHOT.store(Some(Arc::new(LiveHoldings {
        snapshot,
        market_prices,
    })));
}

/// Read the token-database market price of every held mint.
///
/// A mint whose read fails or that has no positive market price is left out, so the
/// worth counts it as unpriced unless the pool service prices it.
pub(super) async fn load_market_prices(snapshot: &WalletSnapshot) -> MarketPrices {
    let chain = crate::chains::active_chain();
    let reads = snapshot.token_balances.iter().map(|balance| async move {
        match crate::tokens::database::get_full_token_async(chain, &balance.mint).await {
            Ok(token) => token
                .map(|token| token.price_sol)
                .filter(|price| price.is_finite() && *price > 0.0)
                .map(|price| (balance.mint.clone(), price)),
            Err(err) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!("Market price read failed for {}: {err}", balance.mint),
                );
                None
            }
        }
    });
    join_all(reads).await.into_iter().flatten().collect()
}

/// The live snapshot, if the monitor has produced (or hydrated) one.
///
/// Public because it is the same source `get_wallet_worth()` reads: an API that
/// serves a balance must serve THIS, not a database row, or the number it returns
/// disagrees with the header and the home hero.
pub fn live_wallet_snapshot() -> Option<Arc<WalletSnapshot>> {
    LIVE_SNAPSHOT
        .load_full()
        .map(|live| Arc::clone(&live.snapshot))
}

/// Ask the balance monitor to take a fresh on-chain snapshot NOW.
///
/// Cheap, sync and idempotent — call it from any hot path that knows the wallet just
/// changed. Requests are coalesced, so hammering it is harmless.
pub fn request_balance_refresh() {
    // `notify_one` STORES a permit when no one is waiting, so a request raised while
    // the monitor is mid-collection is not lost — the next wait returns immediately.
    REFRESH_NOTIFY.notify_one();
}

/// Resolve once someone calls `request_balance_refresh()`.
///
/// Awaited as a `select!` branch, so it must stay cancel-safe: it only waits, and does
/// NOT swallow the notification by doing work (the debounce lives in
/// `settle_refresh_burst`, which runs in the branch BODY where it cannot be cancelled).
pub(super) async fn wait_for_refresh_request() {
    REFRESH_NOTIFY.notified().await;
}

/// Let a burst of activity settle before reading the chain.
///
/// A single swap emits several notifications and a DCA/partial sequence several swaps,
/// so waiting a beat collapses them into one refresh. It also avoids racing the very
/// transaction that woke us: `logsSubscribe` fires at `confirmed`, a hair before the
/// account read would reflect it.
pub(super) async fn settle_refresh_burst() {
    tokio::time::sleep(REFRESH_DEBOUNCE).await;
}

/// Mints currently held (fungible tokens with a non-zero balance).
///
/// Fed to pool discovery so that everything the wallet holds gets a live price —
/// otherwise a token the bot never traded (an airdrop, a manual buy elsewhere) would
/// be permanently unpriced and silently worth 0 in the headline.
pub fn get_held_mints() -> Vec<String> {
    live_wallet_snapshot()
        .map(|snapshot| {
            snapshot
                .token_balances
                .iter()
                .map(|balance| balance.mint.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The live pool price of a mint, when the pool service holds a fresh positive one.
fn live_pool_price(mint: &str) -> Option<f64> {
    crate::pools::get_pool_price(crate::chains::active_chain(), mint)
        .map(|price| price.price_native)
        .filter(|price| price.is_finite() && *price > 0.0)
}

/// Price one held token in SOL: live pool price first, the snapshot's market price
/// second. `None` when neither knows it — the caller must count it as unpriced, never
/// as zero-value-but-priced. Persisted snapshots value holdings with this same rule,
/// so the trend line plots the quantity the headline shows.
fn price_token_native(
    mint: &str,
    market_prices: &MarketPrices,
    pool_price: impl Fn(&str) -> Option<f64>,
) -> Option<f64> {
    pool_price(mint).or_else(|| market_prices.get(mint).copied())
}

/// Value a snapshot's holdings under the one pricing rule.
fn value_snapshot(
    snapshot: &WalletSnapshot,
    market_prices: &MarketPrices,
    pool_price: impl Fn(&str) -> Option<f64>,
) -> WalletWorth {
    let mut tokens_worth_native = 0.0;
    let mut unpriced_token_count = 0;

    for balance in &snapshot.token_balances {
        match price_token_native(&balance.mint, market_prices, &pool_price) {
            Some(price_native) => tokens_worth_native += balance.balance_ui * price_native,
            None => unpriced_token_count += 1,
        }
    }

    WalletWorth {
        native_balance: snapshot.native_balance,
        tokens_worth_native,
        total_equity_native: snapshot.native_balance + tokens_worth_native,
        token_count: snapshot.token_balances.len(),
        unpriced_token_count,
        updated_at: snapshot.snapshot_time,
        has_snapshot: true,
    }
}

/// Worth of a snapshot that is about to be persisted, priced now with the given
/// market prices.
pub(super) fn value_collected_snapshot(
    snapshot: &WalletSnapshot,
    market_prices: &MarketPrices,
) -> WalletWorth {
    value_snapshot(snapshot, market_prices, live_pool_price)
}

/// The price in SOL the live worth assigns to one held mint right now, or `None` when
/// the worth counts it as unpriced. Any surface that lists holdings beside the worth
/// prices them through this, so the rows sum to the headline.
pub fn held_token_price_native(mint: &str) -> Option<f64> {
    match LIVE_SNAPSHOT.load().as_ref() {
        Some(live) => price_token_native(mint, &live.market_prices, live_pool_price),
        None => live_pool_price(mint),
    }
}

/// The wallet's full worth, priced now. See the module doc.
pub fn get_wallet_worth() -> WalletWorth {
    match LIVE_SNAPSHOT.load_full() {
        Some(live) => value_snapshot(&live.snapshot, &live.market_prices, live_pool_price),
        None => WalletWorth::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::RawAmount;
    use crate::wallets::balance_monitor::types::SnapshotTokenBalance;
    use chrono::Utc;

    fn holding(mint: &str, balance_ui: f64) -> SnapshotTokenBalance {
        SnapshotTokenBalance {
            id: None,
            snapshot_id: None,
            mint: mint.to_owned(),
            balance: RawAmount::ZERO,
            balance_ui,
            decimals: 6,
            is_token_2022: false,
        }
    }

    fn snapshot(holdings: Vec<SnapshotTokenBalance>) -> WalletSnapshot {
        WalletSnapshot {
            id: None,
            wallet_address: "wallet".to_owned(),
            snapshot_time: Utc::now(),
            native_balance: 0.5,
            native_balance_raw: 500_000_000,
            total_equity_native: 0.5,
            total_tokens_count: holdings.len() as u32,
            total_nfts_count: 0,
            token_balances: holdings,
            nft_balances: Vec::new(),
        }
    }

    #[test]
    fn pool_price_wins_over_market_price() {
        let market = MarketPrices::from([("pooled".to_owned(), 9.0)]);
        let pool = |mint: &str| (mint == "pooled").then_some(2.0);
        assert_eq!(price_token_native("pooled", &market, pool), Some(2.0));
    }

    #[test]
    fn market_price_prices_a_token_without_a_pool_price() {
        let wallet = snapshot(vec![holding("pooled", 10.0), holding("market", 4.0)]);
        let market = MarketPrices::from([("market".to_owned(), 0.25)]);
        let pool = |mint: &str| (mint == "pooled").then_some(0.01);

        let worth = value_snapshot(&wallet, &market, pool);

        assert!((worth.tokens_worth_native - 1.1).abs() < 1e-12);
        assert!((worth.total_equity_native - 1.6).abs() < 1e-12);
        assert_eq!(worth.token_count, 2);
        assert_eq!(worth.unpriced_token_count, 0);
    }

    #[test]
    fn a_token_neither_source_prices_is_unpriced_never_zero_valued() {
        let wallet = snapshot(vec![holding("unknown", 100.0)]);
        let worth = value_snapshot(&wallet, &MarketPrices::new(), |_| None);
        assert_eq!(worth.tokens_worth_native, 0.0);
        assert_eq!(worth.unpriced_token_count, 1);
    }

    /// The headline is a pure function of the published snapshot, its market prices and
    /// the pool prices: repeated reads with unchanged inputs must return one value, so
    /// no cache with its own expiry can sit between the holdings and their price.
    #[test]
    fn repeated_valuations_of_one_snapshot_are_identical() {
        let wallet = snapshot(vec![
            holding("a", 1.0),
            holding("b", 2.0),
            holding("c", 3.0),
        ]);
        let market = MarketPrices::from([("b".to_owned(), 0.5), ("c".to_owned(), 0.1)]);
        let pool = |mint: &str| (mint == "a").then_some(0.2);

        let first = value_snapshot(&wallet, &market, pool);
        for _ in 0..50 {
            let next = value_snapshot(&wallet, &market, pool);
            assert_eq!(next.total_equity_native, first.total_equity_native);
            assert_eq!(next.unpriced_token_count, first.unpriced_token_count);
        }
    }

    #[test]
    fn valuation_reads_no_token_snapshot_store() {
        let source = include_str!("worth.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            !production.contains("get_cached_token") && !production.contains("tokens::store"),
            "worth must price from the published market prices, not the TTL token store"
        );
    }
}
