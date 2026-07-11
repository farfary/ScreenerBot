//! Slippage resolution for position operations.
//!
//! Every swap gets its slippage from the config (`swaps.slippage.*`). A MANUAL trade
//! may additionally carry a per-trade override, which the user set in the trade
//! dialog to force a fill on an illiquid token. The AUTO-TRADER never sets one — it
//! always passes `None` and stays fully config-driven.

use crate::config::with_config;

/// Slippage (in percent) for an ENTRY swap (open / DCA).
///
/// `None` → the configured quote default.
pub(super) fn entry_slippage(override_pct: Option<f64>) -> f64 {
    override_pct.unwrap_or_else(|| with_config(|cfg| cfg.swaps.slippage.quote_default_pct))
}

/// Slippage ladder (in percent) for an EXIT swap (close / partial close).
///
/// Exits retry with escalating slippage so a position can always be got out of. An
/// override REPLACES the starting rung but does not disable the escalation: the
/// ladder becomes the override followed by every configured step ABOVE it.
///
/// This matters. Simply substituting the override would let a user who picks 1% get
/// stuck in a position the configured 3/10/25 ladder would have exited; and simply
/// prepending it would leave lower rungs that the user has already rejected. Starting
/// at the user's floor and escalating only upward honours the override AND keeps the
/// safety property that an exit eventually goes through.
pub(super) fn exit_slippage_ladder(override_pct: Option<f64>) -> Vec<f64> {
    let configured = with_config(|cfg| cfg.swaps.slippage.exit_retry_steps_pct.clone());

    let Some(pct) = override_pct else {
        return configured;
    };

    let mut ladder = vec![pct];
    ladder.extend(configured.into_iter().filter(|step| *step > pct));
    ladder
}
