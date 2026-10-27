// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests that the store catalog is complete and consistent with the stores' own schemas.

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
            &PAPER_POSITIONS,
            include_str!("../positions/database/types.rs"),
        ),
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
    assert_eq!(
        scoped,
        vec!["positions", "paper_positions", "transactions", "wallet"]
    );
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
    let sources: [(&StoreSpec, &str); 6] = [
        (&STRATEGIES, include_str!("../strategies/database.rs")),
        (&AGENT_CONTROL, include_str!("../agent_control/store.rs")),
        (&AI, include_str!("../llm_analysis/database.rs")),
        (&AI_CHAT, include_str!("../assistant/scheduled/database.rs")),
        (&TOKENS, include_str!("../tokens/schema.rs")),
        (&OHLCVS, include_str!("../ohlcvs/database/mod.rs")),
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
