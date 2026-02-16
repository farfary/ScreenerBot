//! Database row types and structures

use super::super::types::{DelayConfig, DistributionStrategy, SizingConfig, ToolStatus, WalletMode};

// =============================================================================
// VOLUME AGGREGATOR TYPES
// =============================================================================

/// VA session database row
#[derive(Debug, Clone)]
pub struct VaSessionRow {
    pub id: i64,
    pub session_id: String,
    pub token_mint: String,
    pub target_volume_sol: f64,
    pub actual_volume_sol: f64,
    pub delay_type: String,
    pub delay_ms: i64,
    pub delay_max_ms: Option<i64>,
    pub sizing_type: String,
    pub amount_sol: f64,
    pub amount_max_sol: Option<f64>,
    pub strategy: String,
    pub wallet_mode: String,
    pub wallet_addresses: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub error_message: Option<String>,
    pub successful_buys: i32,
    pub successful_sells: i32,
    pub failed_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl VaSessionRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            token_mint: row.get(2).map_err(|e| e.to_string())?,
            target_volume_sol: row.get(3).map_err(|e| e.to_string())?,
            actual_volume_sol: row.get(4).map_err(|e| e.to_string())?,
            delay_type: row.get(5).map_err(|e| e.to_string())?,
            delay_ms: row.get(6).map_err(|e| e.to_string())?,
            delay_max_ms: row.get(7).map_err(|e| e.to_string())?,
            sizing_type: row.get(8).map_err(|e| e.to_string())?,
            amount_sol: row.get(9).map_err(|e| e.to_string())?,
            amount_max_sol: row.get(10).map_err(|e| e.to_string())?,
            strategy: row.get(11).map_err(|e| e.to_string())?,
            wallet_mode: row.get(12).map_err(|e| e.to_string())?,
            wallet_addresses: row.get(13).map_err(|e| e.to_string())?,
            status: row.get(14).map_err(|e| e.to_string())?,
            started_at: row.get(15).map_err(|e| e.to_string())?,
            ended_at: row.get(16).map_err(|e| e.to_string())?,
            error_message: row.get(17).map_err(|e| e.to_string())?,
            successful_buys: row.get(18).map_err(|e| e.to_string())?,
            successful_sells: row.get(19).map_err(|e| e.to_string())?,
            failed_count: row.get(20).map_err(|e| e.to_string())?,
            created_at: row.get(21).map_err(|e| e.to_string())?,
            updated_at: row.get(22).map_err(|e| e.to_string())?,
        })
    }

    /// Parse delay config from row data
    pub fn get_delay_config(&self) -> DelayConfig {
        DelayConfig::from_db_values(&self.delay_type, self.delay_ms, self.delay_max_ms)
    }

    /// Parse sizing config from row data
    pub fn get_sizing_config(&self) -> SizingConfig {
        SizingConfig::from_db_values(&self.sizing_type, self.amount_sol, self.amount_max_sol)
    }

    /// Parse strategy from row data
    pub fn get_strategy(&self) -> DistributionStrategy {
        DistributionStrategy::from_db_value(&self.strategy)
    }

    /// Parse wallet mode from row data
    pub fn get_wallet_mode(&self) -> WalletMode {
        WalletMode::from_db_value(&self.wallet_mode)
    }

    /// Parse wallet addresses from JSON
    pub fn get_wallet_addresses(&self) -> Option<Vec<String>> {
        self.wallet_addresses
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
    }

    /// Parse status from row data
    pub fn get_status(&self) -> ToolStatus {
        match self.status.as_str() {
            "ready" => ToolStatus::Ready,
            "running" => ToolStatus::Running,
            "completed" => ToolStatus::Completed,
            "failed" => ToolStatus::Failed,
            "aborted" => ToolStatus::Aborted,
            _ => ToolStatus::Ready,
        }
    }
}

/// VA swap database row
#[derive(Debug, Clone)]
pub struct VaSwapRow {
    pub id: i64,
    pub session_id: String,
    pub tx_index: i32,
    pub wallet_address: String,
    pub is_buy: bool,
    pub amount_sol: f64,
    pub token_amount: Option<f64>,
    pub signature: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
}

impl VaSwapRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        let is_buy_int: i32 = row.get(4).map_err(|e| e.to_string())?;
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            tx_index: row.get(2).map_err(|e| e.to_string())?,
            wallet_address: row.get(3).map_err(|e| e.to_string())?,
            is_buy: is_buy_int != 0,
            amount_sol: row.get(5).map_err(|e| e.to_string())?,
            token_amount: row.get(6).map_err(|e| e.to_string())?,
            signature: row.get(7).map_err(|e| e.to_string())?,
            status: row.get(8).map_err(|e| e.to_string())?,
            error_message: row.get(9).map_err(|e| e.to_string())?,
            executed_at: row.get(10).map_err(|e| e.to_string())?,
            created_at: row.get(11).map_err(|e| e.to_string())?,
        })
    }
}

/// VA session analytics summary
#[derive(Debug, Clone)]
pub struct VaAnalyticsSummary {
    pub total_sessions: i64,
    pub total_volume_sol: f64,
    pub avg_success_rate: f64,
    pub completed_sessions: i64,
    pub failed_sessions: i64,
    pub aborted_sessions: i64,
}

// =============================================================================
// ATA CACHE TYPES
// =============================================================================

/// Failed ATA database row
#[derive(Debug, Clone)]
pub struct FailedAtaRow {
    pub ata_address: String,
    pub token_mint: Option<String>,
    pub wallet_address: String,
    pub failure_count: i32,
    pub last_error: Option<String>,
    pub first_failed_at: String,
    pub last_failed_at: String,
    pub next_retry_at: Option<String>,
    pub is_permanent_failure: bool,
}

impl FailedAtaRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        let is_permanent_int: i32 = row.get(8).map_err(|e| e.to_string())?;
        Ok(Self {
            ata_address: row.get(0).map_err(|e| e.to_string())?,
            token_mint: row.get(1).map_err(|e| e.to_string())?,
            wallet_address: row.get(2).map_err(|e| e.to_string())?,
            failure_count: row.get(3).map_err(|e| e.to_string())?,
            last_error: row.get(4).map_err(|e| e.to_string())?,
            first_failed_at: row.get(5).map_err(|e| e.to_string())?,
            last_failed_at: row.get(6).map_err(|e| e.to_string())?,
            next_retry_at: row.get(7).map_err(|e| e.to_string())?,
            is_permanent_failure: is_permanent_int != 0,
        })
    }
}

// =============================================================================
// TOOL FAVORITES TYPES
// =============================================================================

/// Tool favorite database row
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolFavoriteRow {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub tool_type: String,
    pub config_json: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub use_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ToolFavoriteRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            mint: row.get(1).map_err(|e| e.to_string())?,
            symbol: row.get(2).map_err(|e| e.to_string())?,
            name: row.get(3).map_err(|e| e.to_string())?,
            logo_url: row.get(4).map_err(|e| e.to_string())?,
            tool_type: row.get(5).map_err(|e| e.to_string())?,
            config_json: row.get(6).map_err(|e| e.to_string())?,
            label: row.get(7).map_err(|e| e.to_string())?,
            notes: row.get(8).map_err(|e| e.to_string())?,
            use_count: row.get(9).map_err(|e| e.to_string())?,
            last_used_at: row.get(10).map_err(|e| e.to_string())?,
            created_at: row.get(11).map_err(|e| e.to_string())?,
            updated_at: row.get(12).map_err(|e| e.to_string())?,
        })
    }
}

// =============================================================================
// MULTI-WALLET TYPES
// =============================================================================

/// Multi-wallet session database row
#[derive(Debug, Clone, serde::Serialize)]
pub struct MwSessionRow {
    pub id: i64,
    pub session_id: String,
    pub session_type: String,
    pub token_mint: Option<String>,
    pub total_wallets: i32,
    pub target_amount_sol: Option<f64>,
    pub min_amount_sol: Option<f64>,
    pub max_amount_sol: Option<f64>,
    pub delay_ms: i64,
    pub delay_max_ms: Option<i64>,
    pub concurrency: i32,
    pub sol_buffer: f64,
    pub status: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub error_message: Option<String>,
    pub wallets_funded: i32,
    pub successful_ops: i32,
    pub failed_ops: i32,
    pub total_sol_spent: f64,
    pub total_sol_recovered: f64,
    pub created_at: String,
    pub updated_at: String,
}

impl MwSessionRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            session_type: row.get(2).map_err(|e| e.to_string())?,
            token_mint: row.get(3).map_err(|e| e.to_string())?,
            total_wallets: row.get(4).map_err(|e| e.to_string())?,
            target_amount_sol: row.get(5).map_err(|e| e.to_string())?,
            min_amount_sol: row.get(6).map_err(|e| e.to_string())?,
            max_amount_sol: row.get(7).map_err(|e| e.to_string())?,
            delay_ms: row.get(8).map_err(|e| e.to_string())?,
            delay_max_ms: row.get(9).map_err(|e| e.to_string())?,
            concurrency: row.get(10).map_err(|e| e.to_string())?,
            sol_buffer: row.get(11).map_err(|e| e.to_string())?,
            status: row.get(12).map_err(|e| e.to_string())?,
            started_at: row.get(13).map_err(|e| e.to_string())?,
            ended_at: row.get(14).map_err(|e| e.to_string())?,
            error_message: row.get(15).map_err(|e| e.to_string())?,
            wallets_funded: row.get(16).map_err(|e| e.to_string())?,
            successful_ops: row.get(17).map_err(|e| e.to_string())?,
            failed_ops: row.get(18).map_err(|e| e.to_string())?,
            total_sol_spent: row.get(19).map_err(|e| e.to_string())?,
            total_sol_recovered: row.get(20).map_err(|e| e.to_string())?,
            created_at: row.get(21).map_err(|e| e.to_string())?,
            updated_at: row.get(22).map_err(|e| e.to_string())?,
        })
    }

    /// Parse status from row data
    pub fn get_status(&self) -> ToolStatus {
        match self.status.as_str() {
            "pending" => ToolStatus::Ready,
            "running" => ToolStatus::Running,
            "completed" => ToolStatus::Completed,
            "failed" => ToolStatus::Failed,
            "aborted" => ToolStatus::Aborted,
            _ => ToolStatus::Ready,
        }
    }
}

/// Multi-wallet operation database row
#[derive(Debug, Clone, serde::Serialize)]
pub struct MwWalletOpRow {
    pub id: i64,
    pub session_id: String,
    pub wallet_id: i32,
    pub wallet_address: String,
    pub op_index: i32,
    pub op_type: String,
    pub amount_sol: Option<f64>,
    pub token_amount: Option<f64>,
    pub signature: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
}

impl MwWalletOpRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            wallet_id: row.get(2).map_err(|e| e.to_string())?,
            wallet_address: row.get(3).map_err(|e| e.to_string())?,
            op_index: row.get(4).map_err(|e| e.to_string())?,
            op_type: row.get(5).map_err(|e| e.to_string())?,
            amount_sol: row.get(6).map_err(|e| e.to_string())?,
            token_amount: row.get(7).map_err(|e| e.to_string())?,
            signature: row.get(8).map_err(|e| e.to_string())?,
            status: row.get(9).map_err(|e| e.to_string())?,
            error_message: row.get(10).map_err(|e| e.to_string())?,
            executed_at: row.get(11).map_err(|e| e.to_string())?,
            created_at: row.get(12).map_err(|e| e.to_string())?,
        })
    }
}

/// Configuration for creating a multi-wallet session
#[derive(Debug, Clone)]
pub struct MwSessionConfig {
    pub session_type: String,
    pub token_mint: Option<String>,
    pub total_wallets: i32,
    pub target_amount_sol: Option<f64>,
    pub min_amount_sol: Option<f64>,
    pub max_amount_sol: Option<f64>,
    pub delay_ms: i64,
    pub delay_max_ms: Option<i64>,
    pub concurrency: i32,
    pub sol_buffer: f64,
}

// =============================================================================
// WATCHED TOKENS TYPES
// =============================================================================

/// Watched token database row
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatchedToken {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub pool_address: String,
    pub pool_source: String,
    pub pool_dex: Option<String>,
    pub pool_pair: Option<String>,
    pub pool_liquidity: Option<f64>,
    pub watch_type: String,
    pub trigger_amount_sol: Option<f64>,
    pub action_amount_sol: Option<f64>,
    pub slippage_bps: i32,
    pub is_active: bool,
    pub last_checked_at: Option<String>,
    pub last_trade_signature: Option<String>,
    pub trades_detected: i32,
    pub actions_triggered: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl WatchedToken {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        let is_active_int: i32 = row.get(12).map_err(|e| e.to_string())?;
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            mint: row.get(1).map_err(|e| e.to_string())?,
            symbol: row.get(2).map_err(|e| e.to_string())?,
            pool_address: row.get(3).map_err(|e| e.to_string())?,
            pool_source: row.get(4).map_err(|e| e.to_string())?,
            pool_dex: row.get(5).map_err(|e| e.to_string())?,
            pool_pair: row.get(6).map_err(|e| e.to_string())?,
            pool_liquidity: row.get(7).map_err(|e| e.to_string())?,
            watch_type: row.get(8).map_err(|e| e.to_string())?,
            trigger_amount_sol: row.get(9).map_err(|e| e.to_string())?,
            action_amount_sol: row.get(10).map_err(|e| e.to_string())?,
            slippage_bps: row.get(11).map_err(|e| e.to_string())?,
            is_active: is_active_int != 0,
            last_checked_at: row.get(13).map_err(|e| e.to_string())?,
            last_trade_signature: row.get(14).map_err(|e| e.to_string())?,
            trades_detected: row.get(15).map_err(|e| e.to_string())?,
            actions_triggered: row.get(16).map_err(|e| e.to_string())?,
            created_at: row.get(17).map_err(|e| e.to_string())?,
            updated_at: row.get(18).map_err(|e| e.to_string())?,
        })
    }
}

/// Configuration for adding a watched token
#[derive(Debug, Clone)]
pub struct WatchedTokenConfig {
    pub mint: String,
    pub symbol: Option<String>,
    pub pool_address: String,
    pub pool_source: String,
    pub pool_dex: Option<String>,
    pub pool_pair: Option<String>,
    pub pool_liquidity: Option<f64>,
    pub watch_type: String,
    pub trigger_amount_sol: Option<f64>,
    pub action_amount_sol: Option<f64>,
    pub slippage_bps: Option<i32>,
}
