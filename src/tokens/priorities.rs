// tokens/priorities.rs

use serde::{Deserialize, Serialize};

/// Token update priority based on token state
/// 
/// Priority levels are named by the token's current state rather than abstract levels.
/// Higher numeric values = more frequent updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Tokens with open trading positions - highest priority (100)
    /// Update interval: 5 seconds
    OpenPosition,
    
    /// Tokens tracked by Pool Service - very high priority (75)
    /// Update interval: 7 seconds
    PoolTracked,
    
    /// Tokens that passed filtering criteria - high priority (60)
    /// Update interval: 8 seconds
    FilterPassed,
    
    /// New tokens without market data yet - medium-high priority (55)
    /// Update interval: 10 seconds (immediate seeding)
    Uninitialized,
    
    /// Tokens with stale market data - medium priority (40)
    /// Update interval: 15 seconds
    Stale,
    
    /// Regular tokens with fresh data - standard priority (25)
    /// Update interval: 20 seconds
    Standard,
    
    /// Oldest tokens being refreshed in background - low priority (10)
    /// Update interval: 30 seconds
    Background,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Standard
    }
}

impl Priority {
    /// Convert integer value to Priority
    pub fn from_value(value: i32) -> Self {
        match value {
            100 => Priority::OpenPosition,
            75 => Priority::PoolTracked,
            60 => Priority::FilterPassed,
            55 => Priority::Uninitialized,
            40 => Priority::Stale,
            25 => Priority::Standard,
            10 => Priority::Background,
            _ => Priority::Standard,
        }
    }

    /// Convert Priority to integer value
    pub fn to_value(&self) -> i32 {
        match self {
            Priority::OpenPosition => 100,
            Priority::PoolTracked => 75,
            Priority::FilterPassed => 60,
            Priority::Uninitialized => 55,
            Priority::Stale => 40,
            Priority::Standard => 25,
            Priority::Background => 10,
        }
    }
}
