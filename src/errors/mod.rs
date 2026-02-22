//! Structured error types used throughout ScreenerBot.
//!
//! Public surface:
//! - `crate::Error` / `crate::errors::Error`
//! - `crate::Result<T>` / `crate::errors::Result<T>`
//!
//! Historical name (kept for compatibility):
//! - `crate::errors::ScreenerBotError` (type alias)

pub mod blockchain;

mod configuration;
mod data;
mod error;
mod network;
mod position;
mod rate_limit;
mod rpc_provider;

pub use blockchain::*;
pub use configuration::*;
pub use data::*;
pub use error::{Error, Result};
pub use network::*;
pub use position::*;
pub use rate_limit::*;
pub use rpc_provider::*;

/// Backward-compatibility alias.
///
/// Prefer `crate::Error` + `crate::Result<T>`.
pub type ScreenerBotError = Error;
