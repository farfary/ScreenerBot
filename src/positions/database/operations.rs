// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position database operations — init, schema, write operations, and row-mapping helpers.

use chrono::{DateTime, NaiveDateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::Ordering;

use crate::database::{self, WriteTransaction};
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{Position, PositionManagement, PositionOrigin};
use crate::positions::{Error, Result};

use super::column_names::{add_missing_columns, rename_unit_neutral_columns};
use super::open_round::{install_open_round_index, query_open_round_id, OPEN_ROUND_INDEX_NAME};
use super::provenance::{merge_ledger_duplicates, migrate_position_provenance};
use super::raw_migration::{canonicalize_amount_tables, migrate_pending_partial_exit_amounts};
use super::types::*;

impl PositionsDatabase {
    /// Create new PositionsDatabase with connection pooling
    pub async fn new(chain: crate::chains::ChainId) -> Result<Self> {
        let database_path = crate::paths::get_positions_db_path();
        Self::new_with_path(database_path, chain).await
    }

    /// Open a positions database at an explicit path and initialize its complete schema.
    async fn new_with_path(
        database_path: impl AsRef<std::path::Path>,
        chain: crate::chains::ChainId,
    ) -> Result<Self> {
        let database_path = database_path.as_ref();
        let database_path_str = database_path.to_string_lossy().to_string();

        // Only log detailed initialization on first database creation
        let is_first_init = !POSITIONS_DB_INITIALIZED.load(Ordering::Relaxed);
        if is_first_init {
            logger::info(
                LogTag::Positions,
                &format!("Initializing positions database at: {database_path_str}"),
            );
        }

        // Configure connection manager with centralized PRAGMAs
        let manager = SqliteConnectionManager::file(database_path)
            .with_init(|c| database::configure_connection(c, database::POSITIONS_DB));

        // Create connection pool
        let pool = Pool::builder()
            .max_size(5)
            .min_idle(Some(1))
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(|e| DatabaseError::Connection {
                message: format!("failed to create positions connection pool: {e}"),
            })?;

        let mut db = PositionsDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: POSITIONS_SCHEMA_VERSION,
            chain,
        };

        // Initialize database schema
        db.initialize_schema(is_first_init).await?;

        if is_first_init {
            logger::info(
                LogTag::Positions,
                "Positions database initialized successfully",
            );
            POSITIONS_DB_INITIALIZED.store(true, Ordering::Relaxed);
        }

        Ok(db)
    }

    /// Bring the store to the current schema in one IMMEDIATE transaction: tables,
    /// historical columns, unit-neutral names, the canonical amount tables, ledger
    /// merges, indexes, the schema version and the data migrations either all commit or
    /// none do, so a refused open leaves the file as it was. Foreign keys are off for
    /// the transaction because a rebuilt root table is dropped before its replacement
    /// takes its name; the rebuild checks them before commit.
    async fn initialize_schema(&mut self, log_initialization: bool) -> Result<()> {
        let mut conn = self.get_connection()?;
        let pragma = |e: rusqlite::Error| Error::SchemaMigration {
            detail: format!("failed to set positions foreign keys: {e}"),
        };
        let foreign_keys: bool = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .map_err(pragma)?;
        conn.pragma_update(None, "foreign_keys", false)
            .map_err(pragma)?;
        let result = self.initialize_schema_in_transaction(&mut conn, log_initialization);
        let restored = conn
            .pragma_update(None, "foreign_keys", foreign_keys)
            .map_err(pragma);
        let summary = result?;
        restored?;
        if log_initialization || !summary.is_empty() {
            logger::info(
                LogTag::Positions,
                &if summary.is_empty() {
                    "Positions database schema is current".to_owned()
                } else {
                    format!("Positions database upgraded: {}", summary.join("; "))
                },
            );
        }
        Ok(())
    }

    /// The steps of [`Self::initialize_schema`], in dependency order, and what each
    /// changed.
    fn initialize_schema_in_transaction(
        &self,
        conn: &mut Connection,
        log_initialization: bool,
    ) -> Result<Vec<String>> {
        let tx = conn.write_tx().map_err(|e| Error::SchemaMigration {
            detail: format!("failed to begin positions schema transaction: {e}"),
        })?;
        let mut summary = Vec::new();

        for (table, ddl) in [
            ("positions", SCHEMA_POSITIONS),
            ("position_states", SCHEMA_POSITION_STATES),
            ("position_exits", SCHEMA_POSITION_EXITS),
            ("position_entries", SCHEMA_POSITION_ENTRIES),
            ("position_tracking", SCHEMA_POSITION_TRACKING),
            ("position_metadata", SCHEMA_POSITION_METADATA),
            ("token_snapshots", SCHEMA_TOKEN_SNAPSHOTS),
        ] {
            tx.execute(ddl, []).map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create {table} table: {e}"),
            })?;
        }

        // Historical columns are appended in the order releases introduced them;
        // provenance backfills from columns the earlier steps guarantee.
        let mut added = add_missing_columns(&tx, "positions", POSITIONS_PNL_COLUMNS)?;
        added.extend(add_missing_columns(
            &tx,
            "positions",
            POSITIONS_ARCHIVE_COLUMNS,
        )?);
        added.extend(migrate_position_provenance(&tx)?);
        added.extend(add_missing_columns(
            &tx,
            "positions",
            POSITIONS_CHAIN_COLUMNS,
        )?);
        if !added.is_empty() {
            summary.push(format!("added positions columns {}", added.join(", ")));
        }

        match rename_unit_neutral_columns(&tx)? {
            0 => {}
            renamed => summary.push(format!("renamed {renamed} unit columns")),
        }

        let canonicalized = canonicalize_amount_tables(&tx)?;
        if !canonicalized.dropped_indexes.is_empty() {
            summary.push(format!(
                "dropped indexes {}",
                canonicalized.dropped_indexes.join(", ")
            ));
        }
        if !canonicalized.rebuilt.is_empty() {
            summary.push(format!(
                "rebuilt {} in canonical form",
                canonicalized.rebuilt.join(", ")
            ));
        }

        if migrate_pending_partial_exit_amounts(&tx)? {
            summary.push("converted pending partial-exit amounts".to_owned());
        }

        match merge_ledger_duplicates(&tx)? {
            0 => {}
            merged => logger::warning(
                LogTag::Positions,
                &format!(
                    "Merged {merged} duplicate wallet-history rows into the positions they belong to"
                ),
            ),
        }

        for index_sql in POSITIONS_INDEXES {
            tx.execute(index_sql, [])
                .map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to create positions index: {e}"),
                })?;
        }
        for duplicate in install_open_round_index(&tx)? {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Positions {:?} of mint {} (chain {}, wallet {}) are all open: a mint has one open position, so the index {OPEN_ROUND_INDEX_NAME} is installed once all but one of them are closed; their books are left unchanged",
                    duplicate.position_ids,
                    duplicate.mint,
                    duplicate.chain_id,
                    duplicate.wallet_address
                ),
            );
        }

        tx.execute(
            "INSERT OR REPLACE INTO position_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to set positions schema version: {e}"),
        })?;

        self.run_data_migrations(&tx, log_initialization)?;

        tx.commit().map_err(|e| Error::SchemaMigration {
            detail: format!("failed to commit positions schema: {e}"),
        })?;
        Ok(summary)
    }

    /// Data migrations for rows written by earlier releases; a failure refuses the open
    /// with the schema transaction.
    fn run_data_migrations(&self, conn: &Connection, log: bool) -> Result<()> {
        // Open positions from before partial exits and DCA carry no remaining amount.
        let updated_count = conn
            .execute(
                r#"
      UPDATE positions
      SET
        remaining_token_amount = token_amount,
        average_entry_price = COALESCE(effective_entry_price, entry_price)
      WHERE remaining_token_amount IS NULL
       AND token_amount IS NOT NULL
       AND exit_time IS NULL
       AND chain_id = ?1
       AND position_type = 'buy'
      "#,
                params![self.chain.as_str()],
            )
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to initialize remaining position amounts: {e}"),
            })?;
        if updated_count > 0 && log {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Migrated {updated_count} existing open positions with partial sell/DCA fields"
                ),
            );
        }

        // State timestamps written by SQLite's datetime('now') become RFC3339.
        let rows = conn
            .execute(
                r#"
      UPDATE position_states
      SET changed_at = strftime('%Y-%m-%dT%H:%M:%SZ', datetime(changed_at))
      WHERE changed_at NOT LIKE '%T%'
      "#,
                [],
            )
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to normalize position state timestamps: {e}"),
            })?;
        if rows > 0 && log {
            logger::info(
                LogTag::Positions,
                &format!("Normalized {rows} legacy position state timestamps to RFC3339"),
            );
        }

        Ok(())
    }

    /// The chain whose positions this store holds.
    pub fn chain(&self) -> crate::chains::ChainId {
        self.chain
    }

    /// Get database connection from pool
    pub(crate) fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| {
            DatabaseError::Connection {
                message: format!("failed to get positions database connection: {e}"),
            }
            .into()
        })
    }

    /// Insert a new position and return the assigned ID. An open position is refused with
    /// [`Error::AlreadyOpen`] while its mint already has an open row in this chain and
    /// wallet, archived or not: the check and the insert share one write transaction, and
    /// the open-round index refuses it in storage as well.
    pub async fn insert_position(&self, position: &Position) -> Result<i64> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Inserting new position for mint {} with entry price {:.6} SOL",
                position.mint, position.entry_price
            ),
        );

        let mut conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let sqlite = |e| DatabaseError::classify_sqlite_failure("insert_position", e);
        // The transaction lives in this block only: it is not `Send`, so it must be gone
        // before the state history is recorded across an await.
        let position_id = {
            let tx = conn.write_tx().map_err(sqlite)?;
            if position.exit_time.is_none() {
                if let Some(open_id) = query_open_round_id(
                    &tx,
                    self.chain.as_str(),
                    &wallet_address,
                    &position.mint,
                    None,
                )? {
                    logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Refused a second open position for mint {}: position {open_id} is its open position",
                        position.mint
                    ),
                );
                    return Err(Error::AlreadyOpen {
                        mint: position.mint.clone(),
                        open_position_id: open_id,
                    });
                }
            }
            let position_id = tx
            .query_row(
                r#"
      INSERT INTO positions (
        chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
        position_type, entry_size_native, total_size_native, price_highest, price_lowest,
        entry_transaction_signature, exit_transaction_signature, token_amount,
        effective_entry_price, effective_exit_price, native_received,
        profit_target_min, profit_target_max, liquidity_tier,
        transaction_entry_verified, transaction_exit_verified,
        entry_fee_raw, exit_fee_raw, current_price, current_price_updated,
        phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
        pnl, pnl_percent, unrealized_pnl, unrealized_pnl_percent,
        remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
        dca_count, average_entry_price, last_dca_time, origin_kind, origin_ref, management,
        round_key, basis_complete, history_complete, holding_state
      ) VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
        ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33,
        ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47,
        ?48, ?49, ?50, ?51
      ) RETURNING id
      "#,
                params![
                    self.chain.as_str(),
                    wallet_address,
                    position.mint,
                    position.symbol,
                    position.name,
                    position.entry_price,
                    position.entry_time.to_rfc3339(),
                    position.exit_price,
                    position.exit_time.map(|t| t.to_rfc3339()),
                    position.position_type,
                    position.entry_size_native,
                    position.total_size_native,
                    position.price_highest,
                    position.price_lowest,
                    position.entry_transaction_signature,
                    position.exit_transaction_signature,
                    position.token_amount,
                    position.effective_entry_price,
                    position.effective_exit_price,
                    position.native_received,
                    position.profit_target_min,
                    position.profit_target_max,
                    position.liquidity_tier,
                    position.transaction_entry_verified,
                    position.transaction_exit_verified,
                    position.entry_fee_raw.map(|f| f as i64),
                    position.exit_fee_raw.map(|f| f as i64),
                    position.current_price,
                    position.current_price_updated.map(|t| t.to_rfc3339()),
                    position.phantom_confirmations as i64,
                    position.phantom_first_seen.map(|t| t.to_rfc3339()),
                    position.synthetic_exit,
                    position.closed_reason,
                    position.pnl,
                    position.pnl_percent,
                    position.unrealized_pnl,
                    position.unrealized_pnl_percent,
                    position.remaining_token_amount,
                    position.total_exited_amount,
                    position.average_exit_price,
                    position.partial_exit_count as i64,
                    position.dca_count as i64,
                    position.average_entry_price,
                    position.last_dca_time.map(|t| t.to_rfc3339()),
                    position.origin.kind(),
                    position.origin.reference(),
                    position.management.as_str(),
                    position.round_key,
                    position.basis_complete,
                    position.history_complete,
                    position.holding_state,
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| DatabaseError::Query {
                operation: "insert_position".to_owned(),
                message: e.to_string(),
            })?;
            tx.commit().map_err(sqlite)?;
            position_id
        };
        drop(conn);

        // Record initial state as Open
        self.record_state_change(
            position_id,
            PositionState::Open,
            Some(POSITION_CREATED_REASON),
        )
        .await?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully inserted position ID {} for mint {} with entry signature {}",
                position_id,
                position.mint,
                position
                    .entry_transaction_signature
                    .as_deref()
                    .unwrap_or("None")
            ),
        );

        logger::info(
            LogTag::Positions,
            &format!(
                "Inserted new position ID {} for mint {}",
                position_id, position.mint
            ),
        );

        Ok(position_id)
    }

    /// Update only the price-related fields for a position
    pub async fn update_position_prices(
        &self,
        position_id: i64,
        current_price: Option<f64>,
        current_price_updated: Option<DateTime<Utc>>,
        price_highest: f64,
        price_lowest: f64,
    ) -> Result<()> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Updating price fields for position ID {} (price={:?}, high={:.11}, low={:.11})",
                position_id, current_price, price_highest, price_lowest
            ),
        );

        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
      UPDATE positions SET
        current_price = ?2,
        current_price_updated = ?3,
        price_highest = ?4,
        price_lowest = ?5,
        updated_at = datetime('now')
      WHERE id = ?1 AND chain_id = ?6
      "#,
                params![
                    position_id,
                    current_price,
                    current_price_updated.map(|t| t.to_rfc3339()),
                    price_highest,
                    price_lowest,
                    self.chain.as_str(),
                ],
            )
            .map_err(|e| DatabaseError::classify_sqlite_failure("update_position_prices", e))?;

        if rows_affected == 0 {
            return Err(Error::NotFoundById { position_id });
        }

        Ok(())
    }

    /// Update the price fields and the unrealized P&L of a position. No booking column is
    /// written. A row that already carries an exit time keeps its unrealized P&L: the close
    /// booking cleared it, and a price computed before that booking must not restore it.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_position_prices_and_pnl(
        &self,
        position_id: i64,
        current_price: Option<f64>,
        current_price_updated: Option<DateTime<Utc>>,
        price_highest: f64,
        price_lowest: f64,
        unrealized_pnl: Option<f64>,
        unrealized_pnl_percent: Option<f64>,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
      UPDATE positions SET
        current_price = ?2,
        current_price_updated = ?3,
        price_highest = ?4,
        price_lowest = ?5,
        unrealized_pnl = CASE WHEN exit_time IS NULL THEN ?6 ELSE unrealized_pnl END,
        unrealized_pnl_percent = CASE WHEN exit_time IS NULL THEN ?7 ELSE unrealized_pnl_percent END,
        updated_at = datetime('now')
      WHERE id = ?1 AND chain_id = ?8
      "#,
                params![
                    position_id,
                    current_price,
                    current_price_updated.map(|t| t.to_rfc3339()),
                    price_highest,
                    price_lowest,
                    unrealized_pnl,
                    unrealized_pnl_percent,
                    self.chain.as_str(),
                ],
            )
            .map_err(|e| {
                DatabaseError::classify_sqlite_failure("update_position_prices_and_pnl", e)
            })?;

        if rows_affected == 0 {
            return Err(Error::NotFoundById { position_id });
        }

        Ok(())
    }

    /// Record a submitted full-exit swap: its signature, the market price at the exit
    /// decision and the pending closed reason. No other column is written. A row whose
    /// exit is already verified is left as booked and the call returns
    /// [`Error::AlreadyClosed`].
    pub async fn record_exit_submission(
        &self,
        position_id: i64,
        exit_signature: &str,
        exit_price: f64,
        closed_reason: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
      UPDATE positions SET
        exit_transaction_signature = ?2,
        exit_price = ?3,
        closed_reason = ?4,
        updated_at = datetime('now')
      WHERE id = ?1 AND chain_id = ?5 AND transaction_exit_verified = 0
      "#,
                params![
                    position_id,
                    exit_signature,
                    exit_price,
                    closed_reason,
                    self.chain.as_str(),
                ],
            )
            .map_err(|e| DatabaseError::classify_sqlite_failure("record_exit_submission", e))?;

        if rows_affected == 0 {
            let verified: Option<bool> = conn
                .query_row(
                    "SELECT transaction_exit_verified FROM positions WHERE id = ?1 AND chain_id = ?2",
                    params![position_id, self.chain.as_str()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| {
                    DatabaseError::classify_sqlite_failure("record_exit_submission", e)
                })?;
            return Err(match verified {
                Some(true) => Error::AlreadyClosed { position_id },
                _ => Error::NotFoundById { position_id },
            });
        }

        Ok(())
    }

    /// Force database synchronization to ensure all connections see recent writes
    /// This should be called after critical updates to prevent race conditions
    pub async fn force_sync(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Force WAL checkpoint to synchronize all connections
        // Use prepare and query since PRAGMA wal_checkpoint returns results
        let mut stmt =
            conn.prepare("PRAGMA wal_checkpoint(FULL);")
                .map_err(|e| Error::Maintenance {
                    operation: "sync",
                    detail: format!("failed to prepare WAL checkpoint: {e}"),
                })?;

        let _result = stmt.query([]).map_err(|e| Error::Maintenance {
            operation: "sync",
            detail: format!("failed to execute WAL checkpoint: {e}"),
        })?;

        Ok(())
    }

    /// Persist a key-value metadata pair via INSERT OR REPLACE
    pub fn set_metadata_value(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.get_connection()?;

        conn.execute(
      "INSERT OR REPLACE INTO position_metadata (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
      params![key, value],
    )
    .map_err(|e| DatabaseError::classify_sqlite_failure(&format!("set_metadata_value({key})"), e))?;

        Ok(())
    }

    /// Fetch a metadata value by key, returning None if not found
    pub fn get_metadata_value(&self, key: &str) -> Result<Option<String>> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare("SELECT value FROM position_metadata WHERE key = ?1 LIMIT 1")
            .map_err(|e| DatabaseError::Query {
                operation: format!("get_metadata_value({key})"),
                message: e.to_string(),
            })?;

        let mut rows = stmt.query(params![key]).map_err(|e| DatabaseError::Query {
            operation: format!("get_metadata_value({key})"),
            message: e.to_string(),
        })?;

        match rows.next().map_err(|e| DatabaseError::Query {
            operation: format!("get_metadata_value({key})"),
            message: e.to_string(),
        })? {
            Some(row) => {
                let value: String = row.get(0).map_err(|e| Error::RowDecode {
                    column: "value",
                    detail: e.to_string(),
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Parse a datetime string leniently, trying multiple formats with UTC fallback.
    ///
    /// Tries in order:
    /// 1. RFC3339 (e.g. "2026-02-26T08:25:54+00:00")
    /// 2. ISO8601 without timezone (e.g. "2026-02-26T08:25:54") → assume UTC
    /// 3. Space-separated without timezone (e.g. "2026-02-26 08:25:54") → assume UTC
    /// 4. Space-separated with fractional seconds (e.g. "2026-02-26 08:25:54.123") → assume UTC
    fn parse_datetime_lenient(s: &str) -> Result<DateTime<Utc>> {
        // 1. RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Fallback formats (no timezone → assume UTC)
        const FALLBACK_FORMATS: &[&str] = &[
            "%Y-%m-%dT%H:%M:%S",    // ISO8601 without tz
            "%Y-%m-%d %H:%M:%S",    // space-separated
            "%Y-%m-%d %H:%M:%S%.f", // space-separated with fractional seconds
        ];

        for fmt in FALLBACK_FORMATS {
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Datetime '{}' is not RFC3339; parsed with fallback format '{}' as UTC",
                        s, fmt
                    ),
                );
                return Ok(naive.and_utc());
            }
        }

        Err(Error::RowDecode {
            column: "<datetime>",
            detail: format!("could not parse datetime '{s}' with any known format"),
        })
    }

    /// Helper function to convert database row to Position struct
    pub(crate) fn row_to_position(&self, row: &rusqlite::Row) -> rusqlite::Result<Position> {
        let entry_time_str: String = row.get("entry_time")?;
        let entry_time = Self::parse_datetime_lenient(&entry_time_str).map_err(|e| {
            rusqlite::Error::InvalidColumnType(
                5,
                format!("Invalid entry_time: {e}"),
                rusqlite::types::Type::Text,
            )
        })?;

        let exit_time = if let Some(exit_time_str) = row.get::<_, Option<String>>("exit_time")? {
            Some(Self::parse_datetime_lenient(&exit_time_str).map_err(|e| {
                rusqlite::Error::InvalidColumnType(
                    7,
                    format!("Invalid exit_time: {e}"),
                    rusqlite::types::Type::Text,
                )
            })?)
        } else {
            None
        };

        let current_price_updated =
            if let Some(updated_str) = row.get::<_, Option<String>>("current_price_updated")? {
                Some(Self::parse_datetime_lenient(&updated_str).map_err(|e| {
                    rusqlite::Error::InvalidColumnType(
                        27,
                        format!("Invalid current_price_updated: {e}"),
                        rusqlite::types::Type::Text,
                    )
                })?)
            } else {
                None
            };

        let phantom_first_seen =
            if let Some(seen_str) = row.get::<_, Option<String>>("phantom_first_seen")? {
                Some(Self::parse_datetime_lenient(&seen_str).map_err(|e| {
                    rusqlite::Error::InvalidColumnType(
                        29,
                        format!("Invalid phantom_first_seen: {e}"),
                        rusqlite::types::Type::Text,
                    )
                })?)
            } else {
                None
            };

        let last_dca_time = if let Some(dca_str) = row.get::<_, Option<String>>("last_dca_time")? {
            Some(Self::parse_datetime_lenient(&dca_str).map_err(|e| {
                rusqlite::Error::InvalidColumnType(
                    35,
                    format!("Invalid last_dca_time: {e}"),
                    rusqlite::types::Type::Text,
                )
            })?)
        } else {
            None
        };

        let archived_at = match row.get::<_, Option<String>>("archived_at") {
            Ok(Some(archived_str)) => Self::parse_datetime_lenient(&archived_str).ok(),
            _ => None,
        };

        Ok(Position {
            id: Some(row.get("id")?),
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            entry_price: row.get("entry_price")?,
            entry_time,
            exit_price: row.get("exit_price")?,
            exit_time,
            position_type: row.get("position_type")?,
            entry_size_native: row.get("entry_size_native")?,
            total_size_native: row.get("total_size_native")?,
            price_highest: row.get("price_highest")?,
            price_lowest: row.get("price_lowest")?,
            entry_transaction_signature: row.get("entry_transaction_signature")?,
            exit_transaction_signature: row.get("exit_transaction_signature")?,
            token_amount: row.get("token_amount")?,
            effective_entry_price: row.get("effective_entry_price")?,
            effective_exit_price: row.get("effective_exit_price")?,
            native_received: row.get("native_received")?,
            profit_target_min: row.get("profit_target_min")?,
            profit_target_max: row.get("profit_target_max")?,
            liquidity_tier: row.get("liquidity_tier")?,
            transaction_entry_verified: row.get("transaction_entry_verified")?,
            transaction_exit_verified: row.get("transaction_exit_verified")?,
            entry_fee_raw: row
                .get::<_, Option<i64>>("entry_fee_raw")?
                .map(|f| f as u64),
            exit_fee_raw: row.get::<_, Option<i64>>("exit_fee_raw")?.map(|f| f as u64),
            current_price: row.get("current_price")?,
            current_price_updated,
            current_price_source: None, // Not persisted
            phantom_remove: false,      // This is not persisted
            phantom_confirmations: row.get::<_, i64>("phantom_confirmations")? as u32,
            phantom_first_seen,
            synthetic_exit: row.get("synthetic_exit")?,
            closed_reason: row.get("closed_reason")?,
            // Pre-calculated P&L fields
            pnl: row.get::<_, Option<f64>>("pnl").ok().flatten(),
            pnl_percent: row.get::<_, Option<f64>>("pnl_percent").ok().flatten(),
            unrealized_pnl: row.get::<_, Option<f64>>("unrealized_pnl").ok().flatten(),
            unrealized_pnl_percent: row
                .get::<_, Option<f64>>("unrealized_pnl_percent")
                .ok()
                .flatten(),
            // New fields for partial exit and DCA support
            remaining_token_amount: row.get("remaining_token_amount")?,
            total_exited_amount: row.get("total_exited_amount")?,
            average_exit_price: row.get("average_exit_price")?,
            partial_exit_count: row.get::<_, i64>("partial_exit_count")? as u32,
            dca_count: row.get::<_, i64>("dca_count")? as u32,
            average_entry_price: row.get("average_entry_price")?,
            last_dca_time,
            // Archival (default false for legacy rows / pre-migration reads)
            archived: row
                .get::<_, Option<bool>>("archived")
                .ok()
                .flatten()
                .unwrap_or(false),
            archived_at,
            origin: PositionOrigin::from_columns(
                &row.get::<_, String>("origin_kind")?,
                row.get("origin_ref")?,
            )
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, e.into())
            })?,
            management: PositionManagement::parse(&row.get::<_, String>("management")?).map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        e.into(),
                    )
                },
            )?,
            round_key: row.get("round_key")?,
            basis_complete: row
                .get::<_, Option<bool>>("basis_complete")?
                .unwrap_or(true),
            history_complete: row
                .get::<_, Option<bool>>("history_complete")?
                .unwrap_or(true),
            holding_state: row.get("holding_state")?,
        })
    }
}

/// Carries from `live` onto `row` the columns a booking does not own, so adopting a
/// committed row never reverts what their own writers set:
/// - the prices [`PositionsDatabase::update_position_prices_and_pnl`] writes, and the source
///   of the current price, which is never persisted. A booking writes these back as it read
///   them, so the in-memory values are the newer ones;
/// - the archive flag, the management mode and the origin, which [`write_position_row`]
///   never writes. Their writers store the row, then mirror memory.
pub(crate) fn carry_columns_not_booked(row: &mut Position, live: &Position) {
    row.current_price = live.current_price;
    row.current_price_updated = live.current_price_updated;
    row.current_price_source = live.current_price_source;
    row.price_highest = live.price_highest;
    row.price_lowest = live.price_lowest;
    row.archived = live.archived;
    row.archived_at = live.archived_at;
    row.management = live.management;
    row.origin = live.origin.clone();
}

/// Writes every persisted column of `position` to its row. Returns the number of rows
/// written: zero when no row has this id on this chain.
pub(super) fn write_position_row(
    conn: &Connection,
    chain: &str,
    position_id: i64,
    position: &Position,
) -> rusqlite::Result<usize> {
    conn.execute(
        r#"
      UPDATE positions SET
        mint = ?2, symbol = ?3, name = ?4, entry_price = ?5, entry_time = ?6,
        exit_price = ?7, exit_time = ?8, position_type = ?9, entry_size_native = ?10,
        total_size_native = ?11, price_highest = ?12, price_lowest = ?13,
        entry_transaction_signature = ?14, exit_transaction_signature = ?15,
        token_amount = ?16, effective_entry_price = ?17, effective_exit_price = ?18,
        native_received = ?19, profit_target_min = ?20, profit_target_max = ?21,
        liquidity_tier = ?22, transaction_entry_verified = ?23, transaction_exit_verified = ?24,
        entry_fee_raw = ?25, exit_fee_raw = ?26, current_price = ?27,
        current_price_updated = ?28, phantom_confirmations = ?29, phantom_first_seen = ?30,
        synthetic_exit = ?31, closed_reason = ?32,
        pnl = ?33, pnl_percent = ?34, unrealized_pnl = ?35, unrealized_pnl_percent = ?36,
        remaining_token_amount = ?37, total_exited_amount = ?38, average_exit_price = ?39,
        partial_exit_count = ?40, dca_count = ?41, average_entry_price = ?42, last_dca_time = ?43,
        round_key = ?44, basis_complete = ?45, history_complete = ?46, holding_state = ?47,
        updated_at = datetime('now')
      WHERE id = ?1 AND chain_id = ?48
      "#,
        params![
            position_id,
            position.mint,
            position.symbol,
            position.name,
            position.entry_price,
            position.entry_time.to_rfc3339(),
            position.exit_price,
            position.exit_time.map(|t| t.to_rfc3339()),
            position.position_type,
            position.entry_size_native,
            position.total_size_native,
            position.price_highest,
            position.price_lowest,
            position.entry_transaction_signature,
            position.exit_transaction_signature,
            position.token_amount,
            position.effective_entry_price,
            position.effective_exit_price,
            position.native_received,
            position.profit_target_min,
            position.profit_target_max,
            position.liquidity_tier,
            position.transaction_entry_verified,
            position.transaction_exit_verified,
            position.entry_fee_raw.map(|f| f as i64),
            position.exit_fee_raw.map(|f| f as i64),
            position.current_price,
            position.current_price_updated.map(|t| t.to_rfc3339()),
            position.phantom_confirmations as i64,
            position.phantom_first_seen.map(|t| t.to_rfc3339()),
            position.synthetic_exit,
            position.closed_reason,
            position.pnl,
            position.pnl_percent,
            position.unrealized_pnl,
            position.unrealized_pnl_percent,
            position.remaining_token_amount,
            position.total_exited_amount,
            position.average_exit_price,
            position.partial_exit_count as i64,
            position.dca_count as i64,
            position.average_entry_price,
            position.last_dca_time.map(|t| t.to_rfc3339()),
            position.round_key,
            position.basis_complete,
            position.history_complete,
            position.holding_state,
            chain,
        ],
    )
}

#[cfg(test)]
mod tests {
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::{params, Connection};

    use crate::chains::RawAmount;

    use super::super::booking::Booking;
    use super::super::open_round::OPEN_ROUND_INDEX_NAME;
    use super::{PositionsDatabase, POSITIONS_INDEXES, POSITIONS_SCHEMA_VERSION};

    fn test_database() -> (PositionsDatabase, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        let manager = SqliteConnectionManager::file(&path);
        let pool = Pool::builder().max_size(1).build(manager).unwrap();
        (
            PositionsDatabase {
                pool,
                database_path: path.to_string_lossy().into_owned(),
                schema_version: POSITIONS_SCHEMA_VERSION,
                chain: crate::chains::ChainId::Solana,
            },
            directory,
        )
    }

    fn assert_current_schema(connection: &Connection) {
        for table in [
            "positions",
            "position_states",
            "position_exits",
            "position_entries",
            "position_tracking",
            "position_metadata",
            "token_snapshots",
        ] {
            let exists = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing table {table}");
        }
        for column in [
            "chain_id",
            "origin_kind",
            "origin_ref",
            "management",
            "round_key",
            "basis_complete",
            "history_complete",
            "holding_state",
        ] {
            let exists = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('positions') WHERE name = ?1",
                    [column],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing positions.{column}");
        }
        for index_sql in POSITIONS_INDEXES {
            let index_name = index_sql
                .split("INDEX IF NOT EXISTS ")
                .nth(1)
                .and_then(|tail| tail.split_whitespace().next())
                .expect("position index statement");
            let exists = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    [index_name],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing index {index_name}");
        }
        assert!(
            has_open_round_index(connection),
            "missing index {OPEN_ROUND_INDEX_NAME}"
        );
        assert_store_is_sound(connection);
        assert_eq!(
            connection
                .query_row(
                    "SELECT value FROM position_metadata WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            POSITIONS_SCHEMA_VERSION.to_string()
        );
    }

    fn has_open_round_index(connection: &Connection) -> bool {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [OPEN_ROUND_INDEX_NAME],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    fn assert_store_is_sound(connection: &Connection) {
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
    }

    /// Every column of every position row and every child row, in id order.
    fn store_contents(connection: &Connection) -> Vec<String> {
        let mut contents = Vec::new();
        for table in ["positions", "position_states", "position_entries"] {
            let mut statement = connection
                .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
                .unwrap();
            let width = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    let mut values = Vec::with_capacity(width);
                    for index in 0..width {
                        values.push(format!("{:?}", row.get_ref(index)?));
                    }
                    Ok(format!("{table}: {}", values.join(" | ")))
                })
                .unwrap();
            for row in rows {
                contents.push(row.unwrap());
            }
        }
        contents
    }

    #[tokio::test]
    async fn full_schema_initialization_leaves_duplicate_open_rows_untouched_until_one_closes() {
        let (mut database, _directory) = test_database();
        {
            let legacy = database.get_connection().unwrap();
            legacy
                .execute_batch(include_str!(
                    "../../../tests/fixtures/v0.2.13-positions.sql"
                ))
                .unwrap();
            for (id, entry_time) in [(41, "2026-01-01"), (42, "2026-01-02")] {
                legacy
                    .execute(
                        "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount, remaining_token_amount, origin_kind, management) VALUES (?1, 'solana', 'wallet', 'mint', 'SYM', 'Token', 0.5, ?2, 'buy', 1.0, 1.0, 0.5, 0.5, 2, 2, 'manual', 'user_only')",
                        params![id, entry_time],
                    )
                    .unwrap();
                legacy
                    .execute(
                        "INSERT INTO position_states (position_id, state) VALUES (?1, 'Open')",
                        [id],
                    )
                    .unwrap();
            }
        }

        database.initialize_schema(false).await.unwrap();
        let initialized = store_contents(&database.get_connection().unwrap());
        database.initialize_schema(false).await.unwrap();

        let connection = database.get_connection().unwrap();
        assert!(
            !has_open_round_index(&connection),
            "the index was installed over two open rows of one mint"
        );
        assert_store_is_sound(&connection);
        assert_eq!(
            store_contents(&connection),
            initialized,
            "a second initialization changed the duplicate rows"
        );
        let open: Vec<(i64, String, String)> = connection
            .prepare("SELECT id, token_amount, remaining_token_amount FROM positions WHERE exit_time IS NULL ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            open,
            vec![
                (41, "2".to_owned(), "2".to_owned()),
                (42, "2".to_owned(), "2".to_owned())
            ],
            "a duplicate open row lost its data"
        );

        connection
            .execute(
                "UPDATE positions SET exit_time = '2026-01-03T00:00:00Z' WHERE id = 41",
                [],
            )
            .unwrap();
        drop(connection);
        database.initialize_schema(false).await.unwrap();
        database.initialize_schema(false).await.unwrap();

        let connection = database.get_connection().unwrap();
        assert_current_schema(&connection);
        let second_open = connection.execute(
            "INSERT INTO positions (chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_native, total_size_native, price_highest, price_lowest) VALUES ('solana', 'wallet', 'mint', 'SYM', 'Token', 0.5, '2026-01-04', 'buy', 1.0, 1.0, 0.5, 0.5)",
            [],
        );
        assert!(second_open.is_err(), "storage accepted a second open row");
    }

    #[tokio::test]
    async fn a_booking_that_unarchives_its_row_clears_the_stored_archive_flag() {
        let (mut database, _directory) = test_database();
        database.initialize_schema(false).await.unwrap();
        database.get_connection().unwrap().execute(
            "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_native, total_size_native, price_highest, price_lowest, archived, archived_at, origin_kind, management) VALUES (41, 'solana', 'wallet', 'mint', 'SYM', 'Token', 0.5, '2026-01-01T00:00:00Z', 'buy', 1.0, 1.0, 0.5, 0.5, 1, '2026-01-02T00:00:00Z', 'manual', 'user_only')",
            [],
        ).unwrap();
        let archived = || -> (bool, Option<String>) {
            database
                .get_connection()
                .unwrap()
                .query_row(
                    "SELECT archived, archived_at FROM positions WHERE id = 41",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap()
        };
        let book = |unarchive: bool| {
            database
                .commit_booking(41, Ok("wallet"), |row, _| {
                    if unarchive {
                        row.archived = false;
                        row.archived_at = None;
                    }
                    Ok(Booking::Write {
                        record: None,
                        outcome: (),
                    })
                })
                .unwrap();
        };

        book(false);
        assert_eq!(archived(), (true, Some("2026-01-02T00:00:00Z".to_owned())));
        book(true);
        assert_eq!(archived(), (false, None));
    }

    #[tokio::test]
    async fn full_schema_initialization_creates_and_reopens_fresh_database_files() {
        let directory = tempfile::tempdir().unwrap();

        for file_name in ["missing.db", "placeholder.db"] {
            let path = directory.path().join(file_name);
            if file_name == "placeholder.db" {
                std::fs::File::create(&path).unwrap();
            } else {
                assert!(!path.exists());
            }

            let database = PositionsDatabase::new_with_path(&path, crate::chains::ChainId::Solana)
                .await
                .unwrap();
            drop(database);
            let database = PositionsDatabase::new_with_path(&path, crate::chains::ChainId::Solana)
                .await
                .unwrap();

            let connection = database.get_connection().unwrap();
            assert_current_schema(&connection);
        }
    }

    #[tokio::test]
    async fn full_schema_initialization_preserves_released_provenance_and_is_idempotent() {
        let (mut database, _directory) = test_database();
        {
            let legacy = database.get_connection().unwrap();
            legacy
                .execute_batch(include_str!(
                    "../../../tests/fixtures/v0.2.13-positions.sql"
                ))
                .unwrap();
            legacy
                .execute(
                    "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount, remaining_token_amount, origin_kind, management) VALUES (41, 'solana', 'wallet', 'mint', 'SYM', 'Token', 0.5, '2026-01-01', 'buy', 1.0, 1.0, 0.5, 0.5, 2, 2, 'manual', 'user_only')",
                    [],
                )
                .unwrap();
            legacy
                .execute(
                    "INSERT INTO position_states (id, position_id, state) VALUES (7, 41, 'Open')",
                    [],
                )
                .unwrap();
        }

        database.initialize_schema(false).await.unwrap();
        database.initialize_schema(false).await.unwrap();

        let connection = database.get_connection().unwrap();
        assert_current_schema(&connection);
        assert_eq!(
            connection
                .query_row(
                    "SELECT chain_id, origin_kind, management, token_amount FROM positions WHERE id = 41",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
                )
                .unwrap(),
            ("solana".to_owned(), "manual".to_owned(), "user_only".to_owned(), "2".to_owned())
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT position_id FROM position_states WHERE id = 7",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            41
        );
    }

    /// Released v0.2.13 storage with the shape files created by earlier releases keep:
    /// chain identity appended with its default, the dropped `manual_management` column,
    /// and the legacy wallet index.
    fn appended_column_storage() -> String {
        include_str!("../../../tests/fixtures/v0.2.13-positions.sql")
            .replacen("  chain_id TEXT NOT NULL,\n", "", 1)
            .replacen(
                "  updated_at TEXT NOT NULL DEFAULT (datetime('now'))\n);",
                "  updated_at TEXT NOT NULL DEFAULT (datetime('now')),\n  manual_management BOOLEAN NOT NULL DEFAULT 0,\n  chain_id TEXT NOT NULL DEFAULT 'solana'\n);\nCREATE INDEX idx_positions_wallet ON positions(wallet_address);",
                1,
            )
    }

    fn sequences(connection: &Connection) -> Vec<(String, i64)> {
        connection
            .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Every stored table and index definition. `ALTER TABLE ... RENAME` stores the new
    /// table name quoted; the quotes are removed so a rebuilt table compares equal to a
    /// fresh one.
    fn table_sql(connection: &Connection) -> Vec<(String, String)> {
        connection
            .prepare("SELECT name, sql FROM sqlite_master WHERE type IN ('table', 'index') AND sql IS NOT NULL ORDER BY name")
            .unwrap()
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let sql: String = row.get(1)?;
                let sql = sql.replacen(
                    &format!("CREATE TABLE \"{name}\""),
                    &format!("CREATE TABLE {name}"),
                    1,
                );
                Ok((name, sql))
            })
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[tokio::test]
    async fn appended_legacy_columns_are_rebuilt_by_name_keeping_rows_children_and_sequences() {
        let (mut database, _directory) = test_database();
        {
            let legacy = database.get_connection().unwrap();
            legacy.execute_batch(&appended_column_storage()).unwrap();
            legacy.pragma_update(None, "foreign_keys", true).unwrap();
            legacy.execute_batch(
                "INSERT INTO positions (id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount, remaining_token_amount, total_exited_amount, origin_kind, management, manual_management) VALUES (41, 'wallet', 'mint', 'SYM', 'Token', 0.5, '2026-01-01T00:00:00Z', 'buy', 1.0, 1.0, 0.5, 0.5, -1, 7, 3, 'manual', 'user_only', 1);
                 INSERT INTO positions (id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest) VALUES (42, 'wallet', 'other', 'OTH', 'Other', 0.5, '2026-01-01T00:00:00Z', 'buy', 1.0, 1.0, 0.5, 0.5);
                 DELETE FROM positions WHERE id = 42;
                 INSERT INTO position_states (id, position_id, state) VALUES (7, 41, 'Open');
                 INSERT INTO position_entries (id, position_id, wallet_address, timestamp, amount, price, sol_spent, transaction_signature, is_dca) VALUES (5, 41, 'wallet', '2026-01-01T00:00:00Z', 9, 0.5, 1.0, 'entry', 0);
                 INSERT INTO position_exits (id, position_id, wallet_address, timestamp, amount, price, sol_received, transaction_signature, is_partial, percentage) VALUES (3, 41, 'wallet', '2026-01-02T00:00:00Z', 3, 0.6, 0.2, 'exit', 1, 30.0);",
            ).unwrap();
        }
        let before = sequences(&database.get_connection().unwrap());

        database.initialize_schema(false).await.unwrap();
        let upgraded = table_sql(&database.get_connection().unwrap());
        database.initialize_schema(false).await.unwrap();

        let connection = database.get_connection().unwrap();
        assert_current_schema(&connection);
        assert_eq!(
            table_sql(&connection),
            upgraded,
            "second open changed the schema"
        );
        let (fresh, _fresh_directory) = test_database();
        let mut fresh = fresh;
        fresh.initialize_schema(false).await.unwrap();
        assert_eq!(upgraded, table_sql(&fresh.get_connection().unwrap()));
        assert_eq!(sequences(&connection), before);
        assert_eq!(
            connection
                .query_row(
                    "SELECT chain_id, management, token_amount, remaining_token_amount, total_exited_amount FROM positions WHERE id = 41",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
                )
                .unwrap(),
            ("solana".to_owned(), "user_only".to_owned(), u64::MAX.to_string(), "7".to_owned(), "3".to_owned())
        );
        let children: (i64, String, String) = connection
            .query_row(
                "SELECT (SELECT position_id FROM position_states WHERE id = 7), (SELECT amount FROM position_entries WHERE id = 5), (SELECT amount FROM position_exits WHERE id = 3)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(children, (41, "9".to_owned(), "3".to_owned()));
        let foreign_keys: bool = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert!(
            foreign_keys,
            "the open restores the connection's own setting"
        );
    }

    #[tokio::test]
    async fn a_refused_upgrade_leaves_every_earlier_step_uncommitted() {
        let (mut database, _directory) = test_database();
        let legacy = appended_column_storage().replacen(
            "  manual_management BOOLEAN NOT NULL DEFAULT 0,\n",
            "  manual_management BOOLEAN NOT NULL DEFAULT 0,\n  unrecognized_flag INTEGER,\n",
            1,
        );
        database
            .get_connection()
            .unwrap()
            .execute_batch(&legacy)
            .unwrap();
        let before = table_sql(&database.get_connection().unwrap());

        let error = database.initialize_schema(false).await.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unrecognized column positions.unrecognized_flag"),
            "{error}"
        );
        assert_eq!(table_sql(&database.get_connection().unwrap()), before);
    }

    #[tokio::test]
    async fn wide_position_amounts_round_trip_through_the_row_codec_and_writer() {
        let (mut database, _directory) = test_database();
        database.initialize_schema(false).await.unwrap();
        database.get_connection().unwrap().execute(
            "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_native, total_size_native, price_highest, price_lowest, token_amount, remaining_token_amount, total_exited_amount, origin_kind, management) VALUES (41, 'solana', 'wallet', 'mint', 'SYM', 'Token', 0.5, '2026-01-01T00:00:00Z', 'buy', 1.0, 1.0, 0.5, 0.5, '340282366920938463463374607431768211455', '18446744073709551616', '18446744073709551617', 'manual', 'user_only')",
            [],
        ).unwrap();
        let position = {
            let connection = database.get_connection().unwrap();
            connection
                .query_row(
                    &format!(
                        "SELECT {} FROM positions WHERE id = 41",
                        super::POSITION_SELECT_COLUMNS
                    ),
                    [],
                    |row| database.row_to_position(row),
                )
                .unwrap()
        };
        assert_eq!(position.token_amount, Some(RawAmount::MAX));
        assert_eq!(
            position.remaining_token_amount,
            Some(RawAmount::new(1u128 << 64))
        );
        assert_eq!(
            position.total_exited_amount,
            RawAmount::new((1u128 << 64) + 1)
        );

        database
            .commit_booking(
                41,
                Err(crate::positions::Error::WalletUnavailable {
                    detail: "no wallet in this store".to_owned(),
                }),
                |row, _| {
                    row.remaining_token_amount = Some(RawAmount::new((1u128 << 64) + 5));
                    Ok(Booking::Write {
                        record: None,
                        outcome: (),
                    })
                },
            )
            .unwrap();
        let stored: (String, String, String) = database
            .get_connection()
            .unwrap()
            .query_row(
                "SELECT token_amount, remaining_token_amount, total_exited_amount FROM positions WHERE id = 41",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            stored,
            (
                "340282366920938463463374607431768211455".to_owned(),
                "18446744073709551621".to_owned(),
                "18446744073709551617".to_owned()
            )
        );
    }
}
