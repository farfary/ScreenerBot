//! Common types for tools module

use serde::{Deserialize, Serialize};

/// Result type for tool operations
pub type ToolResult<T> = Result<T, String>;

/// Status of a tool execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    /// Tool is initialized and ready to run
    Ready,
    /// Tool is currently executing
    Running,
    /// Tool completed successfully
    Completed,
    /// Tool failed with an error
    Failed,
    /// Tool was aborted by user
    Aborted,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolStatus::Ready => write!(f, "ready"),
            ToolStatus::Running => write!(f, "running"),
            ToolStatus::Completed => write!(f, "completed"),
            ToolStatus::Failed => write!(f, "failed"),
            ToolStatus::Aborted => write!(f, "aborted"),
        }
    }
}

impl Default for ToolStatus {
    fn default() -> Self {
        ToolStatus::Ready
    }
}

// =============================================================================
// DELAY CONFIGURATION
// =============================================================================

/// Configuration for delays between operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DelayConfig {
    /// Fixed delay between operations
    Fixed {
        /// Delay in milliseconds
        delay_ms: u64,
    },
    /// Random delay between min and max
    Random {
        /// Minimum delay in milliseconds
        min_ms: u64,
        /// Maximum delay in milliseconds
        max_ms: u64,
    },
}

impl Default for DelayConfig {
    fn default() -> Self {
        DelayConfig::Fixed { delay_ms: 1000 }
    }
}

impl DelayConfig {
    /// Create a fixed delay config
    pub fn fixed(delay_ms: u64) -> Self {
        DelayConfig::Fixed { delay_ms }
    }

    /// Create a random delay config
    pub fn random(min_ms: u64, max_ms: u64) -> Self {
        DelayConfig::Random { min_ms, max_ms }
    }

    /// Get the actual delay to use (handles randomization)
    pub fn get_delay_ms(&self) -> u64 {
        match self {
            DelayConfig::Fixed { delay_ms } => *delay_ms,
            DelayConfig::Random { min_ms, max_ms } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(*min_ms..=*max_ms)
            }
        }
    }

    /// Convert to database values
    pub fn to_db_values(&self) -> (String, i64, Option<i64>) {
        match self {
            DelayConfig::Fixed { delay_ms } => ("fixed".to_string(), *delay_ms as i64, None),
            DelayConfig::Random { min_ms, max_ms } => {
                ("random".to_string(), *min_ms as i64, Some(*max_ms as i64))
            }
        }
    }

    /// Create from database values
    pub fn from_db_values(delay_type: &str, delay_ms: i64, delay_max_ms: Option<i64>) -> Self {
        match delay_type {
            "random" => DelayConfig::Random {
                min_ms: delay_ms as u64,
                max_ms: delay_max_ms.unwrap_or(delay_ms) as u64,
            },
            _ => DelayConfig::Fixed {
                delay_ms: delay_ms as u64,
            },
        }
    }
}

// =============================================================================
// SIZING CONFIGURATION
// =============================================================================

/// Configuration for transaction sizing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SizingConfig {
    /// Fixed amount per transaction
    Fixed {
        /// Amount in SOL
        amount_sol: f64,
    },
    /// Random amount between min and max
    Random {
        /// Minimum amount in SOL
        min_sol: f64,
        /// Maximum amount in SOL
        max_sol: f64,
    },
}

impl Default for SizingConfig {
    fn default() -> Self {
        SizingConfig::Fixed { amount_sol: 0.01 }
    }
}

impl SizingConfig {
    /// Create a fixed sizing config
    pub fn fixed(amount_sol: f64) -> Self {
        SizingConfig::Fixed { amount_sol }
    }

    /// Create a random sizing config
    pub fn random(min_sol: f64, max_sol: f64) -> Self {
        SizingConfig::Random { min_sol, max_sol }
    }

    /// Get the actual amount to use (handles randomization)
    pub fn get_amount_sol(&self) -> f64 {
        match self {
            SizingConfig::Fixed { amount_sol } => *amount_sol,
            SizingConfig::Random { min_sol, max_sol } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(*min_sol..=*max_sol)
            }
        }
    }

    /// Convert to database values
    pub fn to_db_values(&self) -> (String, f64, Option<f64>) {
        match self {
            SizingConfig::Fixed { amount_sol } => ("fixed".to_string(), *amount_sol, None),
            SizingConfig::Random { min_sol, max_sol } => {
                ("random".to_string(), *min_sol, Some(*max_sol))
            }
        }
    }

    /// Create from database values
    pub fn from_db_values(sizing_type: &str, amount_sol: f64, amount_max_sol: Option<f64>) -> Self {
        match sizing_type {
            "random" => SizingConfig::Random {
                min_sol: amount_sol,
                max_sol: amount_max_sol.unwrap_or(amount_sol),
            },
            _ => SizingConfig::Fixed { amount_sol },
        }
    }

    /// Validate the sizing configuration
    pub fn validate(&self) -> Result<(), String> {
        match self {
            SizingConfig::Fixed { amount_sol } => {
                if *amount_sol < 0.001 {
                    return Err("Amount must be at least 0.001 SOL".to_string());
                }
                Ok(())
            }
            SizingConfig::Random { min_sol, max_sol } => {
                if *min_sol < 0.001 {
                    return Err("Minimum amount must be at least 0.001 SOL".to_string());
                }
                if *max_sol < *min_sol {
                    return Err("Maximum amount must be >= minimum amount".to_string());
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// WALLET MODE
// =============================================================================

/// Mode for wallet selection in tools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletMode {
    /// Use a single specific wallet
    Single,
    /// Use specific selected wallets
    Selected,
    /// Automatically select wallets based on criteria
    AutoSelect,
}

impl Default for WalletMode {
    fn default() -> Self {
        WalletMode::AutoSelect
    }
}

impl WalletMode {
    /// Convert to database value
    pub fn to_db_value(&self) -> String {
        match self {
            WalletMode::Single => "single".to_string(),
            WalletMode::Selected => "selected".to_string(),
            WalletMode::AutoSelect => "auto_select".to_string(),
        }
    }

    /// Create from database value
    pub fn from_db_value(value: &str) -> Self {
        match value {
            "single" => WalletMode::Single,
            "selected" => WalletMode::Selected,
            _ => WalletMode::AutoSelect,
        }
    }
}

impl std::fmt::Display for WalletMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletMode::Single => write!(f, "Single Wallet"),
            WalletMode::Selected => write!(f, "Selected Wallets"),
            WalletMode::AutoSelect => write!(f, "Auto Select"),
        }
    }
}

// =============================================================================
// DISTRIBUTION STRATEGY
// =============================================================================

/// Strategy for distributing operations across wallets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DistributionStrategy {
    /// Cycle through wallets in order
    RoundRobin,
    /// Randomly select wallet for each operation
    Random,
    /// Execute multiple operations on same wallet before rotating
    Burst {
        /// Number of operations per wallet before rotating
        burst_size: u32,
    },
}

impl Default for DistributionStrategy {
    fn default() -> Self {
        DistributionStrategy::RoundRobin
    }
}

impl DistributionStrategy {
    /// Create a round-robin strategy
    pub fn round_robin() -> Self {
        DistributionStrategy::RoundRobin
    }

    /// Create a random strategy
    pub fn random() -> Self {
        DistributionStrategy::Random
    }

    /// Create a burst strategy
    pub fn burst(burst_size: u32) -> Self {
        DistributionStrategy::Burst { burst_size }
    }

    /// Convert to database value
    pub fn to_db_value(&self) -> String {
        match self {
            DistributionStrategy::RoundRobin => "round_robin".to_string(),
            DistributionStrategy::Random => "random".to_string(),
            DistributionStrategy::Burst { burst_size } => format!("burst:{}", burst_size),
        }
    }

    /// Create from database value
    pub fn from_db_value(value: &str) -> Self {
        if value.starts_with("burst:") {
            if let Some(size_str) = value.strip_prefix("burst:") {
                if let Ok(size) = size_str.parse::<u32>() {
                    return DistributionStrategy::Burst { burst_size: size };
                }
            }
            return DistributionStrategy::Burst { burst_size: 3 };
        }
        match value {
            "random" => DistributionStrategy::Random,
            _ => DistributionStrategy::RoundRobin,
        }
    }

    /// Select next wallet index based on strategy
    pub fn select_wallet_index(
        &self,
        current_index: usize,
        operation_count: usize,
        wallet_count: usize,
    ) -> usize {
        if wallet_count == 0 {
            return 0;
        }
        match self {
            DistributionStrategy::RoundRobin => operation_count % wallet_count,
            DistributionStrategy::Random => {
                use rand::Rng;
                rand::thread_rng().gen_range(0..wallet_count)
            }
            DistributionStrategy::Burst { burst_size } => {
                let burst = *burst_size as usize;
                if burst == 0 {
                    return operation_count % wallet_count;
                }
                (operation_count / burst) % wallet_count
            }
        }
    }
}

impl std::fmt::Display for DistributionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistributionStrategy::RoundRobin => write!(f, "Round Robin"),
            DistributionStrategy::Random => write!(f, "Random"),
            DistributionStrategy::Burst { burst_size } => write!(f, "Burst ({})", burst_size),
        }
    }
}
