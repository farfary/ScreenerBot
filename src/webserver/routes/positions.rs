use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::logger::{log, LogTag};
use crate::positions;
// Security database deprecated; security info is on Token when available
// Use crate::tokens::store accessors for token data
use crate::transactions::{
    get_transaction, TokenTransfer, Transaction, TransactionDirection, TransactionStatus,
    TransactionType,
};
use crate::utils::lamports_to_sol;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

#[derive(Debug, Deserialize)]
pub struct PositionsQuery {
    pub status: Option<String>, // "open", "closed", "all"
    pub limit: Option<usize>,
    pub mint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub id: Option<i64>,
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: i64, // Unix timestamp
    pub exit_price: Option<f64>,
    pub exit_time: Option<i64>,
    pub position_type: String,
    pub entry_size_sol: f64,
    pub total_size_sol: f64,
    pub price_highest: f64,
    pub price_lowest: f64,
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>,
    pub effective_entry_price: Option<f64>,
    pub effective_exit_price: Option<f64>,
    pub sol_received: Option<f64>,
    pub profit_target_min: Option<f64>,
    pub profit_target_max: Option<f64>,
    pub liquidity_tier: Option<String>,
    pub transaction_entry_verified: bool,
    pub transaction_exit_verified: bool,
    pub entry_fee_lamports: Option<u64>,
    pub exit_fee_lamports: Option<u64>,
    pub current_price: Option<f64>,
    pub current_price_updated: Option<i64>,
    pub phantom_confirmations: u32,
    pub synthetic_exit: bool,
    pub closed_reason: Option<String>,
    // Calculated fields
    pub pnl: Option<f64>,
    pub pnl_percent: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PositionsStatsResponse {
    pub total: usize,
    pub open: usize,
    pub closed: usize,
    pub total_invested_sol: f64,
    pub total_pnl: f64,
}

#[derive(Debug, Serialize)]
pub struct PositionDetailResponse {
    pub position: Option<PositionDetail>,
    pub executions: Vec<PositionExecutionRow>,
    pub transactions: Vec<PositionTransactionSummary>,
    pub state_history: Vec<PositionStateTimelineEntry>,
    pub fetched_at: String,
}

#[derive(Debug, Serialize)]
pub struct PositionDetail {
    #[serde(flatten)]
    pub summary: PositionResponse,
    pub phantom_remove: bool,
    pub phantom_first_seen: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PositionExecutionRow {
    pub kind: String,
    pub timestamp: Option<i64>,
    pub price_sol: Option<f64>,
    pub effective_price_sol: Option<f64>,
    pub size_sol: Option<f64>,
    pub total_size_sol: Option<f64>,
    pub sol_delta: Option<f64>,
    pub token_amount: Option<u64>,
    pub signature: Option<String>,
    pub verified: bool,
    pub fee_lamports: Option<u64>,
    pub fee_sol: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionTokenTransferSummary {
    pub mint: String,
    pub amount: f64,
    pub from: String,
    pub to: String,
    pub program_id: String,
}

#[derive(Debug, Serialize)]
pub struct PositionTransactionSummary {
    pub kind: String,
    pub signature: Option<String>,
    pub available: bool,
    pub status: Option<String>,
    pub success: Option<bool>,
    pub timestamp: Option<i64>,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub fee_sol: Option<f64>,
    pub fee_lamports: Option<u64>,
    pub direction: Option<String>,
    pub transaction_type: Option<String>,
    pub router: Option<String>,
    pub sol_change: Option<f64>,
    pub instructions_count: Option<usize>,
    pub notes: Option<String>,
    pub token_transfers: Vec<TransactionTokenTransferSummary>,
}

#[derive(Debug, Serialize)]
pub struct PositionStateTimelineEntry {
    pub state: String,
    pub changed_at: i64,
    pub reason: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/positions", get(get_positions))
        .route("/positions/stats", get(get_positions_stats))
        .route("/positions/:key/details", get(get_position_details))
        .route("/positions/:mint/debug", get(get_position_debug_info))
}

async fn get_positions(Query(params): Query<PositionsQuery>) -> Json<Vec<PositionResponse>> {
    let status = params.status.as_deref().unwrap_or("all");
    let limit = params.limit.unwrap_or(100);
    let mint_filter = params.mint.as_deref();

    let responses = load_positions_with_filters(status, limit, mint_filter).await;
    Json(responses)
}

pub async fn load_positions_with_filters(
    status: &str,
    limit: usize,
    mint_filter: Option<&str>,
) -> Vec<PositionResponse> {
    let positions_result = match status {
        "open" => positions::db::get_open_positions().await,
        "closed" => positions::db::get_closed_positions().await,
        _ => positions::db::load_all_positions().await,
    };

    let positions = match positions_result {
        Ok(pos) => pos,
        Err(e) => {
            eprintln!("Failed to load positions: {}", e);
            return Vec::new();
        }
    };

    let mut filtered_positions: Vec<_> = if let Some(mint) = mint_filter {
        positions
            .into_iter()
            .filter(|p| p.mint.contains(mint))
            .collect()
    } else {
        positions
    };

    if limit > 0 {
        filtered_positions.truncate(limit);
    }

    filtered_positions
        .iter()
        .map(map_position_to_response)
        .collect()
}

fn map_position_to_response(p: &positions::Position) -> PositionResponse {
    let entry_time_ts = p.entry_time.timestamp();
    let exit_time_ts = p.exit_time.map(|dt| dt.timestamp());
    let current_price_updated_ts = p.current_price_updated.map(|dt| dt.timestamp());

    let (pnl, pnl_percent) = if p.transaction_exit_verified {
        if let (Some(exit_price), Some(sol_received)) = (p.effective_exit_price, p.sol_received) {
            let invested = p.entry_size_sol;
            let pnl_value = sol_received - invested;
            let pnl_pct = if invested > 0.0 {
                (pnl_value / invested) * 100.0
            } else {
                0.0
            };
            (Some(pnl_value), Some(pnl_pct))
        } else if let (Some(exit_price), entry_price) = (p.exit_price, p.entry_price) {
            let pnl_pct = ((exit_price - entry_price) / entry_price) * 100.0;
            let pnl_value = p.entry_size_sol * (pnl_pct / 100.0);
            (Some(pnl_value), Some(pnl_pct))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let (unrealized_pnl, unrealized_pnl_percent) = if !p.transaction_exit_verified {
        if let Some(current_price) = p.current_price {
            let entry_price = p.effective_entry_price.unwrap_or(p.entry_price);
            if entry_price > 0.0 {
                let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
                let pnl_value = p.entry_size_sol * (pnl_pct / 100.0);
                (Some(pnl_value), Some(pnl_pct))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    PositionResponse {
        id: p.id,
        mint: p.mint.clone(),
        symbol: p.symbol.clone(),
        name: p.name.clone(),
        entry_price: p.entry_price,
        entry_time: entry_time_ts,
        exit_price: p.exit_price,
        exit_time: exit_time_ts,
        position_type: p.position_type.clone(),
        entry_size_sol: p.entry_size_sol,
        total_size_sol: p.total_size_sol,
        price_highest: p.price_highest,
        price_lowest: p.price_lowest,
        entry_transaction_signature: p.entry_transaction_signature.clone(),
        exit_transaction_signature: p.exit_transaction_signature.clone(),
        token_amount: p.token_amount,
        effective_entry_price: p.effective_entry_price,
        effective_exit_price: p.effective_exit_price,
        sol_received: p.sol_received,
        profit_target_min: p.profit_target_min,
        profit_target_max: p.profit_target_max,
        liquidity_tier: p.liquidity_tier.clone(),
        transaction_entry_verified: p.transaction_entry_verified,
        transaction_exit_verified: p.transaction_exit_verified,
        entry_fee_lamports: p.entry_fee_lamports,
        exit_fee_lamports: p.exit_fee_lamports,
        current_price: p.current_price,
        current_price_updated: current_price_updated_ts,
        phantom_confirmations: p.phantom_confirmations,
        synthetic_exit: p.synthetic_exit,
        closed_reason: p.closed_reason.clone(),
        pnl,
        pnl_percent,
        unrealized_pnl,
        unrealized_pnl_percent,
    }
}

async fn get_position_details(Path(key): Path<String>) -> Response {
    match resolve_position_by_key(&key).await {
        Ok(Some(position)) => {
            let detail = map_position_to_detail(&position);
            let executions = build_execution_rows(&position);
            let transactions = build_transaction_summaries(&position).await;
            let state_history = load_state_history_entries(&position).await;

            success_response(PositionDetailResponse {
                position: Some(detail),
                executions,
                transactions,
                state_history,
                fetched_at: Utc::now().to_rfc3339(),
            })
        }
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "POSITION_NOT_FOUND",
            "Position not found",
            Some(&format!("No position found for key {}", key)),
        ),
        Err(err) => {
            log(
                LogTag::Webserver,
                "POSITIONS_DETAIL_ERROR",
                &format!("Failed to resolve position for key {}: {}", key, err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "POSITION_DETAIL_ERROR",
                "Failed to load position details",
                Some(&err),
            )
        }
    }
}

async fn resolve_position_by_key(key: &str) -> Result<Option<positions::Position>, String> {
    if let Some(id_part) = key.strip_prefix("id:") {
        let id: i64 = id_part
            .parse()
            .map_err(|_| format!("Invalid position id: {}", id_part))?;
        return positions::db::get_position_by_id(id).await;
    }

    if let Some(mint_part) = key.strip_prefix("mint:") {
        return positions::db::get_position_by_mint(mint_part).await;
    }

    positions::db::get_position_by_mint(key).await
}

fn map_position_to_detail(position: &positions::Position) -> PositionDetail {
    PositionDetail {
        summary: map_position_to_response(position),
        phantom_remove: position.phantom_remove,
        phantom_first_seen: position.phantom_first_seen.map(|dt| dt.timestamp()),
    }
}

fn build_execution_rows(position: &positions::Position) -> Vec<PositionExecutionRow> {
    let mut rows = Vec::with_capacity(2);

    rows.push(PositionExecutionRow {
        kind: "entry".to_string(),
        timestamp: Some(position.entry_time.timestamp()),
        price_sol: Some(position.entry_price),
        effective_price_sol: position.effective_entry_price,
        size_sol: Some(position.entry_size_sol),
        total_size_sol: Some(position.total_size_sol),
        sol_delta: Some(-position.entry_size_sol.abs()),
        token_amount: position.token_amount,
        signature: position.entry_transaction_signature.clone(),
        verified: position.transaction_entry_verified,
        fee_lamports: position.entry_fee_lamports,
        fee_sol: lamports_option_to_sol(position.entry_fee_lamports),
        notes: Some(format!("Position type: {}", position.position_type)),
    });

    let mut exit_notes: Vec<String> = Vec::new();
    if position.synthetic_exit {
        exit_notes.push("Synthetic exit".to_string());
    }
    if let Some(reason) = &position.closed_reason {
        if !reason.is_empty() {
            exit_notes.push(reason.clone());
        }
    }
    if !position.transaction_exit_verified && position.exit_time.is_none() {
        exit_notes.push("Exit pending".to_string());
    }

    rows.push(PositionExecutionRow {
        kind: "exit".to_string(),
        timestamp: position.exit_time.map(|dt| dt.timestamp()),
        price_sol: position.exit_price,
        effective_price_sol: position.effective_exit_price,
        size_sol: Some(position.entry_size_sol),
        total_size_sol: Some(position.total_size_sol),
        sol_delta: position.sol_received,
        token_amount: position.token_amount,
        signature: position.exit_transaction_signature.clone(),
        verified: position.transaction_exit_verified,
        fee_lamports: position.exit_fee_lamports,
        fee_sol: lamports_option_to_sol(position.exit_fee_lamports),
        notes: if exit_notes.is_empty() {
            None
        } else {
            Some(exit_notes.join(" · "))
        },
    });

    rows
}

async fn build_transaction_summaries(
    position: &positions::Position,
) -> Vec<PositionTransactionSummary> {
    let entry_sig = position.entry_transaction_signature.clone();
    let exit_sig = position.exit_transaction_signature.clone();

    let mut summaries = Vec::with_capacity(2);
    summaries.push(fetch_transaction_summary("entry", entry_sig, position).await);
    summaries.push(fetch_transaction_summary("exit", exit_sig, position).await);
    summaries
}

async fn fetch_transaction_summary(
    kind: &str,
    signature: Option<String>,
    position: &positions::Position,
) -> PositionTransactionSummary {
    match signature {
        Some(sig) => match get_transaction(&sig).await {
            Ok(Some(tx)) => PositionTransactionSummary::from_transaction(kind, sig, &tx, position),
            Ok(None) => PositionTransactionSummary::missing(
                kind,
                Some(sig),
                Some("Transaction not available in cache".to_string()),
            ),
            Err(err) => {
                log(
                    LogTag::Webserver,
                    "POSITIONS_DETAIL_TX_ERROR",
                    &format!("Failed to load {} transaction {}: {}", kind, sig, err),
                );
                PositionTransactionSummary::missing(kind, Some(sig), Some(err))
            }
        },
        None => {
            let note = if kind == "exit" && position.synthetic_exit {
                "Synthetic exit - no signature".to_string()
            } else {
                "Signature not recorded".to_string()
            };
            PositionTransactionSummary::missing(kind, None, Some(note))
        }
    }
}

async fn load_state_history_entries(
    position: &positions::Position,
) -> Vec<PositionStateTimelineEntry> {
    let Some(id) = position.id else {
        return Vec::new();
    };

    let db_arc = match positions::get_positions_database().await {
        Ok(db) => db,
        Err(err) => {
            log(
                LogTag::Webserver,
                "POSITIONS_DETAIL_STATE_HISTORY_ERROR",
                &format!(
                    "Failed to access positions database for position {}: {}",
                    id, err
                ),
            );
            return Vec::new();
        }
    };

    let db_clone = {
        let db_guard = db_arc.lock().await;
        db_guard.clone()
    };

    let Some(db) = db_clone else {
        log(
            LogTag::Webserver,
            "POSITIONS_DETAIL_STATE_HISTORY_ERROR",
            &format!(
                "Positions database not initialized when loading history for position {}",
                id
            ),
        );
        return Vec::new();
    };

    match db.get_position_state_history(id).await {
        Ok(history) => history
            .into_iter()
            .map(|entry| PositionStateTimelineEntry {
                state: entry.state.to_string(),
                changed_at: entry.changed_at.timestamp(),
                reason: entry.reason,
            })
            .collect(),
        Err(err) => {
            log(
                LogTag::Webserver,
                "POSITIONS_DETAIL_STATE_HISTORY_ERROR",
                &format!("Failed to load state history for position {}: {}", id, err),
            );
            Vec::new()
        }
    }
}

impl PositionTransactionSummary {
    fn from_transaction(
        kind: &str,
        signature: String,
        tx: &Transaction,
        position: &positions::Position,
    ) -> Self {
        let fee_sol = if let Some(lamports) = tx.fee_lamports {
            Some(lamports_to_sol(lamports))
        } else if tx.fee_sol > 0.0 {
            Some(tx.fee_sol)
        } else {
            None
        };

        let router = tx.token_swap_info.as_ref().map(|info| info.router.clone());

        Self {
            kind: kind.to_string(),
            signature: Some(signature.clone()),
            available: true,
            status: Some(describe_transaction_status(&tx.status)),
            success: Some(tx.success),
            timestamp: Some(tx.timestamp.timestamp()),
            slot: tx.slot,
            block_time: tx.block_time,
            fee_sol,
            fee_lamports: tx.fee_lamports,
            direction: Some(describe_transaction_direction(&tx.direction)),
            transaction_type: Some(describe_transaction_type(&tx.transaction_type)),
            router,
            sol_change: Some(tx.sol_balance_change),
            instructions_count: Some(tx.instructions_count),
            notes: tx.error_message.clone(),
            token_transfers: map_token_transfers(position, &tx.token_transfers),
        }
    }

    fn missing(kind: &str, signature: Option<String>, notes: Option<String>) -> Self {
        Self {
            kind: kind.to_string(),
            signature,
            available: false,
            status: None,
            success: None,
            timestamp: None,
            slot: None,
            block_time: None,
            fee_sol: None,
            fee_lamports: None,
            direction: None,
            transaction_type: None,
            router: None,
            sol_change: None,
            instructions_count: None,
            notes,
            token_transfers: Vec::new(),
        }
    }
}

fn map_token_transfers(
    position: &positions::Position,
    transfers: &[TokenTransfer],
) -> Vec<TransactionTokenTransferSummary> {
    let mut relevant: Vec<&TokenTransfer> = transfers
        .iter()
        .filter(|transfer| transfer.mint == position.mint)
        .collect();

    if relevant.is_empty() {
        relevant = transfers.iter().collect();
    }

    relevant
        .into_iter()
        .take(8)
        .map(|transfer| TransactionTokenTransferSummary {
            mint: transfer.mint.clone(),
            amount: transfer.amount,
            from: transfer.from.clone(),
            to: transfer.to.clone(),
            program_id: transfer.program_id.clone(),
        })
        .collect()
}

fn describe_transaction_status(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Pending => "Pending".to_string(),
        TransactionStatus::Confirmed => "Confirmed".to_string(),
        TransactionStatus::Finalized => "Finalized".to_string(),
        TransactionStatus::Failed(err) => format!("Failed: {}", err),
    }
}

fn describe_transaction_direction(direction: &TransactionDirection) -> String {
    match direction {
        TransactionDirection::Incoming => "Incoming".to_string(),
        TransactionDirection::Outgoing => "Outgoing".to_string(),
        TransactionDirection::Internal => "Internal".to_string(),
        TransactionDirection::Unknown => "Unknown".to_string(),
    }
}

fn describe_transaction_type(transaction_type: &TransactionType) -> String {
    match transaction_type {
        TransactionType::Buy => "Buy".to_string(),
        TransactionType::Sell => "Sell".to_string(),
        TransactionType::Transfer => "Transfer".to_string(),
        TransactionType::Compute => "Compute".to_string(),
        TransactionType::AtaOperation => "ATA Operation".to_string(),
        TransactionType::Failed => "Failed".to_string(),
        TransactionType::Unknown => "Unknown".to_string(),
        TransactionType::SwapSolToToken { router, .. } => {
            format!("Swap SOL→Token ({})", router)
        }
        TransactionType::SwapTokenToSol { router, .. } => {
            format!("Swap Token→SOL ({})", router)
        }
        TransactionType::SwapTokenToToken { router, .. } => {
            format!("Swap Token→Token ({})", router)
        }
        TransactionType::SolTransfer { .. } => "SOL Transfer".to_string(),
        TransactionType::TokenTransfer { mint, amount, .. } => {
            format!("Token Transfer {} ({:.4})", mint, amount)
        }
        TransactionType::AtaClose { token_mint, .. } => {
            format!("ATA Close ({})", token_mint)
        }
        TransactionType::Other { description, .. } => description.clone(),
    }
}

fn lamports_option_to_sol(value: Option<u64>) -> Option<f64> {
    value.map(lamports_to_sol)
}

async fn get_positions_stats() -> Json<PositionsStatsResponse> {
    let open_positions = positions::db::get_open_positions()
        .await
        .unwrap_or_default();
    let closed_positions = positions::db::get_closed_positions()
        .await
        .unwrap_or_default();

    let total = open_positions.len() + closed_positions.len();
    let open = open_positions.len();
    let closed = closed_positions.len();

    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();

    let total_pnl: f64 = closed_positions
        .iter()
        .filter_map(|p| {
            if let (Some(sol_received), entry_size) = (p.sol_received, p.entry_size_sol) {
                Some(sol_received - entry_size)
            } else {
                None
            }
        })
        .sum();

    Json(PositionsStatsResponse {
        total,
        open,
        closed,
        total_invested_sol,
        total_pnl,
    })
}

// =============================================================================
// DEBUG INFO ENDPOINT FOR POSITIONS
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PositionDebugResponse {
    pub mint: String,
    pub timestamp: String,
    pub position_data: Option<PositionData>,
    pub token_info: Option<TokenInfo>,
    pub price_data: Option<PriceData>,
    pub market_data: Option<MarketData>,
    pub pools: Vec<PoolInfo>,
    pub security: Option<SecurityInfo>,
    pub social: Option<SocialInfo>,
    pub position_debug: Option<PositionDebugDetails>,
}

#[derive(Debug, Serialize)]
pub struct PositionData {
    pub open_position: Option<PositionSummary>,
    pub closed_positions_count: usize,
    pub total_pnl: f64,
    pub win_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct PositionSummary {
    pub id: Option<i64>,
    pub entry_price: f64,
    pub entry_time: i64,
    pub entry_size_sol: f64,
    pub current_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_percent: Option<f64>,
    pub phantom_confirmations: u32,
}

#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct PriceData {
    pub pool_price_sol: f64,
    pub pool_price_usd: Option<f64>,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct MarketData {
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub program_kind: String,
    pub dex_name: String,
    pub sol_reserves: f64,
    pub token_reserves: f64,
    pub price_sol: f64,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct SecurityInfo {
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    pub total_holders: i32,
    pub top_10_concentration: Option<f64>,
    pub risks: Vec<RiskInfo>,
    pub analyzed_at: String,
}

#[derive(Debug, Serialize)]
pub struct RiskInfo {
    pub name: String,
    pub level: String,
    pub description: String,
    pub score: i32,
}

#[derive(Debug, Serialize)]
pub struct SocialInfo {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PositionDebugDetails {
    pub transaction_details: TransactionDetails,
    pub fee_details: FeeDetails,
    pub profit_targets: ProfitTargets,
    pub price_tracking: PriceTracking,
    pub phantom_details: Option<PhantomDetails>,
    pub proceeds_metrics: ProceedsMetrics,
}

#[derive(Debug, Serialize)]
pub struct TransactionDetails {
    pub entry_signature: Option<String>,
    pub entry_verified: bool,
    pub exit_signature: Option<String>,
    pub exit_verified: bool,
    pub synthetic_exit: bool,
    pub closed_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeeDetails {
    pub entry_fee_lamports: Option<u64>,
    pub entry_fee_sol: Option<f64>,
    pub exit_fee_lamports: Option<u64>,
    pub exit_fee_sol: Option<f64>,
    pub total_fees_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfitTargets {
    pub min_target_percent: Option<f64>,
    pub max_target_percent: Option<f64>,
    pub liquidity_tier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PriceTracking {
    pub price_highest: f64,
    pub price_lowest: f64,
    pub current_price: Option<f64>,
    pub current_price_updated: Option<String>,
    pub drawdown_from_high: Option<f64>,
    pub gain_from_low: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PhantomDetails {
    pub phantom_remove: bool,
    pub phantom_confirmations: u32,
    pub phantom_first_seen: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProceedsMetrics {
    pub accepted_quotes: u64,
    pub rejected_quotes: u64,
    pub accepted_profit_quotes: u64,
    pub accepted_loss_quotes: u64,
    pub average_shortfall_bps: f64,
    pub worst_shortfall_bps: u64,
}

/// Get comprehensive debug information for a position
async fn get_position_debug_info(Path(mint): Path<String>) -> Json<PositionDebugResponse> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Load decimals from cache
    let decimals = crate::tokens::get_decimals(&mint).await;

    // 1. Get position data
    let open_position = positions::db::get_open_positions()
        .await
        .ok()
        .and_then(|positions| {
            positions.into_iter().find(|p| p.mint == mint).map(|p| {
                let unrealized_pnl = p.current_price.map(|current| {
                    let current_value = current * p.entry_size_sol;
                    current_value - p.entry_size_sol
                });

                let unrealized_pnl_percent =
                    unrealized_pnl.map(|pnl| (pnl / p.entry_size_sol) * 100.0);

                PositionSummary {
                    id: p.id,
                    entry_price: p.entry_price,
                    entry_time: p.entry_time.timestamp(),
                    entry_size_sol: p.entry_size_sol,
                    current_price: p.current_price,
                    unrealized_pnl,
                    unrealized_pnl_percent,
                    phantom_confirmations: p.phantom_confirmations,
                }
            })
        });

    let closed_positions = positions::db::get_closed_positions()
        .await
        .ok()
        .map(|positions| {
            let matching_positions: Vec<_> =
                positions.into_iter().filter(|p| p.mint == mint).collect();
            let count = matching_positions.len();
            let total_pnl: f64 = matching_positions
                .iter()
                .filter_map(|p| p.sol_received.map(|received| received - p.entry_size_sol))
                .sum();
            let wins = matching_positions
                .iter()
                .filter(|p| {
                    p.sol_received
                        .map(|r| r > p.entry_size_sol)
                        .unwrap_or(false)
                })
                .count();
            let win_rate = if count > 0 {
                ((wins as f64) / (count as f64)) * 100.0
            } else {
                0.0
            };
            (count, total_pnl, win_rate)
        })
        .unwrap_or((0, 0.0, 0.0));

    let position_data = Some(PositionData {
        open_position,
        closed_positions_count: closed_positions.0,
        total_pnl: closed_positions.1,
        win_rate: closed_positions.2,
    });

    // 2. Get token info from database (with market data)
    let snapshot = crate::tokens::get_full_token_async(&mint).await.ok().flatten();
    let api_token = snapshot.as_ref();

    let token_info = api_token.map(|token| TokenInfo {
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        decimals,
        logo_url: token.image_url.clone(),
        website: token.websites.first().map(|w| w.url.clone()),
        tags: Vec::new(), // Tags not available in unified Token
        is_verified: token.security_score.map(|s| s >= 500).unwrap_or(false),
    });

    // 3. Get current price from pool service
    let price_data = crate::pools::get_pool_price(&mint).map(|price_result| {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        PriceData {
            pool_price_sol: price_result.price_sol,
            pool_price_usd: None,
            confidence: price_result.confidence,
            last_updated: price_unix_time,
        }
    });

    // 4. Get market data from token database
    let market_data = api_token.as_ref().map(|token| MarketData {
        market_cap: token.market_cap,
        fdv: token.fdv,
        liquidity_usd: token.liquidity_usd,
        volume_24h: token.volume_h24,
    });

    // 5. Get pool info
    let mut pools_vec = Vec::new();
    if let Some(price_result) = crate::pools::get_pool_price(&mint) {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        pools_vec.push(PoolInfo {
            pool_address: price_result.pool_address.clone(),
            program_kind: format!(
                "{:?}",
                price_result
                    .source_pool
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            ),
            dex_name: price_result
                .source_pool
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
                .clone(),
            sol_reserves: price_result.sol_reserves,
            token_reserves: price_result.token_reserves,
            price_sol: price_result.price_sol,
            confidence: price_result.confidence,
            last_updated: price_unix_time,
        });
    }

    // 6. Get security info (temporarily unavailable until SecurityProvider integration)
    let security = None::<SecurityInfo>;

    // 7. Get social info from token database
    let social = api_token.as_ref().map(|token| SocialInfo {
        website: token.websites.first().map(|w| w.url.clone()),
        twitter: token
            .socials
            .iter()
            .find(|s| s.link_type.to_lowercase().contains("twitter"))
            .map(|s| s.url.clone()),
        telegram: token
            .socials
            .iter()
            .find(|s| s.link_type.to_lowercase().contains("telegram"))
            .map(|s| s.url.clone()),
    });

    // 8. Get position debug data
    let position_debug = if position_data
        .as_ref()
        .and_then(|pd| pd.open_position.as_ref())
        .is_some()
    {
        // Get full position details
        let full_position = positions::db::get_open_positions()
            .await
            .ok()
            .and_then(|positions| positions.into_iter().find(|p| p.mint == mint));

        if let Some(pos) = full_position {
            // Transaction details
            let transaction_details = TransactionDetails {
                entry_signature: pos.entry_transaction_signature.clone(),
                entry_verified: pos.transaction_entry_verified,
                exit_signature: pos.exit_transaction_signature.clone(),
                exit_verified: pos.transaction_exit_verified,
                synthetic_exit: pos.synthetic_exit,
                closed_reason: pos.closed_reason.clone(),
            };

            // Fee details
            let entry_fee_sol = pos.entry_fee_lamports.map(|l| (l as f64) / 1_000_000_000.0);
            let exit_fee_sol = pos.exit_fee_lamports.map(|l| (l as f64) / 1_000_000_000.0);
            let total_fees_sol = entry_fee_sol.unwrap_or(0.0) + exit_fee_sol.unwrap_or(0.0);

            let fee_details = FeeDetails {
                entry_fee_lamports: pos.entry_fee_lamports,
                entry_fee_sol,
                exit_fee_lamports: pos.exit_fee_lamports,
                exit_fee_sol,
                total_fees_sol,
            };

            // Profit targets
            let profit_targets = ProfitTargets {
                min_target_percent: pos.profit_target_min,
                max_target_percent: pos.profit_target_max,
                liquidity_tier: pos.liquidity_tier.clone(),
            };

            // Price tracking
            let current = pos.current_price.unwrap_or(pos.entry_price);
            let drawdown_from_high = if pos.price_highest > 0.0 {
                Some(((current - pos.price_highest) / pos.price_highest) * 100.0)
            } else {
                None
            };
            let gain_from_low = if pos.price_lowest > 0.0 {
                Some(((current - pos.price_lowest) / pos.price_lowest) * 100.0)
            } else {
                None
            };

            let price_tracking = PriceTracking {
                price_highest: pos.price_highest,
                price_lowest: pos.price_lowest,
                current_price: pos.current_price,
                current_price_updated: pos.current_price_updated.map(|dt| dt.to_rfc3339()),
                drawdown_from_high,
                gain_from_low,
            };

            // Phantom details
            let phantom_details = if pos.phantom_remove || pos.phantom_confirmations > 0 {
                Some(PhantomDetails {
                    phantom_remove: pos.phantom_remove,
                    phantom_confirmations: pos.phantom_confirmations,
                    phantom_first_seen: pos.phantom_first_seen.map(|dt| dt.to_rfc3339()),
                })
            } else {
                None
            };

            // Proceeds metrics
            let proceeds_metrics = crate::positions::metrics::get_proceeds_metrics_snapshot().await;
            let proceeds = ProceedsMetrics {
                accepted_quotes: proceeds_metrics.accepted_quotes,
                rejected_quotes: proceeds_metrics.rejected_quotes,
                accepted_profit_quotes: proceeds_metrics.accepted_profit_quotes,
                accepted_loss_quotes: proceeds_metrics.accepted_loss_quotes,
                average_shortfall_bps: proceeds_metrics.average_shortfall_bps,
                worst_shortfall_bps: proceeds_metrics.worst_shortfall_bps,
            };

            Some(PositionDebugDetails {
                transaction_details,
                fee_details,
                profit_targets,
                price_tracking,
                phantom_details,
                proceeds_metrics: proceeds,
            })
        } else {
            None
        }
    } else {
        None
    };

    Json(PositionDebugResponse {
        mint,
        timestamp,
        position_data,
        token_info,
        price_data,
        market_data,
        pools: pools_vec,
        security,
        social,
        position_debug,
    })
}
