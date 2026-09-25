//! Wallet validation — input validation for addresses, keypairs, and wallet names.

use crate::errors::DatabaseError;
use crate::paths;
use crate::utils::get_wallet_address;
use crate::wallets::Error;
use rusqlite::{Connection, OptionalExtension};

const TRANSACTIONS_METADATA_TABLE: &str = "db_metadata";
const POSITIONS_METADATA_TABLE: &str = "position_metadata";
const WALLET_METADATA_TABLE: &str = "wallet_metadata";

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

        let databases = [
            (
                "Transactions",
                paths::get_transactions_db_path(),
                TRANSACTIONS_METADATA_TABLE,
            ),
            (
                "Positions",
                paths::get_positions_db_path(),
                POSITIONS_METADATA_TABLE,
            ),
            (
                "Wallet History",
                paths::get_wallet_db_path(),
                WALLET_METADATA_TABLE,
            ),
        ];
        Self::validate_stored_wallets(current_wallet, &databases)
    }

    fn validate_stored_wallets(
        current_wallet: String,
        databases: &[(&str, std::path::PathBuf, &str)],
    ) -> Result<WalletValidationResult, Error> {
        let mut mismatches: Vec<(String, String)> = Vec::new();

        for (system, path, metadata_table) in databases {
            if path.exists() {
                if let Some(stored_wallet) =
                    Self::get_stored_wallet(&path.to_string_lossy(), metadata_table)?
                {
                    if stored_wallet != current_wallet {
                        mismatches.push(((*system).to_owned(), stored_wallet));
                    }
                }
            }
        }

        if mismatches.is_empty() {
            if databases.iter().any(|(_, path, _)| path.exists()) {
                Ok(WalletValidationResult::Valid)
            } else {
                Ok(WalletValidationResult::FirstRun)
            }
        } else {
            let affected_systems = mismatches.iter().map(|(sys, _)| sys.clone()).collect();
            let stored = mismatches[0].1.clone();

            Ok(WalletValidationResult::Mismatch {
                current: current_wallet,
                stored,
                affected_systems,
            })
        }
    }

    /// Get stored wallet address from database metadata table
    fn get_stored_wallet(db_path: &str, metadata_table: &str) -> Result<Option<String>, Error> {
        let conn = Connection::open(db_path).map_err(DatabaseError::from)?;

        let (table_name, query) = match metadata_table {
            TRANSACTIONS_METADATA_TABLE => (
                TRANSACTIONS_METADATA_TABLE,
                "SELECT value FROM db_metadata WHERE key = 'current_wallet'",
            ),
            POSITIONS_METADATA_TABLE => (
                POSITIONS_METADATA_TABLE,
                "SELECT value FROM position_metadata WHERE key = 'current_wallet'",
            ),
            WALLET_METADATA_TABLE => (
                WALLET_METADATA_TABLE,
                "SELECT value FROM wallet_metadata WHERE key = 'current_wallet'",
            ),
            _ => unreachable!("wallet metadata table must be statically whitelisted"),
        };

        let table_exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table_name],
                |_| Ok(()),
            )
            .optional()
            .map_err(DatabaseError::from)?
            .is_some();
        if !table_exists {
            return Ok(None);
        }

        let wallet: Option<String> = conn
            .query_row(query, [], |row| row.get(0))
            .optional()
            .map_err(DatabaseError::from)?;

        Ok(wallet.filter(|w| !w.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{WalletValidationResult, WalletValidator, POSITIONS_METADATA_TABLE};

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
                &[("Positions", path.clone(), POSITIONS_METADATA_TABLE)],
            )
            .unwrap(),
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
            &[("Positions", path.clone(), POSITIONS_METADATA_TABLE)],
        )
        .unwrap();
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
}
