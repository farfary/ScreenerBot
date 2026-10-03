// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Database file path resolution.

use super::get_data_directory;
use crate::chains::ChainId;
use std::path::PathBuf;

/// Returns the transactions database path.
pub fn get_transactions_db_path() -> PathBuf {
    get_data_directory().join("transactions.db")
}

/// Returns the positions database path.
pub fn get_positions_db_path() -> PathBuf {
    get_data_directory().join("positions.db")
}

/// Returns the wallet database path (balance monitor: snapshots, worth history).
pub fn get_wallet_db_path() -> PathBuf {
    get_data_directory().join("wallet.db")
}

/// Returns the wallets database path (wallet list/keys, and the watch service's
/// `watch_targets` / `watch_cursors` tables -- a distinct file from `wallet.db`
/// above, despite the similar name).
pub fn get_wallets_db_path() -> PathBuf {
    get_data_directory().join("wallets.db")
}

/// Returns the events database path.
pub fn get_events_db_path() -> PathBuf {
    get_data_directory().join("events.db")
}

/// Returns the strategies database path.
pub fn get_strategies_db_path() -> PathBuf {
    get_data_directory().join("strategies.db")
}

/// Returns the copy-trading policy and paper-decision database path.
pub fn get_copy_trading_db_path() -> PathBuf {
    get_data_directory().join("copy_trading.db")
}

/// Returns the actions database path.
pub fn get_actions_db_path() -> PathBuf {
    get_data_directory().join("actions.db")
}

/// Returns the tools database path.
pub fn get_tools_db_path() -> PathBuf {
    get_data_directory().join("tools.db")
}

/// Returns the legacy-named LLM-analysis database path.
pub fn get_ai_db_path() -> PathBuf {
    get_data_directory().join("ai.db")
}

/// Returns the agent-control database path (durable client pairings, the
/// external-agent approval queue and the agent-control audit log).
pub fn get_agent_control_db_path() -> PathBuf {
    get_data_directory().join("agent_control.db")
}

/// Returns the legacy-named Assistant chat database path.
pub fn get_ai_chat_db_path() -> PathBuf {
    get_data_directory().join("ai_chat.db")
}

/// The chain-scoped SQLite stores: one file per chain, holding only that
/// chain's rows. Everything else (positions, wallets, transactions, …) is a
/// shared file with a `chain_id` column and does not belong here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    /// Token metadata, market data and blacklists (`tokens.db` on Solana).
    Tokens,
    /// Price history and pool blacklists (`pools.db` on Solana).
    Pools,
    /// Candles, pool bindings, gaps and monitor configuration
    /// (`ohlcvs.db` on Solana).
    Ohlcvs,
    /// RPC call statistics (`rpc_stats.db` on Solana).
    RpcStats,
}

impl DbKind {
    /// The file name this store uses on Solana — the historical names
    /// installed profiles already hold on disk, kept byte-for-byte so a
    /// Solana install never moves a file.
    pub const fn solana_file_name(self) -> &'static str {
        match self {
            Self::Tokens => "tokens.db",
            Self::Pools => "pools.db",
            Self::Ohlcvs => "ohlcvs.db",
            Self::RpcStats => "rpc_stats.db",
        }
    }

    /// The lowercase file stem shared by every chain's file of this kind.
    /// Solana's name is `<stem>.db`; a chain variant added later resolves to
    /// `<stem>-<chain>.db` in the arm [`chain_db_path`] forces for it.
    pub const fn file_stem(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::Pools => "pools",
            Self::Ohlcvs => "ohlcvs",
            Self::RpcStats => "rpc_stats",
        }
    }
}

/// The database file for a chain-scoped store on `chain`.
///
/// Solana keeps its historical un-suffixed file names; every chain variant
/// added later takes `<stem>-<chain>.db` beside them (e.g. a Base variant
/// resolves the pools store to `pools-base.db`), so one chain's store never
/// shares a file with another's. The `chain_id` columns inside each store
/// stay — SQL is identical across chains.
pub fn chain_db_path(kind: DbKind, chain: ChainId) -> PathBuf {
    let name = if chain.keeps_legacy_db_file_names() {
        kind.solana_file_name().to_owned()
    } else {
        format!("{}-{}.db", kind.file_stem(), chain.as_str())
    };
    get_data_directory().join(name)
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
