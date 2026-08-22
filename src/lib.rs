//! ScreenerBot — Core library for automated Solana DeFi trading.
//!
//! Provides token discovery, on-chain analysis, position management,
//! swap execution, and a web dashboard for monitoring and control.

// `dead_code` stays allowed: this is pre-release code with subsystems wired ahead of
// their callers, and auditing ~45 never-used items is a separate deliberate pass.
// Every other lint is ON — a blanket `allow(warnings)` previously hid 463 warnings,
// including API failure stats that were never recorded because an async call sat in a
// synchronous closure and was never awaited.
#![allow(dead_code)]

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
pub mod process;
pub mod reset;
pub mod rpc;
pub mod run;
pub mod secure_storage;
pub mod services;
pub mod strategies;
pub mod swaps;
pub mod telegram;
pub mod tokens;
pub mod tools;
pub mod trader;
pub mod transactions;
pub mod utils;
pub mod version;
pub mod wallets;
pub mod webserver;

pub use apis::sol_price;
pub use errors::Error;
pub use wallets::balance_monitor as wallet;
pub use wallets::validation as wallet_validation;

pub type Result<T> = std::result::Result<T, Error>;
