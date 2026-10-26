// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The ATA cleanup store: its schema and the failed-close cache that parks an account between retries.

mod failed_cache;
mod schema;

pub use failed_cache::{
    get_excluded_atas_for_wallet, get_failed_atas_for_wallet, remove_failed_ata,
    upsert_failed_ata, FailedAtaRow,
};
pub use schema::init_ata_cleanup_db;
pub(crate) use schema::SCHEMA;
