//! AI Chat Database Module
//!
//! SQLite persistence for AI Assistant chat sessions:
//! - Chat sessions with titles and summaries
//! - Chat messages with role, content, and tool calls
//! - Tool execution tracking with inputs/outputs

use crate::logger::{self, LogTag};
use once_cell::sync::OnceCell;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// GLOBAL CONNECTION POOL
// =============================================================================

static GLOBAL_CHAT_POOL: OnceCell<Arc<Pool<SqliteConnectionManager>>> = OnceCell::new();

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Chat session with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub message_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Chat message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String, // "user", "assistant", or "system"
    pub content: String,
    pub tool_calls: Option<String>, // JSON array of tool calls
    pub created_at: String,
}

/// Tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: i64,
    pub message_id: i64,
    pub tool_name: String,
    pub tool_input: String,  // JSON input
    pub tool_output: String, // JSON output
    pub status: String,      // "pending", "success", "error"
    pub created_at: String,
}

// =============================================================================
// DATABASE INITIALIZATION
// =============================================================================

/// Initialize chat database with connection pooling
pub fn init_chat_db() -> Result<Pool<SqliteConnectionManager>, String> {
    let db_path = crate::paths::get_ai_chat_db_path();
    let db_path_str = db_path.to_string_lossy().to_string();

    // Ensure data directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    // Create connection manager
    let manager = SqliteConnectionManager::file(&db_path).with_init(|conn| {
        // Configure connection for optimal performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 5000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?; // Enable foreign key constraints
        conn.busy_timeout(std::time::Duration::from_millis(10_000))?;
        Ok(())
    });

    // Create connection pool
    let pool = Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|e| format!("Failed to create connection pool: {}", e))?;

    // Initialize schema using a connection from the pool
    {
        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get connection from pool: {}", e))?;
        initialize_schema(&conn)?;
    }

    logger::info(
        LogTag::System,
        &format!("AI chat database initialized at {}", db_path_str),
    );

    // Store in global
    let pool_arc = Arc::new(pool);
    GLOBAL_CHAT_POOL
        .set(pool_arc.clone())
        .map_err(|_| "Global chat pool already initialized".to_string())?;

    Ok(Arc::try_unwrap(pool_arc).unwrap_or_else(|arc| (*arc).clone()))
}

/// Get global chat database pool
pub fn get_chat_pool() -> Option<Arc<Pool<SqliteConnectionManager>>> {
    GLOBAL_CHAT_POOL.get().cloned()
}

/// Initialize database schema
fn initialize_schema(conn: &rusqlite::Connection) -> Result<(), String> {
    // Chat sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            summary TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Failed to create chat_sessions table: {}", e))?;

    // Chat messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| format!("Failed to create chat_messages table: {}", e))?;

    // Tool executions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tool_executions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL,
            tool_name TEXT NOT NULL,
            tool_input TEXT NOT NULL,
            tool_output TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| format!("Failed to create tool_executions table: {}", e))?;

    // Indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_session ON chat_messages(session_id, created_at)",
        [],
    )
    .map_err(|e| format!("Failed to create messages index: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_executions_message ON tool_executions(message_id)",
        [],
    )
    .map_err(|e| format!("Failed to create executions index: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_updated ON chat_sessions(updated_at DESC)",
        [],
    )
    .map_err(|e| format!("Failed to create sessions index: {}", e))?;

    Ok(())
}

// =============================================================================
// SESSION CRUD OPERATIONS
// =============================================================================

/// Create a new chat session
pub fn create_session(pool: &Pool<SqliteConnectionManager>, title: &str) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO chat_sessions (title, created_at, updated_at) VALUES (?1, ?2, ?3)",
        params![title, &now, &now],
    )
    .map_err(|e| format!("Failed to insert session: {}", e))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Get all chat sessions ordered by most recent
pub fn get_sessions(pool: &Pool<SqliteConnectionManager>) -> Result<Vec<ChatSession>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.summary, COUNT(m.id) as message_count, 
                    s.created_at, s.updated_at 
             FROM chat_sessions s 
             LEFT JOIN chat_messages m ON s.id = m.session_id 
             GROUP BY s.id 
             ORDER BY s.updated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query sessions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect sessions: {}", e))?;

    Ok(sessions)
}

/// Get a single session by ID
pub fn get_session(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
) -> Result<Option<ChatSession>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.summary, COUNT(m.id) as message_count, 
                    s.created_at, s.updated_at 
             FROM chat_sessions s 
             LEFT JOIN chat_messages m ON s.id = m.session_id 
             WHERE s.id = ?1 
             GROUP BY s.id",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query session: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET summary = ?1, updated_at = ?2 WHERE id = ?3",
        params![summary, &now, id],
    )
    .map_err(|e| format!("Failed to update session summary: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, &now, id],
    )
    .map_err(|e| format!("Failed to update session title: {}", e))?;

    Ok(())
}

/// Touch session (update updated_at timestamp)
pub fn touch_session(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![&now, id],
    )
    .map_err(|e| format!("Failed to touch session: {}", e))?;

    Ok(())
}

/// Delete a session (cascade deletes messages and executions)
pub fn delete_session(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete session: {}", e))?;

    Ok(())
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
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT INTO chat_messages (session_id, role, content, tool_calls, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, role, content, tool_calls, &now],
    )
    .map_err(|e| format!("Failed to insert message: {}", e))?;

    let message_id = tx.last_insert_rowid();

    // Update session timestamp atomically with message insert
    tx.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![&now, session_id],
    )
    .map_err(|e| format!("Failed to update session timestamp: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit message transaction: {}", e))?;

    Ok(message_id)
}

/// Get all messages for a session
pub fn get_messages(
    pool: &Pool<SqliteConnectionManager>,
    session_id: i64,
) -> Result<Vec<ChatMessage>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, tool_calls, created_at 
             FROM chat_messages 
             WHERE session_id = ?1 
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query messages: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect messages: {}", e))?;

    Ok(messages)
}

/// Get a single message by ID
pub fn get_message(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
) -> Result<Option<ChatMessage>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, tool_calls, created_at 
             FROM chat_messages 
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query message: {}", e))?;

    Ok(message)
}

/// Delete a message
pub fn delete_message(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute("DELETE FROM chat_messages WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete message: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO tool_executions 
         (message_id, tool_name, tool_input, tool_output, status, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![message_id, tool_name, tool_input, tool_output, status, &now],
    )
    .map_err(|e| format!("Failed to insert tool execution: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, message_id, tool_name, tool_input, tool_output, status, created_at 
             FROM tool_executions 
             WHERE message_id = ?1 
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query tool executions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tool executions: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute(
        "UPDATE tool_executions SET tool_output = ?1, status = ?2 WHERE id = ?3",
        params![tool_output, status, id],
    )
    .map_err(|e| format!("Failed to update tool execution: {}", e))?;

    Ok(())
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Execute a function with a connection from the pool
pub fn with_chat_db<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T, String>,
{
    let pool = get_chat_pool().ok_or("Chat database pool not initialized")?;
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection from pool: {}", e))?;
    f(&*conn)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_pool() -> Pool<SqliteConnectionManager> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder().max_size(1).build(manager).unwrap();

        let conn = pool.get().unwrap();
        initialize_schema(&conn).unwrap();
        drop(conn);

        pool
    }

    #[test]
    fn test_session_crud() {
        let pool = setup_test_pool();

        // Create session
        let session_id = create_session(&pool, "Test Chat").unwrap();
        assert!(session_id > 0);

        // Get session
        let session = get_session(&pool, session_id).unwrap();
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.title, "Test Chat");
        assert_eq!(session.message_count, 0);

        // Update summary
        update_session_summary(&pool, session_id, "This is a test chat").unwrap();
        let updated = get_session(&pool, session_id).unwrap().unwrap();
        assert_eq!(updated.summary, Some("This is a test chat".to_string()));

        // Update title
        update_session_title(&pool, session_id, "Updated Chat").unwrap();
        let updated = get_session(&pool, session_id).unwrap().unwrap();
        assert_eq!(updated.title, "Updated Chat");

        // List sessions
        let sessions = get_sessions(&pool).unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete session
        delete_session(&pool, session_id).unwrap();
        let deleted = get_session(&pool, session_id).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_message_crud() {
        let pool = setup_test_pool();
        let session_id = create_session(&pool, "Test Chat").unwrap();

        // Add message
        let message_id = add_message(&pool, session_id, "user", "Hello, bot!", None).unwrap();
        assert!(message_id > 0);

        // Get message
        let message = get_message(&pool, message_id).unwrap();
        assert!(message.is_some());
        let message = message.unwrap();
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello, bot!");

        // Add another message with tool calls
        let tool_calls = r#"[{"name": "get_price", "args": {"symbol": "BTC"}}]"#;
        let message_id2 = add_message(
            &pool,
            session_id,
            "assistant",
            "Let me check the price.",
            Some(tool_calls),
        )
        .unwrap();

        // Get messages
        let messages = get_messages(&pool, session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert!(messages[1].tool_calls.is_some());

        // Verify session message count updated
        let session = get_session(&pool, session_id).unwrap().unwrap();
        assert_eq!(session.message_count, 2);

        // Delete message
        delete_message(&pool, message_id).unwrap();
        let messages = get_messages(&pool, session_id).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_tool_execution() {
        let pool = setup_test_pool();
        let session_id = create_session(&pool, "Test Chat").unwrap();
        let message_id = add_message(&pool, session_id, "assistant", "Checking...", None).unwrap();

        // Add tool execution
        let exec_id = add_tool_execution(
            &pool,
            message_id,
            "get_price",
            r#"{"symbol": "BTC"}"#,
            r#"{"price": 45000}"#,
            "success",
        )
        .unwrap();
        assert!(exec_id > 0);

        // Get executions
        let executions = get_tool_executions(&pool, message_id).unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].tool_name, "get_price");
        assert_eq!(executions[0].status, "success");

        // Update execution
        update_tool_execution(&pool, exec_id, r#"{"price": 45500}"#, "success").unwrap();
        let updated = get_tool_executions(&pool, message_id).unwrap();
        assert!(updated[0].tool_output.contains("45500"));
    }

    #[test]
    fn test_cascade_delete() {
        let pool = setup_test_pool();

        // Create session with messages and tool executions
        let session_id = create_session(&pool, "Test Chat").unwrap();
        let message_id = add_message(&pool, session_id, "user", "Hello", None).unwrap();
        add_tool_execution(&pool, message_id, "test_tool", "{}", "{}", "success").unwrap();

        // Delete session should cascade
        delete_session(&pool, session_id).unwrap();

        // Verify everything is deleted
        let session = get_session(&pool, session_id).unwrap();
        assert!(session.is_none());

        let messages = get_messages(&pool, session_id).unwrap();
        assert_eq!(messages.len(), 0);

        let executions = get_tool_executions(&pool, message_id).unwrap();
        assert_eq!(executions.len(), 0);
    }
}
