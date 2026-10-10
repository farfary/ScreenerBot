// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Failures of storage housekeeping: measuring the data directory, deleting a file a retention rule selected, and checkpointing or compacting an open store.

use super::DatabaseError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    /// A file, directory or disk could not be measured.
    #[error("could not measure {path}: {message}")]
    Measure { path: String, message: String },
    /// A file a retention rule selected could not be removed.
    #[error("could not delete {path}: {message}")]
    Delete { path: String, message: String },
    /// A selected path no longer sits directly in its family's directory, or is no
    /// longer a plain file, so it was left in place.
    #[error("{path} is not a plain entry of {root}; left in place")]
    OutsideRoot { path: String, root: String },
    /// A WAL checkpoint, conversion or compaction of an open store failed.
    #[error("housekeeping of {store} failed: {source}")]
    Store {
        store: String,
        #[source]
        source: DatabaseError,
    },
    /// A blocking housekeeping step ended without a result.
    #[error("{operation} did not complete: {message}")]
    Interrupted { operation: String, message: String },
}
