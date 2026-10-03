// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-schema inspection for the per-domain idempotent migrations.

use rusqlite::Connection;

/// Whether `table` already carries `column`: the live-schema gate
/// (`PRAGMA table_info`) every structural migration checks instead of a
/// version stamp. Domains map the error onto their own error type.
pub fn table_has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns.iter().any(|name| name == column))
}
