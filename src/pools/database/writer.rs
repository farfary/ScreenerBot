//! Background writer task for async database writes

use super::super::types::PriceResult;
use super::types::DbPriceResult;
use crate::chains::ChainId;
use crate::logger::{self, LogTag};

use rusqlite::{params, Connection};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

/// Batch size for database operations
const DB_BATCH_SIZE: usize = 100;

/// Database write interval (seconds)
const DB_WRITE_INTERVAL_SECONDS: u64 = 10;

// =============================================================================
// BACKGROUND TASKS
// =============================================================================

/// Background task for batched database writes
pub(super) async fn run_database_writer(
    mut rx: mpsc::UnboundedReceiver<PriceResult>,
    db_connection: Arc<Mutex<Option<Connection>>>,
    chain_id: ChainId,
) {
    let mut write_buffer = Vec::with_capacity(DB_BATCH_SIZE);
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(DB_WRITE_INTERVAL_SECONDS));

    loop {
        tokio::select! {
          // Collect prices from queue
          price = rx.recv() => {
            match price {
              Some(price) => {
                write_buffer.push(price);

                // Flush if buffer is full
                if write_buffer.len() >= DB_BATCH_SIZE {
                  flush_write_buffer(&mut write_buffer, &db_connection, chain_id).await;
                }
              }
              None => {
                // Channel closed, flush remaining and exit
                flush_write_buffer(&mut write_buffer, &db_connection, chain_id).await;
                break;
              }
            }
          }

          // Periodic flush
          _ = interval.tick() => {
            if !write_buffer.is_empty() {
              flush_write_buffer(&mut write_buffer, &db_connection, chain_id).await;
            }
          }
        }
    }
}

/// Flush the write buffer to database
async fn flush_write_buffer(
    buffer: &mut Vec<PriceResult>,
    db_connection: &Arc<Mutex<Option<Connection>>>,
    chain_id: ChainId,
) {
    if buffer.is_empty() {
        return;
    }

    let entries: Vec<PriceResult> = buffer.drain(..).collect();
    let entries_for_task = entries.clone();
    let conn_arc = db_connection.clone();

    match tokio::task::spawn_blocking(move || {
        let connection_guard = conn_arc
            .lock()
            .map_err(|e| format!("Failed to lock connection: {e}"))?;

        let conn = match connection_guard.as_ref() {
            Some(conn) => conn,
            None => return Ok::<usize, String>(0),
        };

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start price history transaction: {e}"))?;

        let mut inserted = 0usize;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO price_history 
           (chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
           timestamp_unix, sol_reserves, token_reserves, source_pool, created_at) 
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .map_err(|e| format!("Failed to prepare price history insert: {e}"))?;

            for price in &entries_for_task {
                let db_price = DbPriceResult::from_price_result(chain_id, price);

                inserted += stmt
                    .execute(params![
                        db_price.chain_id.as_str(),
                        db_price.mint,
                        db_price.pool_address,
                        db_price.price_usd,
                        db_price.price_sol,
                        db_price.confidence,
                        db_price.slot,
                        db_price.timestamp_unix,
                        db_price.sol_reserves,
                        db_price.token_reserves,
                        db_price.source_pool,
                        db_price.created_at.to_rfc3339()
                    ])
                    .map_err(|e| format!("Failed to insert price history entry: {e}"))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit price history transaction: {e}"))?;

        Ok::<usize, String>(inserted)
    })
    .await
    .map_err(|e| format!("Blocking task failed: {e}"))
    {
        Ok(Ok(inserted)) => {
            if inserted > 0 {
                logger::debug(
                    LogTag::PoolCache,
                    &format!("Stored {inserted} price history entries to database"),
                );
            }
        }
        Ok(Err(err)) => {
            buffer.extend(entries.into_iter());
            logger::error(
                LogTag::PoolCache,
                &format!("Failed to persist price history batch: {err}"),
            );
        }
        Err(join_err) => {
            buffer.extend(entries.into_iter());
            logger::error(
                LogTag::PoolCache,
                &format!("Price history writer task panicked: {join_err}"),
            );
        }
    }
}
