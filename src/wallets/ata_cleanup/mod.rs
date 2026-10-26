// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The automatic empty-account cleanup: a background service that closes the
//! wallet's empty Associated Token Accounts to reclaim the rent deposit each one
//! holds. The deposit is the account's own lamports, read from chain; it differs
//! by account size and the current rent rate. Failed closes are parked in the
//! failed-close cache until their retry time.

pub mod database;
mod operations;
mod service;
mod types;

pub use service::start_ata_cleanup_service;
