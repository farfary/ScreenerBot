use std::convert::TryFrom;
use std::str::FromStr;

use chrono::Utc;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;

use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};
use crate::pools::db::{
    load_pool_blacklist_entries, touch_pool_blacklist_entry, upsert_pool_blacklist_entry,
    PoolBlacklistRecord,
};
use crate::positions;
use crate::tokens;
use crate::tokens::events::{self as token_events, TokenEvent};

static POOL_BLACKLIST: Lazy<DashMap<Pubkey, PoolBlacklistEntry>> = Lazy::new(DashMap::new);
static ACCOUNT_BLACKLIST: Lazy<DashMap<Pubkey, ()>> = Lazy::new(DashMap::new);

#[derive(Debug, Clone)]
pub struct PoolBlacklistEntry {
    pub pool_id: Pubkey,
    pub token_mint: String,
    pub reason: String,
    pub missing_accounts: Vec<Pubkey>,
    pub retry_count: u32,
    pub first_seen: i64,
    pub last_seen: i64,
}

impl PoolBlacklistEntry {
    fn missing_accounts_strings(&self) -> Vec<String> {
        self.missing_accounts
            .iter()
            .map(|pk| pk.to_string())
            .collect()
    }
}

impl TryFrom<PoolBlacklistRecord> for PoolBlacklistEntry {
    type Error = String;

    fn try_from(value: PoolBlacklistRecord) -> Result<Self, Self::Error> {
        let pool_id = Pubkey::from_str(&value.pool_id)
            .map_err(|e| format!("Invalid pool id {}: {}", value.pool_id, e))?;

        let mut accounts = Vec::new();
        for account in value.missing_accounts.iter() {
            match Pubkey::from_str(account) {
                Ok(pk) => accounts.push(pk),
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Unable to parse blacklisted account {} for pool {}: {}",
                            account, value.pool_id, e
                        ),
                    );
                }
            }
        }

        Ok(Self {
            pool_id,
            token_mint: value.token_mint,
            reason: value.reason,
            missing_accounts: accounts,
            retry_count: value.retry_count.max(0) as u32,
            first_seen: value.first_seen,
            last_seen: value.last_seen,
        })
    }
}

impl From<&PoolBlacklistEntry> for PoolBlacklistRecord {
    fn from(entry: &PoolBlacklistEntry) -> Self {
        PoolBlacklistRecord {
            pool_id: entry.pool_id.to_string(),
            token_mint: entry.token_mint.clone(),
            reason: entry.reason.clone(),
            missing_accounts: entry.missing_accounts_strings(),
            retry_count: entry.retry_count as i64,
            first_seen: entry.first_seen,
            last_seen: entry.last_seen,
        }
    }
}

/// Load persisted blacklist records into memory
pub async fn initialize() -> Result<(), String> {
    let records = load_pool_blacklist_entries().await?;
    for record in records {
        match PoolBlacklistEntry::try_from(record) {
            Ok(entry) => {
                let pool_id = entry.pool_id;
                for account in entry.missing_accounts.iter() {
                    ACCOUNT_BLACKLIST.insert(*account, ());
                }
                POOL_BLACKLIST.insert(pool_id, entry);
            }
            Err(err) => {
                logger::error(
                    LogTag::PoolFetcher,
                    &format!("Failed to hydrate pool blacklist entry: {}", err),
                );
            }
        }
    }

    Ok(())
}

/// Check if a pool id is blacklisted
pub fn is_pool_blacklisted(pool_id: &Pubkey) -> bool {
    POOL_BLACKLIST.contains_key(pool_id)
}

/// Check if an account should be ignored by the fetcher
pub fn is_account_blacklisted(account: &Pubkey) -> bool {
    ACCOUNT_BLACKLIST.contains_key(account)
}

/// Blacklist a pool and optionally the associated token
pub async fn blacklist_pool(
    pool_id: Pubkey,
    token_mint: &str,
    missing_accounts: Vec<Pubkey>,
    reason: &str,
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    let pool_key = pool_id;
    let missing_strings: Vec<String> = missing_accounts.iter().map(|pk| pk.to_string()).collect();

    if let Some(mut existing) = POOL_BLACKLIST.get_mut(&pool_key) {
        existing.reason = reason.to_string();
        existing.last_seen = now;
        existing.retry_count = existing.retry_count.saturating_add(1);
        if !missing_accounts.is_empty() {
            existing.missing_accounts = missing_accounts.clone();
        }

        touch_pool_blacklist_entry(&pool_key.to_string(), &missing_strings, reason).await?;
    } else {
        let entry = PoolBlacklistEntry {
            pool_id: pool_key,
            token_mint: token_mint.to_string(),
            reason: reason.to_string(),
            missing_accounts: missing_accounts.clone(),
            retry_count: 0,
            first_seen: now,
            last_seen: now,
        };

        let record = PoolBlacklistRecord::from(&entry);
        upsert_pool_blacklist_entry(&record).await?;
        POOL_BLACKLIST.insert(pool_key, entry);
    }

    for account in missing_accounts.iter() {
        ACCOUNT_BLACKLIST.insert(*account, ());
    }

    logger::warning(
        LogTag::PoolFetcher,
        &format!(
            "Blacklisted pool {} for mint {} due to missing accounts ({}): {}",
            pool_key,
            token_mint,
            missing_strings.join(","),
            reason
        ),
    );

    record_safe(Event::warn(
        EventCategory::Pool,
        Some("pool_blacklisted".to_string()),
        Some(token_mint.to_string()),
        Some(pool_key.to_string()),
        json!({
            "missing_accounts": missing_strings,
            "reason": reason,
        }),
    ))
    .await;

    ensure_token_blacklisted(token_mint, reason).await;

    Ok(())
}

/// Return all permanently ignored account pubkeys
pub fn ignored_accounts() -> Vec<Pubkey> {
    ACCOUNT_BLACKLIST.iter().map(|entry| *entry.key()).collect()
}

async fn ensure_token_blacklisted(token_mint: &str, reason: &str) {
    if positions::is_open_position(token_mint).await {
        logger::warning(
            LogTag::PoolFetcher,
            &format!(
                "Skipping token blacklist for {} because an open position exists",
                token_mint
            ),
        );
        return;
    }

    if let Some(db) = tokens::get_global_database() {
        let mint = token_mint.to_string();
        let reason_owned = reason.to_string();
        match tokio::task::spawn_blocking(move || -> tokens::TokenResult<bool> {
            if db.is_blacklisted(&mint)? {
                Ok(false)
            } else {
                db.add_to_blacklist(&mint, &reason_owned, "pool_missing_account")?;
                Ok(true)
            }
        })
        .await
        {
            Ok(Ok(added)) => {
                if added {
                    logger::info(
                        LogTag::PoolFetcher,
                        &format!(
                            "Token {} blacklisted due to missing pool accounts",
                            token_mint
                        ),
                    );
                }
            }
            Ok(Err(e)) => {
                logger::error(
                    LogTag::PoolFetcher,
                    &format!("Failed to update token blacklist for {}: {}", token_mint, e),
                );
                return;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolFetcher,
                    &format!(
                        "Blocking task failed while updating blacklist for {}: {}",
                        token_mint, e
                    ),
                );
                return;
            }
        }
    } else {
        logger::warning(
            LogTag::PoolFetcher,
            "Global token database unavailable; cannot persist blacklist entry",
        );
    }

    token_events::emit(TokenEvent::TokenBlacklisted {
        mint: token_mint.to_string(),
        reason: reason.to_string(),
        at: Utc::now(),
    });

    tokens::filtered_store::mark_token_blacklisted(token_mint);
}
