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

#[cfg(test)]
mod tests {
    //! L0 (pure) unit tests for slippage resolution. Co-located because the functions
    //! are `pub(super)` and unreachable from the `tests/` integration crates. Config
    //! is initialised to defaults in-memory (quote_default_pct = 1.0,
    //! exit_retry_steps_pct = [3, 10, 25]).
    use super::*;
    use crate::config::schemas::Config;
    use crate::config::utils::CONFIG;

    fn init_config() {
        let _ = CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
    }

    #[test]
    fn entry_none_follows_config_default() {
        init_config();
        assert_eq!(entry_slippage(None), 1.0);
    }

    #[test]
    fn entry_override_is_used_verbatim() {
        init_config();
        assert_eq!(entry_slippage(Some(7.5)), 7.5);
    }

    #[test]
    fn exit_none_is_the_configured_ladder() {
        init_config();
        assert_eq!(exit_slippage_ladder(None), vec![3.0, 10.0, 25.0]);
    }

    #[test]
    fn exit_override_below_all_steps_prepends_and_keeps_them() {
        init_config();
        // A 1% floor must not strand the user: the configured escalation still runs above it.
        assert_eq!(exit_slippage_ladder(Some(1.0)), vec![1.0, 3.0, 10.0, 25.0]);
    }

    #[test]
    fn exit_override_between_steps_drops_lower_rungs() {
        init_config();
        // 5% starts the ladder; the already-rejected 3% rung is dropped, higher rungs stay.
        assert_eq!(exit_slippage_ladder(Some(5.0)), vec![5.0, 10.0, 25.0]);
    }

    #[test]
    fn exit_override_is_first_rung_and_ladder_is_ascending() {
        init_config();
        let ladder = exit_slippage_ladder(Some(12.0));
        assert_eq!(
            ladder.first(),
            Some(&12.0),
            "override must be the starting rung"
        );
        assert!(
            ladder.windows(2).all(|w| w[0] < w[1]),
            "ladder must escalate strictly upward: {ladder:?}"
        );
    }
}
