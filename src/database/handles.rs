// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The connection pool each catalog store's owner opened, recorded so housekeeping reaches a store only through that pool and never opens a store file by path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, PoisonError, RwLock};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::catalog::StoreSpec;

/// An open catalog store: its entry, its file and the pool its owner checks connections
/// out of.
#[derive(Clone)]
pub struct StoreHandle {
    pub spec: &'static StoreSpec,
    pub path: PathBuf,
    pub pool: Pool<SqliteConnectionManager>,
}

static HANDLES: LazyLock<RwLock<BTreeMap<PathBuf, StoreHandle>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Record the pool `spec`'s owner opened at `path` once the store is open. A pool opened
/// anywhere other than one of the store's own files (an isolated test file, an offline
/// tool) is not the store and is not recorded.
pub fn register(spec: &'static StoreSpec, path: &Path, pool: &Pool<SqliteConnectionManager>) {
    if !spec.paths().iter().any(|file| file == path) {
        return;
    }
    HANDLES
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(
            path.to_path_buf(),
            StoreHandle {
                spec,
                path: path.to_path_buf(),
                pool: pool.clone(),
            },
        );
}

/// Every store opened in this process, in path order.
pub fn open_stores() -> Vec<StoreHandle> {
    HANDLES
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .values()
        .cloned()
        .collect()
}
