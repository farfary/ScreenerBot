//! Database row types and structures

use crate::errors::DatabaseError;
use crate::tools::Error;

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
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        let is_permanent_int: i32 = row.get(8).map_err(DatabaseError::from)?;
        Ok(Self {
            ata_address: row.get(0).map_err(DatabaseError::from)?,
            token_mint: row.get(1).map_err(DatabaseError::from)?,
            wallet_address: row.get(2).map_err(DatabaseError::from)?,
            failure_count: row.get(3).map_err(DatabaseError::from)?,
            last_error: row.get(4).map_err(DatabaseError::from)?,
            first_failed_at: row.get(5).map_err(DatabaseError::from)?,
            last_failed_at: row.get(6).map_err(DatabaseError::from)?,
            next_retry_at: row.get(7).map_err(DatabaseError::from)?,
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
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get(0).map_err(DatabaseError::from)?,
            mint: row.get(1).map_err(DatabaseError::from)?,
            symbol: row.get(2).map_err(DatabaseError::from)?,
            name: row.get(3).map_err(DatabaseError::from)?,
            logo_url: row.get(4).map_err(DatabaseError::from)?,
            tool_type: row.get(5).map_err(DatabaseError::from)?,
            config_json: row.get(6).map_err(DatabaseError::from)?,
            label: row.get(7).map_err(DatabaseError::from)?,
            notes: row.get(8).map_err(DatabaseError::from)?,
            use_count: row.get(9).map_err(DatabaseError::from)?,
            last_used_at: row.get(10).map_err(DatabaseError::from)?,
            created_at: row.get(11).map_err(DatabaseError::from)?,
            updated_at: row.get(12).map_err(DatabaseError::from)?,
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
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get(0).map_err(DatabaseError::from)?,
            session_id: row.get(1).map_err(DatabaseError::from)?,
            session_type: row.get(2).map_err(DatabaseError::from)?,
            token_mint: row.get(3).map_err(DatabaseError::from)?,
            total_wallets: row.get(4).map_err(DatabaseError::from)?,
            target_amount_sol: row.get(5).map_err(DatabaseError::from)?,
            min_amount_sol: row.get(6).map_err(DatabaseError::from)?,
            max_amount_sol: row.get(7).map_err(DatabaseError::from)?,
            delay_ms: row.get(8).map_err(DatabaseError::from)?,
            delay_max_ms: row.get(9).map_err(DatabaseError::from)?,
            concurrency: row.get(10).map_err(DatabaseError::from)?,
            sol_buffer: row.get(11).map_err(DatabaseError::from)?,
            status: row.get(12).map_err(DatabaseError::from)?,
            started_at: row.get(13).map_err(DatabaseError::from)?,
            ended_at: row.get(14).map_err(DatabaseError::from)?,
            error_message: row.get(15).map_err(DatabaseError::from)?,
            wallets_funded: row.get(16).map_err(DatabaseError::from)?,
            successful_ops: row.get(17).map_err(DatabaseError::from)?,
            failed_ops: row.get(18).map_err(DatabaseError::from)?,
            total_sol_spent: row.get(19).map_err(DatabaseError::from)?,
            total_sol_recovered: row.get(20).map_err(DatabaseError::from)?,
            created_at: row.get(21).map_err(DatabaseError::from)?,
            updated_at: row.get(22).map_err(DatabaseError::from)?,
        })
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
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: row.get(0).map_err(DatabaseError::from)?,
            session_id: row.get(1).map_err(DatabaseError::from)?,
            wallet_id: row.get(2).map_err(DatabaseError::from)?,
            wallet_address: row.get(3).map_err(DatabaseError::from)?,
            op_index: row.get(4).map_err(DatabaseError::from)?,
            op_type: row.get(5).map_err(DatabaseError::from)?,
            amount_sol: row.get(6).map_err(DatabaseError::from)?,
            token_amount: row.get(7).map_err(DatabaseError::from)?,
            signature: row.get(8).map_err(DatabaseError::from)?,
            status: row.get(9).map_err(DatabaseError::from)?,
            error_message: row.get(10).map_err(DatabaseError::from)?,
            executed_at: row.get(11).map_err(DatabaseError::from)?,
            created_at: row.get(12).map_err(DatabaseError::from)?,
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
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        let is_active_int: i32 = row.get(12).map_err(DatabaseError::from)?;
        Ok(Self {
            id: row.get(0).map_err(DatabaseError::from)?,
            mint: row.get(1).map_err(DatabaseError::from)?,
            symbol: row.get(2).map_err(DatabaseError::from)?,
            pool_address: row.get(3).map_err(DatabaseError::from)?,
            pool_source: row.get(4).map_err(DatabaseError::from)?,
            pool_dex: row.get(5).map_err(DatabaseError::from)?,
            pool_pair: row.get(6).map_err(DatabaseError::from)?,
            pool_liquidity: row.get(7).map_err(DatabaseError::from)?,
            watch_type: row.get(8).map_err(DatabaseError::from)?,
            trigger_amount_sol: row.get(9).map_err(DatabaseError::from)?,
            action_amount_sol: row.get(10).map_err(DatabaseError::from)?,
            slippage_bps: row.get(11).map_err(DatabaseError::from)?,
            is_active: is_active_int != 0,
            last_checked_at: row.get(13).map_err(DatabaseError::from)?,
            last_trade_signature: row.get(14).map_err(DatabaseError::from)?,
            trades_detected: row.get(15).map_err(DatabaseError::from)?,
            actions_triggered: row.get(16).map_err(DatabaseError::from)?,
            created_at: row.get(17).map_err(DatabaseError::from)?,
            updated_at: row.get(18).map_err(DatabaseError::from)?,
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
