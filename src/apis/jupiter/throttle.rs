//! Centralized throttle for Jupiter `lite-api.jup.ag` calls.
//!
//! Jupiter's free tier rate-limits per IP across ALL endpoints, so background
//! pollers (SOL price, token discovery, endpoint-health pings) can starve the
//! swap quote/swap calls that are the product's revenue path — producing the
//! HTTP 429s seen in trading. This gate gives swaps priority:
//!
//! - A swap holds a [`SwapGuard`] for the duration of its quote/swap request.
//! - Background callers call [`acquire_background`] first: they defer (bounded)
//!   while any swap is in flight, and are spaced by a minimum interval so bursts
//!   don't exhaust the shared budget.
//!
//! Swap calls deliberately do NOT pass through [`acquire_background`] — they go
//! straight through (with their own retry+backoff in the router), so they always
//! win the budget against background traffic.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Minimum spacing between background Jupiter calls.
const BACKGROUND_MIN_INTERVAL: Duration = Duration::from_millis(400);

/// Max time a background call defers to in-flight swaps before proceeding anyway
/// (so background pollers can never hang indefinitely).
const MAX_SWAP_WAIT: Duration = Duration::from_millis(2500);

static SWAPS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static LAST_BACKGROUND_CALL: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

/// RAII guard marking a swap (quote or swap) as in flight. While any guard is
/// alive, background Jupiter callers defer to it.
pub struct SwapGuard(());

impl Drop for SwapGuard {
    fn drop(&mut self) {
        SWAPS_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Mark a swap as in flight; hold the returned guard for the request duration.
pub fn swap_guard() -> SwapGuard {
    SWAPS_IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
    SwapGuard(())
}

/// True while at least one swap quote/swap is in flight.
pub fn swap_in_flight() -> bool {
    SWAPS_IN_FLIGHT.load(Ordering::SeqCst) > 0
}

/// Acquire permission for a background (low-priority) Jupiter call. Yields to
/// in-flight swaps (bounded by [`MAX_SWAP_WAIT`]) and enforces a minimum spacing
/// between background calls so discovery/price/health bursts don't trigger 429s
/// that would also hit swaps.
pub async fn acquire_background() {
    // Defer to in-flight swaps, but cap total wait so background never hangs.
    let start = Instant::now();
    while swap_in_flight() && start.elapsed() < MAX_SWAP_WAIT {
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    // Space background calls. The lock is held across the sleep so concurrent
    // background callers serialize and inherit the spacing.
    let mut last = LAST_BACKGROUND_CALL.lock().await;
    if let Some(prev) = *last {
        let elapsed = prev.elapsed();
        if elapsed < BACKGROUND_MIN_INTERVAL {
            tokio::time::sleep(BACKGROUND_MIN_INTERVAL - elapsed).await;
        }
    }
    *last = Some(Instant::now());
}
