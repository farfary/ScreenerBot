//! Persistence for subject-relative balance deltas (`subject_asset_deltas`).
//!
//! Sibling module to `operations.rs` / `operations_queries.rs`, extending
//! `TransactionDatabase` via another `impl` block, same style: pooled connection via
//! `self.get_connection()`, `String` errors, one `unchecked_transaction()` per batch
//! write.

use rusqlite::{params, OptionalExtension, Transaction as SqlTransaction};

use crate::chains::ChainId;
use crate::logger::{self, LogTag};
use crate::transactions::deltas::{DeltaKind, SubjectAssetDelta, SUBJECT_DELTAS_VERSION};
use crate::transactions::error::Error;

use super::operations::TransactionDatabase;
use crate::database::WriteTransaction;

/// `db_metadata` key guarding the once-only historical backfill.
const BACKFILL_METADATA_KEY: &str = "subject_deltas_backfill_version";
/// Rows read per page. An existing install's `raw_transactions` can be tens of
/// thousands of rows with large JSON blobs, so the whole table is never loaded at
/// once.
const BACKFILL_BATCH_SIZE: i64 = 500;

/// One cached `raw_transactions` row as both re-extraction passes read it:
/// `(signature, raw JSON, slot, block_time, success)`.
type CachedTransactionRow = (String, String, Option<i64>, Option<i64>, bool);

/// Rebuild the minimal [`Transaction`](crate::transactions::types::Transaction) that
/// `extract_subject_deltas` needs from a cached row, returning `None` when the blob is
/// not JSON at all.
///
/// The row's own `slot`/`block_time` columns are carried over deliberately: they are
/// correct even when the cached blob's encoding hides them, and the extractor falls back
/// to them so no delta is ever written without a timestamp.
fn cached_row_to_transaction(
    row: CachedTransactionRow,
) -> Option<crate::transactions::types::Transaction> {
    let (signature, raw_json, slot, block_time, success) = row;
    let raw_value = serde_json::from_str::<serde_json::Value>(&raw_json).ok()?;

    let mut transaction = crate::transactions::types::Transaction::new(signature);
    transaction.success = success;
    transaction.raw_transaction_data = Some(raw_value);
    transaction.slot = slot.and_then(|s| u64::try_from(s).ok());
    transaction.block_time = block_time;
    Some(transaction)
}

impl TransactionDatabase {
    /// Persist a batch of deltas in one transaction (`INSERT OR REPLACE`, keyed by
    /// `(wallet_address, signature, mint)`). A row whose `delta_raw` cannot fit in
    /// `i64` is logged and skipped rather than corrupting the ledger with a clamped
    /// value; `before_raw`/`after_raw` are informational and fall back to `NULL`.
    pub async fn store_subject_deltas(&self, deltas: &[SubjectAssetDelta]) -> Result<(), Error> {
        if deltas.is_empty() {
            return Ok(());
        }

        let mut conn = self.get_connection()?;
        let tx = conn
            .write_tx()
            .map_err(crate::errors::DatabaseError::from)?;

        for delta in deltas {
            if let Err(e) = self.insert_subject_delta(&tx, delta) {
                logger::warning(
                    LogTag::Transactions,
                    &format!(
                        "Skipping subject delta {} / {} / {}: {}",
                        delta.wallet_address, delta.signature, delta.mint, e
                    ),
                );
            }
        }

        tx.commit().map_err(crate::errors::DatabaseError::from)?;

        Ok(())
    }

    /// Extract and persist one decoded transaction's deltas for `wallet_address`.
    ///
    /// The ONE place a live transaction becomes ledger rows. Both paths that decode our
    /// own wallet call it — the bootstrap processor and the wallet-watch recorder — so a
    /// transaction observed while the bot runs lands in the ledger exactly like one
    /// discovered at startup. It used to be wired into the bootstrap path only, which
    /// meant every trade made while the app was running was recorded, marked known, and
    /// then never converted into a delta by anything: the position it should have opened
    /// or closed stayed invisible for good, because bootstrap skips known signatures.
    pub async fn store_transaction_deltas(
        &self,
        wallet_address: &str,
        transaction: &crate::transactions::types::Transaction,
    ) -> Result<usize, Error> {
        let deltas = crate::chains::solana::transactions::deltas::extract_subject_deltas(
            wallet_address,
            transaction,
        );
        let count = deltas.len();
        self.store_subject_deltas(&deltas).await?;
        Ok(count)
    }

    fn insert_subject_delta(
        &self,
        tx: &SqlTransaction,
        delta: &SubjectAssetDelta,
    ) -> Result<(), Error> {
        let delta_raw = i64::try_from(delta.delta_raw).map_err(|_| Error::RowDecode {
            column: "delta_raw",
            detail: format!("{} does not fit in i64", delta.delta_raw),
        })?;
        let before_raw = delta.before_raw.and_then(|v| i64::try_from(v).ok());
        let after_raw = delta.after_raw.and_then(|v| i64::try_from(v).ok());

        tx.execute(
            "INSERT OR REPLACE INTO subject_asset_deltas (
                chain_id, wallet_address, signature, mint, slot, block_time, tx_index,
                delta_raw, before_raw, after_raw, decimals, kind, venue,
                fee_lamports, success
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                delta.chain.as_str(),
                delta.wallet_address,
                delta.signature,
                delta.mint,
                delta.slot.map(|s| s as i64),
                delta.block_time,
                delta.tx_index,
                delta_raw,
                before_raw,
                after_raw,
                delta.decimals,
                delta.kind.as_str(),
                delta.venue,
                delta.fee_native_raw.map(|f| f as i64),
                delta.success,
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;

        Ok(())
    }

    /// Every stored delta for `wallet_address`, ordered `(slot, tx_index, signature)`.
    /// Rows with a NULL slot (never confirmed / not yet backfilled) sort last.
    pub async fn get_subject_deltas(
        &self,
        wallet_address: &str,
    ) -> Result<Vec<SubjectAssetDelta>, Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT chain_id, wallet_address, signature, mint, slot, block_time, tx_index,
                    delta_raw, before_raw, after_raw, decimals, kind, venue,
                    fee_lamports, success
                 FROM subject_asset_deltas
                 WHERE chain_id = ?1 AND wallet_address = ?2
                 ORDER BY slot IS NULL, slot ASC, tx_index ASC, signature ASC",
            )
            .map_err(crate::errors::DatabaseError::from)?;

        let rows = stmt
            .query_map(params![self.chain.as_str(), wallet_address], |row| {
                let chain_str: String = row.get(0)?;
                let chain = chain_str.parse::<ChainId>().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        e.into(),
                    )
                })?;
                let kind_str: String = row.get(11)?;
                let kind = DeltaKind::parse(&kind_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        11,
                        rusqlite::types::Type::Text,
                        e.into(),
                    )
                })?;

                Ok(SubjectAssetDelta {
                    chain,
                    wallet_address: row.get(1)?,
                    signature: row.get(2)?,
                    mint: row.get(3)?,
                    slot: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    block_time: row.get(5)?,
                    tx_index: row.get::<_, i64>(6)? as u32,
                    delta_raw: row.get::<_, i64>(7)? as i128,
                    before_raw: row
                        .get::<_, Option<i64>>(8)?
                        .and_then(|v| u128::try_from(v).ok()),
                    after_raw: row
                        .get::<_, Option<i64>>(9)?
                        .and_then(|v| u128::try_from(v).ok()),
                    decimals: row.get::<_, i64>(10)? as u8,
                    kind,
                    venue: row.get(12)?,
                    fee_native_raw: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
                    success: row.get(14)?,
                })
            })
            .map_err(crate::errors::DatabaseError::from)?;

        let mut deltas = Vec::new();
        for row in rows {
            deltas.push(row.map_err(crate::errors::DatabaseError::from)?);
        }

        Ok(deltas)
    }

    /// Rebuild `subject_asset_deltas` from the full `raw_transactions` history for
    /// `wallet_address`, once per [`SUBJECT_DELTAS_VERSION`].
    ///
    /// The live hook in `TransactionProcessor::process_transaction` only extracts
    /// deltas for transactions processed FROM NOW ON. An existing install already
    /// has its whole history sitting in `raw_transactions` with every signature
    /// marked known -- bootstrap skips known signatures -- so without this,
    /// `subject_asset_deltas` (and everything built on it) would stay empty
    /// forever on an upgrade. Pages the table instead of loading it whole.
    pub async fn backfill_subject_deltas(&self, wallet_address: &str) -> Result<u64, Error> {
        {
            let conn = self.get_connection()?;
            let stored: Option<String> = conn
                .query_row(
                    "SELECT value FROM db_metadata WHERE key = ?1",
                    params![BACKFILL_METADATA_KEY],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| Error::DeltaBackfill {
                    detail: format!("failed to read subject deltas backfill version: {e}"),
                })?;

            if stored.as_deref() == Some(SUBJECT_DELTAS_VERSION.to_string().as_str()) {
                return Ok(0);
            }
        }

        logger::info(
            LogTag::Transactions,
            &format!(
                "Backfilling subject_asset_deltas for {wallet_address} (v{SUBJECT_DELTAS_VERSION})..."
            ),
        );

        let mut total_transactions: u64 = 0;
        let mut total_deltas: u64 = 0;
        let mut offset: i64 = 0;

        loop {
            let batch: Vec<CachedTransactionRow> = {
                let conn = self.get_connection()?;
                let mut stmt = conn
                    .prepare(
                        "SELECT signature, raw_transaction_data, slot, block_time, success
                         FROM raw_transactions
                         WHERE chain_id = ?1 AND wallet_address = ?2 AND raw_transaction_data IS NOT NULL
                         ORDER BY signature
                         LIMIT ?3 OFFSET ?4",
                    )
                    .map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to prepare subject deltas backfill query: {e}"),
                    })?;

                let rows = stmt
                    .query_map(
                        params![
                            self.chain.as_str(),
                            wallet_address,
                            BACKFILL_BATCH_SIZE,
                            offset
                        ],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get::<_, Option<i64>>(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        },
                    )
                    .map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to execute subject deltas backfill query: {e}"),
                    })?;

                let mut collected = Vec::new();
                for row in rows {
                    collected.push(row.map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to read backfill row: {e}"),
                    })?);
                }
                collected
            };

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len();
            let mut batch_deltas = Vec::new();

            for row in batch {
                let Some(transaction) = cached_row_to_transaction(row) else {
                    continue;
                };

                let deltas = crate::chains::solana::transactions::deltas::extract_subject_deltas(
                    wallet_address,
                    &transaction,
                );
                batch_deltas.extend(deltas);
            }

            total_deltas += batch_deltas.len() as u64;
            self.store_subject_deltas(&batch_deltas).await?;

            total_transactions += batch_len as u64;
            offset += batch_len as i64;

            logger::info(
                LogTag::Transactions,
                &format!(
                    "Subject deltas backfill progress: {total_transactions} transactions scanned, {total_deltas} deltas written so far"
                ),
            );

            if (batch_len as i64) < BACKFILL_BATCH_SIZE {
                break;
            }
        }

        {
            let conn = self.get_connection()?;
            conn.execute(
                "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
                params![BACKFILL_METADATA_KEY, SUBJECT_DELTAS_VERSION.to_string()],
            )
            .map_err(|e| Error::DeltaBackfill {
                detail: format!("failed to persist subject deltas backfill version: {e}"),
            })?;
        }

        logger::info(
            LogTag::Transactions,
            &format!(
                "Subject deltas backfill complete: {total_transactions} transactions scanned, {total_deltas} deltas written"
            ),
        );

        Ok(total_deltas)
    }

    /// Extract deltas for any recent own-wallet transaction that has none — the
    /// self-healing counterpart to the once-per-version backfill.
    ///
    /// The full backfill runs once and never again, so a transaction that was stored
    /// while the extraction was unavailable (a crash between recording and extraction, a
    /// write that failed, a build where the live hook was missing) would stay outside
    /// the ledger for the life of the install. This pass costs one indexed scan per boot
    /// because it starts at the newest slot the ledger already knows about: everything
    /// older is, by construction, already reduced.
    ///
    /// A transaction that genuinely moves nothing produces no delta rows and is
    /// re-examined on the next boot; that is a handful of rows, not a rescan.
    pub async fn fill_subject_delta_gaps(&self, wallet_address: &str) -> Result<u64, Error> {
        let watermark: i64 = {
            let conn = self.get_connection()?;
            conn.query_row(
                "SELECT COALESCE(MAX(slot), 0) FROM subject_asset_deltas WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| Error::DeltaBackfill {
                detail: format!("failed to read subject deltas watermark: {e}"),
            })?
        };

        let mut cursor = String::new();
        let mut repaired: u64 = 0;

        loop {
            let batch: Vec<CachedTransactionRow> = {
                let conn = self.get_connection()?;
                let mut stmt = conn
                    .prepare(
                        "SELECT r.signature, r.raw_transaction_data, r.slot, r.block_time, r.success
                         FROM raw_transactions r
                         WHERE r.chain_id = ?1 AND r.wallet_address = ?2
                           AND r.raw_transaction_data IS NOT NULL
                           AND (r.slot IS NULL OR r.slot >= ?3)
                           AND r.signature > ?4
                           AND NOT EXISTS (
                             SELECT 1 FROM subject_asset_deltas d
                             WHERE d.chain_id = r.chain_id
                               AND d.wallet_address = r.wallet_address
                               AND d.signature = r.signature
                           )
                         ORDER BY r.signature
                         LIMIT ?5",
                    )
                    .map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to prepare subject deltas gap query: {e}"),
                    })?;

                let rows = stmt
                    .query_map(
                        params![
                            self.chain.as_str(),
                            wallet_address,
                            watermark,
                            cursor,
                            BACKFILL_BATCH_SIZE
                        ],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get::<_, Option<i64>>(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        },
                    )
                    .map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to execute subject deltas gap query: {e}"),
                    })?;

                let mut collected = Vec::new();
                for row in rows {
                    collected.push(row.map_err(|e| Error::DeltaBackfill {
                        detail: format!("failed to read gap row: {e}"),
                    })?);
                }
                collected
            };

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len();
            let mut batch_deltas = Vec::new();

            for row in batch {
                // Paged by signature, not offset: rows leave the result set as their
                // deltas are written, so an offset would skip the ones that shifted up.
                cursor = row.0.clone();

                let Some(transaction) = cached_row_to_transaction(row) else {
                    continue;
                };

                let deltas = crate::chains::solana::transactions::deltas::extract_subject_deltas(
                    wallet_address,
                    &transaction,
                );
                if !deltas.is_empty() {
                    repaired += 1;
                    batch_deltas.extend(deltas);
                }
            }

            self.store_subject_deltas(&batch_deltas).await?;

            if (batch_len as i64) < BACKFILL_BATCH_SIZE {
                break;
            }
        }

        if repaired > 0 {
            logger::info(
                LogTag::Transactions,
                &format!("Subject deltas gap fill repaired {repaired} transactions"),
            );
        }

        Ok(repaired)
    }

    /// Count of stored deltas for `wallet_address`.
    pub async fn count_subject_deltas(&self, wallet_address: &str) -> Result<u64, Error> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subject_asset_deltas WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(count as u64)
    }
}
