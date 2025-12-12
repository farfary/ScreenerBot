//! Strategy Executor for Volume Aggregator
//!
//! Manages wallet distribution based on configured strategy (RoundRobin, Random, Burst).
//! Provides functions for delay and amount calculation.

use crate::tools::types::{DelayConfig, DistributionStrategy, SizingConfig};
use crate::wallets::WalletWithKey;

/// Strategy executor for managing wallet distribution
pub struct StrategyExecutor {
    /// Wallets available for distribution
    wallets: Vec<WalletWithKey>,
    /// Distribution strategy to use
    strategy: DistributionStrategy,
    /// Current wallet index (for RoundRobin/Burst)
    current_index: usize,
    /// Operation count (for Burst tracking)
    operation_count: usize,
}

impl StrategyExecutor {
    /// Create a new strategy executor
    pub fn new(wallets: Vec<WalletWithKey>, strategy: DistributionStrategy) -> Self {
        Self {
            wallets,
            strategy,
            current_index: 0,
            operation_count: 0,
        }
    }

    /// Get the number of available wallets
    pub fn wallet_count(&self) -> usize {
        self.wallets.len()
    }

    /// Check if there are wallets available
    pub fn has_wallets(&self) -> bool {
        !self.wallets.is_empty()
    }

    /// Get all wallets
    pub fn wallets(&self) -> &[WalletWithKey] {
        &self.wallets
    }

    /// Get next wallet based on strategy
    /// Returns None if no wallets available
    pub fn next_wallet(&mut self) -> Option<&WalletWithKey> {
        if self.wallets.is_empty() {
            return None;
        }

        let index = match &self.strategy {
            DistributionStrategy::RoundRobin => {
                // Simple round-robin: cycle through wallets in order
                let idx = self.current_index;
                self.current_index = (self.current_index + 1) % self.wallets.len();
                idx
            }
            DistributionStrategy::Random => {
                // Random selection for each operation
                use rand::Rng;
                rand::thread_rng().gen_range(0..self.wallets.len())
            }
            DistributionStrategy::Burst { burst_size } => {
                // Use same wallet for burst_size operations before rotating
                let burst = *burst_size as usize;
                if burst == 0 {
                    // Fallback to round-robin if burst size is 0
                    let idx = self.current_index;
                    self.current_index = (self.current_index + 1) % self.wallets.len();
                    idx
                } else {
                    // Calculate which wallet to use based on operation count
                    let idx = (self.operation_count / burst) % self.wallets.len();
                    idx
                }
            }
        };

        self.operation_count += 1;
        self.wallets.get(index)
    }

    /// Get wallet at specific index
    pub fn get_wallet(&self, index: usize) -> Option<&WalletWithKey> {
        self.wallets.get(index)
    }

    /// Reset the executor state (for resume scenarios)
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.operation_count = 0;
    }

    /// Set operation count (for resume from specific point)
    pub fn set_operation_count(&mut self, count: usize) {
        self.operation_count = count;
    }

    /// Get current operation count
    pub fn operation_count(&self) -> usize {
        self.operation_count
    }
}

/// Calculate delay in milliseconds based on delay configuration
pub fn calculate_delay(config: &DelayConfig) -> u64 {
    config.get_delay_ms()
}

/// Calculate amount in SOL based on sizing configuration
pub fn calculate_amount(config: &SizingConfig) -> f64 {
    config.get_amount_sol()
}

/// Calculate amount with remaining volume constraint
/// Returns the smaller of calculated amount and remaining volume
pub fn calculate_amount_clamped(config: &SizingConfig, remaining_volume: f64) -> f64 {
    let amount = calculate_amount(config);
    amount.min(remaining_volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We don't write actual tests per project rules
    // These are placeholder function signatures only
}
