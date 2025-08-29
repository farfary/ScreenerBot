#![allow(warnings)]

pub mod arguments;
pub mod configs;
pub mod global;
pub mod utils;
pub mod trader;
pub mod logger;
pub mod summary;
pub mod positions;
pub mod tokens;
pub mod profit;
pub mod filtering;
pub mod rpc;
pub mod ata_cleanup;
pub mod transactions;
pub mod transactions_db; // New database module
pub mod positions_db; // Positions database module
pub mod entry;
pub mod ohlcv_analysis;
pub mod swaps;
pub mod errors; // New structured error handling
pub mod dashboard;
pub mod wallet; // Wallet balance monitoring
