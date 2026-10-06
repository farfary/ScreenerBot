// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction database schema migrations — ordered upgrade steps run by
//! `operations::TransactionDatabase::initialize_schema`.

use rusqlite::{params, Connection, OptionalExtension};

use crate::logger::{self, LogTag};
use crate::transactions::types::*;

use super::operations::TransactionDatabase;
use super::schema::*;
use crate::transactions::error::Error;

// The released v7 table definition is retained to reject unsupported schema
// changes before a rebuild can discard them.
const V7_SUBJECT_DELTAS_DDL: &str = r#"CREATE TABLE IF NOT EXISTS subject_asset_deltas (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    wallet_address TEXT NOT NULL,
    signature TEXT NOT NULL,
    mint TEXT NOT NULL,          -- SPL mint, or the literal 'native' for native SOL
    slot INTEGER,
    block_time INTEGER,
    tx_index INTEGER NOT NULL DEFAULT 0,
    delta_raw INTEGER NOT NULL,  -- signed, raw base units
    before_raw INTEGER,          -- NULL when not knowable
    after_raw INTEGER,
    decimals INTEGER NOT NULL,
    kind TEXT NOT NULL,          -- 'trade' | 'transfer' | 'defi' | 'other'
    venue TEXT,                  -- router name when a known DEX program is present
    fee_lamports INTEGER,
    success BOOLEAN NOT NULL DEFAULT 1,
    PRIMARY KEY (chain_id, wallet_address, signature, mint)
);"#;

impl TransactionDatabase {
    /// Rebuild only the released v7 ledger shape; never discard an unrecognized
    /// column, constraint, index, or trigger during a table replacement. Runs inside
    /// the opener's transaction.
    pub(super) fn migrate_subject_delta_amounts(&self, tx: &Connection) -> Result<(), Error> {
        let conn = tx;
        let mut stmt = conn
            .prepare("PRAGMA table_xinfo(subject_asset_deltas)")
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?;
        let columns = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?;
        drop(stmt);
        let types = ["delta_raw", "before_raw", "after_raw"].map(|name| {
            columns
                .iter()
                .find(|(column, ..)| column == name)
                .map(|(_, ty, ..)| ty.as_str())
        });
        if types == [Some("TEXT"), Some("TEXT"), Some("TEXT")] {
            return Ok(());
        }
        if types != [Some("INTEGER"), Some("INTEGER"), Some("INTEGER")] {
            return Err(Error::Migration {
                step: "inspect v8 subject delta amounts".to_owned(),
                detail: format!("unexpected raw amount column types: {types:?}"),
            });
        }

        let expected = [
            ("chain_id", "TEXT", 1, Some("'solana'"), 1),
            ("wallet_address", "TEXT", 1, None, 2),
            ("signature", "TEXT", 1, None, 3),
            ("mint", "TEXT", 1, None, 4),
            ("slot", "INTEGER", 0, None, 0),
            ("block_time", "INTEGER", 0, None, 0),
            ("tx_index", "INTEGER", 1, Some("0"), 0),
            ("delta_raw", "INTEGER", 1, None, 0),
            ("before_raw", "INTEGER", 0, None, 0),
            ("after_raw", "INTEGER", 0, None, 0),
            ("decimals", "INTEGER", 1, None, 0),
            ("kind", "TEXT", 1, None, 0),
            ("venue", "TEXT", 0, None, 0),
            ("fee_lamports", "INTEGER", 0, None, 0),
            ("success", "BOOLEAN", 1, Some("1"), 0),
        ];
        if columns.len() != expected.len()
            || expected.iter().any(|&(name, ty, not_null, default, pk)| {
                !columns.iter().any(
                    |(
                        actual_name,
                        actual_ty,
                        actual_not_null,
                        actual_default,
                        actual_pk,
                        hidden,
                    )| {
                        actual_name == name
                            && actual_ty == ty
                            && *actual_not_null == not_null
                            && actual_default.as_deref() == default
                            && *actual_pk == pk
                            && *hidden == 0
                    },
                )
            })
        {
            return Err(Error::Migration {
                step: "inspect v8 subject delta table".to_owned(),
                detail: "unexpected v7 column, constraint, or primary-key shape".to_owned(),
            });
        }

        let stored_ddl: String = conn
            .query_row("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'subject_asset_deltas'", [], |row| row.get(0))
            .map_err(|e| Error::SchemaInspect { detail: e.to_string() })?;
        fn ddl_body(ddl: &str) -> Option<&str> {
            ddl.find('(')
                .map(|start| ddl[start..].trim().trim_end_matches(';').trim())
        }
        if ddl_body(&stored_ddl) != ddl_body(V7_SUBJECT_DELTAS_DDL) {
            return Err(Error::Migration {
                step: "inspect v8 subject delta table".to_owned(),
                detail: "unrecognized v7 table definition would be lost by rebuild".to_owned(),
            });
        }

        let indexes = conn
            .prepare("SELECT name, origin FROM pragma_index_list('subject_asset_deltas')")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?;
        for (name, origin) in indexes {
            if !((origin == "pk" && name.starts_with("sqlite_autoindex_subject_asset_deltas_"))
                || (origin == "c"
                    && (name == "idx_subject_deltas_chain_wallet_mint"
                        || name == "idx_subject_deltas_chain_wallet_order")))
            {
                return Err(Error::Migration {
                    step: "inspect v8 subject delta indexes".to_owned(),
                    detail: format!("unrecognized index {name} would be lost by table rebuild"),
                });
            }
        }

        let mut stmt = conn
            .prepare("SELECT type, name, sql FROM sqlite_master WHERE tbl_name = 'subject_asset_deltas' AND type IN ('index', 'trigger') AND sql IS NOT NULL")
            .map_err(|e| Error::SchemaInspect { detail: e.to_string() })?;
        let objects = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| Error::SchemaInspect {
                detail: e.to_string(),
            })?;
        for (kind, name, sql) in objects {
            let canonical = INDEXES.iter().find(|index| {
                index.contains(&format!(
                    "INDEX IF NOT EXISTS {name} ON subject_asset_deltas("
                ))
            });
            if kind != "index"
                || canonical.is_none_or(|index| {
                    index.trim_end_matches(';') != sql
                        && index.replace(" IF NOT EXISTS", "").trim_end_matches(';') != sql
                })
            {
                return Err(Error::Migration {
                    step: "inspect v8 subject delta indexes".to_owned(),
                    detail: format!("unrecognized {kind} {name} would be lost by table rebuild"),
                });
            }
        }
        drop(stmt);

        let invalid: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM subject_asset_deltas WHERE typeof(delta_raw) != 'integer' OR (before_raw IS NOT NULL AND (typeof(before_raw) != 'integer' OR before_raw < 0)) OR (after_raw IS NOT NULL AND (typeof(after_raw) != 'integer' OR after_raw < 0))",
                [],
                |row| row.get(0),
            )
            .map_err(|e| Error::Migration { step: "validate v7 subject delta values".to_owned(), detail: e.to_string() })?;
        if invalid != 0 {
            return Err(Error::Migration {
                step: "validate v7 subject delta values".to_owned(),
                detail: format!("{invalid} rows have invalid raw amount storage"),
            });
        }

        let create = SCHEMA_SUBJECT_ASSET_DELTAS.replacen(
            "CREATE TABLE IF NOT EXISTS subject_asset_deltas (",
            "CREATE TABLE subject_asset_deltas__v8 (",
            1,
        );
        tx.execute(&create, []).map_err(|e| Error::Migration {
            step: "create subject_asset_deltas__v8".to_owned(),
            detail: e.to_string(),
        })?;
        tx.execute(
            "INSERT INTO subject_asset_deltas__v8 (chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_raw, success)
             SELECT chain_id, wallet_address, signature, mint, slot, block_time, tx_index,
                    CAST(delta_raw AS TEXT), CAST(before_raw AS TEXT), CAST(after_raw AS TEXT),
                    decimals, kind, venue, fee_lamports, success FROM subject_asset_deltas",
            [],
        )
        .map_err(|e| Error::Migration { step: "copy v8 subject deltas".to_owned(), detail: e.to_string() })?;
        let before: i64 = tx
            .query_row("SELECT COUNT(*) FROM subject_asset_deltas", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::Migration {
                step: "count v7 subject deltas".to_owned(),
                detail: e.to_string(),
            })?;
        let after: i64 = tx
            .query_row("SELECT COUNT(*) FROM subject_asset_deltas__v8", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::Migration {
                step: "count v8 subject deltas".to_owned(),
                detail: e.to_string(),
            })?;
        if before != after {
            return Err(Error::Migration {
                step: "verify v8 subject deltas".to_owned(),
                detail: format!("row count mismatch: {before} != {after}"),
            });
        }
        tx.execute("DROP TABLE subject_asset_deltas", [])
            .map_err(|e| Error::Migration {
                step: "drop v7 subject deltas".to_owned(),
                detail: e.to_string(),
            })?;
        tx.execute(
            "ALTER TABLE subject_asset_deltas__v8 RENAME TO subject_asset_deltas",
            [],
        )
        .map_err(|e| Error::Migration {
            step: "rename v8 subject deltas".to_owned(),
            detail: e.to_string(),
        })?;
        Ok(())
    }

    /// Apply schema migrations that are safe before chain identity exists.
    ///
    /// The fee and delta columns may still carry their legacy unit names here (the
    /// rename runs after the versioned rebuilds); either name counts as present, and
    /// a missing column is added under its canonical name.
    pub(super) fn apply_pre_chain_migrations(&self, conn: &Connection) -> Result<bool, Error> {
        // Ensure processed_transactions has the fee column for MCP tools compatibility
        let mut has_fee_native = false;
        let mut has_native_delta = false;
        let mut has_type_kind = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(processed_transactions)")
            .map_err(|e| Error::SchemaInspect {
                detail: format!("failed to inspect processed_transactions schema: {e}"),
            })?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| Error::SchemaInspect {
                detail: format!("failed to read processed_transactions schema: {e}"),
            })?;
        for r in rows {
            let name = r.map_err(|e| Error::SchemaInspect {
                detail: format!("failed to parse schema row: {e}"),
            })?;
            if name.eq_ignore_ascii_case("fee_sol") || name.eq_ignore_ascii_case("fee_native") {
                has_fee_native = true;
            } else if name.eq_ignore_ascii_case("sol_delta")
                || name.eq_ignore_ascii_case("native_delta")
            {
                has_native_delta = true;
            } else if name.eq_ignore_ascii_case("type_kind") {
                has_type_kind = true;
            }
        }
        drop(stmt);
        // A table missing these columns predates the historical rebuilds that
        // follow, which read the legacy names; the unit rename at the end of the
        // open moves them to the canonical names.
        if !has_fee_native {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN fee_sol REAL NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|e| Error::Migration {
                step: "add fee_sol column".to_owned(),
                detail: e.to_string(),
            })?;
        }

        if !has_native_delta {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN sol_delta REAL",
                [],
            )
            .map_err(|e| Error::Migration {
                step: "add sol_delta column".to_owned(),
                detail: e.to_string(),
            })?;
        }

        if !has_type_kind {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN type_kind TEXT NOT NULL DEFAULT 'unknown'",
                [],
            )
            .map_err(|e| Error::Migration {
                step: "add type_kind column".to_owned(),
                detail: e.to_string(),
            })?;
            // Existing rows hold a `Debug` rendering in `transaction_type`; map what
            // can be mapped so the list stays filterable before the reclassification
            // sweep re-derives every row from its cached raw transaction.
            Self::backfill_type_kind(conn)?;
        }

        Ok(!has_native_delta)
    }

    /// Seed `type_kind` for rows written before the column existed.
    fn backfill_type_kind(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "UPDATE processed_transactions SET type_kind = CASE \
                WHEN transaction_type LIKE 'Buy%' OR transaction_type LIKE 'SwapSolToToken%' THEN 'buy' \
                WHEN transaction_type LIKE 'Sell%' OR transaction_type LIKE 'SwapTokenToSol%' THEN 'sell' \
                WHEN transaction_type LIKE 'SwapTokenToToken%' THEN 'swap' \
                WHEN transaction_type LIKE 'SolTransfer%' THEN 'sol_transfer' \
                WHEN transaction_type LIKE 'TokenTransfer%' THEN 'token_transfer' \
                WHEN transaction_type LIKE 'Transfer%' THEN 'transfer' \
                WHEN transaction_type LIKE 'AtaClose%' THEN 'ata_close' \
                WHEN transaction_type LIKE 'AtaCreate%' THEN 'ata_create' \
                WHEN transaction_type LIKE 'AtaOperation%' THEN 'ata' \
                WHEN transaction_type LIKE 'Compute%' THEN 'compute' \
                WHEN transaction_type LIKE 'Failed%' THEN 'failed' \
                ELSE 'unknown' END",
            [],
        )
        .map_err(|e| Error::Migration {
            step: "backfill type_kind".to_owned(),
            detail: e.to_string(),
        })?;
        Ok(())
    }

    /// Ensure the chain-scoped bootstrap row after the v7 table rebuild.
    pub(super) fn initialize_chain_bootstrap_state(&self, conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (chain_id, id, full_history_completed) VALUES (?1, 1, 0)",
            params![self.chain.as_str()],
        )
        .map_err(|e| Error::Migration {
            step: "initialize bootstrap_state row".to_owned(),
            detail: e.to_string(),
        })?;

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
        tx: &rusqlite::Transaction<'_>,
    ) -> Result<(), Error> {
        let conn: &Connection = tx;
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

        let own_wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::Migration {
                step: "resolve own wallet address for schema migration".to_owned(),
                detail: e.to_string(),
            })?;

        logger::info(
            LogTag::Transactions,
            "Migrating transactions schema to composite (signature, wallet_address) keys (v5)...",
        );

        // SQLite's own recommended procedure for a table rebuild that other tables
        // reference by foreign key: enforcement is disabled for the duration (the
        // opener does so before BEGIN, since it cannot be toggled inside a
        // transaction) so an orphaned processed_transactions row -- possible today,
        // see `IntegrityReport` -- cannot abort the whole migration; it is simply
        // carried over as still-orphaned.
        Self::rebuild_raw_transactions(tx, &own_wallet_address)?;
        Self::rebuild_processed_transactions(tx, &own_wallet_address)?;
        Self::rebuild_known_signatures(tx, &own_wallet_address)?;
        Self::rebuild_pending_transactions(tx, &own_wallet_address)?;
        Self::rebuild_deferred_retries(tx, &own_wallet_address)?;

        logger::info(
            LogTag::Transactions,
            "Transactions schema migration to v5 complete",
        );

        Ok(())
    }

    /// The stored `schema_version`, or `None` when `db_metadata` has no row for it yet
    /// (a database that has never finished `initialize_schema`, including a brand new
    /// install).
    fn read_schema_version(conn: &Connection) -> Result<Option<u32>, Error> {
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM db_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Error::SchemaInspect {
                detail: format!("failed to read schema_version: {e}"),
            })?;

        raw.map(|v| {
            v.parse::<u32>().map_err(|e| Error::Migration {
                step: "parse schema_version".to_owned(),
                detail: format!("invalid stored schema_version '{v}': {e}"),
            })
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
    ) -> Result<bool, Error> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| Error::SchemaInspect {
                detail: format!("failed to inspect {table} schema: {e}"),
            })?;

        let pk_columns = stmt
            .query_map([], |row| row.get::<_, i64>(5))
            .map_err(|e| Error::SchemaInspect {
                detail: format!("failed to read {table} schema: {e}"),
            })?
            .filter_map(|r| r.ok())
            .filter(|&pk| pk > 0)
            .count();

        Ok(pk_columns >= 2)
    }

    /// Rebuilds every chain-owned transaction table into the v7 key shape. Legacy
    /// rows are Solana rows by definition; copying is transactional and is verified
    /// before the schema version advances so a crash leaves the prior database intact.
    /// Runs inside the opener's transaction, with foreign-key enforcement already off.
    ///
    /// Each table is rebuilt into its canonical definition from the legacy columns it
    /// was released with, so the unit columns are read under their legacy names and
    /// written under their canonical ones. A table created earlier in this open
    /// already has the canonical names and is read under those.
    pub(super) fn migrate_chain_identity_tables(&self, tx: &Connection) -> Result<(), Error> {
        let conn = tx;
        let stored_version = Self::read_schema_version(conn)?;
        if stored_version.unwrap_or(0) >= 7 {
            return Ok(());
        }
        let has_chain = |table: &str| -> Result<bool, Error> {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .map_err(|e| Error::SchemaInspect {
                    detail: format!("failed to inspect {table}: {e}"),
                })?;
            let columns = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| Error::SchemaInspect {
                    detail: format!("failed to inspect {table}: {e}"),
                })?;
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

        let tables = [
            ("raw_transactions", SCHEMA_RAW_TRANSACTIONS, "chain_id, signature, wallet_address, slot, block_time, timestamp, status, success, error_message, fee_raw, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, created_at, updated_at", "signature, wallet_address, slot, block_time, timestamp, status, success, error_message, fee_lamports, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, created_at, updated_at"),
            ("processed_transactions", SCHEMA_PROCESSED_TRANSACTIONS, "chain_id, signature, wallet_address, transaction_type, type_kind, direction, native_balance_change, token_balance_changes, token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info, analysis_duration_ms, cached_analysis, analysis_version, fee_native, native_delta, processed_at, updated_at", "signature, wallet_address, transaction_type, type_kind, direction, sol_balance_change, token_balance_changes, token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info, analysis_duration_ms, cached_analysis, analysis_version, fee_sol, sol_delta, processed_at, updated_at"),
            ("known_signatures", SCHEMA_KNOWN_SIGNATURES, "chain_id, signature, wallet_address, status, added_at", "signature, wallet_address, status, added_at"),
            ("deferred_retries", SCHEMA_DEFERRED_RETRIES, "chain_id, signature, wallet_address, next_retry_at, remaining_attempts, current_delay_secs, last_error, created_at, updated_at", "signature, wallet_address, next_retry_at, remaining_attempts, current_delay_secs, last_error, created_at, updated_at"),
            ("pending_transactions", SCHEMA_PENDING_TRANSACTIONS, "chain_id, signature, wallet_address, added_at, last_checked_at, check_count", "signature, wallet_address, added_at, last_checked_at, check_count"),
            ("bootstrap_state", SCHEMA_BOOTSTRAP_STATE, "chain_id, id, backfill_before_cursor, full_history_completed, updated_at", "id, backfill_before_cursor, full_history_completed, updated_at"),
            ("subject_asset_deltas", SCHEMA_SUBJECT_ASSET_DELTAS, "chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_raw, success", "wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success"),
        ];
        for (table, schema, columns, legacy_columns) in tables {
            let create = schema.replacen(
                &format!("CREATE TABLE IF NOT EXISTS {table} ("),
                &format!("CREATE TABLE {table}__v7 ("),
                1,
            );
            tx.execute(&create, []).map_err(|e| Error::Migration {
                step: format!("create {table}__v7"),
                detail: e.to_string(),
            })?;
            let source_columns = super::column_names::live_column_list(tx, table, legacy_columns)?;
            tx.execute(
                &format!("INSERT INTO {table}__v7 ({columns}) SELECT 'solana', {source_columns} FROM {table}"),
                [],
            ).map_err(|e| Error::Migration {
                step: format!("copy {table} into v7"),
                detail: e.to_string(),
            })?;
            let before: i64 = tx
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .map_err(|e| Error::Migration {
                    step: format!("count {table}"),
                    detail: e.to_string(),
                })?;
            let after: i64 = tx
                .query_row(&format!("SELECT COUNT(*) FROM {table}__v7"), [], |row| {
                    row.get(0)
                })
                .map_err(|e| Error::Migration {
                    step: format!("count {table}__v7"),
                    detail: e.to_string(),
                })?;
            if before != after {
                return Err(Error::Migration {
                    step: format!("v7 row count check for {table}"),
                    detail: format!("row count mismatch: {before} != {after}"),
                });
            }
            tx.execute(&format!("DROP TABLE {table}"), [])
                .map_err(|e| Error::Migration {
                    step: format!("drop {table}"),
                    detail: e.to_string(),
                })?;
            tx.execute(&format!("ALTER TABLE {table}__v7 RENAME TO {table}"), [])
                .map_err(|e| Error::Migration {
                    step: format!("rename {table}__v7"),
                    detail: e.to_string(),
                })?;
        }
        let fk_errors: i64 = tx
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::Migration {
                step: "v7 migration foreign_key_check".to_owned(),
                detail: e.to_string(),
            })?;
        if fk_errors != 0 {
            return Err(Error::Migration {
                step: "v7 migration foreign_key_check".to_owned(),
                detail: format!("found {fk_errors} errors"),
            });
        }
        Ok(())
    }

    /// Runs inside the opener's transaction, after the unit column rename.
    pub(super) fn backfill_processed_native_delta(&self, conn: &Connection) -> Result<(), Error> {
        const BATCH_SIZE: i64 = 1000;
        let mut total_updated = 0usize;

        // Get wallet address for filtering (this is a migration function, so it operates on current wallet data only)
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| Error::Migration {
            step: "get wallet address for native_delta backfill".to_owned(),
            detail: e.to_string(),
        })?;

        loop {
            let mut stmt = conn
                .prepare(
                    "SELECT signature, native_balance_change FROM processed_transactions WHERE chain_id = ?1 AND wallet_address = ?2 AND native_delta IS NULL LIMIT ?3",
                )
                .map_err(|e| Error::Migration {
                    step: "prepare native_delta backfill query".to_owned(),
                    detail: e.to_string(),
                })?;

            let rows = stmt
                .query_map(
                    params![self.chain.as_str(), wallet_address, BATCH_SIZE],
                    |row| {
                        let signature: String = row.get(0)?;
                        let change_json: Option<String> = row.get(1)?;
                        Ok((signature, change_json))
                    },
                )
                .map_err(|e| Error::Migration {
                    step: "iterate native_delta backfill rows".to_owned(),
                    detail: e.to_string(),
                })?;

            let mut batch: Vec<(String, Option<String>)> = Vec::new();
            for row in rows {
                let (signature, change_json) = row.map_err(|e| Error::Migration {
                    step: "read native_delta row".to_owned(),
                    detail: e.to_string(),
                })?;
                batch.push((signature, change_json));
            }

            if batch.is_empty() {
                break;
            }

            drop(stmt);

            for (signature, change_json) in batch.into_iter() {
                let delta = Self::compute_native_delta_from_json(change_json.as_deref());
                conn.execute(
                    "UPDATE processed_transactions SET native_delta = ?1 WHERE chain_id = ?2 AND signature = ?3 AND wallet_address = ?4",
                    params![delta, self.chain.as_str(), signature, wallet_address],
                )
                .map_err(|e| Error::Migration {
                    step: "update native_delta".to_owned(),
                    detail: e.to_string(),
                })?;
                total_updated += 1;
            }
        }

        if total_updated > 0 {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Backfilled native_delta for {} processed transactions",
                    total_updated
                ),
            );
        }

        Ok(())
    }

    fn compute_native_delta_from_json(payload: Option<&str>) -> f64 {
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
