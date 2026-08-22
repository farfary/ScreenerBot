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

use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use tokio::sync::Notify;

use super::types::{WalletSnapshot, WalletWorth};

/// Collapse a burst of on-chain activity (a swap emits several notifications, and a
/// DCA/partial sequence emits several swaps) into ONE refresh. Also lets the balance
/// settle: `logsSubscribe` fires at `confirmed`, and reading the account a beat later
/// avoids racing the very transaction that woke us.
const REFRESH_DEBOUNCE: Duration = Duration::from_millis(800);

/// The latest fully-populated snapshot, published by the balance monitor. Read
/// lock-free by every worth query.
static LIVE_SNAPSHOT: ArcSwapOption<WalletSnapshot> = ArcSwapOption::const_empty();

static REFRESH_NOTIFY: std::sync::LazyLock<Notify> = std::sync::LazyLock::new(Notify::new);

/// Publish a freshly collected snapshot as the live one.
pub(super) fn publish_snapshot(snapshot: Arc<WalletSnapshot>) {
    LIVE_SNAPSHOT.store(Some(snapshot));
}

/// The live snapshot, if the monitor has produced (or hydrated) one.
///
/// Public because it is the same source `get_wallet_worth()` reads: an API that
/// serves a balance must serve THIS, not a database row, or the number it returns
/// disagrees with the header and the home hero.
pub fn live_wallet_snapshot() -> Option<Arc<WalletSnapshot>> {
    LIVE_SNAPSHOT.load_full()
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

/// Price one held token in SOL: live pool price first, token-database market price
/// second. `None` when neither knows it — the caller must count it as unpriced, never
/// as zero-value-but-priced.
fn price_token_sol(mint: &str) -> Option<f64> {
    if let Some(price) = crate::pools::get_pool_price(mint) {
        if price.price_sol.is_finite() && price.price_sol > 0.0 {
            return Some(price.price_sol);
        }
    }

    crate::tokens::get_cached_token(crate::chains::active_chain(), mint)
        .map(|token| token.price_sol)
        .filter(|price| price.is_finite() && *price > 0.0)
}

/// The wallet's full worth, priced now. See the module doc.
pub fn get_wallet_worth() -> WalletWorth {
    let Some(snapshot) = live_wallet_snapshot() else {
        return WalletWorth::default();
    };

    let mut tokens_worth_sol = 0.0;
    let mut unpriced_token_count = 0;

    for balance in &snapshot.token_balances {
        match price_token_sol(&balance.mint) {
            Some(price_sol) => tokens_worth_sol += balance.balance_ui * price_sol,
            None => unpriced_token_count += 1,
        }
    }

    WalletWorth {
        sol_balance: snapshot.sol_balance,
        tokens_worth_sol,
        total_equity_sol: snapshot.sol_balance + tokens_worth_sol,
        token_count: snapshot.token_balances.len(),
        unpriced_token_count,
        updated_at: snapshot.snapshot_time,
        has_snapshot: true,
    }
}
