// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Database initialization and connection pooling for SQLite storage.
//
// All SQLite connections must use `configure::configure_connection()` via
// `with_init()` to ensure PRAGMAs survive connection pool recycling.

pub mod backup;
pub mod configure;
pub mod maintenance;
pub mod schema;
pub mod transaction;

pub use configure::*;
pub use maintenance::start_maintenance_task as start_db_maintenance_task;
pub use transaction::WriteTransaction;
