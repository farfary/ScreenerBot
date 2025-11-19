//! Trader module constants

// Monitor intervals
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

// Cycle timing
pub const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
pub const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

// Timeouts and limits
pub const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;
pub const ENTRY_RESERVATION_TIMEOUT_SECS: u64 = 30;

// Debug flags
pub const DEBUG_FORCE_SELL_MODE: bool = false;
pub const DEBUG_FORCE_BUY_MODE: bool = false;
