// tokens/priorities.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn pools_refresh_ttl_secs(self) -> u64 {
        match self {
            Priority::Critical => 30,
            Priority::High => 60,
            Priority::Medium => 300,
            Priority::Low => 900,
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}
