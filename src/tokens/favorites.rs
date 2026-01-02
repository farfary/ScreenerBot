/// Token favorites system
/// Allows users to save tokens to a favorites list with optional notes
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::tokens::database::get_global_database;
use crate::tokens::types::{TokenError, TokenResult};

// =============================================================================
// TYPES
// =============================================================================

/// A favorite token with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteToken {
    pub id: i64,
    pub mint: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to add a new favorite
#[derive(Debug, Clone, Deserialize)]
pub struct AddFavoriteRequest {
    pub mint: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo_url: Option<String>,
    pub notes: Option<String>,
}

/// Request to update an existing favorite
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFavoriteRequest {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub notes: Option<String>,
}

// =============================================================================
// SCHEMA
// =============================================================================

/// SQL to create the favorites table
pub const CREATE_FAVORITES_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS token_favorites (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL UNIQUE,
        name TEXT,
        symbol TEXT,
        logo_url TEXT,
        notes TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
"#;

/// SQL to create indexes for favorites
pub const CREATE_FAVORITES_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_favorites_mint ON token_favorites(mint)",
    "CREATE INDEX IF NOT EXISTS idx_favorites_created ON token_favorites(created_at DESC)",
];

/// Initialize favorites schema (called from database initialization)
pub fn initialize_favorites_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(CREATE_FAVORITES_TABLE, [])
        .map_err(|e| format!("Failed to create token_favorites table: {}", e))?;

    for statement in CREATE_FAVORITES_INDEXES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create favorites index: {}", e))?;
    }

    Ok(())
}

// =============================================================================
// DATABASE OPERATIONS
// =============================================================================

/// Add a token to favorites
pub fn add_favorite(
    conn: &Mutex<Connection>,
    request: &AddFavoriteRequest,
) -> TokenResult<FavoriteToken> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    conn.execute(
        r#"
        INSERT INTO token_favorites (mint, name, symbol, logo_url, notes, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
        ON CONFLICT(mint) DO UPDATE SET
            name = COALESCE(excluded.name, token_favorites.name),
            symbol = COALESCE(excluded.symbol, token_favorites.symbol),
            logo_url = COALESCE(excluded.logo_url, token_favorites.logo_url),
            notes = COALESCE(excluded.notes, token_favorites.notes),
            updated_at = datetime('now')
        "#,
        params![
            request.mint,
            request.name,
            request.symbol,
            request.logo_url,
            request.notes
        ],
    )
    .map_err(|e| TokenError::Database(format!("Failed to add favorite: {}", e)))?;

    // Fetch the newly created/updated favorite
    get_favorite_internal(&conn, &request.mint)?
        .ok_or_else(|| TokenError::Database("Failed to retrieve favorite after insert".to_string()))
}

/// Remove a token from favorites
pub fn remove_favorite(conn: &Mutex<Connection>, mint: &str) -> TokenResult<bool> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    let rows_affected = conn
        .execute("DELETE FROM token_favorites WHERE mint = ?1", params![mint])
        .map_err(|e| TokenError::Database(format!("Failed to remove favorite: {}", e)))?;

    Ok(rows_affected > 0)
}

/// Get all favorites ordered by creation date (newest first)
pub fn get_favorites(conn: &Mutex<Connection>) -> TokenResult<Vec<FavoriteToken>> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, name, symbol, logo_url, notes, created_at, updated_at
            FROM token_favorites
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|e| TokenError::Database(format!("Failed to prepare query: {}", e)))?;

    let favorites = stmt
        .query_map([], |row| {
            Ok(FavoriteToken {
                id: row.get(0)?,
                mint: row.get(1)?,
                name: row.get(2)?,
                symbol: row.get(3)?,
                logo_url: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| TokenError::Database(format!("Failed to query favorites: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TokenError::Database(format!("Failed to collect favorites: {}", e)))?;

    Ok(favorites)
}

/// Get a single favorite by mint address (internal helper, conn already locked)
fn get_favorite_internal(conn: &Connection, mint: &str) -> TokenResult<Option<FavoriteToken>> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, name, symbol, logo_url, notes, created_at, updated_at
            FROM token_favorites
            WHERE mint = ?1
            "#,
        )
        .map_err(|e| TokenError::Database(format!("Failed to prepare query: {}", e)))?;

    let favorite = stmt
        .query_row(params![mint], |row| {
            Ok(FavoriteToken {
                id: row.get(0)?,
                mint: row.get(1)?,
                name: row.get(2)?,
                symbol: row.get(3)?,
                logo_url: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .optional()
        .map_err(|e| TokenError::Database(format!("Failed to query favorite: {}", e)))?;

    Ok(favorite)
}

/// Get a single favorite by mint address
pub fn get_favorite(conn: &Mutex<Connection>, mint: &str) -> TokenResult<Option<FavoriteToken>> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    get_favorite_internal(&conn, mint)
}

/// Update a favorite's metadata/notes
pub fn update_favorite(
    conn: &Mutex<Connection>,
    mint: &str,
    request: &UpdateFavoriteRequest,
) -> TokenResult<Option<FavoriteToken>> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    // Build dynamic update query based on provided fields
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = request.name {
        updates.push("name = ?");
        values.push(Box::new(name.clone()));
    }
    if let Some(ref symbol) = request.symbol {
        updates.push("symbol = ?");
        values.push(Box::new(symbol.clone()));
    }
    if let Some(ref notes) = request.notes {
        updates.push("notes = ?");
        values.push(Box::new(notes.clone()));
    }

    if updates.is_empty() {
        // No updates provided, just return current favorite
        return get_favorite_internal(&conn, mint);
    }

    updates.push("updated_at = datetime('now')");
    values.push(Box::new(mint.to_string()));

    let sql = format!(
        "UPDATE token_favorites SET {} WHERE mint = ?",
        updates.join(", ")
    );

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();

    conn.execute(&sql, params.as_slice())
        .map_err(|e| TokenError::Database(format!("Failed to update favorite: {}", e)))?;

    get_favorite_internal(&conn, mint)
}

/// Check if a token is in favorites
pub fn is_favorite(conn: &Mutex<Connection>, mint: &str) -> bool {
    let Ok(conn) = conn.lock() else {
        return false;
    };

    conn.query_row(
        "SELECT 1 FROM token_favorites WHERE mint = ?1",
        params![mint],
        |_| Ok(()),
    )
    .is_ok()
}

/// Get count of favorites
pub fn get_favorites_count(conn: &Mutex<Connection>) -> TokenResult<usize> {
    let conn = conn
        .lock()
        .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM token_favorites", [], |row| row.get(0))
        .map_err(|e| TokenError::Database(format!("Failed to count favorites: {}", e)))?;

    Ok(count as usize)
}

// =============================================================================
// ASYNC WRAPPERS
// =============================================================================

/// Add a favorite (async wrapper)
pub async fn add_favorite_async(request: AddFavoriteRequest) -> TokenResult<FavoriteToken> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || add_favorite(&db.connection(), &request))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}

/// Remove a favorite (async wrapper)
pub async fn remove_favorite_async(mint: String) -> TokenResult<bool> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || remove_favorite(&db.connection(), &mint))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}

/// Get all favorites (async wrapper)
pub async fn get_favorites_async() -> TokenResult<Vec<FavoriteToken>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || get_favorites(&db.connection()))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}

/// Get a single favorite (async wrapper)
pub async fn get_favorite_async(mint: String) -> TokenResult<Option<FavoriteToken>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || get_favorite(&db.connection(), &mint))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}

/// Update a favorite (async wrapper)
pub async fn update_favorite_async(
    mint: String,
    request: UpdateFavoriteRequest,
) -> TokenResult<Option<FavoriteToken>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || update_favorite(&db.connection(), &mint, &request))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}

/// Check if a token is in favorites (async wrapper)
pub async fn is_favorite_async(mint: String) -> bool {
    let Some(db) = get_global_database() else {
        return false;
    };

    tokio::task::spawn_blocking(move || is_favorite(&db.connection(), &mint))
        .await
        .unwrap_or(false)
}

/// Get count of favorites (async wrapper)
pub async fn get_favorites_count_async() -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || get_favorites_count(&db.connection()))
        .await
        .map_err(|e| TokenError::Database(format!("Task join error: {}", e)))?
}
