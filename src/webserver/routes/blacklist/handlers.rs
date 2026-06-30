use axum::response::Json;
use std::collections::HashMap;

use crate::pools::db::{
    list_blacklisted_accounts, list_blacklisted_pools, BlacklistedAccountRecord,
    BlacklistedPoolRecord,
};
use crate::tokens::cleanup::get_blacklist_summary;
use crate::tokens::database::get_global_database;

use super::types::*;

/// Get blacklist statistics
pub(super) async fn get_blacklist_stats() -> Json<BlacklistStatsResponse> {
    let db = match get_global_database() {
        Some(db) => db,
        None => {
            return Json(BlacklistStatsResponse {
                total_count: 0,
                by_reason: HashMap::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    match get_blacklist_summary(&db) {
        Ok(summary) => {
            let mut by_reason = HashMap::new();
            by_reason.insert("MintAuthority".to_owned(), summary.authority_mint_count);
            by_reason.insert("FreezeAuthority".to_owned(), summary.authority_freeze_count);
            by_reason.insert("Manual".to_owned(), summary.manual_count);
            if summary.non_authority_auto_count > 0 {
                by_reason.insert(
                    "NonAuthorityAuto".to_owned(),
                    summary.non_authority_auto_count,
                );
                for (reason, count) in summary.non_authority_breakdown.iter() {
                    by_reason.insert(format!("NonAuthority::{reason}"), *count);
                }
            }

            Json(BlacklistStatsResponse {
                total_count: summary.total_count,
                by_reason,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(_e) => {
            // Return empty stats on error
            Json(BlacklistStatsResponse {
                total_count: 0,
                by_reason: HashMap::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

pub(super) async fn get_blacklist_details() -> Json<BlacklistDetailsResponse> {
    let pool_records = list_blacklisted_pools(Some(200)).await.unwrap_or_default();
    let account_records = list_blacklisted_accounts(Some(200))
        .await
        .unwrap_or_default();

    Json(BlacklistDetailsResponse {
        pools: pool_records.into_iter().map(map_pool_record).collect(),
        accounts: account_records
            .into_iter()
            .map(map_account_record)
            .collect(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

fn map_pool_record(record: BlacklistedPoolRecord) -> PoolBlacklistEntry {
    PoolBlacklistEntry {
        pool_id: record.pool_id,
        token_mint: record.token_mint,
        reason: record.reason,
        program_id: record.program_id,
        error_count: record.error_count,
        first_failed_at: format_unix(record.first_failed_at),
        last_failed_at: format_unix(record.last_failed_at),
        added_at: format_unix(record.added_at),
    }
}

fn map_account_record(record: BlacklistedAccountRecord) -> AccountBlacklistEntry {
    AccountBlacklistEntry {
        account_pubkey: record.account_pubkey,
        token_mint: record.token_mint,
        pool_id: record.pool_id,
        reason: record.reason,
        source: record.source,
        error_count: record.error_count,
        first_failed_at: format_unix(record.first_failed_at),
        last_failed_at: format_unix(record.last_failed_at),
        added_at: format_unix(record.added_at),
    }
}

fn format_unix(value: i64) -> String {
    if value <= 0 {
        return "n/a".to_owned();
    }
    match chrono::NaiveDateTime::from_timestamp_opt(value, 0) {
        Some(naive) => chrono::DateTime::<chrono::Utc>::from_utc(naive, chrono::Utc).to_rfc3339(),
        None => "n/a".to_owned(),
    }
}
