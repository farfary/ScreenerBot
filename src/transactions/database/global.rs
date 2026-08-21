//! Global transaction database singleton — shared access to the transaction data store.
//
// Global database instance management

use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::Mutex;

use crate::{
    chains::ChainId,
    logger::{self, LogTag},
};

use super::operations::TransactionDatabase;

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global database instance for cross-module access
static GLOBAL_TRANSACTION_DATABASE: LazyLock<Arc<Mutex<Option<Arc<TransactionDatabase>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Initialize global transaction database
pub async fn init_transaction_database() -> Result<Arc<TransactionDatabase>, String> {
    let db = TransactionDatabase::new(ChainId::Solana).await?;
    let db_arc = Arc::new(db);

    let mut global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    *global = Some(Arc::clone(&db_arc));

    logger::info(
        LogTag::Transactions,
        "Global transaction database initialized",
    );
    Ok(db_arc)
}

/// Get global transaction database instance
pub async fn get_transaction_database() -> Option<Arc<TransactionDatabase>> {
    let global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    global.as_ref().map(Arc::clone)
}
