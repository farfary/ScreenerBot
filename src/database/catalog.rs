// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The inventory of every app-owned SQLite store: where it lives, how it is configured, what losing it costs and which rows survive a rebuild.

use std::path::PathBuf;

use crate::chains::ChainId;
use crate::paths::{self, chain_db_path, DbKind};

use super::configure::{self, DbConfig};
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
    /// The rule declared for `table`, if any.
    pub fn rule(&self, table: &str) -> Option<TableRule> {
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
    /// Table holding the `current_wallet` key.
    pub metadata_table: &'static str,
    pub label: &'static str,
}

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
    }
}

const fn per_chain(
    kind: DbKind,
    config: DbConfig,
    class: StoreClass,
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
    &[],
    &[
        keep("wallets", Vault),
        bounded("wallet_token_balances"),
        keep("watch_targets", UserDecision),
        keep("watch_cursors", Dedupe),
    ],
)
.checkpoint_only();
pub const POSITIONS: StoreSpec = shared(
    "positions",
    paths::get_positions_db_path,
    configure::POSITIONS_DB,
    StoreClass::Ledger,
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
.wallet_scoped("position_metadata", "Positions");
pub const COPY_TRADING: StoreSpec = shared(
    "copy_trading",
    paths::get_copy_trading_db_path,
    configure::COPY_TRADING_DB,
    StoreClass::Ledger,
    &[],
    &[
        keep("copy_tasks", UserDecision),
        keep("copy_activity", Ledger),
        keep("copy_decisions", Ledger),
        keep("copy_live_claims", Ledger),
        keep("copy_paper_positions", Ledger),
        keep("copy_position_links", Ledger),
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
    &[rule("strategies"), rule("strategy_assignments")],
    &[
        keep("strategies", UserDecision),
        keep("strategy_assignments", UserDecision),
        keep("strategy_templates", UserDecision),
        keep("strategy_backtests", Ledger),
        keep("strategy_performance", Ledger),
        keep("schema_version", Schema),
    ],
);
pub const AGENT_CONTROL: StoreSpec = shared(
    "agent_control",
    paths::get_agent_control_db_path,
    configure::AGENT_CONTROL_DB,
    StoreClass::Ledger,
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
    &[rule("ai_instructions")],
    &[keep("ai_instructions", UserDecision)],
);
pub const AI_CHAT: StoreSpec = shared(
    "ai_chat",
    paths::get_ai_chat_db_path,
    configure::AI_CHAT_DB,
    StoreClass::Ledger,
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
.wallet_scoped("db_metadata", "Transactions");
pub const WALLET: StoreSpec = shared(
    "wallet",
    paths::get_wallet_db_path,
    configure::WALLET_MONITOR_DB,
    StoreClass::Derived,
    &[],
    &[
        bounded("token_balances"),
        bounded("nft_balances"),
        bounded("native_flow_cache"),
        keep("wallet_metadata", Schema),
    ],
)
.wallet_scoped("wallet_metadata", "Wallet History");
pub const TOKENS: StoreSpec = per_chain(
    DbKind::Tokens,
    configure::TOKENS_DB,
    StoreClass::Cache,
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
);
pub const OHLCVS: StoreSpec = per_chain(
    DbKind::Ohlcvs,
    configure::OHLCVS_DB,
    StoreClass::Cache,
    &[OHLCV_MANUAL_MONITORS],
    &[
        evicted("ohlcv_monitor_config"),
        evicted("ohlcv_pools"),
        evicted("ohlcv_candles"),
        evicted("ohlcv_gaps"),
        keep("ohlcv_data_versions", Schema),
        keep("schema_migrations", Schema),
    ],
);
pub const POOLS: StoreSpec = per_chain(
    DbKind::Pools,
    configure::POOLS_DB,
    StoreClass::Cache,
    &[],
    &[bounded("blacklist_accounts"), bounded("blacklist_pools")],
);
pub const RPC_STATS: StoreSpec = per_chain(
    DbKind::RpcStats,
    configure::RPC_STATS_DB,
    StoreClass::Cache,
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
    &[],
    &[],
);
pub const ACTIONS: StoreSpec = shared(
    "actions",
    paths::get_actions_db_path,
    configure::ACTIONS_WRITE_DB,
    StoreClass::Cache,
    &[],
    &[],
);
pub const TOOLS: StoreSpec = shared(
    "tools",
    paths::get_tools_db_path,
    configure::TOOLS_DB,
    StoreClass::Cache,
    &[rule("tool_favorites"), rule("watched_tokens")],
    &[
        keep("tool_favorites", UserDecision),
        keep("watched_tokens", UserDecision),
        keep("ata_sessions", Ledger),
        keep("ata_closures", Ledger),
        keep("mw_sessions", Ledger),
        keep("mw_wallet_ops", Ledger),
        keep("schema_version", Schema),
    ],
);

/// Every app-owned store, vault first and caches last.
pub const STORES: [&StoreSpec; 16] = [
    &WALLETS,
    &POSITIONS,
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
    &TOOLS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_store_has_one_entry_and_one_file() {
        let ids: HashSet<&str> = STORES.iter().map(|store| store.id).collect();
        assert_eq!(ids.len(), STORES.len());
        let files: HashSet<PathBuf> = STORES
            .iter()
            .map(|store| store.path(ChainId::Solana))
            .collect();
        assert_eq!(files.len(), STORES.len());
        for store in STORES {
            let file = store.path(ChainId::Solana);
            assert_eq!(
                file.file_name().and_then(|name| name.to_str()),
                Some(format!("{}.db", store.id).as_str()),
                "{} names its Solana file",
                store.id
            );
        }
    }

    #[test]
    fn stores_are_ordered_by_class_and_only_the_vault_is_a_vault() {
        let rank = |class: StoreClass| match class {
            StoreClass::Vault => 0,
            StoreClass::Ledger => 1,
            StoreClass::Derived => 2,
            StoreClass::Cache => 3,
        };
        assert!(STORES
            .windows(2)
            .all(|pair| rank(pair[0].class) <= rank(pair[1].class)));
        let vaults: Vec<&str> = STORES
            .iter()
            .filter(|store| store.class == StoreClass::Vault)
            .map(|store| store.id)
            .collect();
        assert_eq!(vaults, vec!["wallets"]);
        let per_chain: Vec<&str> = STORES
            .iter()
            .filter(|store| matches!(store.location, StoreLocation::PerChain(_)))
            .map(|store| store.id)
            .collect();
        assert_eq!(per_chain, vec!["tokens", "ohlcvs", "pools", "rpc_stats"]);
    }

    /// Every store path function and every chain-scoped kind is a catalog entry, so no
    /// store can ship without a class, a compaction rule and a place in every derived list.
    #[test]
    fn every_store_path_function_and_kind_is_catalogued() {
        let catalog = include_str!("catalog.rs");
        let path_functions: Vec<&str> = include_str!("../paths/database.rs")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub fn "))
            .filter_map(|rest| rest.split_once("()"))
            .map(|(name, _)| name)
            .filter(|name| name.ends_with("_db_path"))
            .collect();
        assert!(!path_functions.is_empty());
        for name in path_functions {
            assert!(
                catalog.contains(&format!("paths::{name},")),
                "paths::{name} names a store the catalog does not list"
            );
        }
        let kind_entry = |kind: DbKind| match kind {
            DbKind::Tokens => &TOKENS,
            DbKind::Pools => &POOLS,
            DbKind::Ohlcvs => &OHLCVS,
            DbKind::RpcStats => &RPC_STATS,
        };
        for kind in [
            DbKind::Tokens,
            DbKind::Pools,
            DbKind::Ohlcvs,
            DbKind::RpcStats,
        ] {
            assert!(matches!(
                kind_entry(kind).location,
                StoreLocation::PerChain(entry) if entry == kind
            ));
            assert!(STORES.iter().any(|store| store.id == kind.file_stem()));
        }
    }

    /// The key vault is never rewritten; every other store may be compacted.
    #[test]
    fn only_the_vault_is_checkpoint_only() {
        for store in STORES {
            let expected = if store.class == StoreClass::Vault {
                Compaction::CheckpointOnly
            } else {
                Compaction::Incremental
            };
            assert_eq!(store.compaction, expected, "{}", store.id);
        }
    }

    /// The wallet-scoped stores are the wallet's own history, and each names a metadata
    /// table its schema creates.
    #[test]
    fn wallet_scoped_stores_name_their_metadata_tables() {
        let sources = [
            (&POSITIONS, include_str!("../positions/database/types.rs")),
            (
                &TRANSACTIONS,
                include_str!("../transactions/database/schema.rs"),
            ),
            (
                &WALLET,
                include_str!("../wallets/balance_monitor/database/schema.rs"),
            ),
        ];
        let scoped: Vec<&str> = wallet_scoped_stores().map(|(store, _)| store.id).collect();
        assert_eq!(scoped, vec!["positions", "transactions", "wallet"]);
        for (store, source) in sources {
            let scope = store.wallet_scope.expect("wallet-scoped store");
            assert!(
                source.contains(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (",
                    scope.metadata_table
                )),
                "{} names {}, which its schema does not create",
                store.id,
                scope.metadata_table
            );
        }
    }

    /// Salvage names tables by hand; each must be a table its store creates.
    #[test]
    fn salvage_rules_name_tables_their_store_creates() {
        let sources: [(&StoreSpec, &str); 7] = [
            (&STRATEGIES, include_str!("../strategies/database.rs")),
            (&AGENT_CONTROL, include_str!("../agent_control/store.rs")),
            (&AI, include_str!("../llm_analysis/database.rs")),
            (&AI_CHAT, include_str!("../assistant/scheduled/database.rs")),
            (&TOKENS, include_str!("../tokens/schema.rs")),
            (&OHLCVS, include_str!("../ohlcvs/database/mod.rs")),
            (&TOOLS, include_str!("../tools/database/schema.rs")),
        ];
        let salvaging: HashSet<&str> = STORES
            .iter()
            .filter(|store| !store.salvage.is_empty())
            .map(|store| store.id)
            .collect();
        assert_eq!(
            salvaging,
            sources.iter().map(|(store, _)| store.id).collect()
        );
        for (store, source) in sources {
            for rule in store.salvage {
                assert!(
                    source.contains(&format!("CREATE TABLE IF NOT EXISTS {} (", rule.table)),
                    "{} salvages {}, which its schema does not create",
                    store.id,
                    rule.table
                );
                for column in rule.columns.unwrap_or_default() {
                    assert!(
                        source.contains(&format!("{column} ")),
                        "{} salvages {}.{column}",
                        store.id,
                        rule.table
                    );
                }
            }
        }
    }
}
