//! Token favorites system
//! Allows users to save tokens to a favorites list with optional notes

use crate::errors::{DatabaseError, InternalError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::tokens::database::get_global_database;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

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
// DATABASE OPERATIONS
// =============================================================================

/// Add a token to favorites
pub fn add_favorite(
    conn: &Connection,
    chain_id: &str,
    request: &AddFavoriteRequest,
) -> TokenResult<FavoriteToken> {
    // Reject blank mints up front — an empty-mint favorite is unaddressable (it
    // can never map to a real token and can't be deleted by mint afterwards).
    if request.mint.trim().is_empty() {
        return Err(Error::InvalidMint {
            value: request.mint.clone(),
        });
    }

    conn.execute(
        r#"
        INSERT INTO token_favorites (chain_id, mint, name, symbol, logo_url, notes, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))
        ON CONFLICT(chain_id, mint) DO UPDATE SET
            name = COALESCE(excluded.name, token_favorites.name),
            symbol = COALESCE(excluded.symbol, token_favorites.symbol),
            logo_url = COALESCE(excluded.logo_url, token_favorites.logo_url),
            notes = COALESCE(excluded.notes, token_favorites.notes),
            updated_at = datetime('now')
        "#,
        params![
            chain_id, request.mint,
            request.name,
            request.symbol,
            request.logo_url,
            request.notes
        ],
    )
    .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to add favorite".to_owned(), message: e.to_string() }))?;

    // Fetch the newly created/updated favorite
    get_favorite_internal(&conn, chain_id, &request.mint)?.ok_or_else(|| {
        Error::Internal(InternalError::InvariantViolation {
            message: "favorite row missing immediately after insert".to_owned(),
        })
    })
}

/// Remove a token from favorites
pub fn remove_favorite(conn: &Connection, chain_id: &str, mint: &str) -> TokenResult<bool> {
    let rows_affected = conn
        .execute(
            "DELETE FROM token_favorites WHERE chain_id = ?1 AND mint = ?2",
            params![chain_id, mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to remove favorite".to_owned(),
                message: e.to_string(),
            })
        })?;

    Ok(rows_affected > 0)
}

/// Get all favorites ordered by creation date (newest first)
pub fn get_favorites(conn: &Connection, chain_id: &str) -> TokenResult<Vec<FavoriteToken>> {
    // Self-heal: purge any corrupt rows with a blank mint (legacy bad data that
    // can't be removed by mint and would otherwise show as an unremovable row).
    let _ = conn.execute(
        "DELETE FROM token_favorites WHERE chain_id = ?1 AND (mint IS NULL OR TRIM(mint) = '')",
        params![chain_id],
    );

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, name, symbol, logo_url, notes, created_at, updated_at
            FROM token_favorites
            WHERE chain_id = ?1 AND mint IS NOT NULL AND TRIM(mint) != ''
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare query".to_owned(),
                message: e.to_string(),
            })
        })?;

    let favorites = stmt
        .query_map(params![chain_id], |row| {
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
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to query favorites".to_owned(),
                message: e.to_string(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to collect favorites".to_owned(),
                message: e.to_string(),
            })
        })?;

    Ok(favorites)
}

/// Get a single favorite by mint address (internal helper, conn already locked)
fn get_favorite_internal(
    conn: &Connection,
    chain_id: &str,
    mint: &str,
) -> TokenResult<Option<FavoriteToken>> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, name, symbol, logo_url, notes, created_at, updated_at
            FROM token_favorites
            WHERE chain_id = ?1 AND mint = ?2
            "#,
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare query".to_owned(),
                message: e.to_string(),
            })
        })?;

    let favorite = stmt
        .query_row(params![chain_id, mint], |row| {
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
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to query favorite".to_owned(),
                message: e.to_string(),
            })
        })?;

    Ok(favorite)
}

/// Get a single favorite by mint address
pub fn get_favorite(
    conn: &Connection,
    chain_id: &str,
    mint: &str,
) -> TokenResult<Option<FavoriteToken>> {
    get_favorite_internal(&conn, chain_id, mint)
}

/// Update a favorite's metadata/notes
pub fn update_favorite(
    conn: &Connection,
    chain_id: &str,
    mint: &str,
    request: &UpdateFavoriteRequest,
) -> TokenResult<Option<FavoriteToken>> {
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
        return get_favorite_internal(&conn, chain_id, mint);
    }

    updates.push("updated_at = datetime('now')");
    values.push(Box::new(chain_id.to_string()));
    values.push(Box::new(mint.to_string()));

    let sql = format!(
        "UPDATE token_favorites SET {} WHERE chain_id = ? AND mint = ?",
        updates.join(", ")
    );

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();

    conn.execute(&sql, params.as_slice()).map_err(|e| {
        Error::Database(DatabaseError::Query {
            operation: "Failed to update favorite".to_owned(),
            message: e.to_string(),
        })
    })?;

    get_favorite_internal(&conn, chain_id, mint)
}

/// Check if a token is in favorites
pub fn is_favorite(conn: &Connection, chain_id: &str, mint: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM token_favorites WHERE chain_id = ?1 AND mint = ?2",
        params![chain_id, mint],
        |_| Ok(()),
    )
    .is_ok()
}

/// Get count of favorites
pub fn get_favorites_count(conn: &Connection, chain_id: &str) -> TokenResult<usize> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM token_favorites WHERE chain_id = ?1",
            params![chain_id],
            |row| row.get(0),
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to count favorites".to_owned(),
                message: e.to_string(),
            })
        })?;

    Ok(count as usize)
}

// =============================================================================
// ASYNC WRAPPERS
// =============================================================================

/// Add a favorite (async wrapper)
pub async fn add_favorite_async(request: AddFavoriteRequest) -> TokenResult<FavoriteToken> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        add_favorite(&conn, db.chain_id(), &request)
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}

/// Remove a favorite (async wrapper)
pub async fn remove_favorite_async(mint: String) -> TokenResult<bool> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        remove_favorite(&conn, db.chain_id(), &mint)
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}

/// Get all favorites (async wrapper)
pub async fn get_favorites_async() -> TokenResult<Vec<FavoriteToken>> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        get_favorites(&conn, db.chain_id())
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}

/// Get a single favorite (async wrapper)
pub async fn get_favorite_async(mint: String) -> TokenResult<Option<FavoriteToken>> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        get_favorite(&conn, db.chain_id(), &mint)
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}

/// Update a favorite (async wrapper)
pub async fn update_favorite_async(
    mint: String,
    request: UpdateFavoriteRequest,
) -> TokenResult<Option<FavoriteToken>> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        update_favorite(&conn, db.chain_id(), &mint, &request)
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}

/// Check if a token is in favorites (async wrapper)
pub async fn is_favorite_async(mint: String) -> bool {
    let Some(db) = get_global_database() else {
        return false;
    };

    tokio::task::spawn_blocking(move || match db.conn() {
        Ok(conn) => is_favorite(&conn, db.chain_id(), &mint),
        Err(_) => false,
    })
    .await
    .unwrap_or_default()
}

/// Get count of favorites (async wrapper)
pub async fn get_favorites_count_async() -> TokenResult<usize> {
    let db = get_global_database().ok_or_else(|| Error::NotInitialized {
        resource: "Token database not initialized".to_owned(),
    })?;

    tokio::task::spawn_blocking(move || {
        let conn = db.conn()?;
        get_favorites_count(&conn, db.chain_id())
    })
    .await
    .map_err(|e| Error::Internal(InternalError::from(e)))?
}
