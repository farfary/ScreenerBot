//! Database file path resolution.

use super::get_data_directory;
use std::path::PathBuf;

/// Returns the tokens database path.
pub fn get_tokens_db_path() -> PathBuf {
    get_data_directory().join("tokens.db")
}

/// Returns the transactions database path.
pub fn get_transactions_db_path() -> PathBuf {
    get_data_directory().join("transactions.db")
}

/// Returns the positions database path.
pub fn get_positions_db_path() -> PathBuf {
    get_data_directory().join("positions.db")
}

/// Returns the wallet database path.
pub fn get_wallet_db_path() -> PathBuf {
    get_data_directory().join("wallet.db")
}

/// Returns the events database path.
pub fn get_events_db_path() -> PathBuf {
    get_data_directory().join("events.db")
}

/// Returns the pools database path.
pub fn get_pools_db_path() -> PathBuf {
    get_data_directory().join("pools.db")
}

/// Returns the strategies database path.
pub fn get_strategies_db_path() -> PathBuf {
    get_data_directory().join("strategies.db")
}

/// Returns the OHLCV database path.
pub fn get_ohlcvs_db_path() -> PathBuf {
    get_data_directory().join("ohlcvs.db")
}

/// Returns the actions database path.
pub fn get_actions_db_path() -> PathBuf {
    get_data_directory().join("actions.db")
}

/// Returns the tools database path.
pub fn get_tools_db_path() -> PathBuf {
    get_data_directory().join("tools.db")
}

/// Returns the AI database path.
pub fn get_ai_db_path() -> PathBuf {
    get_data_directory().join("ai.db")
}

/// Returns the AI chat database path.
pub fn get_ai_chat_db_path() -> PathBuf {
    get_data_directory().join("ai_chat.db")
}

/// Returns all related files for a SQLite database (main DB, SHM, WAL).
///
/// SQLite databases create additional files for write-ahead logging and
/// shared memory. This helper returns all three files for cleanup operations.
pub fn get_db_with_wal_files(db_path: PathBuf) -> Vec<PathBuf> {
    vec![
        db_path.clone(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ]
}
