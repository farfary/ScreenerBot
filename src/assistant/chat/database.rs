// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Assistant chat database module.
//!
//! SQLite persistence for dashboard assistant chat sessions:
//! - Chat sessions with titles and summaries
//! - Chat messages with role, content, and tool calls
//! - Tool execution tracking with inputs/outputs

use crate::assistant::error::{Error, Result};
use crate::database;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::DatabaseError;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};

// Re-export all query operations so callers can use `database::create_session(...)` etc.
pub use super::database_queries::*;

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
// SCHEMA STEP AND CONNECTIONS
// =============================================================================

/// The schema of `ai_chat.db`: chat sessions and the scheduled assistant tasks.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 1,
    min_reader_version: 1,
    baseline: 1,
    renames: None,
    step,
    legacy_stamp: None,
};

/// Open the chat store.
pub fn init_chat_db() -> Result<()> {
    database::open::store(&database::catalog::AI_CHAT)?;
    Ok(())
}

/// The chat store's pool, `None` while the store is unavailable.
pub fn get_chat_pool() -> Option<Pool<SqliteConnectionManager>> {
    database::open::store(&database::catalog::AI_CHAT).ok()
}

fn step(tx: &Transaction<'_>, _: &SchemaContext) -> std::result::Result<(), DatabaseError> {
    create_schema(tx).map_err(|error| match error {
        Error::Database(error) => error,
        other => DatabaseError::Migration {
            store: "ai_chat.db".to_owned(),
            detail: other.to_string(),
        },
    })
}

fn create_schema(tx: &Transaction<'_>) -> Result<()> {
    ddl::run(
        tx,
        "create chat_sessions table",
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            summary TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )?;
    ddl::run(
        tx,
        "create chat_messages table",
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        )",
    )?;
    ddl::run(
        tx,
        "create tool_executions table",
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
    )?;
    ddl::run(
        tx,
        "create idx_messages_session index",
        "CREATE INDEX IF NOT EXISTS idx_messages_session ON chat_messages(session_id, created_at)",
    )?;
    ddl::run(
        tx,
        "create idx_executions_message index",
        "CREATE INDEX IF NOT EXISTS idx_executions_message ON tool_executions(message_id)",
    )?;
    ddl::run(
        tx,
        "create idx_sessions_updated index",
        "CREATE INDEX IF NOT EXISTS idx_sessions_updated ON chat_sessions(updated_at DESC)",
    )?;
    // Scheduled task sessions are hidden from the session list.
    ddl::add_column(
        tx,
        "chat_sessions",
        "is_hidden",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    crate::assistant::scheduled::database::initialize_scheduled_tables(tx)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn test_pool() -> Pool<SqliteConnectionManager> {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder()
        .max_size(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .build(manager)
        .expect("chat test pool");
    database::open::upgrade_connection(
        &mut pool.get().expect("chat test connection"),
        &database::catalog::AI_CHAT,
        vec![crate::chains::ChainId::Solana],
    )
    .expect("chat test schema");
    pool
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Execute a function with a connection from the pool
pub fn with_chat_db<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T>,
{
    let conn = database::open::store(&database::catalog::AI_CHAT)?
        .get()
        .map_err(DatabaseError::from)?;
    f(&*conn)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_pool() -> Pool<SqliteConnectionManager> {
        test_pool()
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
        assert_eq!(updated.summary, Some("This is a test chat".to_owned()));

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
    fn session_titles_carry_no_direction_controls() {
        let pool = setup_test_pool();
        let marked =
            "\u{2068}Chat\u{2069} \u{200F}Oct 26\u{200E}, \u{202B}10:05\u{202C} \u{2066}PM\u{2069}";
        let session_id = create_session(&pool, marked).unwrap();
        assert_eq!(
            get_session(&pool, session_id).unwrap().unwrap().title,
            "Chat Oct 26, 10:05 PM"
        );

        update_session_title(&pool, session_id, "\u{202E}Renamed\u{202A}").unwrap();
        let stored: String = pool
            .get()
            .unwrap()
            .query_row(
                "SELECT title FROM chat_sessions WHERE id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, "Renamed");

        // A row written before titles were cleaned reads without its controls.
        pool.get()
            .unwrap()
            .execute(
                "UPDATE chat_sessions SET title = ?1 WHERE id = ?2",
                rusqlite::params![marked, session_id],
            )
            .unwrap();
        assert_eq!(
            get_session(&pool, session_id).unwrap().unwrap().title,
            "Chat Oct 26, 10:05 PM"
        );
        assert_eq!(
            get_sessions(&pool).unwrap()[0].title,
            "Chat Oct 26, 10:05 PM"
        );
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
        let _message_id2 = add_message(
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
