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
//! Built-in exit rules (stop loss, trailing stop, break-even stop, take profit, time override,
//! stagnation) are in separate files,
//! and `exit_rules` owns the order they are checked in for the live and the paper books.
//! `exit_time_watch` reports which time rule watches an open position and when it is due.
//! These are NOT strategy-system strategies - they're hardcoded rules with config toggles.

pub mod dca;
pub mod entry;
pub mod exit;
pub mod exit_break_even;
pub mod exit_rules;
pub mod exit_stop_loss;
pub mod exit_take_profit;
pub mod exit_time;
pub mod exit_time_stagnation;
pub mod exit_time_watch;
pub mod exit_trailing;
pub mod strategies;

// Re-exports for convenience
pub use dca::{
    default_add_size_native, process_dca_opportunities, DcaCalculations, DcaConfigSnapshot,
    DcaContext, DcaEvaluation,
};
pub use entry::{entry_cycle, evaluate_entry_for_token};
pub use exit::{evaluate_exit_for_position, exit_cycle};
pub use strategies::StrategyEvaluator;

#[cfg(test)]
mod tests {
    /// The evaluators that propose a position-sized change (an add, a partial exit) stay
    /// quiet while the positions store awaits reconciliation, instead of proposing one
    /// every tick for the operation to refuse. The pure rule files never ask: the live
    /// coordinator gates the partial it acts on.
    #[test]
    fn position_sized_evaluators_check_reconciliation() {
        let gate = "awaiting_reconciliation(";
        assert!(include_str!("dca.rs").contains(gate));
        assert!(include_str!("exit.rs").contains(gate));
        for (file, source) in [
            ("exit_rules.rs", include_str!("exit_rules.rs")),
            ("exit_break_even.rs", include_str!("exit_break_even.rs")),
            ("exit_stop_loss.rs", include_str!("exit_stop_loss.rs")),
            ("exit_take_profit.rs", include_str!("exit_take_profit.rs")),
            ("exit_time.rs", include_str!("exit_time.rs")),
            (
                "exit_time_stagnation.rs",
                include_str!("exit_time_stagnation.rs"),
            ),
            ("exit_time_watch.rs", include_str!("exit_time_watch.rs")),
            ("exit_trailing.rs", include_str!("exit_trailing.rs")),
        ] {
            assert!(
                !source.contains(gate),
                "{file} is pure and must not wait for reconciliation"
            );
        }
    }

    /// Automatic adds stop while the loss limit blocks entries: the DCA cycle reads the
    /// same gate entry admission reads, never a copy of its rule. Admission reads it
    /// through the trader's entry halt, which the controller owns.
    #[test]
    fn the_dca_cycle_reads_the_entry_loss_limit_gate() {
        let gate = "loss_limit::is_entry_blocked_by_loss_limit()";
        assert!(include_str!("dca.rs").contains(gate));
        let controller = include_str!("../controller.rs");
        let halt = &controller[controller
            .find("pub fn entry_halt(")
            .expect("the controller owns the entry halt")..];
        let halt = &halt[..halt.find("\n}\n").expect("the entry halt ends")];
        assert!(
            halt.contains(gate),
            "the entry halt reads the loss limit gate"
        );
        let admission = include_str!("../admission.rs");
        assert!(admission.contains("controller::entry_halt(TradingMode::Live)"));
        assert!(
            !admission.contains(gate),
            "admission reads the gate only through the entry halt"
        );
    }
}
