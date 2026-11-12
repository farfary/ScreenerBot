// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Blacklist operations for accounts and pools.

use super::operations::PoolsDatabase;
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord, PoolFailureOutcome};
use crate::errors::{DatabaseError, InternalError};
use crate::pools::types::{PoolBlacklistPolicy, PoolFailureRecord};
use crate::pools::Error;

use rusqlite::{params, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

// =============================================================================
// BLACKLIST OPERATIONS
// =============================================================================

impl PoolsDatabase {
    /// Add account to blacklist until the blacklist TTL passes after this failure
    pub async fn add_account_to_blacklist(
        &self,
        account_pubkey: &str,
        reason: &str,
        source: Option<&str>,
        pool_id: Option<&str>,
        token_mint: Option<&str>,
        policy: PoolBlacklistPolicy,
    ) -> Result<(), Error> {
        let account_key = account_pubkey.to_string();
        let reason_str = reason.to_string();
        let source_str = source.map(|s| s.to_string());
        let pool_id_str = pool_id.map(|s| s.to_string());
        let token_mint_str = token_mint.map(|s| s.to_string());
        let chain_id = self.chain_id.as_str().to_owned();
        // Update memory immediately
        let ttl_secs = policy.ttl_secs;
        {
            let mut accounts = self.blacklisted_accounts.write().unwrap();
            accounts.insert(account_key.clone(), unix_now().saturating_add(ttl_secs));
        }

        let pool = self.shared_pool()?;
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap()
          .as_secs() as i64;

        // Check if already exists
        let exists: bool = conn
          .query_row(
            "SELECT 1 FROM blacklist_accounts WHERE chain_id = ?1 AND account_pubkey = ?2",
            params![&chain_id, &account_key],
            |_| Ok(true),
          )
          .unwrap_or_default();

        if exists {
          // Increment error count and update last_failed_at
          conn.execute(
            "UPDATE blacklist_accounts 
             SET error_count = error_count + 1, last_failed_at = ?1 
             WHERE chain_id = ?2 AND account_pubkey = ?3",
            params![now, &chain_id, &account_key],
          )
          .map_err(|e| DatabaseError::Query { operation: "update blacklist_accounts".to_owned(), message: e.to_string() })?;
        } else {
          // Insert new entry
          conn.execute(
            "INSERT INTO blacklist_accounts 
             (chain_id, account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7, ?7)",
            params![&chain_id, &account_key, &reason_str, source_str.as_deref(), pool_id_str.as_deref(), token_mint_str.as_deref(), now],
          )
          .map_err(|e| DatabaseError::Query { operation: "insert into blacklist_accounts".to_owned(), message: e.to_string() })?;
        }

        Ok(())
    })
    .await
    .map_err(InternalError::from)?
    }

    /// Check if account is blacklisted: only while its blacklist entry has not expired
    pub async fn is_account_blacklisted(&self, account_pubkey: &str) -> Result<bool, Error> {
        // Hot path: memory only
        let accounts = self.blacklisted_accounts.read().unwrap();
        Ok(accounts
            .get(account_pubkey)
            .is_some_and(|expires_at| unix_now() < *expires_at))
    }

    /// Record failures of a pool and apply the pool blacklist policy.
    ///
    /// `observed_failures` is how many failures this call reports (see
    /// `PoolBlacklistPolicy::record_failure`). The row keeps the failure count;
    /// a failure after the policy TTL restarts it at this call's failures. The
    /// pool is excluded only once the count reaches the threshold, until the TTL
    /// passes after its latest failure.
    pub async fn add_pool_to_blacklist(
        &self,
        pool_id: &str,
        reason: &str,
        token_mint: Option<&str>,
        program_id: Option<&str>,
        observed_failures: u32,
        policy: PoolBlacklistPolicy,
    ) -> Result<PoolFailureOutcome, Error> {
        let pool_id_str = pool_id.to_string();
        let reason_str = reason.to_string();
        let token_mint_str = token_mint.map(|s| s.to_string());
        let program_id_str = program_id.map(|s| s.to_string());
        let chain_id = self.chain_id.as_str().to_owned();

        let pool = self.shared_pool()?;
        let record = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

        let now = unix_now();

        let previous = conn
          .query_row(
            "SELECT error_count, first_failed_at, last_failed_at FROM blacklist_pools WHERE chain_id = ?1 AND pool_id = ?2",
            params![&chain_id, &pool_id_str],
            |row| {
              Ok(PoolFailureRecord {
                error_count: row.get::<_, Option<i64>>(0)?.unwrap_or(1),
                first_failed_at: row.get(1)?,
                last_failed_at: row.get(2)?,
              })
            },
          )
          .optional()
          .map_err(|e| DatabaseError::Query { operation: "read blacklist_pools".to_owned(), message: e.to_string() })?;

        let record = policy.record_failure(previous, observed_failures, now);

        match previous {
          Some(previous) if !policy.is_stale(previous.last_failed_at, now) => {
            // Same failure episode: count it
            conn.execute(
              "UPDATE blacklist_pools
               SET error_count = ?1, last_failed_at = ?2
               WHERE chain_id = ?3 AND pool_id = ?4",
              params![record.error_count, record.last_failed_at, &chain_id, &pool_id_str],
            )
            .map_err(|e| DatabaseError::Query { operation: "update blacklist_pools".to_owned(), message: e.to_string() })?;
          }
          Some(_) => {
            // The previous failures expired: this failure starts a new episode
            conn.execute(
              "UPDATE blacklist_pools
               SET reason = ?1, token_mint = ?2, program_id = ?3, error_count = ?4, first_failed_at = ?5, last_failed_at = ?6
               WHERE chain_id = ?7 AND pool_id = ?8",
              params![&reason_str, token_mint_str.as_deref(), program_id_str.as_deref(), record.error_count, record.first_failed_at, record.last_failed_at, &chain_id, &pool_id_str],
            )
            .map_err(|e| DatabaseError::Query { operation: "restart blacklist_pools entry".to_owned(), message: e.to_string() })?;
          }
          None => {
            conn.execute(
              "INSERT INTO blacklist_pools
               (chain_id, pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7)",
              params![&chain_id, &pool_id_str, &reason_str, token_mint_str.as_deref(), program_id_str.as_deref(), record.error_count, now],
            )
            .map_err(|e| DatabaseError::Query { operation: "insert into blacklist_pools".to_owned(), message: e.to_string() })?;
          }
        }

        Ok::<_, Error>(record)
    })
    .await
    .map_err(InternalError::from)??;

        let blacklisted_until = policy
            .blacklist_expiry(&record)
            .filter(|expires_at| unix_now() < *expires_at);
        {
            let mut pools = self.blacklisted_pools.write().unwrap();
            match blacklisted_until {
                Some(expires_at) => {
                    pools.insert(pool_id.to_string(), expires_at);
                }
                None => {
                    pools.remove(pool_id);
                }
            }
        }

        Ok(PoolFailureOutcome {
            error_count: record.error_count,
            blacklisted_until,
        })
    }

    /// Check if pool is blacklisted: only while its blacklist entry has not expired
    pub async fn is_pool_blacklisted(&self, pool_id: &str) -> Result<bool, Error> {
        // Hot path: memory only
        let pools = self.blacklisted_pools.read().unwrap();
        Ok(pools
            .get(pool_id)
            .is_some_and(|expires_at| unix_now() < *expires_at))
    }

    /// Remove account from blacklist
    pub async fn remove_account_from_blacklist(&self, account_pubkey: &str) -> Result<(), Error> {
        // Update memory immediately
        {
            let mut accounts = self.blacklisted_accounts.write().unwrap();
            accounts.remove(account_pubkey);
        }
        // Persist
        let account_key = account_pubkey.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;
            conn.execute(
                "DELETE FROM blacklist_accounts WHERE chain_id = ?1 AND account_pubkey = ?2",
                params![chain_id.as_str(), &account_key],
            )
            .map_err(|e| DatabaseError::Query {
                operation: "remove from blacklist_accounts".to_owned(),
                message: e.to_string(),
            })?;
            Ok(())
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Remove pool from blacklist
    pub async fn remove_pool_from_blacklist(&self, pool_id: &str) -> Result<(), Error> {
        // Update memory immediately
        {
            let mut pools = self.blacklisted_pools.write().unwrap();
            pools.remove(pool_id);
        }
        // Persist
        let pool_key = pool_id.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;
            conn.execute(
                "DELETE FROM blacklist_pools WHERE chain_id = ?1 AND pool_id = ?2",
                params![chain_id.as_str(), &pool_key],
            )
            .map_err(|e| DatabaseError::Query {
                operation: "remove from blacklist_pools".to_owned(),
                message: e.to_string(),
            })?;
            Ok(())
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Remove every pool and account blacklist row that names `token_mint`, from memory and
    /// storage, whether or not the row currently blacklists. Returns the pool and account
    /// rows removed.
    pub async fn clear_token_blacklists(&self, token_mint: &str) -> Result<(usize, usize), Error> {
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id;
        let mint = token_mint.to_owned();
        let (pool_ids, account_keys) = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;
            let column = |table: &str, column: &str| -> Result<Vec<String>, DatabaseError> {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT {column} FROM {table} WHERE chain_id = ?1 AND token_mint = ?2"
                    ))
                    .map_err(|e| {
                        DatabaseError::classify_sqlite_failure("prepare blacklist rows by token", e)
                    })?;
                let rows = stmt
                    .query_map(params![chain_id.as_str(), &mint], |row| row.get(0))
                    .map_err(|e| {
                        DatabaseError::classify_sqlite_failure("query blacklist rows by token", e)
                    })?;
                rows.collect::<Result<Vec<String>, _>>().map_err(|e| {
                    DatabaseError::classify_sqlite_failure("read blacklist rows by token", e)
                })
            };
            Ok::<_, DatabaseError>((
                column("blacklist_pools", "pool_id")?,
                column("blacklist_accounts", "account_pubkey")?,
            ))
        })
        .await
        .map_err(InternalError::from)??;
        for pool_id in &pool_ids {
            self.remove_pool_from_blacklist(pool_id).await?;
        }
        for account in &account_keys {
            self.remove_account_from_blacklist(account).await?;
        }
        Ok((pool_ids.len(), account_keys.len()))
    }

    /// Get blacklist statistics: currently blacklisted accounts and pools
    pub async fn get_blacklist_stats(&self) -> Result<(usize, usize), Error> {
        let now = unix_now();
        let accounts = self
            .blacklisted_accounts
            .read()
            .unwrap()
            .values()
            .filter(|expires_at| now < **expires_at)
            .count();
        let pools = self
            .blacklisted_pools
            .read()
            .unwrap()
            .values()
            .filter(|expires_at| now < **expires_at)
            .count();
        Ok((accounts, pools))
    }

    /// List accounts currently blacklisted under `policy` (latest failure within the TTL, the
    /// rule `add_account_to_blacklist` and `is_account_blacklisted` apply; an account
    /// blacklists on its first failure, so there is no failure-count threshold), most recent
    /// first. Rows past the TTL stay in the table and are not listed.
    pub async fn list_blacklisted_accounts(
        &self,
        limit: Option<usize>,
        policy: PoolBlacklistPolicy,
    ) -> Result<Vec<BlacklistedAccountRecord>, Error> {
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id;
        let failed_after = unix_now().saturating_sub(policy.ttl_secs);
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_accounts WHERE chain_id = ? AND last_failed_at > ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_accounts query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), failed_after, limit_value], |row| {
            Ok(BlacklistedAccountRecord {
              chain_id,
              account_pubkey: row.get(0)?,
              reason: row.get(1)?,
              source: row.get(2)?,
              pool_id: row.get(3)?,
              token_mint: row.get(4)?,
              error_count: row.get(5)?,
              first_failed_at: row.get(6)?,
              last_failed_at: row.get(7)?,
              added_at: row.get(8)?,
            })
          })
          .map_err(|e| DatabaseError::Query { operation: "query blacklist_accounts".to_owned(), message: e.to_string() })?;

        for row in rows {
          records.push(row.map_err(|e| DatabaseError::Query { operation: "read blacklist_accounts row".to_owned(), message: e.to_string() })?);
        }
      } else {
        let mut stmt = conn
          .prepare(
            "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_accounts WHERE chain_id = ? AND last_failed_at > ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_accounts query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), failed_after], |row| {
            Ok(BlacklistedAccountRecord {
              chain_id,
              account_pubkey: row.get(0)?,
              reason: row.get(1)?,
              source: row.get(2)?,
              pool_id: row.get(3)?,
              token_mint: row.get(4)?,
              error_count: row.get(5)?,
              first_failed_at: row.get(6)?,
              last_failed_at: row.get(7)?,
              added_at: row.get(8)?,
            })
          })
          .map_err(|e| DatabaseError::Query { operation: "query blacklist_accounts".to_owned(), message: e.to_string() })?;

        for row in rows {
          records.push(row.map_err(|e| DatabaseError::Query { operation: "read blacklist_accounts row".to_owned(), message: e.to_string() })?);
        }
      }

      Ok::<_, Error>(records)
    })
    .await
    .map_err(InternalError::from)?
    }

    /// List pools currently blacklisted under `policy` (failure count at the
    /// threshold, latest failure within the TTL — the same rule as
    /// `PoolBlacklistPolicy::is_blacklisted`), most recent first. Rows below the
    /// threshold or past the TTL stay in the table and are not listed.
    pub async fn list_blacklisted_pools(
        &self,
        limit: Option<usize>,
        policy: PoolBlacklistPolicy,
    ) -> Result<Vec<BlacklistedPoolRecord>, Error> {
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id;
        let min_error_count = i64::from(policy.threshold.max(1));
        let failed_after = unix_now().saturating_sub(policy.ttl_secs);
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_pools WHERE chain_id = ? AND COALESCE(error_count, 1) >= ? AND last_failed_at > ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_pools query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), min_error_count, failed_after, limit_value], |row| {
            Ok(BlacklistedPoolRecord {
              chain_id,
              pool_id: row.get(0)?,
              reason: row.get(1)?,
              token_mint: row.get(2)?,
              program_id: row.get(3)?,
              error_count: row.get(4)?,
              first_failed_at: row.get(5)?,
              last_failed_at: row.get(6)?,
              added_at: row.get(7)?,
            })
          })
          .map_err(|e| DatabaseError::Query { operation: "query blacklist_pools".to_owned(), message: e.to_string() })?;

        for row in rows {
          records.push(row.map_err(|e| DatabaseError::Query { operation: "read blacklist_pools row".to_owned(), message: e.to_string() })?);
        }
      } else {
        let mut stmt = conn
          .prepare(
            "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_pools WHERE chain_id = ? AND COALESCE(error_count, 1) >= ? AND last_failed_at > ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_pools query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), min_error_count, failed_after], |row| {
            Ok(BlacklistedPoolRecord {
              chain_id,
              pool_id: row.get(0)?,
              reason: row.get(1)?,
              token_mint: row.get(2)?,
              program_id: row.get(3)?,
              error_count: row.get(4)?,
              first_failed_at: row.get(5)?,
              last_failed_at: row.get(6)?,
              added_at: row.get(7)?,
            })
          })
          .map_err(|e| DatabaseError::Query { operation: "query blacklist_pools".to_owned(), message: e.to_string() })?;

        for row in rows {
          records.push(row.map_err(|e| DatabaseError::Query { operation: "read blacklist_pools row".to_owned(), message: e.to_string() })?);
        }
      }

      Ok::<_, Error>(records)
    })
    .await
    .map_err(InternalError::from)?
    }
}

#[cfg(test)]
mod tests {
    use super::super::operations::pooled_legacy_database;
    use super::*;

    const POLICY: PoolBlacklistPolicy = PoolBlacklistPolicy {
        threshold: 2,
        ttl_secs: 86_400,
    };

    fn stored_count(db: &PoolsDatabase, pool_id: &str) -> i64 {
        let conn = db
            .shared_pool()
            .expect("test pool")
            .get()
            .expect("checkout test connection");
        conn.query_row(
            "SELECT error_count FROM blacklist_pools WHERE chain_id = 'solana' AND pool_id = ?1",
            [pool_id],
            |row| row.get(0),
        )
        .expect("blacklist row")
    }

    #[tokio::test]
    async fn a_pool_is_blacklisted_only_once_the_threshold_is_reached() {
        let (db, _dir) = pooled_legacy_database("blacklist-threshold");

        let first = db
            .add_pool_to_blacklist("PoolA", "analysis_failed", Some("MintA"), None, 1, POLICY)
            .await
            .expect("record first failure");
        assert_eq!(first.error_count, 1);
        assert_eq!(first.blacklisted_until, None);
        assert!(!db.is_pool_blacklisted("PoolA").await.unwrap());
        assert!(db
            .list_blacklisted_pools(None, POLICY)
            .await
            .unwrap()
            .is_empty());

        let second = db
            .add_pool_to_blacklist("PoolA", "analysis_failed", Some("MintA"), None, 1, POLICY)
            .await
            .expect("record second failure");
        assert_eq!(second.error_count, 2);
        assert!(second.blacklisted_until.is_some());
        assert!(db.is_pool_blacklisted("PoolA").await.unwrap());
        assert_eq!(db.get_blacklist_stats().await.unwrap().1, 1);
        let listed = db.list_blacklisted_pools(None, POLICY).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].pool_id, "PoolA");
    }

    #[tokio::test]
    async fn an_expired_entry_stops_blacklisting_and_its_count_restarts() {
        let (db, _dir) = pooled_legacy_database("blacklist-expiry");
        let stale = unix_now() - POLICY.ttl_secs - 1;
        {
            let conn = db
                .shared_pool()
                .expect("test pool")
                .get()
                .expect("checkout test connection");
            conn.execute(
                "INSERT INTO blacklist_pools (chain_id, pool_id, reason, token_mint, error_count, first_failed_at, last_failed_at, added_at)
                 VALUES ('solana', 'PoolB', 'analysis_failed', 'MintB', 9, ?1, ?1, ?1)",
                [stale],
            )
            .expect("insert stale row");
        }
        // An entry whose expiry has passed no longer blacklists, without a restart
        db.blacklisted_pools
            .write()
            .unwrap()
            .insert("PoolB".to_owned(), stale + POLICY.ttl_secs);
        assert!(!db.is_pool_blacklisted("PoolB").await.unwrap());
        assert_eq!(db.get_blacklist_stats().await.unwrap().1, 0);
        assert!(db
            .list_blacklisted_pools(None, POLICY)
            .await
            .unwrap()
            .is_empty());

        let outcome = db
            .add_pool_to_blacklist("PoolB", "decoder_failed", Some("MintB"), None, 1, POLICY)
            .await
            .expect("record failure after expiry");
        assert_eq!(outcome.error_count, 1);
        assert_eq!(outcome.blacklisted_until, None);
        assert_eq!(stored_count(&db, "PoolB"), 1);
        assert!(!db.is_pool_blacklisted("PoolB").await.unwrap());
    }

    #[tokio::test]
    async fn an_account_blacklist_entry_expires_with_the_blacklist_ttl() {
        let (db, _dir) = pooled_legacy_database("blacklist-account-expiry");
        db.add_account_to_blacklist(
            "AccountA",
            "account_not_found_threshold",
            None,
            None,
            None,
            POLICY,
        )
        .await
        .expect("record account failure");
        assert!(db.is_account_blacklisted("AccountA").await.unwrap());
        assert_eq!(db.get_blacklist_stats().await.unwrap().0, 1);

        // Its expiry passes: the account is fetched again without a restart
        db.blacklisted_accounts
            .write()
            .unwrap()
            .insert("AccountA".to_owned(), unix_now() - 1);
        assert!(!db.is_account_blacklisted("AccountA").await.unwrap());
        assert_eq!(db.get_blacklist_stats().await.unwrap().0, 0);
    }

    #[tokio::test]
    async fn an_account_row_past_the_ttl_is_not_listed() {
        let (db, _dir) = pooled_legacy_database("blacklist-account-list-ttl");
        for account in ["AccountFresh", "AccountStale"] {
            db.add_account_to_blacklist(
                account,
                "account_not_found_threshold",
                None,
                None,
                Some("MintD"),
                POLICY,
            )
            .await
            .expect("record account failure");
        }
        let stale = unix_now() - POLICY.ttl_secs - 1;
        db.shared_pool()
            .expect("test pool")
            .get()
            .expect("checkout test connection")
            .execute(
                "UPDATE blacklist_accounts SET last_failed_at = ?1 WHERE account_pubkey = 'AccountStale'",
                [stale],
            )
            .expect("backdate the row");

        let listed = db.list_blacklisted_accounts(None, POLICY).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].account_pubkey, "AccountFresh");
        let limited = db.list_blacklisted_accounts(Some(5), POLICY).await.unwrap();
        assert_eq!(limited.len(), 1);
    }

    #[tokio::test]
    async fn clearing_a_token_removes_only_its_pool_and_account_rows() {
        let (db, _dir) = pooled_legacy_database("blacklist-clear-token");
        for (pool, account, mint) in [
            ("PoolE", "AccountE", "MintE"),
            ("PoolF", "AccountF", "MintF"),
        ] {
            db.add_pool_to_blacklist(pool, "decoder_failed", Some(mint), None, 2, POLICY)
                .await
                .expect("blacklist the pool");
            db.add_account_to_blacklist(
                account,
                "account_not_found_threshold",
                None,
                Some(pool),
                Some(mint),
                POLICY,
            )
            .await
            .expect("blacklist the account");
        }

        let cleared = db.clear_token_blacklists("MintE").await.expect("clear");
        assert_eq!(cleared, (1, 1));
        assert!(!db.is_pool_blacklisted("PoolE").await.unwrap());
        assert!(!db.is_account_blacklisted("AccountE").await.unwrap());
        assert!(db.is_pool_blacklisted("PoolF").await.unwrap());
        assert!(db.is_account_blacklisted("AccountF").await.unwrap());
        let pools = db.list_blacklisted_pools(None, POLICY).await.unwrap();
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].pool_id, "PoolF");
        let accounts = db.list_blacklisted_accounts(None, POLICY).await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].account_pubkey, "AccountF");
        assert_eq!(db.clear_token_blacklists("MintE").await.unwrap(), (0, 0));
    }

    #[tokio::test]
    async fn a_caller_counted_threshold_blacklists_on_its_first_report() {
        let (db, _dir) = pooled_legacy_database("blacklist-counted");
        let outcome = db
            .add_pool_to_blacklist("PoolC", "missing_accounts", Some("MintC"), None, 2, POLICY)
            .await
            .expect("record counted failures");
        assert_eq!(outcome.error_count, 2);
        assert!(db.is_pool_blacklisted("PoolC").await.unwrap());

        db.remove_pool_from_blacklist("PoolC").await.unwrap();
        assert!(!db.is_pool_blacklisted("PoolC").await.unwrap());
    }
}
