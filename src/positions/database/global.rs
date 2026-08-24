//! Global position database singleton — shared access to the positions data store.

use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

use crate::logger::{self, LogTag};
use crate::positions::{Error, Result};

use super::types::PositionsDatabase;

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global positions database instance
pub(super) static GLOBAL_POSITIONS_DB: LazyLock<Arc<Mutex<Option<PositionsDatabase>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global positions database
pub async fn initialize_positions_database() -> Result<()> {
    let mut db_lock = GLOBAL_POSITIONS_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = PositionsDatabase::new(crate::chains::active_chain()).await?;
    *db_lock = Some(db);

    logger::info(
        LogTag::Positions,
        "Global positions database initialized successfully",
    );
    Ok(())
}

/// Get reference to global positions database
pub async fn get_positions_database() -> Result<Arc<Mutex<Option<PositionsDatabase>>>> {
    Ok(GLOBAL_POSITIONS_DB.clone())
}

/// Execute operation with global database
pub async fn with_positions_database<F, R>(operation: F) -> Result<R>
where
    F: FnOnce(&PositionsDatabase) -> Result<R>,
{
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => operation(db),
        None => Err(Error::NotInitialised),
    }
}

/// Execute async operation with global database
pub async fn with_positions_database_async<F, Fut, R>(operation: F) -> Result<R>
where
    F: FnOnce(&PositionsDatabase) -> Fut,
    Fut: std::future::Future<Output = Result<R>>,
{
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = operation(db).await;
            result
        }
        None => Err(Error::NotInitialised),
    }
}
