#![allow(warnings)]

pub mod arguments;
pub mod ata_cleanup;
pub mod configs;
pub mod dashboard;
pub mod entry; // New improved entry system
pub mod errors; // New structured error handling
pub mod filtering;
pub mod global;
pub mod logger;
pub mod pool_calculator; // Pool price calculator service
pub mod pool_cleanup; // Pool cleanup service for data maintenance
pub mod pool_monitor; // Pool monitor service for task health monitoring
pub mod pool_tokens; // Pool tokens service for loading tokens from database
pub mod pool_db; // Pool database module
pub mod pool_decoders; // Pool data decoders
pub mod pool_discovery; // Pool discovery service for finding pools via APIs
pub mod pool_fetcher; // Pool token account fetcher service
pub mod pool_interface; // Pool service interface
pub mod pool_service; // New modular pool service
pub mod positions;
pub mod positions_db; // Positions database module
pub mod positions_lib; // Position management library
pub mod positions_types; // Position type definitions
pub mod profit;
pub mod rpc;
pub mod run;
pub mod solscan; // Solscan API integration
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
pub mod websocket; // WebSocket client for real-time transaction monitoring
