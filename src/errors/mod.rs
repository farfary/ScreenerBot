//! Structured error types used throughout ScreenerBot.
//!
//! Public surface:
//! - `crate::Error` / `crate::errors::Error`
//! - `crate::Result<T>` / `crate::errors::Result<T>`
//!
//! Historical name (kept for compatibility):
//! - `crate::errors::ScreenerBotError` (type alias)

pub mod blockchain;

mod database;
mod configuration;
mod data;
mod error;
mod internal;
mod io;
mod network;
mod position;
mod rate_limit;
mod rpc_provider;
mod service;

pub use blockchain::*;
pub use database::*;
pub use configuration::*;
pub use data::*;
pub use error::{Error, Result};
pub use internal::*;
pub use io::*;
pub use network::*;
pub use position::*;
pub use rate_limit::*;
pub use rpc_provider::*;
pub use service::*;

/// Backward-compatibility alias.
///
/// Prefer `crate::Error` + `crate::Result<T>`.
pub type ScreenerBotError = Error;
