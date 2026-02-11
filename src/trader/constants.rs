//! Trader module constants

// Monitor intervals
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

// Cycle timing
pub const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
pub const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

// Timeouts and limits
pub const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;
pub const ENTRY_RESERVATION_TIMEOUT_SECS: u64 = 120; // 2 minutes for slow evaluations
pub const STRATEGY_EVALUATION_TIMEOUT_SECS: u64 = 5;

// Safety thresholds
pub const EMERGENCY_LOSS_THRESHOLD_PCT: f64 = 90.0;

// Trade size limits
pub const MAX_TRADE_SIZE_MULTIPLIER: f64 = 100.0;
pub const MIN_TRADE_SIZE_SOL: f64 = 0.001;

// History limits
pub const MANUAL_TRADE_HISTORY_LIMIT: usize = 1000;

// Strategy engine cache
pub const STRATEGY_CACHE_MAX_ENTRIES: usize = 1000;
