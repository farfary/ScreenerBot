// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Process-level concerns: the single-instance lock, profiling, the shutdown flag, the panic hook and restart.

mod error;
pub use error::{Error, Result};

pub mod lock;
pub mod panic_hook;
pub mod profiling;
pub mod restart;
pub mod shutdown;
