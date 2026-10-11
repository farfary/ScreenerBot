// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Evaluation logic for trading decisions
//!
//! This module contains all business logic for determining whether to enter or exit trades:
//! - Entry evaluation (safety checks + strategy signals)
//! - Exit evaluation (priority-based exit conditions)
//! - DCA evaluation (dollar cost averaging logic)
//! - Strategy evaluation (user-configured trading strategies)
//!
//! Built-in exit rules (roi, trailing stop, stop loss, time override) are in separate files.
//! These are NOT strategy-system strategies - they're hardcoded rules with config toggles.

pub mod dca;
pub mod entry;
pub mod exit;
pub mod exit_roi;
pub mod exit_stop_loss;
pub mod exit_time;
pub mod exit_trailing;
pub mod strategies;

// Re-exports for convenience
pub use dca::{process_dca_opportunities, DcaCalculations, DcaConfigSnapshot, DcaEvaluation};
pub use entry::evaluate_entry_for_token;
pub use exit::evaluate_exit_for_position;
pub use strategies::StrategyEvaluator;

#[cfg(test)]
mod tests {
    /// The evaluators that propose a position-sized change (an add, a partial exit) stay
    /// quiet while the positions store awaits reconciliation, instead of proposing one
    /// every tick for the operation to refuse. Full-close rules never ask.
    #[test]
    fn position_sized_evaluators_check_reconciliation() {
        let gate = "awaiting_reconciliation()";
        assert!(include_str!("dca.rs").contains(gate));
        assert!(include_str!("exit_stop_loss.rs").contains(gate));
        for (file, source) in [
            ("exit_roi.rs", include_str!("exit_roi.rs")),
            ("exit_time.rs", include_str!("exit_time.rs")),
            ("exit_trailing.rs", include_str!("exit_trailing.rs")),
            ("exit.rs", include_str!("exit.rs")),
        ] {
            assert!(
                !source.contains(gate),
                "{file} proposes full closes and must not wait for reconciliation"
            );
        }
    }
}
