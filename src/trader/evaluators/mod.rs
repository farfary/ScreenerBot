//! Evaluation logic for trading decisions
//!
//! This module contains all business logic for determining whether to enter or exit trades:
//! - Entry evaluation (safety checks + strategy signals)
//! - Exit evaluation (priority-based exit conditions)
//! - DCA evaluation (dollar cost averaging logic)
//! - Strategy evaluation (configured trading strategies)
//!
//! Exit strategies (roi, trailing stop, time override) are in separate files for clarity.

pub mod dca;
pub mod entry;
pub mod exit;
pub mod exit_roi;
pub mod exit_time;
pub mod exit_trailing;
pub mod strategies;

// Re-exports for convenience
pub use dca::{process_dca_opportunities, DcaCalculations, DcaConfigSnapshot, DcaEvaluation};
pub use entry::evaluate_entry_for_token;
pub use exit::evaluate_exit_for_position;
pub use strategies::StrategyEvaluator;
