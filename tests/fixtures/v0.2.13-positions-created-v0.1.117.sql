-- Positions storage as v0.2.13 leaves a store first created by v0.1.117 to v0.1.120:
-- `manual_management` in the original table, the provenance and chain columns appended,
-- INTEGER amounts under the legacy unit names, and the four legacy `positions` indexes.

CREATE TABLE positions (
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
, origin_kind TEXT NOT NULL DEFAULT 'auto', origin_ref TEXT, management TEXT NOT NULL DEFAULT 'auto_trader', round_key TEXT, basis_complete BOOLEAN NOT NULL DEFAULT 1, history_complete BOOLEAN NOT NULL DEFAULT 1, holding_state TEXT, chain_id TEXT NOT NULL DEFAULT 'solana');
CREATE TABLE position_states (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  position_id INTEGER NOT NULL,
  state TEXT NOT NULL, -- 'Open', 'Closing', 'Closed', 'ExitPending', 'ExitFailed', 'Phantom', 'Reconciling'
  changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  reason TEXT, -- Optional reason for state change
  FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
CREATE TABLE position_exits (
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
CREATE TABLE position_entries (
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
CREATE TABLE position_tracking (
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
CREATE TABLE position_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE token_snapshots (
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
CREATE INDEX idx_positions_entry_time ON positions(entry_time DESC);
CREATE INDEX idx_positions_exit_time ON positions(exit_time DESC);
CREATE INDEX idx_positions_mint_exit_time ON positions(mint, exit_time DESC);
CREATE INDEX idx_positions_state ON positions(id, position_type, exit_time);
CREATE INDEX idx_position_states_position_id ON position_states(position_id, changed_at DESC);
CREATE INDEX idx_position_states_state ON position_states(state, changed_at DESC);
CREATE INDEX idx_position_tracking_position_id ON position_tracking(position_id, tracked_at DESC);
CREATE INDEX idx_position_tracking_price ON position_tracking(price, tracked_at DESC);
CREATE INDEX idx_token_snapshots_position_id ON token_snapshots(position_id, snapshot_type);
CREATE INDEX idx_token_snapshots_mint ON token_snapshots(mint, snapshot_time DESC);
CREATE INDEX idx_token_snapshots_type ON token_snapshots(snapshot_type, snapshot_time DESC);
CREATE INDEX idx_position_exits_wallet ON position_exits(wallet_address);
CREATE INDEX idx_position_exits_position_id ON position_exits(position_id, timestamp DESC);
CREATE INDEX idx_position_exits_timestamp ON position_exits(timestamp DESC);
CREATE INDEX idx_position_entries_wallet ON position_entries(wallet_address);
CREATE INDEX idx_position_entries_position_id ON position_entries(position_id, timestamp DESC);
CREATE INDEX idx_position_entries_timestamp ON position_entries(timestamp DESC);
CREATE INDEX idx_positions_archived ON positions(archived);
CREATE UNIQUE INDEX idx_positions_round_key ON positions(chain_id, wallet_address, round_key) WHERE round_key IS NOT NULL;
CREATE INDEX idx_positions_chain_wallet ON positions(chain_id, wallet_address);
CREATE INDEX idx_positions_chain_mint ON positions(chain_id, mint);
CREATE INDEX idx_positions_chain_entry_signature ON positions(chain_id, entry_transaction_signature);
CREATE INDEX idx_positions_chain_exit_signature ON positions(chain_id, exit_transaction_signature);
CREATE INDEX idx_positions_wallet ON positions(wallet_address);
CREATE INDEX idx_positions_mint ON positions(mint);
CREATE INDEX idx_positions_entry_signature ON positions(entry_transaction_signature);
CREATE INDEX idx_positions_exit_signature ON positions(exit_transaction_signature);
