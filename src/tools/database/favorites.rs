//! Tool favorites database operations

use chrono::Utc;
use rusqlite::params;

use super::schema::get_connection;
use super::types::ToolFavoriteRow;

// =============================================================================
// TOOL FAVORITES OPERATIONS
// =============================================================================

/// Add or update a tool favorite (upsert)
pub fn upsert_tool_favorite(
    mint: &str,
    symbol: Option<&str>,
    name: Option<&str>,
    logo_url: Option<&str>,
    tool_type: &str,
    config_json: Option<&str>,
    label: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO tool_favorites (mint, symbol, name, logo_url, tool_type, config_json, label, notes, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
        ON CONFLICT(mint, tool_type) DO UPDATE SET
            symbol = COALESCE(?2, symbol),
            name = COALESCE(?3, name),
            logo_url = COALESCE(?4, logo_url),
            config_json = COALESCE(?6, config_json),
            label = COALESCE(?7, label),
            notes = COALESCE(?8, notes),
            updated_at = ?9
        "#,
        params![mint, symbol, name, logo_url, tool_type, config_json, label, notes, now],
    )
    .map_err(|e| format!("Failed to upsert tool favorite: {e}"))?;

    // Get the ID (either inserted or existing)
    conn.query_row(
        "SELECT id FROM tool_favorites WHERE mint = ?1 AND tool_type = ?2",
        params![mint, tool_type],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get favorite ID: {e}"))
}

/// Get all tool favorites, optionally filtered by tool type
pub fn get_tool_favorites(tool_type: Option<&str>) -> Result<Vec<ToolFavoriteRow>, String> {
    let conn = get_connection()?;

    let mut favorites = Vec::new();

    if let Some(tt) = tool_type {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, mint, symbol, name, logo_url, tool_type, config_json, label, notes,
                       use_count, last_used_at, created_at, updated_at
                FROM tool_favorites
                WHERE tool_type = ?1
                ORDER BY use_count DESC, updated_at DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let rows = stmt
            .query_map(params![tt], |row| Ok(ToolFavoriteRow::from_row(row)))
            .map_err(|e| format!("Failed to query favorites: {e}"))?;

        for row in rows {
            match row {
                Ok(Ok(fav)) => favorites.push(fav),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Failed to read row: {e}")),
            }
        }
    } else {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, mint, symbol, name, logo_url, tool_type, config_json, label, notes,
                       use_count, last_used_at, created_at, updated_at
                FROM tool_favorites
                ORDER BY use_count DESC, updated_at DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let rows = stmt
            .query_map([], |row| Ok(ToolFavoriteRow::from_row(row)))
            .map_err(|e| format!("Failed to query favorites: {e}"))?;

        for row in rows {
            match row {
                Ok(Ok(fav)) => favorites.push(fav),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Failed to read row: {e}")),
            }
        }
    }

    Ok(favorites)
}

/// Remove a tool favorite by ID
pub fn remove_tool_favorite(id: i64) -> Result<bool, String> {
    let conn = get_connection()?;

    let rows = conn
        .execute("DELETE FROM tool_favorites WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to remove tool favorite: {e}"))?;

    Ok(rows > 0)
}

/// Increment use count for a favorite
pub fn increment_tool_favorite_use(id: i64) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE tool_favorites SET use_count = use_count + 1, last_used_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| format!("Failed to increment use count: {e}"))?;

    Ok(())
}

/// Update a tool favorite's config/label/notes
pub fn update_tool_favorite(
    id: i64,
    config_json: Option<&str>,
    label: Option<&str>,
    notes: Option<&str>,
) -> Result<bool, String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    let rows = conn
        .execute(
            r#"
            UPDATE tool_favorites SET
                config_json = COALESCE(?1, config_json),
                label = COALESCE(?2, label),
                notes = COALESCE(?3, notes),
                updated_at = ?4
            WHERE id = ?5
            "#,
            params![config_json, label, notes, now, id],
        )
        .map_err(|e| format!("Failed to update tool favorite: {e}"))?;

    Ok(rows > 0)
}
