#![allow(warnings)]

pub mod arguments;
pub mod ata_cleanup;
pub mod config; // New unified config system with zero repetition
pub mod configs; // Legacy config system (to be migrated)
pub mod entry; // New improved entry system
pub mod errors; // New structured error handling
pub mod events; // Event recording system for analytics and debugging
pub mod filtering;
pub mod global;
pub mod learner;
pub mod logger;
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
