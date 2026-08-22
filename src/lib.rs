//! ScreenerBot — Core library for automated Solana DeFi trading.
//!
//! Provides token discovery, on-chain analysis, position management,
//! swap execution, and a web dashboard for monitoring and control.

#![allow(warnings)]

pub mod account;
pub mod actions;
pub mod ai;
pub mod apis;
pub mod arguments;
pub mod chains;
pub mod config;
pub mod connectivity;
pub mod database;
pub mod errors;
pub use errors::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub mod events;
pub mod features;
pub mod filtering;
pub mod global;
pub mod logger;
pub mod net;
pub mod ohlcvs;
pub mod paths;
pub mod pools;
pub mod positions;
pub mod process_lock;
pub mod profiling;
pub mod reset;
pub mod rpc;
pub mod run;
pub mod secure_storage;
pub mod services;
pub use apis::sol_price;
pub mod startup;
pub mod strategies;
pub mod swaps;
pub mod telegram;
pub mod tokens;
pub mod tools;
pub mod trader;
pub mod transactions;
pub mod utils;
pub mod version;
pub use wallets::balance_monitor as wallet;
pub use wallets::validation as wallet_validation;
pub mod wallets;
pub mod webserver;

use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide "the app is shutting down" flag.
///
/// Lives in the library, not in `main`, because the modules that need to *read*
/// it are all library modules: once shutdown begins, in-flight work is
/// abandoned on purpose, and a component that fails because a peer has already
/// torn down its channel is reporting an expected consequence of exiting, not a
/// fault. Without a library-visible flag those components had no way to tell the
/// two apart, so they logged teardown as failure — the pool analyzer filed a
/// warning for every fetch request it handed to the already-closed fetcher
/// channel, burying the real shutdown sequence.
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Whether shutdown has been requested. Use this to downgrade or suppress a
/// diagnostic that is only meaningful while the app is running — never to skip
/// cleanup work.
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

/// Request application shutdown.
pub fn request_shutdown() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}
