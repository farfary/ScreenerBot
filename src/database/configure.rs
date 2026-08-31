//! Shared SQLite configuration for all databases.
//!
//! Every database connection in the bot MUST use `configure_connection()` via
//! `with_init()` so that PRAGMAs survive r2d2 connection recycling.

use rusqlite::Connection;

/// Workload-based presets for SQLite tuning.
///
/// Each preset defines cache_size (pages × 4 KB), mmap_size, and busy_timeout
/// that match the access pattern of the database.
#[derive(Debug, Clone, Copy)]
pub enum DbPreset {
    /// High-frequency reads/writes: tokens, transactions
    Hot,
    /// Moderate traffic: events, actions, positions, wallet, ohlcvs
    Standard,
    /// Infrequent access: tools, strategies, wallets list, rpc_stats, ai
    Cold,
}

/// Per-database configuration that combines a preset with optional overrides.
#[derive(Debug, Clone, Copy)]
pub struct DbConfig {
    pub preset: DbPreset,
    /// Override cache_size (pages). 0 = use preset default.
    pub cache_size: i64,
    /// Override mmap_size (bytes). -1 = use preset default.
    pub mmap_size: i64,
}

impl DbConfig {
    pub const fn new(preset: DbPreset) -> Self {
        Self {
            preset,
            cache_size: 0,
            mmap_size: -1,
        }
    }

    pub const fn with_cache_size(mut self, pages: i64) -> Self {
        self.cache_size = pages;
        self
    }

    pub const fn with_mmap_size(mut self, bytes: i64) -> Self {
        self.mmap_size = bytes;
        self
    }
}

/// Apply all standard PRAGMAs to a SQLite connection.
///
/// Called via `SqliteConnectionManager::file(path).with_init(|c| configure_connection(c, cfg))`
/// so that EVERY connection (including recycled ones) gets the correct settings.
pub fn configure_connection(conn: &Connection, cfg: DbConfig) -> rusqlite::Result<()> {
    let (default_cache, default_mmap) = match cfg.preset {
        DbPreset::Hot => (5000_i64, 268_435_456_i64), // 20 MB cache, 256 MB mmap
        DbPreset::Standard => (2000_i64, 0_i64),      // 8 MB cache, no mmap
        DbPreset::Cold => (500_i64, 0_i64),           // 2 MB cache, no mmap
    };

    let cache_size = if cfg.cache_size > 0 {
        cfg.cache_size
    } else {
        default_cache
    };
    let mmap_size = if cfg.mmap_size >= 0 {
        cfg.mmap_size
    } else {
        default_mmap
    };

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", cache_size)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "mmap_size", mmap_size)?;
    conn.pragma_update(None, "foreign_keys", 1)?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;

    Ok(())
}

// ── Per-database configurations ──────────────────────────────────────────────

/// tokens.db — Hot: pooled, heavy read/write
pub const TOKENS_DB: DbConfig = DbConfig::new(DbPreset::Hot);

/// transactions.db — Hot: frequent inserts and lookups
pub const TRANSACTIONS_DB: DbConfig = DbConfig::new(DbPreset::Hot).with_cache_size(3000);

/// events.db write pool — Standard: moderate write frequency
pub const EVENTS_WRITE_DB: DbConfig = DbConfig::new(DbPreset::Standard).with_cache_size(1000);

/// events.db read pool — Standard: dashboard polling reads
pub const EVENTS_READ_DB: DbConfig = DbConfig::new(DbPreset::Standard).with_mmap_size(134_217_728); // 128 MB

/// actions.db write pool — Standard: moderate write frequency
pub const ACTIONS_WRITE_DB: DbConfig = DbConfig::new(DbPreset::Standard).with_cache_size(1000);

/// actions.db read pool — Standard: dashboard reads
pub const ACTIONS_READ_DB: DbConfig = DbConfig::new(DbPreset::Standard);

/// positions.db — Standard: position tracking reads/writes
pub const POSITIONS_DB: DbConfig = DbConfig::new(DbPreset::Standard).with_cache_size(1000);

/// wallet.db (balance monitor/snapshots) — Standard
pub const WALLET_MONITOR_DB: DbConfig = DbConfig::new(DbPreset::Standard)
    .with_cache_size(1000)
    .with_mmap_size(33_554_432); // 32 MB

/// ohlcvs.db — Standard: candle data
pub const OHLCVS_DB: DbConfig = DbConfig::new(DbPreset::Standard);

/// tools.db — Cold: infrequent access
pub const TOOLS_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// strategies.db — Cold: infrequent access
pub const STRATEGIES_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// copy_trading.db — Cold: human-managed tasks and append-only decisions
pub const COPY_TRADING_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// wallets.db — Cold: wallet list, rarely changes
pub const WALLETS_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// rpc_stats.db — Cold: stats aggregation, batch inserts
pub const RPC_STATS_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// ai_chat.db — Cold: Assistant chat history (legacy filename)
pub const AI_CHAT_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// pools.db — Standard: frequent price history writes, 729 MB on disk
pub const POOLS_DB: DbConfig = DbConfig::new(DbPreset::Standard).with_cache_size(3000);

/// ai.db — Cold: LLM-analysis history/instructions (legacy filename)
pub const AI_DB: DbConfig = DbConfig::new(DbPreset::Cold);

/// agent_control.db — Cold: client pairings, the external-agent approval queue
/// and the agent-control audit log. Low write volume, small on disk.
pub const AGENT_CONTROL_DB: DbConfig = DbConfig::new(DbPreset::Cold);
