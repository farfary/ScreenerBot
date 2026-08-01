//! Structured error types used throughout ScreenerBot.
//!
//! Public surface:
//! - `crate::Error` / `crate::errors::Error`
//! - `crate::Result<T>` / `crate::errors::Result<T>`

mod account;
pub mod blockchain;
mod blockchain_parser;

mod configuration;
mod data;
mod database;
mod error;
mod internal;
mod io;
mod network;
mod position;
mod rate_limit;
mod rpc_provider;
mod service;
mod startup;

pub use account::*;
pub use blockchain::*;
pub use blockchain_parser::*;
pub use configuration::*;
pub use data::*;
pub use database::*;
pub use error::{Error, Result};
pub use internal::*;
pub use io::*;
pub use network::*;
pub use position::*;
pub use rate_limit::*;
pub use rpc_provider::*;
pub use service::*;
pub use startup::{StartupError, StartupErrorCode, StartupRecovery};
