// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Blocking-pool dispatch for synchronous token-database calls made by the update tasks.

use crate::errors::InternalError;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;
use std::sync::Arc;

/// Runs a synchronous token-database call on tokio's blocking pool.
///
/// The update loops and their per-token writes take the pooled SQLite connection and
/// selection queries can run for seconds on a large database; issued directly inside an
/// async fn they park a runtime worker for that whole time and stall every task
/// scheduled on it.
pub(super) async fn blocking_db<T, F>(db: &Arc<TokenDatabase>, call: F) -> TokenResult<T>
where
    T: Send + 'static,
    F: FnOnce(&TokenDatabase) -> TokenResult<T> + Send + 'static,
{
    let db = Arc::clone(db);
    tokio::task::spawn_blocking(move || call(&db))
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
}
