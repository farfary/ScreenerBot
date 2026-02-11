//! AI Instructions Database Module
//!
//! SQLite persistence for:
//! - User-created AI instructions
//! - Decision history tracking
//! - Built-in instruction templates

use crate::logger::{self, LogTag};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

static GLOBAL_AI_DB: OnceCell<Arc<Mutex<Connection>>> = OnceCell::new();

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// User-created AI instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub category: String,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// AI decision history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub decision: String,
    pub confidence: u8,
    pub reasoning: Option<String>,
    pub risk_level: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub tokens_used: u32,
    pub latency_ms: f64,
    pub cached: bool,
    pub created_at: String,
}

/// Built-in instruction template
#[derive(Debug, Clone)]
pub struct InstructionTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub content: &'static str,
    pub tags: &'static [&'static str],
}

// =============================================================================
// DATABASE INITIALIZATION
// =============================================================================

/// Initialize AI database with schema
pub fn init_ai_database() -> Result<Connection, String> {
    let db_path = crate::paths::get_ai_db_path();
    let db_path_str = db_path.to_string_lossy().to_string();

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open AI database at {}: {}", db_path_str, e))?;

    // Configure connection for optimal performance
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to set journal mode: {}", e))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
    conn.pragma_update(None, "cache_size", 5000)
        .map_err(|e| format!("Failed to set cache size: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_millis(10_000))
        .map_err(|e| format!("Failed to set busy timeout: {}", e))?;

    // Create schema
    initialize_schema(&conn)?;

    logger::info(
        LogTag::System,
        &format!("AI database initialized at {}", db_path_str),
    );

    // Store in global
    GLOBAL_AI_DB
        .set(Arc::new(Mutex::new(conn)))
        .map_err(|_| "Global AI database already initialized".to_string())?;

    // Return a new connection for immediate use
    Connection::open(&db_path).map_err(|e| format!("Failed to reopen AI database: {}", e))
}

/// Get global AI database connection
pub fn get_ai_database() -> Option<Arc<Mutex<Connection>>> {
    GLOBAL_AI_DB.get().cloned()
}

/// Initialize database schema
fn initialize_schema(conn: &Connection) -> Result<(), String> {
    // User instructions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_instructions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'general',
            priority INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Failed to create ai_instructions table: {}", e))?;

    // Decision history table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_decision_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mint TEXT NOT NULL,
            symbol TEXT,
            decision TEXT NOT NULL,
            confidence INTEGER NOT NULL,
            reasoning TEXT,
            risk_level TEXT,
            provider TEXT NOT NULL,
            model TEXT,
            tokens_used INTEGER NOT NULL DEFAULT 0,
            latency_ms REAL NOT NULL DEFAULT 0,
            cached INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Failed to create ai_decision_history table: {}", e))?;

    // Indexes for decision history
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_decisions_mint ON ai_decision_history(mint)",
        [],
    )
    .map_err(|e| format!("Failed to create mint index: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_decisions_created ON ai_decision_history(created_at DESC)",
        [],
    )
    .map_err(|e| format!("Failed to create created_at index: {}", e))?;

    Ok(())
}

// =============================================================================
// INSTRUCTION CRUD OPERATIONS
// =============================================================================

/// List all instructions ordered by priority (highest first) and name
pub fn list_instructions(db: &Connection) -> Result<Vec<Instruction>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, name, content, category, priority, enabled, created_at, updated_at 
             FROM ai_instructions 
             ORDER BY priority DESC, name ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let instructions = stmt
        .query_map([], |row| {
            Ok(Instruction {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                priority: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("Failed to query instructions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect instructions: {}", e))?;

    Ok(instructions)
}

/// Get a single instruction by ID
pub fn get_instruction(db: &Connection, id: i64) -> Result<Option<Instruction>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, name, content, category, priority, enabled, created_at, updated_at 
             FROM ai_instructions 
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let instruction = stmt
        .query_row(params![id], |row| {
            Ok(Instruction {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                priority: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query instruction: {}", e))?;

    Ok(instruction)
}

/// Create a new instruction
pub fn create_instruction(
    db: &Connection,
    name: &str,
    content: &str,
    category: &str,
) -> Result<i64, String> {
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(
        "INSERT INTO ai_instructions (name, content, category, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, content, category, &now, &now],
    )
    .map_err(|e| format!("Failed to insert instruction: {}", e))?;

    let id = db.last_insert_rowid();
    Ok(id)
}

/// Update an instruction (only provided fields are updated)
pub fn update_instruction(
    db: &Connection,
    id: i64,
    name: Option<&str>,
    content: Option<&str>,
    category: Option<&str>,
    priority: Option<i32>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut updates = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(n) = name {
        updates.push("name = ?");
        params_vec.push(Box::new(n.to_string()));
    }
    if let Some(c) = content {
        updates.push("content = ?");
        params_vec.push(Box::new(c.to_string()));
    }
    if let Some(cat) = category {
        updates.push("category = ?");
        params_vec.push(Box::new(cat.to_string()));
    }
    if let Some(p) = priority {
        updates.push("priority = ?");
        params_vec.push(Box::new(p));
    }
    if let Some(e) = enabled {
        updates.push("enabled = ?");
        params_vec.push(Box::new(if e { 1 } else { 0 }));
    }

    if updates.is_empty() {
        return Ok(());
    }

    updates.push("updated_at = ?");
    params_vec.push(Box::new(now));

    let sql = format!(
        "UPDATE ai_instructions SET {} WHERE id = ?",
        updates.join(", ")
    );

    params_vec.push(Box::new(id));

    // Convert to params for rusqlite
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    db.execute(&sql, params_refs.as_slice())
        .map_err(|e| format!("Failed to update instruction: {}", e))?;

    Ok(())
}

/// Delete an instruction by ID
pub fn delete_instruction(db: &Connection, id: i64) -> Result<(), String> {
    db.execute("DELETE FROM ai_instructions WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete instruction: {}", e))?;

    Ok(())
}

/// Reorder instructions by setting priorities based on the order of IDs
/// First ID in the list gets the highest priority
pub fn reorder_instructions(db: &Connection, ids: &[i64]) -> Result<(), String> {
    let tx = db
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = tx
        .prepare("UPDATE ai_instructions SET priority = ?1, updated_at = ?2 WHERE id = ?3")
        .map_err(|e| format!("Failed to prepare update statement: {}", e))?;

    for (index, id) in ids.iter().enumerate() {
        let priority = (ids.len() - index) as i32; // Reverse: first = highest priority
        stmt.execute(params![priority, &now, id])
            .map_err(|e| format!("Failed to update priority for instruction {}: {}", id, e))?;
    }

    drop(stmt);
    tx.commit()
        .map_err(|e| format!("Failed to commit reorder transaction: {}", e))?;

    Ok(())
}

// =============================================================================
// DECISION HISTORY OPERATIONS
// =============================================================================

/// Record an AI decision
pub fn record_decision(db: &Connection, record: &DecisionRecord) -> Result<i64, String> {
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(
        "INSERT INTO ai_decision_history 
         (mint, symbol, decision, confidence, reasoning, risk_level, provider, model, 
          tokens_used, latency_ms, cached, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            &record.mint,
            &record.symbol,
            &record.decision,
            record.confidence,
            &record.reasoning,
            &record.risk_level,
            &record.provider,
            &record.model,
            record.tokens_used,
            record.latency_ms,
            if record.cached { 1 } else { 0 },
            &now,
        ],
    )
    .map_err(|e| format!("Failed to insert decision record: {}", e))?;

    let id = db.last_insert_rowid();
    Ok(id)
}

/// List recent decisions with pagination
pub fn list_decisions(
    db: &Connection,
    limit: usize,
    offset: usize,
) -> Result<Vec<DecisionRecord>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, mint, symbol, decision, confidence, reasoning, risk_level, 
                    provider, model, tokens_used, latency_ms, cached, created_at 
             FROM ai_decision_history 
             ORDER BY created_at DESC 
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let decisions = stmt
        .query_map(params![limit, offset], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                mint: row.get(1)?,
                symbol: row.get(2)?,
                decision: row.get(3)?,
                confidence: row.get(4)?,
                reasoning: row.get(5)?,
                risk_level: row.get(6)?,
                provider: row.get(7)?,
                model: row.get(8)?,
                tokens_used: row.get(9)?,
                latency_ms: row.get(10)?,
                cached: row.get::<_, i32>(11)? != 0,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to query decisions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect decisions: {}", e))?;

    Ok(decisions)
}

/// Get a single decision by ID
pub fn get_decision(db: &Connection, id: i64) -> Result<Option<DecisionRecord>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, mint, symbol, decision, confidence, reasoning, risk_level, 
                    provider, model, tokens_used, latency_ms, cached, created_at 
             FROM ai_decision_history 
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let decision = stmt
        .query_row(params![id], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                mint: row.get(1)?,
                symbol: row.get(2)?,
                decision: row.get(3)?,
                confidence: row.get(4)?,
                reasoning: row.get(5)?,
                risk_level: row.get(6)?,
                provider: row.get(7)?,
                model: row.get(8)?,
                tokens_used: row.get(9)?,
                latency_ms: row.get(10)?,
                cached: row.get::<_, i32>(11)? != 0,
                created_at: row.get(12)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query decision: {}", e))?;

    Ok(decision)
}

/// List decisions for a specific mint address
pub fn list_decisions_for_mint(
    db: &Connection,
    mint: &str,
    limit: usize,
) -> Result<Vec<DecisionRecord>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, mint, symbol, decision, confidence, reasoning, risk_level, 
                    provider, model, tokens_used, latency_ms, cached, created_at 
             FROM ai_decision_history 
             WHERE mint = ?1 
             ORDER BY created_at DESC 
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let decisions = stmt
        .query_map(params![mint, limit], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                mint: row.get(1)?,
                symbol: row.get(2)?,
                decision: row.get(3)?,
                confidence: row.get(4)?,
                reasoning: row.get(5)?,
                risk_level: row.get(6)?,
                provider: row.get(7)?,
                model: row.get(8)?,
                tokens_used: row.get(9)?,
                latency_ms: row.get(10)?,
                cached: row.get::<_, i32>(11)? != 0,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to query decisions for mint: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect decisions: {}", e))?;

    Ok(decisions)
}

/// List decisions for a specific mint with pagination support
pub fn list_decisions_for_mint_paginated(
    db: &Connection,
    mint: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<DecisionRecord>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id, mint, symbol, decision, confidence, reasoning, risk_level, 
                    provider, model, tokens_used, latency_ms, cached, created_at 
             FROM ai_decision_history 
             WHERE mint = ?1 
             ORDER BY created_at DESC 
             LIMIT ?2 OFFSET ?3",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let decisions = stmt
        .query_map(params![mint, limit, offset], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                mint: row.get(1)?,
                symbol: row.get(2)?,
                decision: row.get(3)?,
                confidence: row.get(4)?,
                reasoning: row.get(5)?,
                risk_level: row.get(6)?,
                provider: row.get(7)?,
                model: row.get(8)?,
                tokens_used: row.get(9)?,
                latency_ms: row.get(10)?,
                cached: row.get::<_, i32>(11)? != 0,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to query decisions for mint: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect decisions: {}", e))?;

    Ok(decisions)
}

/// Clear old decision records (older than specified days)
pub fn clear_old_decisions(db: &Connection, days: i64) -> Result<usize, String> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.to_rfc3339();

    let affected = db
        .execute(
            "DELETE FROM ai_decision_history WHERE created_at < ?1",
            params![cutoff_str],
        )
        .map_err(|e| format!("Failed to delete old decisions: {}", e))?;

    Ok(affected)
}

// =============================================================================
// BUILT-IN TEMPLATES
// =============================================================================

/// Get all built-in instruction templates
pub fn get_builtin_templates() -> Vec<InstructionTemplate> {
    vec![
        InstructionTemplate {
            id: "liquidity_guard",
            name: "Liquidity Guard",
            category: "filtering",
            content: "Reject any token with total liquidity below $10,000 USD. Low liquidity increases slippage risk and makes exit difficult. For tokens under $50K liquidity, flag as HIGH RISK even if other metrics are acceptable.",
            tags: &["safety", "liquidity", "risk-management"],
        },
        InstructionTemplate {
            id: "holder_distribution",
            name: "Holder Distribution Check",
            category: "filtering",
            content: "Flag tokens where the top 10 holders control more than 50% of the supply as MEDIUM RISK. If top 5 holders control >40%, consider it HIGH RISK. Concentrated ownership increases pump-and-dump risk and manipulation potential.",
            tags: &["holders", "distribution", "rug-risk"],
        },
        InstructionTemplate {
            id: "honeypot_detection",
            name: "Honeypot Detection",
            category: "filtering",
            content: "Analyze token authority settings and contract permissions. REJECT if: freeze authority is enabled, mint authority is still active after initial distribution, or there are unusual transfer restrictions. Check for contract upgrade authority that could enable malicious changes.",
            tags: &["security", "honeypot", "authority"],
        },
        InstructionTemplate {
            id: "momentum_filter",
            name: "Momentum Filter",
            category: "trading",
            content: "Prefer tokens showing positive price momentum over 1h, 6h, and 24h timeframes. Look for increasing volume trend alongside price action. Be cautious of sudden spikes without volume confirmation - these are often pump schemes.",
            tags: &["momentum", "price-action", "volume"],
        },
        InstructionTemplate {
            id: "new_token_caution",
            name: "New Token Caution",
            category: "analysis",
            content: "Exercise extra caution with tokens less than 24 hours old. Require higher confidence thresholds and stronger fundamentals. New tokens lack price history and holder stability - what looks promising in hour 1 often dumps by hour 12.",
            tags: &["age", "new-tokens", "caution"],
        },
        InstructionTemplate {
            id: "whale_activity",
            name: "Whale Activity Monitor",
            category: "analysis",
            content: "Monitor for large holder changes (>5% of supply moving). Whale accumulation can signal upcoming price action, but whale distribution often precedes dumps. Flag unusual wallet activity, especially from deployer/early wallets.",
            tags: &["whales", "large-holders", "activity"],
        },
    ]
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get a connection from the global database
pub fn with_ai_db<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&Connection) -> Result<T, String>,
{
    let db = get_ai_database().ok_or("AI database not initialized")?;
    let conn = db
        .lock()
        .map_err(|e| format!("Failed to lock AI database: {}", e))?;
    f(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let templates = get_builtin_templates();
        assert_eq!(templates.len(), 6);
        assert!(templates.iter().any(|t| t.id == "liquidity_guard"));
        assert!(templates.iter().any(|t| t.id == "whale_activity"));
    }

    #[test]
    fn test_instruction_crud() {
        // Create in-memory database
        let conn = Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();

        // Create instruction
        let id =
            create_instruction(&conn, "Test Instruction", "This is a test", "general").unwrap();
        assert!(id > 0);

        // Get instruction
        let instruction = get_instruction(&conn, id).unwrap();
        assert!(instruction.is_some());
        let instruction = instruction.unwrap();
        assert_eq!(instruction.name, "Test Instruction");
        assert_eq!(instruction.content, "This is a test");
        assert_eq!(instruction.category, "general");
        assert!(instruction.enabled);

        // Update instruction
        update_instruction(
            &conn,
            id,
            Some("Updated Name"),
            None,
            None,
            Some(10),
            Some(false),
        )
        .unwrap();

        let updated = get_instruction(&conn, id).unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.priority, 10);
        assert!(!updated.enabled);

        // List instructions
        let all = list_instructions(&conn).unwrap();
        assert_eq!(all.len(), 1);

        // Delete instruction
        delete_instruction(&conn, id).unwrap();
        let deleted = get_instruction(&conn, id).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_decision_record() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();

        let record = DecisionRecord {
            id: 0, // Will be set by database
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            symbol: Some("USDC".to_string()),
            decision: "PASS".to_string(),
            confidence: 85,
            reasoning: Some("Good liquidity and holder distribution".to_string()),
            risk_level: Some("LOW".to_string()),
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            tokens_used: 1500,
            latency_ms: 234.5,
            cached: false,
            created_at: String::new(), // Will be set by database
        };

        let id = record_decision(&conn, &record).unwrap();
        assert!(id > 0);

        let retrieved = get_decision(&conn, id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.mint, record.mint);
        assert_eq!(retrieved.decision, "PASS");
        assert_eq!(retrieved.confidence, 85);

        // Test list
        let decisions = list_decisions(&conn, 10, 0).unwrap();
        assert_eq!(decisions.len(), 1);

        // Test list by mint
        let mint_decisions =
            list_decisions_for_mint(&conn, "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 10)
                .unwrap();
        assert_eq!(mint_decisions.len(), 1);
    }
}
