// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The inventory of every app-owned SQLite store: where it lives, how it is configured, what losing it costs and which rows survive a rebuild.

use std::path::PathBuf;

use crate::chains::ChainId;
use crate::paths::{self, chain_db_path, DbKind};

use super::configure::{self, DbConfig};

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

/// One app-owned store.
#[derive(Debug, Clone, Copy)]
pub struct StoreSpec {
    /// The store's name: the file stem it uses on Solana.
    pub id: &'static str,
    pub location: StoreLocation,
    pub config: DbConfig,
    pub class: StoreClass,
    pub salvage: &'static [SalvageRule],
}

impl StoreSpec {
    /// The store's file for `chain`; a shared store ignores the chain.
    pub fn path(&self, chain: ChainId) -> PathBuf {
        match self.location {
            StoreLocation::Shared(path) => path(),
            StoreLocation::PerChain(kind) => chain_db_path(kind, chain),
        }
    }
}

const fn rule(table: &'static str) -> SalvageRule {
    SalvageRule {
        table,
        columns: None,
        filter: None,
    }
}

const fn shared(
    id: &'static str,
    path: fn() -> PathBuf,
    config: DbConfig,
    class: StoreClass,
    salvage: &'static [SalvageRule],
) -> StoreSpec {
    StoreSpec {
        id,
        location: StoreLocation::Shared(path),
        config,
        class,
        salvage,
    }
}

const fn per_chain(
    kind: DbKind,
    config: DbConfig,
    class: StoreClass,
    salvage: &'static [SalvageRule],
) -> StoreSpec {
    StoreSpec {
        id: kind.file_stem(),
        location: StoreLocation::PerChain(kind),
        config,
        class,
        salvage,
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
);
pub const POSITIONS: StoreSpec = shared(
    "positions",
    paths::get_positions_db_path,
    configure::POSITIONS_DB,
    StoreClass::Ledger,
    &[],
);
pub const COPY_TRADING: StoreSpec = shared(
    "copy_trading",
    paths::get_copy_trading_db_path,
    configure::COPY_TRADING_DB,
    StoreClass::Ledger,
    &[],
);
pub const STRATEGIES: StoreSpec = shared(
    "strategies",
    paths::get_strategies_db_path,
    configure::STRATEGIES_DB,
    StoreClass::Ledger,
    &[rule("strategies"), rule("strategy_assignments")],
);
pub const AGENT_CONTROL: StoreSpec = shared(
    "agent_control",
    paths::get_agent_control_db_path,
    configure::AGENT_CONTROL_DB,
    StoreClass::Ledger,
    &[rule("audit")],
);
pub const AI: StoreSpec = shared(
    "ai",
    paths::get_ai_db_path,
    configure::AI_DB,
    StoreClass::Ledger,
    &[rule("ai_instructions")],
);
pub const AI_CHAT: StoreSpec = shared(
    "ai_chat",
    paths::get_ai_chat_db_path,
    configure::AI_CHAT_DB,
    StoreClass::Ledger,
    &[rule("ai_scheduled_tasks")],
);
pub const TRANSACTIONS: StoreSpec = shared(
    "transactions",
    paths::get_transactions_db_path,
    configure::TRANSACTIONS_DB,
    StoreClass::Derived,
    &[],
);
pub const WALLET: StoreSpec = shared(
    "wallet",
    paths::get_wallet_db_path,
    configure::WALLET_MONITOR_DB,
    StoreClass::Derived,
    &[],
);
pub const TOKENS: StoreSpec = per_chain(
    DbKind::Tokens,
    configure::TOKENS_DB,
    StoreClass::Cache,
    &[
        rule("token_favorites"),
        rule("blacklist"),
        rule("authority_reputation"),
    ],
);
pub const OHLCVS: StoreSpec = per_chain(
    DbKind::Ohlcvs,
    configure::OHLCVS_DB,
    StoreClass::Cache,
    &[OHLCV_MANUAL_MONITORS],
);
pub const POOLS: StoreSpec = per_chain(DbKind::Pools, configure::POOLS_DB, StoreClass::Cache, &[]);
pub const RPC_STATS: StoreSpec = per_chain(
    DbKind::RpcStats,
    configure::RPC_STATS_DB,
    StoreClass::Cache,
    &[],
);
pub const EVENTS: StoreSpec = shared(
    "events",
    paths::get_events_db_path,
    configure::EVENTS_WRITE_DB,
    StoreClass::Cache,
    &[],
);
pub const ACTIONS: StoreSpec = shared(
    "actions",
    paths::get_actions_db_path,
    configure::ACTIONS_WRITE_DB,
    StoreClass::Cache,
    &[],
);
pub const TOOLS: StoreSpec = shared(
    "tools",
    paths::get_tools_db_path,
    configure::TOOLS_DB,
    StoreClass::Cache,
    &[rule("tool_favorites"), rule("watched_tokens")],
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
