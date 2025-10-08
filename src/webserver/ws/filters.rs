/// Per-topic filter definitions
///
/// Each topic can have custom filters that are applied server-side
/// before messages are queued for a connection.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ============================================================================
// FILTER STRUCTS
// ============================================================================

/// Services filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ServicesFilter {
    /// Only show unhealthy services
    #[serde(default)]
    pub only_unhealthy: bool,

    /// Filter by service names
    #[serde(default)]
    pub names: Vec<String>,

    /// Filter by priority range
    #[serde(default)]
    pub min_priority: Option<i32>,
    #[serde(default)]
    pub max_priority: Option<i32>,
}

/// Positions filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PositionsFilter {
    /// Filter by status (e.g., "open", "closed")
    #[serde(default)]
    pub status: Vec<String>,

    /// Filter by specific mints
    #[serde(default)]
    pub mints: Vec<String>,

    /// Only show positions with profit/loss
    #[serde(default)]
    pub only_profitable: Option<bool>,
}

/// Prices filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PricesFilter {
    /// Filter by specific mints
    #[serde(default)]
    pub mints: Vec<String>,

    /// Minimum price change threshold (percent)
    #[serde(default)]
    pub min_change_percent: Option<f64>,
}

/// Tokens filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TokensFilter {
    /// Only watched tokens
    #[serde(default)]
    pub watched: bool,

    /// Exclude blacklisted tokens
    #[serde(default)]
    pub exclude_blacklisted: bool,

    /// Search query (symbol/name/mint)
    #[serde(default)]
    pub search: Option<String>,

    /// Minimum security score
    #[serde(default)]
    pub min_security_score: Option<i32>,
}

/// Events filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EventsFilter {
    /// Filter by categories
    #[serde(default)]
    pub categories: Vec<String>,

    /// Minimum severity level
    #[serde(default)]
    pub min_level: Option<String>,

    /// Filter by mint reference
    #[serde(default)]
    pub mint: Option<String>,

    /// Filter by any reference
    #[serde(default)]
    pub reference: Option<String>,

    /// Only events since this ID
    #[serde(default)]
    pub since_id: Option<i64>,
}

/// OHLCV filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OhlcvFilter {
    /// Filter by specific mints
    #[serde(default)]
    pub mints: Vec<String>,

    /// Filter by timeframe (e.g., "1m", "5m", "1h")
    #[serde(default)]
    pub timeframes: Vec<String>,
}

/// Trader state filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TraderFilter {
    /// Include entry mode details
    #[serde(default)]
    pub include_entry_mode: bool,

    /// Include position limits
    #[serde(default)]
    pub include_limits: bool,
}

/// Wallet balances filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WalletFilter {
    /// Minimum balance to show (SOL)
    #[serde(default)]
    pub min_balance_sol: Option<f64>,

    /// Only show specific tokens
    #[serde(default)]
    pub token_mints: Vec<String>,
}

/// Transactions filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TransactionsFilter {
    /// Filter by transaction type (swap, buy, sell, transfer, ata)
    #[serde(default)]
    pub types: Vec<String>,

    /// Filter by mint
    #[serde(default)]
    pub mint: Option<String>,

    /// Only confirmed transactions
    #[serde(default)]
    pub only_confirmed: bool,
}

/// Security alerts filter
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SecurityFilter {
    /// Minimum risk level (e.g., "high", "critical")
    #[serde(default)]
    pub min_risk_level: Option<String>,

    /// Filter by specific mints
    #[serde(default)]
    pub mints: Vec<String>,
}

/// System status filter (usually no filtering needed, but included for completeness)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SystemStatusFilter {
    /// Include detailed RPC stats
    #[serde(default)]
    pub include_rpc_stats: bool,

    /// Include OHLCV stats
    #[serde(default)]
    pub include_ohlcv_stats: bool,
}

// ============================================================================
// FILTER APPLICATION HELPERS
// ============================================================================

impl ServicesFilter {
    /// Check if a service matches this filter
    pub fn matches(&self, service_name: &str, is_unhealthy: bool, priority: i32) -> bool {
        // Check unhealthy filter
        if self.only_unhealthy && !is_unhealthy {
            return false;
        }

        // Check name filter
        if !self.names.is_empty() && !self.names.contains(&service_name.to_string()) {
            return false;
        }

        // Check priority range
        if let Some(min) = self.min_priority {
            if priority < min {
                return false;
            }
        }
        if let Some(max) = self.max_priority {
            if priority > max {
                return false;
            }
        }

        true
    }
}

impl PositionsFilter {
    /// Check if a position matches this filter
    pub fn matches(&self, status: &str, mint: &str, is_profitable: Option<bool>) -> bool {
        // Check status filter
        if !self.status.is_empty() && !self.status.contains(&status.to_string()) {
            return false;
        }

        // Check mint filter
        if !self.mints.is_empty() && !self.mints.contains(&mint.to_string()) {
            return false;
        }

        // Check profitable filter
        if let (Some(filter_profitable), Some(actual_profitable)) =
            (self.only_profitable, is_profitable)
        {
            if filter_profitable != actual_profitable {
                return false;
            }
        }

        true
    }
}

impl PricesFilter {
    /// Check if a price update matches this filter
    pub fn matches(&self, mint: &str, change_percent: Option<f64>) -> bool {
        // Check mint filter
        if !self.mints.is_empty() && !self.mints.contains(&mint.to_string()) {
            return false;
        }

        // Check change threshold
        if let (Some(threshold), Some(actual)) = (self.min_change_percent, change_percent) {
            if actual.abs() < threshold {
                return false;
            }
        }

        true
    }
}

impl TokensFilter {
    /// Check if a token matches this filter
    pub fn matches(
        &self,
        is_watched: bool,
        is_blacklisted: bool,
        symbol: &str,
        name: &str,
        mint: &str,
        security_score: i32,
    ) -> bool {
        // Check watched filter
        if self.watched && !is_watched {
            return false;
        }

        // Check blacklist exclusion
        if self.exclude_blacklisted && is_blacklisted {
            return false;
        }

        // Check search query
        if let Some(ref query) = self.search {
            let query_lower = query.to_lowercase();
            let matches = symbol.to_lowercase().contains(&query_lower)
                || name.to_lowercase().contains(&query_lower)
                || mint.to_lowercase().contains(&query_lower);
            if !matches {
                return false;
            }
        }

        // Check security score
        if let Some(min_score) = self.min_security_score {
            if security_score < min_score {
                return false;
            }
        }

        true
    }
}

impl EventsFilter {
    /// Check if an event matches this filter
    pub fn matches(
        &self,
        category: &str,
        severity: &str,
        mint: Option<&str>,
        reference: Option<&str>,
        event_id: i64,
    ) -> bool {
        // Check category filter
        if !self.categories.is_empty() && !self.categories.contains(&category.to_string()) {
            return false;
        }

        // Check severity filter
        if let Some(ref min_level) = self.min_level {
            let severity_rank = match severity.to_lowercase().as_str() {
                "debug" => 0,
                "info" => 1,
                "warn" => 2,
                "error" => 3,
                _ => 1,
            };
            let min_rank = match min_level.to_lowercase().as_str() {
                "debug" => 0,
                "info" => 1,
                "warn" => 2,
                "error" => 3,
                _ => 1,
            };
            if severity_rank < min_rank {
                return false;
            }
        }

        // Check mint filter
        if let Some(ref filter_mint) = self.mint {
            if mint != Some(filter_mint.as_str()) {
                return false;
            }
        }

        // Check reference filter
        if let Some(ref filter_ref) = self.reference {
            if reference != Some(filter_ref.as_str()) {
                return false;
            }
        }

        // Check since_id filter
        if let Some(since) = self.since_id {
            if event_id <= since {
                return false;
            }
        }

        true
    }
}

impl OhlcvFilter {
    /// Check if an OHLCV update matches this filter
    pub fn matches(&self, mint: &str, timeframe: &str) -> bool {
        // Check mint filter
        if !self.mints.is_empty() && !self.mints.contains(&mint.to_string()) {
            return false;
        }

        // Check timeframe filter
        if !self.timeframes.is_empty() && !self.timeframes.contains(&timeframe.to_string()) {
            return false;
        }

        true
    }
}

impl TransactionsFilter {
    /// Check if a transaction matches this filter
    pub fn matches(&self, tx_type: &str, mint: Option<&str>, is_confirmed: bool) -> bool {
        // Check type filter
        if !self.types.is_empty() && !self.types.contains(&tx_type.to_string()) {
            return false;
        }

        // Check mint filter
        if let Some(ref filter_mint) = self.mint {
            if mint != Some(filter_mint.as_str()) {
                return false;
            }
        }

        // Check confirmed filter
        if self.only_confirmed && !is_confirmed {
            return false;
        }

        true
    }
}

impl SecurityFilter {
    /// Check if a security alert matches this filter
    pub fn matches(&self, risk_level: &str, mint: &str) -> bool {
        // Check risk level filter
        if let Some(ref min_level) = self.min_risk_level {
            let risk_rank = match risk_level.to_lowercase().as_str() {
                "low" => 0,
                "medium" => 1,
                "high" => 2,
                "critical" => 3,
                _ => 1,
            };
            let min_rank = match min_level.to_lowercase().as_str() {
                "low" => 0,
                "medium" => 1,
                "high" => 2,
                "critical" => 3,
                _ => 1,
            };
            if risk_rank < min_rank {
                return false;
            }
        }

        // Check mint filter
        if !self.mints.is_empty() && !self.mints.contains(&mint.to_string()) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_services_filter() {
        let filter = ServicesFilter {
            only_unhealthy: true,
            names: vec!["trader".to_string()],
            min_priority: None,
            max_priority: None,
        };

        assert!(filter.matches("trader", true, 50));
        assert!(!filter.matches("trader", false, 50));
        assert!(!filter.matches("pools", true, 50));
    }

    #[test]
    fn test_events_filter() {
        let filter = EventsFilter {
            categories: vec!["swap".to_string()],
            min_level: Some("warn".to_string()),
            mint: None,
            reference: None,
            since_id: Some(100),
        };

        assert!(filter.matches("swap", "error", None, None, 101));
        assert!(!filter.matches("swap", "info", None, None, 101));
        assert!(!filter.matches("position", "error", None, None, 101));
        assert!(!filter.matches("swap", "error", None, None, 100));
    }
}
