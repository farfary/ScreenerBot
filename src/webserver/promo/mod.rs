//! Owner-only promotional fixtures and capture state.
//!
//! Provides realistic, INTERNALLY CONSISTENT data for showcasing the bot in
//! screenshots, videos and social posts. Every aggregate is derived from the two
//! token arrays in `data.rs` (see `aggregates.rs`), so P&L, win rate, invested,
//! trade counts and wallet worth all reconcile across endpoints.
//!
//! Enable fixtures with: cargo run --bin screenerbot -- --gui --promo-fixtures
//!
//! Affected endpoints:
//! - /api/dashboard/home, /api/dashboard/overview, /api/dashboard/portfolio-calendar
//! - /api/positions, /api/positions/stats
//! - /api/copy-trading/overview
//! - /api/events/head
//! - /api/wallet/current, /api/wallet/tokens
//! - /api/trader/stats
//! - /api/header/metrics (SOL price is LIVE when the network is reachable)

use std::sync::atomic::{AtomicBool, Ordering};

mod aggregates;
mod ai;
mod copy_trading;
mod dashboard;
mod data;
mod events;
mod header;
mod positions;
mod trader;
mod wallet;

pub use ai::get_promo_ai_status;
pub use copy_trading::get_promo_copy_trading_overview;
pub use dashboard::{
    get_promo_dashboard_overview, get_promo_home_dashboard, get_promo_portfolio_calendar,
};
pub use events::get_promo_events;
pub use header::get_promo_header_metrics;
pub use positions::{get_promo_positions, get_promo_positions_stats};
pub use trader::get_promo_trader_stats;
pub use wallet::{get_promo_wallet_address, get_promo_wallet_current, get_promo_wallet_tokens};

/// Fallback SOL/USD price used only when the live price service has not yet
/// produced a fresh quote (e.g. offline). A live value always takes precedence.
pub(crate) const PROMO_SOL_PRICE_FALLBACK: f64 = 176.42;

/// Promotional fixture flag, set at startup from `--promo-fixtures`.
pub static PROMO_FIXTURES_ENABLED: AtomicBool = AtomicBool::new(false);

/// Promo capture runtime — the dashboard loads its capture layer (overlays,
/// narration, quiescence reporting) so an external driver can produce
/// screenshots and recordings without guessing at timings.
pub static PROMO_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Promo freeze — every remaining live value is pinned to its promo constant so
/// repeated capture runs render identical frames.
pub static PROMO_FREEZE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check whether promotional fixtures should replace live endpoint data.
pub fn are_promo_fixtures_enabled() -> bool {
    PROMO_FIXTURES_ENABLED.load(Ordering::Relaxed)
}

/// Check if the dashboard should serve and load the promo capture runtime.
pub fn is_promo_capture_enabled() -> bool {
    PROMO_CAPTURE_ENABLED.load(Ordering::Relaxed)
}

/// Check if live values must be pinned to their promo constants.
pub fn is_promo_frozen() -> bool {
    PROMO_FREEZE_ENABLED.load(Ordering::Relaxed)
}

/// Enable promotional fixtures.
pub fn enable_promo_fixtures() {
    PROMO_FIXTURES_ENABLED.store(true, Ordering::SeqCst);
}

/// Enable the promo capture runtime (called at startup for --promo-capture).
pub fn enable_promo_capture() {
    PROMO_CAPTURE_ENABLED.store(true, Ordering::SeqCst);
}

/// Enable promo freeze (called at startup for --promo-freeze).
pub fn enable_promo_freeze() {
    PROMO_FREEZE_ENABLED.store(true, Ordering::SeqCst);
}
