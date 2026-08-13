//! Demo mode for dashboard screenshots and marketing materials.
//!
//! Provides realistic, INTERNALLY CONSISTENT data for showcasing the bot in
//! screenshots, videos and social posts. Every aggregate is derived from the two
//! token arrays in `data.rs` (see `aggregates.rs`), so P&L, win rate, invested,
//! trade counts and wallet worth all reconcile across endpoints.
//!
//! Enable with: cargo run --bin screenerbot -- --gui --dashboard-demo
//!
//! Affected endpoints:
//! - /api/dashboard/home, /api/dashboard/overview, /api/dashboard/portfolio-calendar
//! - /api/positions, /api/positions/stats
//! - /api/wallet/current, /api/wallet/tokens
//! - /api/trader/stats
//! - /api/header/metrics (SOL price is LIVE when the network is reachable)

use std::sync::atomic::{AtomicBool, Ordering};

mod aggregates;
mod ai;
mod dashboard;
mod data;
mod header;
mod positions;
mod trader;
mod wallet;

pub use ai::get_demo_ai_status;
pub use dashboard::{
    get_demo_dashboard_overview, get_demo_home_dashboard, get_demo_portfolio_calendar,
};
pub use header::get_demo_header_metrics;
pub use positions::{get_demo_positions, get_demo_positions_stats};
pub use trader::get_demo_trader_stats;
pub use wallet::{get_demo_wallet_address, get_demo_wallet_current, get_demo_wallet_tokens};

/// Fallback SOL/USD price used only when the live price service has not yet
/// produced a fresh quote (e.g. offline). A live value always takes precedence.
pub(crate) const DEMO_SOL_PRICE_FALLBACK: f64 = 176.42;

/// Global flag for demo mode - set at startup based on --dashboard-demo argument.
pub static DEMO_MODE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Demo capture runtime — the dashboard loads its capture layer (overlays,
/// narration, quiescence reporting) so an external driver can produce
/// screenshots and recordings without guessing at timings.
pub static DEMO_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Demo freeze — every remaining live value is pinned to its demo constant so
/// repeated capture runs render identical frames.
pub static DEMO_FREEZE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if demo mode is active.
pub fn is_demo_mode() -> bool {
    DEMO_MODE_ENABLED.load(Ordering::Relaxed)
}

/// Check if the dashboard should serve and load the demo capture runtime.
pub fn is_demo_capture() -> bool {
    DEMO_CAPTURE_ENABLED.load(Ordering::Relaxed)
}

/// Check if live values must be pinned to their demo constants.
pub fn is_demo_frozen() -> bool {
    DEMO_FREEZE_ENABLED.load(Ordering::Relaxed)
}

/// Enable demo mode (called at startup if --dashboard-demo flag is present).
pub fn enable_demo_mode() {
    DEMO_MODE_ENABLED.store(true, Ordering::SeqCst);
}

/// Enable the demo capture runtime (called at startup for --demo-capture).
pub fn enable_demo_capture() {
    DEMO_CAPTURE_ENABLED.store(true, Ordering::SeqCst);
}

/// Enable demo freeze (called at startup for --demo-freeze).
pub fn enable_demo_freeze() {
    DEMO_FREEZE_ENABLED.store(true, Ordering::SeqCst);
}
