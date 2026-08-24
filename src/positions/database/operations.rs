//! Position database operations — init, schema, write operations, and row-mapping helpers.

use chrono::{DateTime, NaiveDateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::Ordering;

use crate::database;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{Position, PositionManagement, PositionOrigin};
use crate::positions::{Error, Result};

use super::provenance::{merge_ledger_duplicates, migrate_position_provenance};
use super::types::*;

impl PositionsDatabase {
    /// Create new PositionsDatabase with connection pooling
    pub async fn new(chain: crate::chains::ChainId) -> Result<Self> {
        let database_path = crate::paths::get_positions_db_path();
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
        let manager = SqliteConnectionManager::file(&database_path)
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

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self, log_initialization: bool) -> Result<()> {
        let conn = self.get_connection()?;

        Self::migrate_chain_identity(&conn)?;

        // Create all tables
        conn.execute(SCHEMA_POSITIONS, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create positions table: {e}"),
            })?;

        conn.execute(SCHEMA_POSITION_STATES, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create position_states table: {e}"),
            })?;

        conn.execute(SCHEMA_POSITION_EXITS, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create position_exits table: {e}"),
            })?;

        conn.execute(SCHEMA_POSITION_ENTRIES, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create position_entries table: {e}"),
            })?;

        conn.execute(SCHEMA_POSITION_TRACKING, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create position_tracking table: {e}"),
            })?;

        conn.execute(SCHEMA_POSITION_METADATA, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create position_metadata table: {e}"),
            })?;

        conn.execute(SCHEMA_TOKEN_SNAPSHOTS, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create token_snapshots table: {e}"),
            })?;

        // Migrate existing database to add PnL fields if needed
        // Check if migration is needed by attempting to add columns
        match conn.execute_batch(MIGRATION_ADD_PNL_FIELDS) {
            Ok(_) => {
                if log_initialization {
                    crate::logger::info(
                        crate::logger::LogTag::Positions,
                        "PnL columns migration completed successfully",
                    );
                }
            }
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                // SQLite returns "duplicate column name"if columns already exist - this is OK
                if err_msg.contains("duplicate column") {
                    if log_initialization {
                        crate::logger::debug(
                            crate::logger::LogTag::Positions,
                            "PnL columns already exist, skipping migration",
                        );
                    }
                } else {
                    // Real error - this is critical and should be logged
                    crate::logger::error(
                        crate::logger::LogTag::Positions,
                        &format!("CRITICAL: Failed to migrate PnL columns: {e}"),
                    );
                    return Err(Error::SchemaMigration {
                        detail: format!("PnL columns migration failed: {e}"),
                    });
                }
            }
        }

        // Migrate existing database to add archival fields if needed
        match conn.execute_batch(MIGRATION_ADD_ARCHIVE_FIELDS) {
            Ok(_) => {
                if log_initialization {
                    crate::logger::info(
                        crate::logger::LogTag::Positions,
                        "Archival columns migration completed successfully",
                    );
                }
            }
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                // SQLite returns "duplicate column name" if columns already exist - this is OK
                if err_msg.contains("duplicate column") {
                    if log_initialization {
                        crate::logger::debug(
                            crate::logger::LogTag::Positions,
                            "Archival columns already exist, skipping migration",
                        );
                    }
                } else {
                    crate::logger::error(
                        crate::logger::LogTag::Positions,
                        &format!("CRITICAL: Failed to migrate archival columns: {e}"),
                    );
                    return Err(Error::SchemaMigration {
                        detail: format!("archival columns migration failed: {e}"),
                    });
                }
            }
        }

        migrate_position_provenance(&conn)?;

        match merge_ledger_duplicates(&conn)? {
            0 => {}
            merged => logger::warning(
                LogTag::Positions,
                &format!(
                    "Merged {merged} duplicate wallet-history rows into the positions they belong to"
                ),
            ),
        }

        // Create all indexes
        for index_sql in POSITIONS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to create positions index: {e}"),
                })?;
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO position_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to set positions schema version: {e}"),
        })?;

        // Run migrations for existing positions (one-time data migration)
        self.run_data_migrations(&conn, log_initialization)?;

        if log_initialization {
            logger::info(
                LogTag::Positions,
                "Positions database schema initialized with all tables and indexes",
            );
        }

        Ok(())
    }

    /// Upgrades legacy shared storage in one transaction. Position IDs remain stable so
    /// every state, entry, exit, tracking and snapshot row continues to belong to its root.
    fn migrate_chain_identity(conn: &Connection) -> Result<()> {
        let has_positions: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='positions'",
                [],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to inspect positions schema: {e}"),
            })?
            .is_some();
        if !has_positions {
            return Ok(());
        }
        let has_chain: bool = conn
            .prepare("PRAGMA table_info(positions)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to inspect positions identity columns: {e}"),
            })?
            .iter()
            .any(|column| column == "chain_id");
        if has_chain {
            return Ok(());
        }
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to begin positions chain migration: {e}"),
            })?;
        tx.execute(
            "ALTER TABLE positions ADD COLUMN chain_id TEXT NOT NULL DEFAULT 'solana'",
            [],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to add positions chain identity: {e}"),
        })?;
        tx.execute("DROP INDEX IF EXISTS idx_positions_round_key", [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to replace legacy round index: {e}"),
            })?;
        for index in [
            "idx_positions_wallet",
            "idx_positions_mint",
            "idx_positions_entry_signature",
            "idx_positions_exit_signature",
        ] {
            tx.execute(&format!("DROP INDEX IF EXISTS {index}"), [])
                .map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to replace legacy positions index: {e}"),
                })?;
        }
        tx.execute(
            "CREATE UNIQUE INDEX idx_positions_round_key ON positions(chain_id, wallet_address, round_key) WHERE round_key IS NOT NULL",
            [],
        ).map_err(|e| Error::SchemaMigration {
            detail: format!("failed to create chain-qualified round index: {e}"),
        })?;
        let violations: i64 = tx
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(1_i64))
            .optional()
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to validate positions foreign keys: {e}"),
            })?
            .unwrap_or(0);
        if violations != 0 {
            return Err(Error::SchemaMigration {
                detail: "positions chain migration failed foreign-key validation".to_owned(),
            });
        }
        tx.commit().map_err(|e| Error::SchemaMigration {
            detail: format!("failed to commit positions chain migration: {e}"),
        })
    }

    /// Run data migrations for existing positions
    fn run_data_migrations(&self, conn: &Connection, log: bool) -> Result<()> {
        // Migration: Initialize remaining_token_amount and average_entry_price for existing open positions
        // This is a one-time migration for positions created before partial sell/DCA support
        let migration_result = conn.execute(
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
        );

        match migration_result {
            Ok(updated_count) => {
                if updated_count > 0 && log {
                    crate::logger::warning(
                        crate::logger::LogTag::Positions,
                        &format!(
                            "Migrated {} existing open positions with partial sell/DCA fields",
                            updated_count
                        ),
                    );
                }
            }
            Err(e) => {
                // Non-fatal: columns might not exist yet (fresh install)
                if log {
                    crate::logger::debug(
                        crate::logger::LogTag::Positions,
                        &format!(
                            "Position migration skipped (expected on fresh install): {}",
                            e
                        ),
                    );
                }
            }
        }

        // Migration: Normalize position_states.changed_at to RFC3339 format for old rows
        // Only touch legacy entries that still use the "YYYY-MM-DD HH:MM:SS"style emitted by sqlite's datetime('now')
        match conn.execute(
            r#"
      UPDATE position_states
      SET changed_at = strftime('%Y-%m-%dT%H:%M:%SZ', datetime(changed_at))
      WHERE changed_at NOT LIKE '%T%'
      "#,
            [],
        ) {
            Ok(rows) if rows > 0 && log => {
                crate::logger::info(
                    crate::logger::LogTag::Positions,
                    &format!(
                        "Normalized {} legacy position state timestamps to RFC3339",
                        rows
                    ),
                );
            }
            Ok(_) => {}
            Err(e) => {
                if log {
                    crate::logger::warning(
                        crate::logger::LogTag::Positions,
                        &format!(
                            "Failed to normalize legacy position state timestamps: {}",
                            e
                        ),
                    );
                }
            }
        }

        Ok(())
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

    /// Insert new position and return the assigned ID
    pub async fn insert_position(&self, position: &Position) -> Result<i64> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Inserting new position for mint {} with entry price {:.6} SOL",
                position.mint, position.entry_price
            ),
        );

        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let position_id = conn
            .query_row(
                r#"
      INSERT INTO positions (
        chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
        position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
        entry_transaction_signature, exit_transaction_signature, token_amount,
        effective_entry_price, effective_exit_price, sol_received,
        profit_target_min, profit_target_max, liquidity_tier,
        transaction_entry_verified, transaction_exit_verified,
        entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
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
                    position.entry_size_sol,
                    position.total_size_sol,
                    position.price_highest,
                    position.price_lowest,
                    position.entry_transaction_signature,
                    position.exit_transaction_signature,
                    position.token_amount.map(|t| t as i64),
                    position.effective_entry_price,
                    position.effective_exit_price,
                    position.sol_received,
                    position.profit_target_min,
                    position.profit_target_max,
                    position.liquidity_tier,
                    position.transaction_entry_verified,
                    position.transaction_exit_verified,
                    position.entry_fee_lamports.map(|f| f as i64),
                    position.exit_fee_lamports.map(|f| f as i64),
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
                    position.remaining_token_amount.map(|t| t as i64),
                    position.total_exited_amount as i64,
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

        // Record initial state as Open
        self.record_state_change(position_id, PositionState::Open, Some("Position created"))
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

    /// Update existing position by ID
    pub async fn update_position(&self, position: &Position) -> Result<()> {
        let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
            transition: "update",
            mint: position.mint.clone(),
            detail: "position has no id".to_owned(),
        })?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Updating position ID {} for mint {} with current price {:.11} SOL",
                position_id,
                position.mint,
                position.current_price.unwrap_or_default()
            ),
        );

        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
      UPDATE positions SET
        mint = ?2, symbol = ?3, name = ?4, entry_price = ?5, entry_time = ?6,
        exit_price = ?7, exit_time = ?8, position_type = ?9, entry_size_sol = ?10,
        total_size_sol = ?11, price_highest = ?12, price_lowest = ?13,
        entry_transaction_signature = ?14, exit_transaction_signature = ?15,
        token_amount = ?16, effective_entry_price = ?17, effective_exit_price = ?18,
        sol_received = ?19, profit_target_min = ?20, profit_target_max = ?21,
        liquidity_tier = ?22, transaction_entry_verified = ?23, transaction_exit_verified = ?24,
        entry_fee_lamports = ?25, exit_fee_lamports = ?26, current_price = ?27,
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
                    position.entry_size_sol,
                    position.total_size_sol,
                    position.price_highest,
                    position.price_lowest,
                    position.entry_transaction_signature,
                    position.exit_transaction_signature,
                    position.token_amount.map(|t| t as i64),
                    position.effective_entry_price,
                    position.effective_exit_price,
                    position.sol_received,
                    position.profit_target_min,
                    position.profit_target_max,
                    position.liquidity_tier,
                    position.transaction_entry_verified,
                    position.transaction_exit_verified,
                    position.entry_fee_lamports.map(|f| f as i64),
                    position.exit_fee_lamports.map(|f| f as i64),
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
                    position.remaining_token_amount.map(|t| t as i64),
                    position.total_exited_amount as i64,
                    position.average_exit_price,
                    position.partial_exit_count as i64,
                    position.dca_count as i64,
                    position.average_entry_price,
                    position.last_dca_time.map(|t| t.to_rfc3339()),
                    position.round_key,
                    position.basis_complete,
                    position.history_complete,
                    position.holding_state,
                    self.chain.as_str(),
                ],
            )
            .map_err(|e| DatabaseError::Query {
                operation: "update_position".to_owned(),
                message: e.to_string(),
            })?;

        if rows_affected == 0 {
            return Err(Error::NotFoundById { position_id });
        }

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully updated position ID {} ({} rows affected)",
                position_id, rows_affected
            ),
        );

        // Force WAL checkpoint to ensure all connections see the update immediately
        // This is critical for preventing race conditions in concurrent read operations
        if let Ok(mut stmt) = conn.prepare("PRAGMA wal_checkpoint(PASSIVE);") {
            let _ = stmt.query([]);
        }

        Ok(())
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
            .map_err(|e| DatabaseError::Query {
                operation: "update_position_prices".to_owned(),
                message: e.to_string(),
            })?;

        if rows_affected == 0 {
            return Err(Error::NotFoundById { position_id });
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
    .map_err(|e| DatabaseError::Query {
        operation: format!("set_metadata_value({key})"),
        message: e.to_string(),
    })?;

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
            entry_size_sol: row.get("entry_size_sol")?,
            total_size_sol: row.get("total_size_sol")?,
            price_highest: row.get("price_highest")?,
            price_lowest: row.get("price_lowest")?,
            entry_transaction_signature: row.get("entry_transaction_signature")?,
            exit_transaction_signature: row.get("exit_transaction_signature")?,
            token_amount: row.get::<_, Option<i64>>("token_amount")?.map(|t| t as u64),
            effective_entry_price: row.get("effective_entry_price")?,
            effective_exit_price: row.get("effective_exit_price")?,
            sol_received: row.get("sol_received")?,
            profit_target_min: row.get("profit_target_min")?,
            profit_target_max: row.get("profit_target_max")?,
            liquidity_tier: row.get("liquidity_tier")?,
            transaction_entry_verified: row.get("transaction_entry_verified")?,
            transaction_exit_verified: row.get("transaction_exit_verified")?,
            entry_fee_lamports: row
                .get::<_, Option<i64>>("entry_fee_lamports")?
                .map(|f| f as u64),
            exit_fee_lamports: row
                .get::<_, Option<i64>>("exit_fee_lamports")?
                .map(|f| f as u64),
            current_price: row.get("current_price")?,
            current_price_updated,
            phantom_remove: false, // This is not persisted
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
            remaining_token_amount: row
                .get::<_, Option<i64>>("remaining_token_amount")?
                .map(|t| t as u64),
            total_exited_amount: row.get::<_, i64>("total_exited_amount")? as u64,
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

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::PositionsDatabase;

    #[test]
    fn chain_migration_preserves_legacy_roots_and_children_and_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE positions (id INTEGER PRIMARY KEY, wallet_address TEXT NOT NULL, round_key TEXT);
             CREATE TABLE position_states (id INTEGER PRIMARY KEY, position_id INTEGER NOT NULL REFERENCES positions(id));
             INSERT INTO positions (id, wallet_address, round_key) VALUES (41, 'wallet', 'round');
             INSERT INTO position_states (id, position_id) VALUES (7, 41);
             CREATE UNIQUE INDEX idx_positions_round_key ON positions(wallet_address, round_key) WHERE round_key IS NOT NULL;",
        ).unwrap();

        PositionsDatabase::migrate_chain_identity(&connection).unwrap();
        PositionsDatabase::migrate_chain_identity(&connection).unwrap();

        assert_eq!(
            connection
                .query_row("SELECT chain_id FROM positions WHERE id=41", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "solana"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT position_id FROM position_states WHERE id=7",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            41
        );
        connection.execute(
            "INSERT INTO positions (id, wallet_address, round_key, chain_id) VALUES (?1, ?2, ?3, ?4)",
            params![42, "wallet", "round", "future-chain"],
        ).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
    }
}
