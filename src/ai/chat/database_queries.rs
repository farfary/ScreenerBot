//! AI Chat Database Query Operations
//!
//! CRUD operations for chat sessions, messages, and tool executions.
//! Data types are defined in `database.rs`; this module contains the query functions.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};

use super::database::{ChatMessage, ChatSession, ToolExecution};

// =============================================================================
// SESSION CRUD OPERATIONS
// =============================================================================

/// Create a new chat session
pub fn create_session(pool: &Pool<SqliteConnectionManager>, title: &str) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO chat_sessions (title, created_at, updated_at) VALUES (?1, ?2, ?3)",
        params![title, &now, &now],
    )
    .map_err(|e| format!("Failed to insert session: {e}"))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Create a hidden chat session (for scheduled task runs)
pub fn create_hidden_session(
    pool: &Pool<SqliteConnectionManager>,
    title: &str,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO chat_sessions (title, is_hidden, created_at, updated_at) VALUES (?1, 1, ?2, ?3)",
        params![title, &now, &now],
    )
    .map_err(|e| format!("Failed to insert hidden session: {e}"))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Get all chat sessions ordered by most recent
pub fn get_sessions(pool: &Pool<SqliteConnectionManager>) -> Result<Vec<ChatSession>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.summary, COUNT(m.id) as message_count, 
                    s.created_at, s.updated_at 
             FROM chat_sessions s 
             LEFT JOIN chat_messages m ON s.id = m.session_id 
             WHERE s.is_hidden = 0
             GROUP BY s.id 
             ORDER BY s.updated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                message_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query sessions: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect sessions: {e}"))?;

    Ok(sessions)
}

/// Get a single session by ID
pub fn get_session(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
) -> Result<Option<ChatSession>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.summary, COUNT(m.id) as message_count, 
                    s.created_at, s.updated_at 
             FROM chat_sessions s 
             LEFT JOIN chat_messages m ON s.id = m.session_id 
             WHERE s.id = ?1 AND s.is_hidden = 0
             GROUP BY s.id",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let session = stmt
        .query_row(params![id], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                message_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query session: {e}"))?;

    Ok(session)
}

/// Update session summary
pub fn update_session_summary(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    summary: &str,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET summary = ?1, updated_at = ?2 WHERE id = ?3",
        params![summary, &now, id],
    )
    .map_err(|e| format!("Failed to update session summary: {e}"))?;

    Ok(())
}

/// Update session title
pub fn update_session_title(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    title: &str,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, &now, id],
    )
    .map_err(|e| format!("Failed to update session title: {e}"))?;

    Ok(())
}

/// Touch session (update updated_at timestamp)
pub fn touch_session(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![&now, id],
    )
    .map_err(|e| format!("Failed to touch session: {e}"))?;

    Ok(())
}

/// Delete a session (cascade deletes messages and executions)
pub fn delete_session(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    conn.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete session: {e}"))?;

    Ok(())
}

/// Delete hidden sessions older than the specified number of days
pub fn cleanup_hidden_sessions(
    pool: &Pool<SqliteConnectionManager>,
    older_than_days: i64,
) -> Result<usize, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let cutoff = (chrono::Utc::now() - chrono::Duration::days(older_than_days)).to_rfc3339();

    // Use transaction to ensure both deletes happen atomically
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {e}"))?;

    // Delete messages from old hidden sessions first
    tx.execute(
        "DELETE FROM chat_messages WHERE session_id IN (
            SELECT id FROM chat_sessions WHERE is_hidden = 1 AND created_at < ?1
        )",
        params![cutoff],
    )
    .map_err(|e| format!("Failed to delete hidden session messages: {e}"))?;

    // Delete the hidden sessions
    let deleted = tx
        .execute(
            "DELETE FROM chat_sessions WHERE is_hidden = 1 AND created_at < ?1",
            params![cutoff],
        )
        .map_err(|e| format!("Failed to delete hidden sessions: {e}"))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    Ok(deleted)
}

// =============================================================================
// MESSAGE CRUD OPERATIONS
// =============================================================================

/// Add a message to a session
pub fn add_message(
    pool: &Pool<SqliteConnectionManager>,
    session_id: i64,
    role: &str,
    content: &str,
    tool_calls: Option<&str>,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    tx.execute(
        "INSERT INTO chat_messages (session_id, role, content, tool_calls, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, role, content, tool_calls, &now],
    )
    .map_err(|e| format!("Failed to insert message: {e}"))?;

    let message_id = tx.last_insert_rowid();

    // Update session timestamp atomically with message insert
    tx.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![&now, session_id],
    )
    .map_err(|e| format!("Failed to update session timestamp: {e}"))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit message transaction: {e}"))?;

    Ok(message_id)
}

/// Get all messages for a session
pub fn get_messages(
    pool: &Pool<SqliteConnectionManager>,
    session_id: i64,
) -> Result<Vec<ChatMessage>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, tool_calls, created_at 
             FROM chat_messages 
             WHERE session_id = ?1 
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let messages = stmt
        .query_map(params![session_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query messages: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect messages: {e}"))?;

    Ok(messages)
}

/// Get a single message by ID
pub fn get_message(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
) -> Result<Option<ChatMessage>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, tool_calls, created_at 
             FROM chat_messages 
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let message = stmt
        .query_row(params![id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query message: {e}"))?;

    Ok(message)
}

/// Delete a message
pub fn delete_message(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    conn.execute("DELETE FROM chat_messages WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete message: {e}"))?;

    Ok(())
}

// =============================================================================
// TOOL EXECUTION OPERATIONS
// =============================================================================

/// Add a tool execution record
pub fn add_tool_execution(
    pool: &Pool<SqliteConnectionManager>,
    message_id: i64,
    tool_name: &str,
    tool_input: &str,
    tool_output: &str,
    status: &str,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO tool_executions 
         (message_id, tool_name, tool_input, tool_output, status, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![message_id, tool_name, tool_input, tool_output, status, &now],
    )
    .map_err(|e| format!("Failed to insert tool execution: {e}"))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Get all tool executions for a message
pub fn get_tool_executions(
    pool: &Pool<SqliteConnectionManager>,
    message_id: i64,
) -> Result<Vec<ToolExecution>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, message_id, tool_name, tool_input, tool_output, status, created_at 
             FROM tool_executions 
             WHERE message_id = ?1 
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let executions = stmt
        .query_map(params![message_id], |row| {
            Ok(ToolExecution {
                id: row.get(0)?,
                message_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_input: row.get(3)?,
                tool_output: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query tool executions: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tool executions: {e}"))?;

    Ok(executions)
}

/// Update tool execution status and output
pub fn update_tool_execution(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    tool_output: &str,
    status: &str,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    conn.execute(
        "UPDATE tool_executions SET tool_output = ?1, status = ?2 WHERE id = ?3",
        params![tool_output, status, id],
    )
    .map_err(|e| format!("Failed to update tool execution: {e}"))?;

    Ok(())
}
