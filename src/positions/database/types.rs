//! Position database types — row structs and state enums for SQLite persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::LazyLock;

// Static flag to track if database has been initialized (to reduce log noise)
pub(super) static POSITIONS_DB_INITIALIZED: LazyLock<AtomicBool> =
    LazyLock::new(|| AtomicBool::new(false));

// Database schema version
pub(super) const POSITIONS_SCHEMA_VERSION: u32 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

// Column list for SELECT queries (DRY principle)
pub(super) const POSITION_SELECT_COLUMNS: &str = r#"
  id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
  position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
  entry_transaction_signature, exit_transaction_signature, token_amount,
  effective_entry_price, effective_exit_price, sol_received,
  profit_target_min, profit_target_max, liquidity_tier,
  transaction_entry_verified, transaction_exit_verified,
  entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
  phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
  pnl, pnl_percent, unrealized_pnl, unrealized_pnl_percent,
  remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
  dca_count, average_entry_price, last_dca_time,
  archived, archived_at, manual_management
"#;

pub(super) const SCHEMA_POSITIONS: &str = r#"
CREATE TABLE IF NOT EXISTS positions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  wallet_address TEXT NOT NULL,
  mint TEXT NOT NULL,
  symbol TEXT NOT NULL,
  name TEXT NOT NULL,
  entry_price REAL NOT NULL,
  entry_time TEXT NOT NULL,
  exit_price REAL,
  exit_time TEXT,
 position_type TEXT NOT NULL, -- 'buy'or 'sell'
  entry_size_sol REAL NOT NULL, -- Initial SOL spent on first entry
  total_size_sol REAL NOT NULL, -- Cumulative SOL invested (includes DCA)
  price_highest REAL NOT NULL,
  price_lowest REAL NOT NULL,
  -- Real swap tracking
  entry_transaction_signature TEXT,
  exit_transaction_signature TEXT,
  token_amount INTEGER, -- Initial amount of tokens bought (first entry)
  effective_entry_price REAL, -- Initial entry price (deprecated, use average_entry_price)
  effective_exit_price REAL, -- Final exit price (deprecated, use average_exit_price)
  sol_received REAL, -- Total SOL received after all exits
  -- Smart profit targeting
  profit_target_min REAL, -- Minimum profit target percentage
  profit_target_max REAL, -- Maximum profit target percentage
  liquidity_tier TEXT, -- Liquidity tier for reference
  -- Transaction verification status
  transaction_entry_verified BOOLEAN NOT NULL DEFAULT false,
  transaction_exit_verified BOOLEAN NOT NULL DEFAULT false,
  -- Actual transaction fees (in lamports)
  entry_fee_lamports INTEGER, -- Actual entry transaction fee
  exit_fee_lamports INTEGER, -- Actual exit transaction fee
  -- Current price tracking
  current_price REAL, -- Current market price
  current_price_updated TEXT, -- When current_price was last updated
  -- Phantom detection tracking
  phantom_confirmations INTEGER NOT NULL DEFAULT 0,
  phantom_first_seen TEXT, -- When first confirmed phantom
  synthetic_exit BOOLEAN NOT NULL DEFAULT false,
  closed_reason TEXT, -- Optional reason for closure
  -- Pre-calculated P&L values (updated by positions system)
  pnl REAL, -- Realized P&L in SOL (for closed positions)
  pnl_percent REAL, -- Realized P&L percentage (for closed positions)
  unrealized_pnl REAL, -- Unrealized P&L in SOL (for open positions)
  unrealized_pnl_percent REAL, -- Unrealized P&L percentage (for open positions)
  -- Partial exit tracking
  remaining_token_amount INTEGER, -- Current holdings after partial exits
  total_exited_amount INTEGER NOT NULL DEFAULT 0, -- Cumulative tokens sold
  average_exit_price REAL, -- Weighted average exit price
  partial_exit_count INTEGER NOT NULL DEFAULT 0, -- Number of partial exits
  -- DCA tracking
  dca_count INTEGER NOT NULL DEFAULT 0, -- Number of additional entries (DCA)
  average_entry_price REAL NOT NULL DEFAULT 0, -- Weighted average entry price
  last_dca_time TEXT, -- Last DCA timestamp for cooldown
  -- Archival (dashboard "Remove -> Archive"; reversible, hides from open/closed lists)
  archived BOOLEAN NOT NULL DEFAULT 0, -- True when user archived the position
  archived_at TEXT, -- When the position was archived (RFC3339)
  -- Manual management (manual/force buy -> auto-trader never auto-sells/DCAs it)
  manual_management BOOLEAN NOT NULL DEFAULT 0, -- True when opened by a manual/force buy
  -- Timestamps
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub(super) const SCHEMA_POSITION_STATES: &str = r#"
CREATE TABLE IF NOT EXISTS position_states (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
  state TEXT NOT NULL, -- 'Open', 'Closing', 'Closed', 'ExitPending', 'ExitFailed', 'Phantom', 'Reconciling'
  changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  reason TEXT, -- Optional reason for state change
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_POSITION_EXITS: &str = r#"
CREATE TABLE IF NOT EXISTS position_exits (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
  wallet_address TEXT NOT NULL,
  timestamp TEXT NOT NULL,
  amount INTEGER NOT NULL, -- Tokens sold
  price REAL NOT NULL, -- Exit price per token
  sol_received REAL NOT NULL, -- SOL received
  transaction_signature TEXT NOT NULL,
  is_partial BOOLEAN NOT NULL, -- true if partial, false if full exit
  percentage REAL NOT NULL, -- % of position sold
  fees_lamports INTEGER, -- Transaction fee
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_POSITION_ENTRIES: &str = r#"
CREATE TABLE IF NOT EXISTS position_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
  wallet_address TEXT NOT NULL,
  timestamp TEXT NOT NULL,
  amount INTEGER NOT NULL, -- Tokens bought
  price REAL NOT NULL, -- Entry price per token
  sol_spent REAL NOT NULL, -- SOL spent
  transaction_signature TEXT NOT NULL,
  is_dca BOOLEAN NOT NULL, -- true if DCA, false if initial entry
  fees_lamports INTEGER, -- Transaction fee
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_POSITION_TRACKING: &str = r#"
CREATE TABLE IF NOT EXISTS position_tracking (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
  price REAL NOT NULL,
  price_source TEXT NOT NULL, -- 'pool', 'api', 'cache'
  pool_type TEXT, -- e.g., 'RAYDIUM CPMM'
  pool_address TEXT,
  api_price REAL,
  tracked_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_POSITION_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS position_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub(super) const SCHEMA_TOKEN_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS token_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
 snapshot_type TEXT NOT NULL, -- 'opening'or 'closing'
  mint TEXT NOT NULL,
  -- DexScreener token data
  symbol TEXT,
  name TEXT,
  price_sol REAL,
  price_usd REAL,
  price_native REAL,
  dex_id TEXT,
  pair_address TEXT,
  pair_url TEXT,
  fdv REAL,
  market_cap REAL,
  pair_created_at INTEGER,
  -- Liquidity data
  liquidity_usd REAL,
  liquidity_base REAL,
  liquidity_quote REAL,
  -- Volume data
  volume_h24 REAL,
  volume_h6 REAL,
  volume_h1 REAL,
  volume_m5 REAL,
  -- Transaction stats
  txns_h24_buys INTEGER,
  txns_h24_sells INTEGER,
  txns_h6_buys INTEGER,
  txns_h6_sells INTEGER,
  txns_h1_buys INTEGER,
  txns_h1_sells INTEGER,
  txns_m5_buys INTEGER,
  txns_m5_sells INTEGER,
  -- Price change data
  price_change_h24 REAL,
  price_change_h6 REAL,
  price_change_h1 REAL,
  price_change_m5 REAL,
  -- Token meta
  token_uri TEXT,
  token_description TEXT,
  token_image TEXT,
  token_website TEXT,
  token_twitter TEXT,
  token_telegram TEXT,
  -- Snapshot metadata
  snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
  api_fetch_time TEXT NOT NULL DEFAULT (datetime('now')),
  data_freshness_score INTEGER DEFAULT 0, -- 0-100 based on data recency
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

pub(super) const MIGRATION_ADD_PNL_FIELDS: &str = r#"
-- Add P&L fields to positions table (safe migration - columns are nullable)
ALTER TABLE positions ADD COLUMN pnl REAL;
ALTER TABLE positions ADD COLUMN pnl_percent REAL;
ALTER TABLE positions ADD COLUMN unrealized_pnl REAL;
ALTER TABLE positions ADD COLUMN unrealized_pnl_percent REAL;
"#;

pub(super) const MIGRATION_ADD_ARCHIVE_FIELDS: &str = r#"
-- Add archival fields to positions table (safe migration - columns are nullable/defaulted)
ALTER TABLE positions ADD COLUMN archived BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE positions ADD COLUMN archived_at TEXT;
"#;

pub(super) const MIGRATION_ADD_MANUAL_MANAGEMENT: &str = r#"
-- Add manual-management flag to positions table (safe migration - defaulted to 0).
-- Positions opened via a manual/force buy are flagged so the auto-trader skips them.
ALTER TABLE positions ADD COLUMN manual_management BOOLEAN NOT NULL DEFAULT 0;
"#;

// Performance indexes
pub(super) const POSITIONS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_positions_wallet ON positions(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_positions_mint ON positions(mint);",
    "CREATE INDEX IF NOT EXISTS idx_positions_entry_time ON positions(entry_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_exit_time ON positions(exit_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_mint_exit_time ON positions(mint, exit_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_entry_signature ON positions(entry_transaction_signature);",
    "CREATE INDEX IF NOT EXISTS idx_positions_exit_signature ON positions(exit_transaction_signature);",
    "CREATE INDEX IF NOT EXISTS idx_positions_state ON positions(id, position_type, exit_time);",
    "CREATE INDEX IF NOT EXISTS idx_positions_archived ON positions(archived);",
    "CREATE INDEX IF NOT EXISTS idx_position_states_position_id ON position_states(position_id, changed_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_states_state ON position_states(state, changed_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_tracking_position_id ON position_tracking(position_id, tracked_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_tracking_price ON position_tracking(price, tracked_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_position_id ON token_snapshots(position_id, snapshot_type);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_mint ON token_snapshots(mint, snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_type ON token_snapshots(snapshot_type, snapshot_time DESC);",
    // New indexes for partial exit and DCA tracking
    "CREATE INDEX IF NOT EXISTS idx_position_exits_wallet ON position_exits(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_position_exits_position_id ON position_exits(position_id, timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_exits_timestamp ON position_exits(timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_entries_wallet ON position_entries(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_position_entries_position_id ON position_entries(position_id, timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_entries_timestamp ON position_entries(timestamp DESC);",
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Position state enum with comprehensive lifecycle tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PositionState {
    Open,        // No exit transaction, actively trading
    Closing,     // Exit transaction submitted but not yet verified
    Closed,      // Exit transaction verified and exit_price set
    ExitPending, // Exit transaction in verification queue (similar to Closing but more explicit)
    ExitFailed,  // Exit transaction failed and needs retry
    Phantom,     // Position exists but wallet has zero tokens (needs reconciliation)
    Reconciling, // Auto-healing in progress for phantom positions
}

impl std::fmt::Display for PositionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionState::Open => write!(f, "Open"),
            PositionState::Closing => write!(f, "Closing"),
            PositionState::Closed => write!(f, "Closed"),
            PositionState::ExitPending => write!(f, "ExitPending"),
            PositionState::ExitFailed => write!(f, "ExitFailed"),
            PositionState::Phantom => write!(f, "Phantom"),
            PositionState::Reconciling => write!(f, "Reconciling"),
        }
    }
}

impl std::str::FromStr for PositionState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(PositionState::Open),
            "Closing" => Ok(PositionState::Closing),
            "Closed" => Ok(PositionState::Closed),
            "ExitPending" => Ok(PositionState::ExitPending),
            "ExitFailed" => Ok(PositionState::ExitFailed),
            "Phantom" => Ok(PositionState::Phantom),
            "Reconciling" => Ok(PositionState::Reconciling),
            _ => Err(format!("Unknown position state: {s}")),
        }
    }
}

/// Position state history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStateHistory {
    pub position_id: i64,
    pub state: PositionState,
    pub changed_at: DateTime<Utc>,
    pub reason: Option<String>,
}

/// Token snapshot captured at position opening or closing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub id: Option<i64>,
    pub position_id: i64,
    pub snapshot_type: String, // "opening"or "closing"
    pub mint: String,
    // DexScreener data
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_native: Option<f64>,
    pub dex_id: Option<String>,
    pub pair_address: Option<String>,
    pub pair_url: Option<String>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub pair_created_at: Option<i64>,
    // Liquidity data
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,
    // Volume data
    pub volume_h24: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_m5: Option<f64>,
    // Transaction stats
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    // Price change data
    pub price_change_h24: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_m5: Option<f64>,
    // Token meta
    pub token_uri: Option<String>,
    pub token_description: Option<String>,
    pub token_image: Option<String>,
    pub token_website: Option<String>,
    pub token_twitter: Option<String>,
    pub token_telegram: Option<String>,
    // Snapshot metadata
    pub snapshot_time: DateTime<Utc>,
    pub api_fetch_time: DateTime<Utc>,
    pub data_freshness_score: i32,
}

/// Position tracking record for price updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionTracking {
    pub position_id: i64,
    pub price: f64,
    pub price_source: String,      // "pool", "api", "cache"
    pub pool_type: Option<String>, // e.g., "RAYDIUM CPMM"
    pub pool_address: Option<String>,
    pub api_price: Option<f64>,
    pub tracked_at: DateTime<Utc>,
}

/// Aggregated trading statistics for a time period (dashboard optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodTradingStats {
    pub buys: i64,
    pub sells: i64,
    pub profit_sol: f64,
    pub loss_sol: f64,
    pub net_pnl_sol: f64,
    pub drawdown_percent: f64,
    pub win_rate: f64,
}

/// Statistics about positions database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionsDatabaseStats {
    pub total_positions: u64,
    pub open_positions: u64,
    pub closed_positions: u64,
    pub phantom_positions: u64,
    pub total_state_history: u64,
    pub total_tracking_records: u64,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

/// High-performance, thread-safe database manager for positions
/// Replaces JSON file-based storage with SQLite database
#[derive(Clone)]
pub struct PositionsDatabase {
    pub(super) pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    pub(super) database_path: String,
    pub(super) schema_version: u32,
}
