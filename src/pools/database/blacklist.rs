//! Blacklist operations for accounts and pools

use super::operations::PoolsDatabase;
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord};
use crate::errors::{DatabaseError, InternalError};
use crate::pools::Error;

use rusqlite::params;
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// BLACKLIST OPERATIONS
// =============================================================================

impl PoolsDatabase {
    /// Add account to blacklist
    pub async fn add_account_to_blacklist(
        &self,
        account_pubkey: &str,
        reason: &str,
        source: Option<&str>,
        pool_id: Option<&str>,
        token_mint: Option<&str>,
    ) -> Result<(), Error> {
        let account_key = account_pubkey.to_string();
        let reason_str = reason.to_string();
        let source_str = source.map(|s| s.to_string());
        let pool_id_str = pool_id.map(|s| s.to_string());
        let token_mint_str = token_mint.map(|s| s.to_string());
        let chain_id = self.chain_id.as_str().to_owned();
        // Update memory immediately
        {
            let mut set = self.blacklisted_accounts.write().unwrap();
            set.insert(account_key.clone());
        }

        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
      let conn_guard = conn_arc
        .lock()
        .map_err(|e| DatabaseError::Query { operation: "lock connection".to_owned(), message: e.to_string() })?;

      if let Some(ref conn) = *conn_guard {
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
      } else {
        Err(Error::NotInitialized)
      }
    })
    .await
    .map_err(InternalError::from)?
    }

    /// Check if account is blacklisted
    pub async fn is_account_blacklisted(&self, account_pubkey: &str) -> Result<bool, Error> {
        // Hot path: memory only
        let set = self.blacklisted_accounts.read().unwrap();
        Ok(set.contains(account_pubkey))
    }

    /// Add pool to blacklist
    pub async fn add_pool_to_blacklist(
        &self,
        pool_id: &str,
        reason: &str,
        token_mint: Option<&str>,
        program_id: Option<&str>,
    ) -> Result<(), Error> {
        let pool_id_str = pool_id.to_string();
        let reason_str = reason.to_string();
        let token_mint_str = token_mint.map(|s| s.to_string());
        let program_id_str = program_id.map(|s| s.to_string());
        let chain_id = self.chain_id.as_str().to_owned();
        // Update memory immediately
        {
            let mut set = self.blacklisted_pools.write().unwrap();
            set.insert(pool_id_str.clone());
        }

        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
      let conn_guard = conn_arc
        .lock()
        .map_err(|e| DatabaseError::Query { operation: "lock connection".to_owned(), message: e.to_string() })?;

      if let Some(ref conn) = *conn_guard {
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap()
          .as_secs() as i64;

        // Check if already exists
        let exists: bool = conn
          .query_row(
            "SELECT 1 FROM blacklist_pools WHERE chain_id = ?1 AND pool_id = ?2",
            params![&chain_id, &pool_id_str],
            |_| Ok(true),
          )
          .unwrap_or_default();

        if exists {
          // Increment error count and update last_failed_at
          conn.execute(
            "UPDATE blacklist_pools 
             SET error_count = error_count + 1, last_failed_at = ?1 
             WHERE chain_id = ?2 AND pool_id = ?3",
            params![now, &chain_id, &pool_id_str],
          )
          .map_err(|e| DatabaseError::Query { operation: "update blacklist_pools".to_owned(), message: e.to_string() })?;
        } else {
          // Insert new entry
          conn.execute(
            "INSERT INTO blacklist_pools 
             (chain_id, pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?6)",
            params![&chain_id, &pool_id_str, &reason_str, token_mint_str.as_deref(), program_id_str.as_deref(), now],
          )
          .map_err(|e| DatabaseError::Query { operation: "insert into blacklist_pools".to_owned(), message: e.to_string() })?;
        }

        Ok(())
      } else {
        Err(Error::NotInitialized)
      }
    })
    .await
    .map_err(InternalError::from)?
    }

    /// Check if pool is blacklisted
    pub async fn is_pool_blacklisted(&self, pool_id: &str) -> Result<bool, Error> {
        // Hot path: memory only
        let set = self.blacklisted_pools.read().unwrap();
        Ok(set.contains(pool_id))
    }

    /// Remove account from blacklist
    pub async fn remove_account_from_blacklist(&self, account_pubkey: &str) -> Result<(), Error> {
        // Update memory immediately
        {
            let mut set = self.blacklisted_accounts.write().unwrap();
            set.remove(account_pubkey);
        }
        // Persist
        let account_key = account_pubkey.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc.lock().map_err(|e| DatabaseError::Query {
                operation: "lock connection".to_owned(),
                message: e.to_string(),
            })?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_accounts WHERE chain_id = ?1 AND account_pubkey = ?2",
                    params![chain_id.as_str(), &account_key],
                )
                .map_err(|e| DatabaseError::Query {
                    operation: "remove from blacklist_accounts".to_owned(),
                    message: e.to_string(),
                })?;
                Ok(())
            } else {
                Err(Error::NotInitialized)
            }
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Remove pool from blacklist
    pub async fn remove_pool_from_blacklist(&self, pool_id: &str) -> Result<(), Error> {
        // Update memory immediately
        {
            let mut set = self.blacklisted_pools.write().unwrap();
            set.remove(pool_id);
        }
        // Persist
        let pool_key = pool_id.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc.lock().map_err(|e| DatabaseError::Query {
                operation: "lock connection".to_owned(),
                message: e.to_string(),
            })?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_pools WHERE chain_id = ?1 AND pool_id = ?2",
                    params![chain_id.as_str(), &pool_key],
                )
                .map_err(|e| DatabaseError::Query {
                    operation: "remove from blacklist_pools".to_owned(),
                    message: e.to_string(),
                })?;
                Ok(())
            } else {
                Err(Error::NotInitialized)
            }
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Get blacklist statistics
    pub async fn get_blacklist_stats(&self) -> Result<(usize, usize), Error> {
        let accounts = self.blacklisted_accounts.read().unwrap().len();
        let pools = self.blacklisted_pools.read().unwrap().len();
        Ok((accounts, pools))
    }

    /// List blacklisted accounts with optional limit, ordered by most recent first
    pub async fn list_blacklisted_accounts(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedAccountRecord>, Error> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
      let connection_guard = conn_arc
        .lock()
        .map_err(|e| DatabaseError::Query { operation: "lock connection".to_owned(), message: e.to_string() })?;

      let conn = connection_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized)?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_accounts WHERE chain_id = ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_accounts query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), limit_value], |row| {
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
             FROM blacklist_accounts WHERE chain_id = ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_accounts query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map([chain_id.as_str()], |row| {
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

    /// List blacklisted pools with optional limit, ordered by most recent first
    pub async fn list_blacklisted_pools(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedPoolRecord>, Error> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
      let connection_guard = conn_arc
        .lock()
        .map_err(|e| DatabaseError::Query { operation: "lock connection".to_owned(), message: e.to_string() })?;

      let conn = connection_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized)?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_pools WHERE chain_id = ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_pools query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map(params![chain_id.as_str(), limit_value], |row| {
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
             FROM blacklist_pools WHERE chain_id = ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| DatabaseError::Query { operation: "prepare blacklist_pools query".to_owned(), message: e.to_string() })?;

        let rows = stmt
          .query_map([chain_id.as_str()], |row| {
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
