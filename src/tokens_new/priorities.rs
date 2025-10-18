// tokens_new/priorities.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn decimals_refresh_ttl_secs(self) -> u64 {
        match self {
            Priority::Critical => 120,
            Priority::High => 300,
            Priority::Medium => 900,
            Priority::Low => 3600,
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}
