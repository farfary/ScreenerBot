// tokens/priorities.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    Pool,
    High,
    Medium,
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

impl Priority {
    /// Convert integer value to Priority
    pub fn from_value(value: i32) -> Self {
        match value {
            100 => Priority::Critical,
            75 => Priority::Pool,
            50 => Priority::High,
            25 => Priority::Medium,
            _ => Priority::Low,
        }
    }

    /// Convert Priority to integer value
    pub fn to_value(&self) -> i32 {
        match self {
            Priority::Critical => 100,
            Priority::Pool => 75,
            Priority::High => 50,
            Priority::Medium => 25,
            Priority::Low => 10,
        }
    }
}
