// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Storage housekeeping: the declared retention of every file family, a pure planner that holds the app's disk usage to an adaptive budget, and the one engine that deletes files and checkpoints or compacts stores.
//!
//! Low free disk space only tightens retention and is reported in the logs; it never
//! changes trading.

mod engine;
pub mod families;
pub mod plan;
pub mod types;
pub mod usage;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod families_tests;
#[cfg(test)]
mod plan_tests;

pub use engine::{carry_out, run_pass, start_housekeeping, Outcome};
pub use families::{classify, LOG_FILE_MAX_BYTES};
