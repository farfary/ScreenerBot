//! Transaction database schema migrations — ordered upgrade steps run by
//! `operations::TransactionDatabase::initialize_schema`.

use rusqlite::{params, Connection, OptionalExtension};

use crate::logger::{self, LogTag};
use crate::transactions::types::*;

use super::operations::TransactionDatabase;
use super::schema::*;
use crate::database::WriteTransaction;

impl TransactionDatabase {
    /// Apply schema migrations that are safe before chain identity exists.
    pub(super) fn apply_pre_chain_migrations(&self, conn: &mut Connection) -> Result<bool, String> {
        // Ensure processed_transactions has fee_sol column for MCP tools compatibility
        let mut has_fee_sol = false;
        let mut has_sol_delta = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(processed_transactions)")
            .map_err(|e| format!("Failed to inspect processed_transactions schema: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| format!("Failed to read processed_transactions schema: {e}"))?;
        for r in rows {
            let name = r.map_err(|e| format!("Failed to parse schema row: {e}"))?;
            if name.eq_ignore_ascii_case("fee_sol") {
                has_fee_sol = true;
            } else if name.eq_ignore_ascii_case("sol_delta") {
                has_sol_delta = true;
            }
        }
        drop(stmt);
        if !has_fee_sol {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN fee_sol REAL NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|e| format!("Failed to add fee_sol column: {e}"))?;
        }

        if !has_sol_delta {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN sol_delta REAL",
                [],
            )
            .map_err(|e| format!("Failed to add sol_delta column: {e}"))?;
        }

        Ok(!has_sol_delta)
    }

    /// Ensure the chain-scoped bootstrap row after the v7 table rebuild.
    pub(super) fn initialize_chain_bootstrap_state(
        &self,
        conn: &mut Connection,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (chain_id, id, full_history_completed) VALUES (?1, 1, 0)",
            params![self.chain.as_str()],
        )
        .map_err(|e| format!("Failed to initialize bootstrap_state row: {e}"))?;

        Ok(())
    }

    // =========================================================================
    // §7.1 MIGRATION: composite (signature, wallet_address) primary key
    // =========================================================================

    /// Rebuild `raw_transactions`, `processed_transactions`, `known_signatures`,
    /// `pending_transactions` and `deferred_retries` onto a composite
    /// `(signature, wallet_address)` primary key.
    ///
    /// Before this migration all five declared `signature TEXT PRIMARY KEY`, so one
    /// signature could hold exactly one subject's perspective. That is invisible while
    /// the own wallet is the only subject, and wrong the moment a watched wallet is
    /// recorded too: our wallet and a target appear in the same transaction whenever
    /// the target sends to us, or we and the target trade the same pool in one bundle,
    /// and the later write would silently replace the earlier row.
    ///
    /// Gated on the stored schema version (fast path: a no-op read once already at
    /// v5+) and, per table, on the table's actual on-disk shape (defensive: a fresh
    /// install's tables are already composite from `CREATE TABLE IF NOT EXISTS`, and a
    /// crash mid-migration leaves only the untouched tables needing another pass), so
    /// this is safe to call on every boot.
    pub(super) fn migrate_signature_wallet_tables(
        &self,
        conn: &mut Connection,
    ) -> Result<(), String> {
        let stored_version = Self::read_schema_version(conn)?;
        if stored_version.unwrap_or(0) >= 5 {
            return Ok(());
        }

        // Fresh databases are created directly in the composite shape. Do not make
        // their explicit-path/test constructor depend on process-global wallet
        // configuration merely to discover there is nothing to migrate.
        let signature_tables = [
            "raw_transactions",
            "processed_transactions",
            "known_signatures",
            "pending_transactions",
            "deferred_retries",
        ];
        if signature_tables
            .iter()
            .map(|table| Self::has_composite_signature_wallet_key(conn, table))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|composite| composite)
        {
            return Ok(());
        }

        let own_wallet_address = crate::utils::get_wallet_address().map_err(|e| {
            format!("Failed to resolve own wallet address for schema migration: {e}")
        })?;

        logger::info(
            LogTag::Transactions,
            "Migrating transactions schema to composite (signature, wallet_address) keys (v5)...",
        );

        // SQLite's own recommended procedure for a table rebuild that other tables
        // reference by foreign key: disable enforcement for the duration (it cannot be
        // toggled inside a transaction, so this happens before BEGIN) so an orphaned
        // processed_transactions row -- possible today, see `IntegrityReport` -- cannot
        // abort the whole migration; it is simply carried over as still-orphaned.
        conn.pragma_update(None, "foreign_keys", 0)
            .map_err(|e| format!("Failed to disable foreign_keys for migration: {e}"))?;

        let migration_result = (|| -> Result<(), String> {
            let tx = conn
                .write_tx()
                .map_err(|e| format!("Failed to begin v5 schema migration: {e}"))?;

            Self::rebuild_raw_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_processed_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_known_signatures(&tx, &own_wallet_address)?;
            Self::rebuild_pending_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_deferred_retries(&tx, &own_wallet_address)?;

            tx.commit()
                .map_err(|e| format!("Failed to commit v5 schema migration: {e}"))
        })();

        conn.pragma_update(None, "foreign_keys", 1)
            .map_err(|e| format!("Failed to re-enable foreign_keys after migration: {e}"))?;

        migration_result?;

        logger::info(
            LogTag::Transactions,
            "Transactions schema migration to v5 complete",
        );

        Ok(())
    }

    /// The stored `schema_version`, or `None` when `db_metadata` has no row for it yet
    /// (a database that has never finished `initialize_schema`, including a brand new
    /// install).
    fn read_schema_version(conn: &Connection) -> Result<Option<u32>, String> {
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM db_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to read schema_version: {e}"))?;

        raw.map(|v| {
            v.parse::<u32>()
                .map_err(|e| format!("Invalid stored schema_version '{v}': {e}"))
        })
        .transpose()
    }

    /// True when `table`'s primary key already spans more than one column. SQLite
    /// reports each PK column's 1-based position via `PRAGMA table_info`'s `pk` field,
    /// so counting columns with `pk > 0` tells composite apart from single-column.
    /// Also `false` when the table does not exist -- callers only reach this after the
    /// `CREATE TABLE IF NOT EXISTS` pass, so that case does not arise in practice, but
    /// treating it as "not yet composite" rather than erroring keeps the check total.
    pub(super) fn has_composite_signature_wallet_key(
        conn: &Connection,
        table: &str,
    ) -> Result<bool, String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| format!("Failed to inspect {table} schema: {e}"))?;

        let pk_columns = stmt
            .query_map([], |row| row.get::<_, i64>(5))
            .map_err(|e| format!("Failed to read {table} schema: {e}"))?
            .filter_map(|r| r.ok())
            .filter(|&pk| pk > 0)
            .count();

        Ok(pk_columns >= 2)
    }

    /// Rebuilds every chain-owned transaction table into the v7 key shape. Legacy
    /// rows are Solana rows by definition; copying is transactional and is verified
    /// before the schema version advances so a crash leaves the prior database intact.
    pub(super) fn migrate_chain_identity_tables(
        &self,
        conn: &mut Connection,
    ) -> Result<(), String> {
        let stored_version = Self::read_schema_version(conn)?;
        if stored_version.unwrap_or(0) >= 7 {
            return Ok(());
        }
        let has_chain = |table: &str| -> Result<bool, String> {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .map_err(|e| format!("Failed to inspect {table}: {e}"))?;
            let columns = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("Failed to inspect {table}: {e}"))?;
            let has_chain = columns
                .filter_map(Result::ok)
                .any(|name| name == "chain_id");
            Ok(has_chain)
        };
        if has_chain("raw_transactions")?
            && has_chain("processed_transactions")?
            && has_chain("known_signatures")?
            && has_chain("deferred_retries")?
            && has_chain("pending_transactions")?
            && has_chain("bootstrap_state")?
            && has_chain("subject_asset_deltas")?
        {
            return Ok(());
        }

        conn.execute("PRAGMA foreign_keys = OFF", [])
            .map_err(|e| format!("Failed to disable foreign keys for v7 migration: {e}"))?;
        let result = (|| -> Result<(), String> {
            let tx = conn
                .write_tx()
                .map_err(|e| format!("Failed to begin v7 chain identity migration: {e}"))?;
            let tables = [
                ("raw_transactions", SCHEMA_RAW_TRANSACTIONS, "chain_id, signature, wallet_address, slot, block_time, timestamp, status, success, error_message, fee_lamports, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, created_at, updated_at"),
                ("processed_transactions", SCHEMA_PROCESSED_TRANSACTIONS, "chain_id, signature, wallet_address, transaction_type, direction, sol_balance_change, token_balance_changes, token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info, analysis_duration_ms, cached_analysis, analysis_version, fee_sol, sol_delta, processed_at, updated_at"),
                ("known_signatures", SCHEMA_KNOWN_SIGNATURES, "chain_id, signature, wallet_address, status, added_at"),
                ("deferred_retries", SCHEMA_DEFERRED_RETRIES, "chain_id, signature, wallet_address, next_retry_at, remaining_attempts, current_delay_secs, last_error, created_at, updated_at"),
                ("pending_transactions", SCHEMA_PENDING_TRANSACTIONS, "chain_id, signature, wallet_address, added_at, last_checked_at, check_count"),
                ("bootstrap_state", SCHEMA_BOOTSTRAP_STATE, "chain_id, id, backfill_before_cursor, full_history_completed, updated_at"),
                ("subject_asset_deltas", SCHEMA_SUBJECT_ASSET_DELTAS, "chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success"),
            ];
            for (table, schema, columns) in tables {
                let create = schema.replacen(
                    &format!("CREATE TABLE IF NOT EXISTS {table} ("),
                    &format!("CREATE TABLE {table}__v7 ("),
                    1,
                );
                tx.execute(&create, [])
                    .map_err(|e| format!("Failed to create {table}__v7: {e}"))?;
                let legacy_columns = columns.strip_prefix("chain_id, ").unwrap_or(columns);
                tx.execute(
                    &format!("INSERT INTO {table}__v7 ({columns}) SELECT 'solana', {legacy_columns} FROM {table}"),
                    [],
                ).map_err(|e| format!("Failed to copy {table} into v7: {e}"))?;
                let before: i64 = tx
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .map_err(|e| format!("Failed to count {table}: {e}"))?;
                let after: i64 = tx
                    .query_row(&format!("SELECT COUNT(*) FROM {table}__v7"), [], |row| {
                        row.get(0)
                    })
                    .map_err(|e| format!("Failed to count {table}__v7: {e}"))?;
                if before != after {
                    return Err(format!(
                        "v7 migration row count mismatch for {table}: {before} != {after}"
                    ));
                }
                tx.execute(&format!("DROP TABLE {table}"), [])
                    .map_err(|e| format!("Failed to drop {table}: {e}"))?;
                tx.execute(&format!("ALTER TABLE {table}__v7 RENAME TO {table}"), [])
                    .map_err(|e| format!("Failed to rename {table}__v7: {e}"))?;
            }
            let fk_errors: i64 = tx
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get(0)
                })
                .map_err(|e| format!("Failed to run foreign_key_check for v7 migration: {e}"))?;
            if fk_errors != 0 {
                return Err(format!(
                    "v7 migration foreign key check found {fk_errors} errors"
                ));
            }
            tx.commit()
                .map_err(|e| format!("Failed to commit v7 chain identity migration: {e}"))
        })();
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| format!("Failed to re-enable foreign keys after v7 migration: {e}"))?;
        result
    }

    pub(super) fn backfill_processed_sol_delta(&self, conn: &mut Connection) -> Result<(), String> {
        const BATCH_SIZE: i64 = 1000;
        let mut total_updated = 0usize;

        // Get wallet address for filtering (this is a migration function, so it operates on current wallet data only)
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address for sol_delta backfill: {e}"))?;

        loop {
            let mut stmt = conn
                .prepare(
                    "SELECT signature, sol_balance_change FROM processed_transactions WHERE chain_id = ?1 AND wallet_address = ?2 AND sol_delta IS NULL LIMIT ?3",
                )
                .map_err(|e| format!("Failed to prepare sol_delta backfill query: {e}"))?;

            let rows = stmt
                .query_map(
                    params![self.chain.as_str(), wallet_address, BATCH_SIZE],
                    |row| {
                        let signature: String = row.get(0)?;
                        let change_json: Option<String> = row.get(1)?;
                        Ok((signature, change_json))
                    },
                )
                .map_err(|e| format!("Failed to iterate sol_delta backfill rows: {e}"))?;

            let mut batch: Vec<(String, Option<String>)> = Vec::new();
            for row in rows {
                let (signature, change_json) =
                    row.map_err(|e| format!("Failed to read sol_delta row: {e}"))?;
                batch.push((signature, change_json));
            }

            if batch.is_empty() {
                break;
            }

            drop(stmt);

            let tx = conn
                .write_tx()
                .map_err(|e| format!("Failed to start sol_delta backfill transaction: {e}"))?;

            for (signature, change_json) in batch.into_iter() {
                let delta = Self::compute_sol_delta_from_json(change_json.as_deref());
                tx.execute(
                    "UPDATE processed_transactions SET sol_delta = ?1 WHERE chain_id = ?2 AND signature = ?3 AND wallet_address = ?4",
                    params![delta, self.chain.as_str(), signature, wallet_address],
                )
                .map_err(|e| format!("Failed to update sol_delta: {e}"))?;
                total_updated += 1;
            }

            tx.commit()
                .map_err(|e| format!("Failed to commit sol_delta backfill: {e}"))?;
        }

        if total_updated > 0 {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Backfilled sol_delta for {} processed transactions",
                    total_updated
                ),
            );
        }

        Ok(())
    }

    fn compute_sol_delta_from_json(payload: Option<&str>) -> f64 {
        let Some(raw) = payload else {
            return 0.0;
        };

        if raw.trim().is_empty() {
            return 0.0;
        }

        match serde_json::from_str::<Vec<SolBalanceChange>>(raw) {
            Ok(changes) => changes.iter().map(|change| change.change).sum(),
            Err(err) => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to parse sol_balance_change payload: {err}"),
                );
                0.0
            }
        }
    }
}
