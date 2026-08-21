//! Blacklist operations for accounts and pools

use super::operations::PoolsDatabase;
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord};
use crate::chains::ChainId;

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
    ) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to lock connection: {e}"))?;

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
          .map_err(|e| format!("Failed to update blacklist_accounts: {e}"))?;
        } else {
          // Insert new entry
          conn.execute(
            "INSERT INTO blacklist_accounts 
             (chain_id, account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7, ?7)",
            params![&chain_id, &account_key, &reason_str, source_str.as_deref(), pool_id_str.as_deref(), token_mint_str.as_deref(), now],
          )
          .map_err(|e| format!("Failed to insert into blacklist_accounts: {e}"))?;
        }

        Ok(())
      } else {
        Err("Database connection not available".to_owned())
      }
    })
    .await
    .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Check if account is blacklisted
    pub async fn is_account_blacklisted(&self, account_pubkey: &str) -> Result<bool, String> {
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
    ) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to lock connection: {e}"))?;

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
          .map_err(|e| format!("Failed to update blacklist_pools: {e}"))?;
        } else {
          // Insert new entry
          conn.execute(
            "INSERT INTO blacklist_pools 
             (chain_id, pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?6)",
            params![&chain_id, &pool_id_str, &reason_str, token_mint_str.as_deref(), program_id_str.as_deref(), now],
          )
          .map_err(|e| format!("Failed to insert into blacklist_pools: {e}"))?;
        }

        Ok(())
      } else {
        Err("Database connection not available".to_owned())
      }
    })
    .await
    .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Check if pool is blacklisted
    pub async fn is_pool_blacklisted(&self, pool_id: &str) -> Result<bool, String> {
        // Hot path: memory only
        let set = self.blacklisted_pools.read().unwrap();
        Ok(set.contains(pool_id))
    }

    /// Remove account from blacklist
    pub async fn remove_account_from_blacklist(&self, account_pubkey: &str) -> Result<(), String> {
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
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_accounts WHERE chain_id = ?1 AND account_pubkey = ?2",
                    params![chain_id.as_str(), &account_key],
                )
                .map_err(|e| format!("Failed to remove from blacklist_accounts: {e}"))?;
                Ok(())
            } else {
                Err("Database connection not available".to_owned())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Remove pool from blacklist
    pub async fn remove_pool_from_blacklist(&self, pool_id: &str) -> Result<(), String> {
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
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_pools WHERE chain_id = ?1 AND pool_id = ?2",
                    params![chain_id.as_str(), &pool_key],
                )
                .map_err(|e| format!("Failed to remove from blacklist_pools: {e}"))?;
                Ok(())
            } else {
                Err("Database connection not available".to_owned())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Get blacklist statistics
    pub async fn get_blacklist_stats(&self) -> Result<(usize, usize), String> {
        let accounts = self.blacklisted_accounts.read().unwrap().len();
        let pools = self.blacklisted_pools.read().unwrap().len();
        Ok((accounts, pools))
    }

    /// List blacklisted accounts with optional limit, ordered by most recent first
    pub async fn list_blacklisted_accounts(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedAccountRecord>, String> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
      let connection_guard = conn_arc
        .lock()
        .map_err(|e| format!("Failed to lock connection: {e}"))?;

      let conn = connection_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_owned())?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_accounts WHERE chain_id = ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| format!("Failed to prepare blacklist_accounts query: {e}"))?;

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
          .map_err(|e| format!("Failed to query blacklist_accounts: {e}"))?;

        for row in rows {
          records.push(row.map_err(|e| format!("Failed to read blacklist_accounts row: {e}"))?);
        }
      } else {
        let mut stmt = conn
          .prepare(
            "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_accounts WHERE chain_id = ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| format!("Failed to prepare blacklist_accounts query: {e}"))?;

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
          .map_err(|e| format!("Failed to query blacklist_accounts: {e}"))?;

        for row in rows {
          records.push(row.map_err(|e| format!("Failed to read blacklist_accounts row: {e}"))?);
        }
      }

      Ok::<_, String>(records)
    })
    .await
    .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// List blacklisted pools with optional limit, ordered by most recent first
    pub async fn list_blacklisted_pools(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedPoolRecord>, String> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::task::spawn_blocking(move || {
      let connection_guard = conn_arc
        .lock()
        .map_err(|e| format!("Failed to lock connection: {e}"))?;

      let conn = connection_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_owned())?;

      let mut records = Vec::new();

      if let Some(limit_value) = limit.map(|l| l as i64) {
        let mut stmt = conn
          .prepare(
            "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_pools WHERE chain_id = ? \
             ORDER BY last_failed_at DESC \
             LIMIT ?",
          )
          .map_err(|e| format!("Failed to prepare blacklist_pools query: {e}"))?;

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
          .map_err(|e| format!("Failed to query blacklist_pools: {e}"))?;

        for row in rows {
          records.push(row.map_err(|e| format!("Failed to read blacklist_pools row: {e}"))?);
        }
      } else {
        let mut stmt = conn
          .prepare(
            "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
             FROM blacklist_pools WHERE chain_id = ? \
             ORDER BY last_failed_at DESC",
          )
          .map_err(|e| format!("Failed to prepare blacklist_pools query: {e}"))?;

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
          .map_err(|e| format!("Failed to query blacklist_pools: {e}"))?;

        for row in rows {
          records.push(row.map_err(|e| format!("Failed to read blacklist_pools row: {e}"))?);
        }
      }

      Ok::<_, String>(records)
    })
    .await
    .map_err(|e| format!("Blocking task failed: {e}"))?
    }
}
