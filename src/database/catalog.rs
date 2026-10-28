// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The inventory of every app-owned SQLite store: where it lives, how it is configured, what losing it costs and which rows survive a rebuild.

use std::path::PathBuf;
use std::time::Duration;

use rusqlite::Connection;

use crate::chains::ChainId;
use crate::errors::DatabaseError;
use crate::paths::{self, chain_db_path, DbKind};

use super::configure::{self, DbConfig};
use super::schema::StoreSchema;
use KeepReason::{Aggregate, Dedupe, Ledger, Schema, UserDecision, Vault};

/// What losing a store costs, which decides how a failed open is recovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreClass {
    /// Key material that exists nowhere else. Never moved, recreated or deleted
    /// automatically.
    Vault,
    /// Authored or money-bearing records that cannot be re-derived in full.
    Ledger,
    /// Records re-derivable from the chain at the cost of a backfill.
    Derived,
    /// Re-fetchable or disposable data; only the salvage tables are worth keeping.
    Cache,
}

/// Where a store's file lives.
#[derive(Debug, Clone, Copy)]
pub enum StoreLocation {
    /// One file shared by every chain.
    Shared(fn() -> PathBuf),
    /// One file per chain.
    PerChain(DbKind),
}

/// Rows of one table copied out of a quarantined store into its rebuilt file.
#[derive(Debug, Clone, Copy)]
pub struct SalvageRule {
    pub table: &'static str,
    /// The columns copied; `None` copies every column both files share.
    pub columns: Option<&'static [&'static str]>,
    /// A `WHERE` condition on the quarantined rows; `None` copies them all.
    pub filter: Option<&'static str>,
}

/// How housekeeping may shrink a store's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compaction {
    /// Free pages are reclaimed with incremental vacuum, and a file not yet in
    /// incremental auto-vacuum mode is converted once while it is small enough to
    /// rewrite online.
    Incremental,
    /// Only the write-ahead log is checkpointed; the file is never rewritten,
    /// vacuumed or converted.
    CheckpointOnly,
}

/// Why housekeeping never deletes a table's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepReason {
    /// Key material that exists nowhere else.
    Vault,
    /// Money-bearing or authored records that cannot be re-derived in full.
    Ledger,
    /// Keys that stop finished work from being repeated, such as scanned signatures.
    Dedupe,
    /// A choice made in the app: a favorite, a blacklist entry, a saved setting.
    UserDecision,
    /// Totals that summarise rows no longer kept.
    Aggregate,
    /// The store's own bookkeeping: schema versions, wallet binding, cursors.
    Schema,
}

/// What housekeeping may do with the rows of one table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRule {
    /// Rows are never deleted by housekeeping.
    Keep {
        table: &'static str,
        reason: KeepReason,
    },
    /// The table's owner caps its rows as it writes them.
    BoundedByOwner { table: &'static str },
    /// Rebuildable rows the store's evictor deletes by value, least recently used
    /// first, when the app's usage exceeds its budget.
    Evicted { table: &'static str },
}

impl TableRule {
    pub const fn table(&self) -> &'static str {
        match *self {
            TableRule::Keep { table, .. }
            | TableRule::BoundedByOwner { table }
            | TableRule::Evicted { table } => table,
        }
    }
}

/// The retention rule of every table a store creates.
#[derive(Debug, Clone, Copy)]
pub struct StoreRetention {
    pub tables: &'static [TableRule],
}

impl StoreRetention {
    /// The rule declared for `table`, if any. The schema ledger every store carries is
    /// declared once here.
    pub fn rule(&self, table: &str) -> Option<TableRule> {
        if table == super::schema::ledger::TABLE {
            return Some(TableRule::Keep {
                table: super::schema::ledger::TABLE,
                reason: KeepReason::Schema,
            });
        }
        self.tables
            .iter()
            .copied()
            .find(|rule| rule.table() == table)
    }

    /// Whether the store's evictor deletes any of its rows.
    pub fn evicts(&self) -> bool {
        self.tables
            .iter()
            .any(|rule| matches!(rule, TableRule::Evicted { .. }))
    }
}

/// The metadata table of a store whose rows belong to one wallet, and the name the
/// startup wallet check reports for it.
#[derive(Debug, Clone, Copy)]
pub struct WalletScope {
    /// Table holding the key that records the store's wallet.
    pub metadata_table: &'static str,
    pub label: &'static str,
}

/// The connection pool the storage phase builds for a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolShape {
    pub max_size: u32,
    /// Connections kept open while idle; `None` keeps as many as were used.
    pub min_idle: Option<u32>,
    /// How long a caller waits for a free connection.
    pub connection_timeout: Duration,
}

impl PoolShape {
    const fn sized(max_size: u32, min_idle: Option<u32>) -> Self {
        Self {
            max_size,
            min_idle,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// A second pool on the same file whose connections only read, for a store whose
/// readers must not wait behind its writer.
#[derive(Debug, Clone, Copy)]
pub struct ReaderPool {
    pub config: DbConfig,
    pub shape: PoolShape,
}

/// A check a snapshot must pass before it may replace a damaged store, beyond
/// `quick_check`.
pub type SnapshotCheck = fn(&Connection) -> Result<bool, DatabaseError>;

/// One app-owned store.
#[derive(Debug, Clone, Copy)]
pub struct StoreSpec {
    /// The store's name: the file stem it uses on Solana.
    pub id: &'static str,
    pub location: StoreLocation,
    pub config: DbConfig,
    pub class: StoreClass,
    pub salvage: &'static [SalvageRule],
    pub compaction: Compaction,
    pub retention: StoreRetention,
    /// Set for a store that holds one wallet's history and is cleared when the main
    /// wallet changes.
    pub wallet_scope: Option<WalletScope>,
    /// The schema this core supports and the step that brings the store to it.
    pub schema: StoreSchema,
    pub pool: PoolShape,
    pub reader: Option<ReaderPool>,
    pub verify_snapshot: Option<SnapshotCheck>,
}

impl StoreSpec {
    /// The store's file for `chain`; a shared store ignores the chain.
    pub fn path(&self, chain: ChainId) -> PathBuf {
        match self.location {
            StoreLocation::Shared(path) => path(),
            StoreLocation::PerChain(kind) => chain_db_path(kind, chain),
        }
    }

    /// The store's file on every chain this build supports; a shared store has one.
    pub fn paths(&self) -> Vec<PathBuf> {
        match self.location {
            StoreLocation::Shared(path) => vec![path()],
            StoreLocation::PerChain(kind) => ChainId::ALL
                .iter()
                .map(|chain| chain_db_path(kind, *chain))
                .collect(),
        }
    }

    const fn checkpoint_only(mut self) -> Self {
        self.compaction = Compaction::CheckpointOnly;
        self
    }

    const fn wallet_scoped(mut self, metadata_table: &'static str, label: &'static str) -> Self {
        self.wallet_scope = Some(WalletScope {
            metadata_table,
            label,
        });
        self
    }

    const fn pooled(mut self, max_size: u32, min_idle: Option<u32>) -> Self {
        self.pool = PoolShape::sized(max_size, min_idle);
        self
    }

    const fn with_reader(mut self, config: DbConfig, max_size: u32, min_idle: Option<u32>) -> Self {
        self.reader = Some(ReaderPool {
            config,
            shape: PoolShape::sized(max_size, min_idle),
        });
        self
    }

    const fn verified_by(mut self, check: SnapshotCheck) -> Self {
        self.verify_snapshot = Some(check);
        self
    }
}

/// The stores whose rows belong to the main wallet.
pub fn wallet_scoped_stores() -> impl Iterator<Item = (&'static StoreSpec, WalletScope)> {
    STORES
        .into_iter()
        .filter_map(|store| store.wallet_scope.map(|scope| (store, scope)))
}

const fn rule(table: &'static str) -> SalvageRule {
    SalvageRule {
        table,
        columns: None,
        filter: None,
    }
}

const fn keep(table: &'static str, reason: KeepReason) -> TableRule {
    TableRule::Keep { table, reason }
}

const fn bounded(table: &'static str) -> TableRule {
    TableRule::BoundedByOwner { table }
}

const fn evicted(table: &'static str) -> TableRule {
    TableRule::Evicted { table }
}

const fn shared(
    id: &'static str,
    path: fn() -> PathBuf,
    config: DbConfig,
    class: StoreClass,
    schema: StoreSchema,
    salvage: &'static [SalvageRule],
    tables: &'static [TableRule],
) -> StoreSpec {
    StoreSpec {
        id,
        location: StoreLocation::Shared(path),
        config,
        class,
        salvage,
        compaction: Compaction::Incremental,
        retention: StoreRetention { tables },
        wallet_scope: None,
        schema,
        pool: PoolShape::sized(3, None),
        reader: None,
        verify_snapshot: None,
    }
}

const fn per_chain(
    kind: DbKind,
    config: DbConfig,
    class: StoreClass,
    schema: StoreSchema,
    salvage: &'static [SalvageRule],
    tables: &'static [TableRule],
) -> StoreSpec {
    StoreSpec {
        id: kind.file_stem(),
        location: StoreLocation::PerChain(kind),
        config,
        class,
        salvage,
        compaction: Compaction::Incremental,
        retention: StoreRetention { tables },
        wallet_scope: None,
        schema,
        pool: PoolShape::sized(3, None),
        reader: None,
        verify_snapshot: None,
    }
}

/// Manually monitored tokens survive a rebuild; backfill progress does not, so the
/// rebuilt store backfills them from scratch.
const OHLCV_MANUAL_MONITORS: SalvageRule = SalvageRule {
    table: "ohlcv_monitor_config",
    columns: Some(&[
        "chain_id",
        "mint",
        "priority",
        "fetch_interval_seconds",
        "source",
        "is_active",
        "last_activity",
        "created_at",
        "updated_at",
    ]),
    filter: Some("source = 'manual'"),
};

pub const WALLETS: StoreSpec = shared(
    "wallets",
    paths::get_wallets_db_path,
    configure::WALLETS_DB,
    StoreClass::Vault,
    crate::wallets::database::SCHEMA,
    &[],
    &[
        keep("wallets", Vault),
        keep("watch_targets", UserDecision),
        keep("watch_cursors", Dedupe),
    ],
)
.checkpoint_only()
.pooled(6, None)
.verified_by(crate::wallets::database::snapshot_keys_decrypt);
pub const POSITIONS: StoreSpec = shared(
    "positions",
    paths::get_positions_db_path,
    configure::POSITIONS_DB,
    StoreClass::Ledger,
    crate::positions::database::SCHEMA,
    &[],
    &[
        keep("positions", Ledger),
        keep("position_entries", Ledger),
        keep("position_exits", Ledger),
        keep("position_states", Ledger),
        keep("position_tracking", Ledger),
        keep("token_snapshots", Ledger),
        keep("position_metadata", Schema),
    ],
)
.wallet_scoped("position_metadata", "Positions")
.pooled(5, Some(1));
pub const PAPER_POSITIONS: StoreSpec = shared(
    "paper_positions",
    paths::get_paper_positions_db_path,
    configure::POSITIONS_DB,
    StoreClass::Ledger,
    crate::positions::database::PAPER_SCHEMA,
    &[],
    &[
        keep("positions", Ledger),
        keep("position_entries", Ledger),
        keep("position_exits", Ledger),
        keep("position_states", Ledger),
        keep("position_tracking", Ledger),
        keep("token_snapshots", Ledger),
        keep("paper_legs", Ledger),
        keep("paper_account", UserDecision),
        keep("paper_exit_attempts", Ledger),
        keep("position_metadata", Schema),
    ],
)
.wallet_scoped("position_metadata", "Paper positions")
.pooled(5, Some(1));
pub const COPY_TRADING: StoreSpec = shared(
    "copy_trading",
    paths::get_copy_trading_db_path,
    configure::COPY_TRADING_DB,
    StoreClass::Ledger,
    crate::trader::copy::database::SCHEMA,
    &[],
    &[
        keep("copy_tasks", UserDecision),
        keep("copy_activity", Ledger),
        keep("copy_decisions", Ledger),
        keep("copy_live_claims", Ledger),
        keep("copy_paper_legacy_positions", Ledger),
        keep("copy_paper_positions", Ledger),
        keep("copy_spend", Ledger),
        keep("copy_target_holdings", Ledger),
        keep("copy_target_events", Dedupe),
        keep("copy_metadata", Schema),
    ],
);
pub const STRATEGIES: StoreSpec = shared(
    "strategies",
    paths::get_strategies_db_path,
    configure::STRATEGIES_DB,
    StoreClass::Ledger,
    crate::strategies::database::SCHEMA,
    &[rule("strategies")],
    &[
        keep("strategies", UserDecision),
        keep("strategy_templates", UserDecision),
        keep("strategy_backtests", Ledger),
        bounded("strategy_performance"),
        keep("strategy_evaluation_totals", Aggregate),
        keep("schema_version", Schema),
    ],
);
pub const AGENT_CONTROL: StoreSpec = shared(
    "agent_control",
    paths::get_agent_control_db_path,
    configure::AGENT_CONTROL_DB,
    StoreClass::Ledger,
    crate::agent_control::store::SCHEMA,
    &[rule("audit")],
    &[
        keep("pairings", UserDecision),
        keep("approvals", UserDecision),
        bounded("audit"),
        bounded("submissions"),
        keep("schema_version", Schema),
    ],
);
pub const AI: StoreSpec = shared(
    "ai",
    paths::get_ai_db_path,
    configure::AI_DB,
    StoreClass::Ledger,
    crate::llm_analysis::database::SCHEMA,
    &[rule("ai_instructions")],
    &[keep("ai_instructions", UserDecision)],
);
pub const AI_CHAT: StoreSpec = shared(
    "ai_chat",
    paths::get_ai_chat_db_path,
    configure::AI_CHAT_DB,
    StoreClass::Ledger,
    crate::assistant::chat::database::SCHEMA,
    &[rule("ai_scheduled_tasks")],
    &[
        keep("ai_scheduled_tasks", UserDecision),
        bounded("chat_sessions"),
        bounded("chat_messages"),
        bounded("tool_executions"),
    ],
);
pub const TRANSACTIONS: StoreSpec = shared(
    "transactions",
    paths::get_transactions_db_path,
    configure::TRANSACTIONS_DB,
    StoreClass::Derived,
    crate::transactions::database::SCHEMA,
    &[],
    &[
        keep("raw_transactions", Ledger),
        keep("processed_transactions", Ledger),
        keep("subject_asset_deltas", Ledger),
        keep("pending_transactions", Ledger),
        keep("deferred_retries", Ledger),
        keep("known_signatures", Dedupe),
        keep("bootstrap_state", Schema),
        keep("db_metadata", Schema),
    ],
)
.wallet_scoped("db_metadata", "Transactions")
.pooled(5, None);
pub const WALLET: StoreSpec = shared(
    "wallet",
    paths::get_wallet_db_path,
    configure::WALLET_MONITOR_DB,
    StoreClass::Derived,
    crate::wallets::balance_monitor::SCHEMA,
    &[],
    &[
        bounded("token_balances"),
        bounded("nft_balances"),
        bounded("native_flow_cache"),
        keep("wallet_metadata", Schema),
    ],
)
.wallet_scoped("wallet_metadata", "Wallet History")
.pooled(3, Some(1));
pub const TOKENS: StoreSpec = per_chain(
    DbKind::Tokens,
    configure::TOKENS_DB,
    StoreClass::Cache,
    crate::tokens::schema::SCHEMA,
    &[
        rule("token_favorites"),
        rule("blacklist"),
        rule("authority_reputation"),
    ],
    &[
        evicted("tokens"),
        evicted("market_dexscreener"),
        evicted("market_geckoterminal"),
        evicted("security_rugcheck"),
        evicted("update_tracking"),
        evicted("token_pools"),
        evicted("token_media"),
        keep("token_favorites", UserDecision),
        keep("blacklist", UserDecision),
        keep("authority_reputation", Aggregate),
    ],
)
.pooled(8, None);
pub const OHLCVS: StoreSpec = per_chain(
    DbKind::Ohlcvs,
    configure::OHLCVS_DB,
    StoreClass::Cache,
    crate::ohlcvs::database::SCHEMA,
    &[OHLCV_MANUAL_MONITORS],
    &[
        evicted("ohlcv_monitor_config"),
        evicted("ohlcv_pools"),
        evicted("ohlcv_candles"),
        evicted("ohlcv_gaps"),
        keep("ohlcv_data_versions", Schema),
        keep("schema_migrations", Schema),
    ],
)
.pooled(4, None);
pub const POOLS: StoreSpec = per_chain(
    DbKind::Pools,
    configure::POOLS_DB,
    StoreClass::Cache,
    crate::pools::database::SCHEMA,
    &[],
    &[bounded("blacklist_accounts"), bounded("blacklist_pools")],
)
.pooled(4, None);
pub const RPC_STATS: StoreSpec = per_chain(
    DbKind::RpcStats,
    configure::RPC_STATS_DB,
    StoreClass::Cache,
    crate::rpc::stats::database::SCHEMA,
    &[],
    &[
        keep("providers", Aggregate),
        keep("provider_health", Aggregate),
        bounded("sessions"),
    ],
);
pub const EVENTS: StoreSpec = shared(
    "events",
    paths::get_events_db_path,
    configure::EVENTS_WRITE_DB,
    StoreClass::Cache,
    crate::events::database::SCHEMA,
    &[],
    &[],
)
.pooled(2, Some(1))
.with_reader(configure::EVENTS_READ_DB, 4, Some(1));
pub const ACTIONS: StoreSpec = shared(
    "actions",
    paths::get_actions_db_path,
    configure::ACTIONS_WRITE_DB,
    StoreClass::Cache,
    crate::actions::database::SCHEMA,
    &[],
    &[],
)
.pooled(2, Some(1))
.with_reader(configure::ACTIONS_READ_DB, 4, Some(1));
pub const ATA_CLEANUP: StoreSpec = shared(
    "tools",
    paths::get_ata_cleanup_db_path,
    configure::ATA_CLEANUP_DB,
    StoreClass::Cache,
    crate::wallets::ata_cleanup::database::SCHEMA,
    &[],
    &[],
)
.pooled(3, Some(1));

/// Every app-owned store, vault first and caches last.
pub const STORES: [&StoreSpec; 17] = [
    &WALLETS,
    &POSITIONS,
    &PAPER_POSITIONS,
    &COPY_TRADING,
    &STRATEGIES,
    &AGENT_CONTROL,
    &AI,
    &AI_CHAT,
    &TRANSACTIONS,
    &WALLET,
    &TOKENS,
    &OHLCVS,
    &POOLS,
    &RPC_STATS,
    &EVENTS,
    &ACTIONS,
    &ATA_CLEANUP,
];

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
