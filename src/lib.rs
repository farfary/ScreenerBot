#![allow(warnings)]

pub mod arguments;
pub mod ata_cleanup;
pub mod configs;
pub mod dashboard;
pub mod entry;
pub mod errors; // New structured error handling
pub mod filtering;
pub mod global;
pub mod logger;
pub mod ohlcv_analysis;
pub mod positions;
pub mod positions_db; // Positions database module
pub mod positions_lib; // Position management library
pub mod positions_types; // Position type definitions
pub mod profit;
pub mod rpc;
pub mod summary;
pub mod swaps;
pub mod tokens;
pub mod trader;
pub mod transactions;
pub mod transactions_db; // New database module
pub mod transactions_debug; // Transaction debugging utilities
pub mod transactions_lib; // Transaction analysis library
pub mod transactions_types; // Transaction type definitions
pub mod utils;
pub mod wallet; // Wallet balance monitoring
