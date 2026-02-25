use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlacklistStatsResponse {
    pub total_count: usize,
    pub by_reason: HashMap<String, usize>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct PoolBlacklistEntry {
    pub pool_id: String,
    pub token_mint: Option<String>,
    pub reason: String,
    pub program_id: Option<String>,
    pub error_count: i64,
    pub first_failed_at: String,
    pub last_failed_at: String,
    pub added_at: String,
}

#[derive(Debug, Serialize)]
pub struct AccountBlacklistEntry {
    pub account_pubkey: String,
    pub token_mint: Option<String>,
    pub pool_id: Option<String>,
    pub reason: String,
    pub source: Option<String>,
    pub error_count: i64,
    pub first_failed_at: String,
    pub last_failed_at: String,
    pub added_at: String,
}

#[derive(Debug, Serialize)]
pub struct BlacklistDetailsResponse {
    pub pools: Vec<PoolBlacklistEntry>,
    pub accounts: Vec<AccountBlacklistEntry>,
    pub timestamp: String,
}
