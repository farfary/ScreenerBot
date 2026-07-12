//! Position route types — data structures for position API responses.

use serde::{Deserialize, Serialize};

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
    // Token decimals (for converting raw token_amount to whole tokens in the UI)
    pub token_decimals: Option<u8>,
    // Archival
    pub archived: bool,
    pub archived_at: Option<i64>,
    // Manual management: true when opened by a manual/force buy (auto-trader skips it)
    pub manual_management: bool,
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
            solscan: format!("https://solscan.io/token/{mint}"),
            dexscreener: format!("https://dexscreener.com/solana/{mint}"),
            birdeye: format!("https://birdeye.so/token/{mint}?chain=solana"),
            rugcheck: format!("https://rugcheck.xyz/tokens/{mint}"),
            photon: format!("https://photon-sol.tinyastro.io/en/lp/{mint}"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PositionDetailResponse {
    pub position: Option<PositionDetail>,
    pub entries: Vec<EntryRecordResponse>,
    pub exits: Vec<ExitRecordResponse>,
    /// The position's swaps as ONE merged timeline (see [`PositionActivityEvent`]).
    pub activity: Vec<PositionActivityEvent>,
    pub activity_totals: PositionActivityTotals,
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
pub struct TransactionTokenTransferSummary {
    pub mint: String,
    pub amount: f64,
    pub from: String,
    pub to: String,
    pub program_id: String,
}

/// One swap in a position's life — the opening buy, every DCA add, every partial exit and
/// the final close — with the position RECORD and the on-chain TRANSACTION merged into a
/// single event.
///
/// The two used to be served as separate arrays (`entries`/`exits` and `transactions`) and
/// the dashboard re-joined them by signature in two different tabs, each losing something
/// the other had: the record view had no chain status/fee/router, and the chain view had no
/// timestamp for a swap that was not in the transaction cache (it sorted to the bottom
/// showing "—" even though its record carried the real time). One event, one join, one
/// source of truth.
///
/// A swap that is submitted but not yet verified has NO record yet — its numbers land on
/// the position only on verification — so the record half is `None` and `state` is
/// `pending`. That is deliberate: the activity list must never report tokens or SOL that
/// the position has not actually booked.
#[derive(Debug, Serialize)]
pub struct PositionActivityEvent {
    /// `entry` | `dca` | `partial_exit` | `exit`
    pub kind: String,
    /// `entry` | `exit` — which side of the book this event is on.
    pub side: String,
    /// 1-based index within its side, so the UI can label "DCA #2" / "Exit #3".
    pub sequence: u32,
    /// `pending` | `confirmed` | `failed` | `synthetic`
    pub state: String,
    pub signature: Option<String>,
    /// Record time when recorded, else the chain's, else the pending registry's submit time.
    pub timestamp: Option<i64>,

    // --- position record (written on verification) ---
    /// True once verification wrote the entry/exit record — i.e. the position has booked it.
    pub recorded: bool,
    pub record_id: Option<i64>,
    /// Raw on-chain token units (scale by the token's decimals before display).
    pub token_amount: Option<u64>,
    /// SOL per whole token.
    pub price: Option<f64>,
    /// SOL spent (entry side) or received (exit side).
    pub sol_amount: Option<f64>,
    /// Exits only — percentage of the then-remaining position this swap sold.
    pub exit_percentage: Option<f64>,
    /// Swap fee booked on the record, when it carried one.
    pub record_fee_sol: Option<f64>,

    // --- on-chain transaction (from the transactions cache) ---
    /// False when the transaction is not in the cache — the record half still stands.
    pub available: bool,
    pub status: Option<String>,
    pub success: Option<bool>,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub fee_sol: Option<f64>,
    pub direction: Option<String>,
    pub transaction_type: Option<String>,
    pub router: Option<String>,
    pub sol_change: Option<f64>,
    pub instructions_count: Option<usize>,
    pub compute_units: Option<u64>,
    pub accounts_count: Option<usize>,
    pub notes: Option<String>,
    pub token_transfers: Vec<TransactionTokenTransferSummary>,

    // --- running position state AFTER this event ---
    /// Tokens held once this event settled (raw units).
    pub tokens_after: Option<u64>,
    /// Cost basis still on the books once this event settled (SOL).
    pub invested_after: Option<f64>,
    /// Exits only: the cost basis of the tokens this swap sold, valued at the average entry
    /// price in force AT THAT MOMENT — not the position's final average.
    pub cost_basis: Option<f64>,
    /// Exits only: `sol_amount - cost_basis`.
    pub realized_pnl: Option<f64>,
    pub realized_pnl_percent: Option<f64>,
}

/// Aggregates over the whole activity timeline, so the UI never re-sums the events itself.
#[derive(Debug, Default, Serialize)]
pub struct PositionActivityTotals {
    pub events: usize,
    pub entries: usize,
    pub dca_entries: usize,
    pub exits: usize,
    pub partial_exits: usize,
    pub pending: usize,
    pub failed: usize,
    /// Raw token units.
    pub tokens_bought: u64,
    pub tokens_sold: u64,
    pub sol_invested: f64,
    pub sol_returned: f64,
    /// Network fees across every swap that reported one.
    pub network_fees_sol: f64,
    /// Sum of the per-exit realized P&L.
    pub realized_pnl: f64,
}

#[derive(Debug, Serialize)]
pub struct PositionStateTimelineEntry {
    pub state: String,
    pub changed_at: i64,
    pub reason: Option<String>,
}
