use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::positions;
use crate::transactions::{
    TokenTransfer, Transaction, TransactionDirection, TransactionStatus, TransactionType,
};
use crate::utils::lamports_to_sol;

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
    pub logo_url: Option<String>,
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
    // DCA & Partial Exit fields
    pub dca_count: u32,
    pub average_entry_price: f64,
    pub partial_exit_count: u32,
    pub average_exit_price: Option<f64>,
    pub remaining_token_amount: Option<u64>,
    pub total_exited_amount: u64,
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
pub struct EntryRecordResponse {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub amount: u64,
    pub price: f64,
    pub sol_spent: f64,
    pub transaction_signature: String,
    pub is_dca: bool,
    pub fees_sol: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ExitRecordResponse {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub amount: u64,
    pub price: f64,
    pub sol_received: f64,
    pub transaction_signature: String,
    pub is_partial: bool,
    pub percentage: f64,
    pub fees_sol: Option<f64>,
}

/// Token information for position detail view
#[derive(Debug, Serialize)]
pub struct PositionTokenInfo {
    pub decimals: Option<u8>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

/// Market data for position detail view
#[derive(Debug, Serialize)]
pub struct PositionMarketData {
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub holder_count: Option<i64>,
}

/// Security summary for position detail view
#[derive(Debug, Serialize)]
pub struct PositionSecuritySummary {
    pub score_normalized: Option<i32>,
    pub risk_level: String,
    pub has_mint_authority: bool,
    pub has_freeze_authority: bool,
    pub top_risks: Vec<String>,
}

/// Pool info for position detail view
#[derive(Debug, Serialize)]
pub struct PositionPoolInfo {
    pub pool_address: Option<String>,
    pub dex_name: Option<String>,
    pub liquidity_sol: Option<f64>,
}

/// External links for blockchain explorers and tools
#[derive(Debug, Serialize)]
pub struct ExternalLinks {
    pub solscan: String,
    pub dexscreener: String,
    pub birdeye: String,
    pub rugcheck: String,
    pub photon: String,
}

impl ExternalLinks {
    pub fn for_mint(mint: &str) -> Self {
        Self {
            solscan: format!("https://solscan.io/token/{}", mint),
            dexscreener: format!("https://dexscreener.com/solana/{}", mint),
            birdeye: format!("https://birdeye.so/token/{}?chain=solana", mint),
            rugcheck: format!("https://rugcheck.xyz/tokens/{}", mint),
            photon: format!("https://photon-sol.tinyastro.io/en/lp/{}", mint),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PositionDetailResponse {
    pub position: Option<PositionDetail>,
    pub entries: Vec<EntryRecordResponse>,
    pub exits: Vec<ExitRecordResponse>,
    pub executions: Vec<PositionExecutionRow>,
    pub transactions: Vec<PositionTransactionSummary>,
    pub state_history: Vec<PositionStateTimelineEntry>,
    pub token_info: Option<PositionTokenInfo>,
    pub market_data: Option<PositionMarketData>,
    pub security: Option<PositionSecuritySummary>,
    pub pool_info: Option<PositionPoolInfo>,
    pub external_links: ExternalLinks,
    pub position_age_seconds: Option<i64>,
    pub sol_price_usd: Option<f64>,
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

impl PositionTransactionSummary {
    pub fn from_transaction(
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

    pub fn missing(kind: &str, signature: Option<String>, notes: Option<String>) -> Self {
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

#[derive(Debug, Serialize)]
pub struct PositionStateTimelineEntry {
    pub state: String,
    pub changed_at: i64,
    pub reason: Option<String>,
}

// Helper functions used by impl block

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
        TransactionStatus::Failed(err) => format!("Failed: {err}"),
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
