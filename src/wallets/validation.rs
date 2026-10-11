// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet validation — input validation for addresses, keypairs, and wallet names.

use std::path::PathBuf;

use crate::chains::ChainId;
use crate::database::catalog::{self, StoreSpec};
use crate::database::health::{self, StoreHealth};
use crate::database::recovery::RecoveryAction;
use crate::database::schema::ddl;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::utils::get_wallet_address;
use crate::wallets::balance_monitor::recorded_wallet_sql;
use crate::wallets::Error;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

/// One file of a wallet-scoped store: its catalog entry, the system name shown when the
/// wallet differs, the file and the table that records its wallet.
type WalletScopedFile = (&'static StoreSpec, &'static str, PathBuf, &'static str);

#[derive(Debug, Clone)]
pub enum WalletValidationResult {
    /// Wallet validation passed - current wallet matches stored data
    Valid,
    /// Wallet changed - data cleanup required
    Mismatch {
        current: String,
        stored: String,
        affected_systems: Vec<String>,
    },
    /// First run - no existing databases
    FirstRun,
}

pub struct WalletValidator;

impl WalletValidator {
    /// Check if wallet changed across all systems
    pub async fn validate_wallet_consistency() -> Result<WalletValidationResult, Error> {
        let current_wallet = get_wallet_address().map_err(|e| Error::Dependency {
            dependency: "config",
            detail: e.to_string(),
        })?;

        let databases: Vec<_> = catalog::wallet_scoped_stores()
            .flat_map(|(store, scope)| {
                store
                    .paths()
                    .into_iter()
                    .map(move |path| (store, scope.label, path, scope.metadata_table))
            })
            .collect();
        Ok(Self::validate_stored_wallets(current_wallet, &databases))
    }

    /// Compare the wallet each wallet-scoped store records with `current_wallet`. A store
    /// that cannot be read never stops the check: it is recorded as unavailable in the
    /// health registry and its own opener and services handle it. A store the registry
    /// already reports as not open keeps its recorded state and is not read.
    fn validate_stored_wallets(
        current_wallet: String,
        databases: &[WalletScopedFile],
    ) -> WalletValidationResult {
        let mut mismatches: Vec<(String, String)> = Vec::new();

        for (spec, system, path, metadata_table) in databases {
            if !path.exists() {
                continue;
            }
            if let Some(status) = health::status(path).filter(|status| !status.health.is_open()) {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "{system} wallet check skipped: {} is {}",
                        path.display(),
                        status.health.as_str()
                    ),
                );
                continue;
            }
            match Self::get_stored_wallet(&path.to_string_lossy(), metadata_table) {
                Ok(Some(stored_wallet)) if stored_wallet != current_wallet => {
                    mismatches.push(((*system).to_owned(), stored_wallet));
                }
                Ok(_) => {}
                Err(error) => {
                    logger::error(
                        LogTag::Wallet,
                        &format!(
                            "{system} wallet check could not read {}: {error}",
                            path.display()
                        ),
                    );
                    health::publish(
                        spec,
                        path,
                        StoreHealth::unavailable(&error, RecoveryAction::Unavailable),
                    );
                }
            }
        }

        if mismatches.is_empty() {
            if databases.iter().any(|(_, _, path, _)| path.exists()) {
                WalletValidationResult::Valid
            } else {
                WalletValidationResult::FirstRun
            }
        } else {
            let affected_systems = mismatches.iter().map(|(sys, _)| sys.clone()).collect();
            let stored = mismatches[0].1.clone();

            WalletValidationResult::Mismatch {
                current: current_wallet,
                stored,
                affected_systems,
            }
        }
    }

    /// Get stored wallet address from database metadata table: the chain-scoped key
    /// when the store has one, else the unscoped key. The store is opened read-only, so
    /// the check never creates or changes a file. The table name is a catalog
    /// constant, never input, so it is safe to splice into the statement.
    fn get_stored_wallet(
        db_path: &str,
        metadata_table: &'static str,
    ) -> Result<Option<String>, DatabaseError> {
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(DatabaseError::from)?;
        if !ddl::has_table(&conn, metadata_table)? {
            return Ok(None);
        }

        let query = format!(
            "SELECT {}",
            recorded_wallet_sql(metadata_table, ChainId::Solana)
        );
        let wallet: Option<String> = conn
            .query_row(&query, [], |row| row.get(0))
            .optional()
            .map_err(DatabaseError::from)?;

        Ok(wallet.filter(|w| !w.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{WalletValidationResult, WalletValidator};
    use crate::database::catalog::{self, StoreClass};
    use crate::database::health::{self, StoreHealth};
    use crate::database::recovery::FailureKind;

    const POSITIONS_METADATA_TABLE: &str = match catalog::POSITIONS.wallet_scope {
        Some(scope) => scope.metadata_table,
        None => panic!("positions is a wallet-scoped store"),
    };

    #[test]
    fn stored_wallet_treats_missing_metadata_table_as_first_run_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        Connection::open(&path).unwrap();

        assert_eq!(
            WalletValidator::get_stored_wallet(&path.to_string_lossy(), POSITIONS_METADATA_TABLE)
                .unwrap(),
            None
        );
    }

    #[test]
    fn stored_wallet_distinguishes_empty_matching_and_mismatching_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE position_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .unwrap();

        assert_eq!(
            WalletValidator::get_stored_wallet(&path.to_string_lossy(), POSITIONS_METADATA_TABLE)
                .unwrap(),
            None
        );

        connection
            .execute(
                "INSERT INTO position_metadata (key, value) VALUES ('current_wallet', ?1)",
                ["matching-wallet"],
            )
            .unwrap();
        assert_eq!(
            WalletValidator::get_stored_wallet(&path.to_string_lossy(), POSITIONS_METADATA_TABLE)
                .unwrap(),
            Some("matching-wallet".to_owned())
        );
        assert!(matches!(
            WalletValidator::validate_stored_wallets(
                "matching-wallet".to_owned(),
                &[(
                    &catalog::POSITIONS,
                    "Positions",
                    path.clone(),
                    POSITIONS_METADATA_TABLE
                )],
            ),
            WalletValidationResult::Valid
        ));

        connection
            .execute(
                "UPDATE position_metadata SET value = ?1 WHERE key = 'current_wallet'",
                ["mismatching-wallet"],
            )
            .unwrap();
        let validation = WalletValidator::validate_stored_wallets(
            "matching-wallet".to_owned(),
            &[(
                &catalog::POSITIONS,
                "Positions",
                path.clone(),
                POSITIONS_METADATA_TABLE,
            )],
        );
        assert!(matches!(
            validation,
            WalletValidationResult::Mismatch {
                current,
                stored,
                affected_systems,
            } if current == "matching-wallet"
                && stored == "mismatching-wallet"
                && affected_systems == ["Positions"]
        ));
    }

    /// `wallet.db` records its wallet under the chain-scoped key; the check reads that key
    /// before the unscoped one older builds wrote.
    #[test]
    fn a_changed_wallet_recorded_under_the_chain_scoped_key_is_detected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("wallet.db");
        Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE wallet_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO wallet_metadata (key, value) VALUES ('current_wallet:solana', 'scoped-wallet');
                 INSERT INTO wallet_metadata (key, value) VALUES ('current_wallet', 'unscoped-wallet');",
            )
            .unwrap();
        let wallet_scope = catalog::WALLET
            .wallet_scope
            .expect("wallet.db is a wallet-scoped store");

        assert_eq!(
            WalletValidator::get_stored_wallet(
                &path.to_string_lossy(),
                wallet_scope.metadata_table
            )
            .unwrap(),
            Some("scoped-wallet".to_owned())
        );
        let files = [(
            &catalog::WALLET,
            wallet_scope.label,
            path.clone(),
            wallet_scope.metadata_table,
        )];
        assert!(matches!(
            WalletValidator::validate_stored_wallets("scoped-wallet".to_owned(), &files),
            WalletValidationResult::Valid
        ));
        assert!(matches!(
            WalletValidator::validate_stored_wallets("new-wallet".to_owned(), &files),
            WalletValidationResult::Mismatch { stored, .. } if stored == "scoped-wallet"
        ));
    }

    #[test]
    fn stored_wallet_propagates_sqlite_errors_after_schema_is_present() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE position_metadata (key TEXT PRIMARY KEY);")
            .unwrap();

        assert!(WalletValidator::get_stored_wallet(
            &path.to_string_lossy(),
            POSITIONS_METADATA_TABLE
        )
        .is_err());
    }

    /// An unreadable positions store never stops the wallet check: it is published as
    /// unavailable and the check goes on to the readable stores.
    #[test]
    fn an_unreadable_wallet_scoped_store_is_published_unavailable_and_the_check_continues() {
        let directory = tempfile::tempdir().unwrap();
        let damaged = directory.path().join("positions.db");
        std::fs::write(&damaged, b"this file holds no SQLite header".repeat(128)).unwrap();
        let readable = directory.path().join("positions-readable.db");
        Connection::open(&readable)
            .unwrap()
            .execute_batch(
                "CREATE TABLE position_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO position_metadata (key, value) VALUES ('current_wallet', 'stored-wallet');",
            )
            .unwrap();

        let validation = WalletValidator::validate_stored_wallets(
            "current-wallet".to_owned(),
            &[
                (
                    &catalog::POSITIONS,
                    "Positions",
                    damaged.clone(),
                    POSITIONS_METADATA_TABLE,
                ),
                (
                    &catalog::POSITIONS,
                    "Positions",
                    readable.clone(),
                    POSITIONS_METADATA_TABLE,
                ),
            ],
        );
        assert!(matches!(
            validation,
            WalletValidationResult::Mismatch { stored, .. } if stored == "stored-wallet"
        ));
        let status = health::status(&damaged).expect("the unreadable store is published");
        assert!(matches!(
            status.health,
            StoreHealth::Unavailable {
                failure: FailureKind::NotADatabase,
                isolated: false,
                ..
            }
        ));
        assert!(health::status(&readable).is_none());
    }

    /// A store its opener already left unavailable keeps that state and is not read.
    #[test]
    fn a_store_already_unavailable_keeps_its_recorded_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        std::fs::write(&path, b"this file holds no SQLite header".repeat(128)).unwrap();
        let isolated = StoreHealth::unavailable(
            &crate::errors::DatabaseError::Corrupt {
                operation: "check".to_owned(),
                message: "damaged".to_owned(),
            },
            crate::database::recovery::RecoveryAction::Isolate,
        );
        health::publish(&catalog::POSITIONS, &path, isolated.clone());

        assert!(matches!(
            WalletValidator::validate_stored_wallets(
                "current-wallet".to_owned(),
                &[(
                    &catalog::POSITIONS,
                    "Positions",
                    path.clone(),
                    POSITIONS_METADATA_TABLE
                )],
            ),
            WalletValidationResult::Valid
        ));
        assert_eq!(health::status(&path).unwrap().health, isolated);
    }

    /// The vault is never a wallet-scoped store, so the wallet check can never absorb a
    /// failure of `wallets.db`: its open in the wallet manager still stops full mode.
    #[test]
    fn the_vault_is_outside_the_wallet_check() {
        assert!(catalog::wallet_scoped_stores()
            .all(|(store, _)| store.class != StoreClass::Vault && store.id != catalog::WALLETS.id));
        assert!(catalog::WALLETS.wallet_scope.is_none());
    }
}
