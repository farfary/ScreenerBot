#![allow(warnings)]

pub mod arguments;
pub mod ata_cleanup;
pub mod config; // Unified config system with zero repetition
pub mod constants; // Global constants used across modules
pub mod entry; // New improved entry system
pub mod errors; // New structured error handling
pub mod events; // Event recording system for analytics and debugging
pub mod filtering;
pub mod global;
pub mod learner;
pub mod logger;
pub mod ohlcvs; // OHLCV data module for chart data
pub mod pools; // New modular pool system
pub mod positions;
pub mod profit;
pub mod rpc;
pub mod run;
pub mod services; // Service manager for orchestrating all services
pub mod sol_price; // SOL price service
pub mod summary;
pub mod swaps;
pub mod tokens;
pub mod trader;
pub mod transactions; // New modular transactions system
pub mod utils;
pub mod wallet; // Wallet balance monitoring
pub mod webserver; // Webserver dashboard for monitoring and management
