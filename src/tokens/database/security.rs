use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::logger::{self, LogTag};
use crate::tokens::store;
use crate::tokens::types::{RugcheckData, SecurityRisk, TokenError, TokenHolder, TokenResult};

use super::TokenDatabase;

impl TokenDatabase {
    pub fn upsert_rugcheck_data(&self, mint: &str, data: &RugcheckData) -> TokenResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let risks_json = serde_json::to_string(&data.risks)
            .map_err(|e| TokenError::Database(format!("Failed to serialize risks: {}", e)))?;
        let holders_json = serde_json::to_string(&data.top_holders)
            .map_err(|e| TokenError::Database(format!("Failed to serialize holders: {}", e)))?;
        let markets_json = data
            .markets
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| TokenError::Database(format!("Failed to serialize markets: {}", e)))?;

        let rugged_flag = if data.rugged { 1 } else { 0 };

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM security_rugcheck WHERE mint = ?1",
                params![mint],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count == 0)
                },
            )
            .unwrap_or(true);

        let now_ts = data.security_data_last_fetched_at.timestamp();
        let first_fetched_ts = if is_first_insert {
            now_ts
        } else {
            // Preserve existing first_fetched_at on updates
            conn.query_row(
                "SELECT security_data_first_fetched_at FROM security_rugcheck WHERE mint = ?1",
                params![mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        let is_mutable_flag = data.is_mutable.map(|b| if b { 1 } else { 0 });

        conn.execute(
            "INSERT INTO security_rugcheck (
                mint,
                token_type,
                token_decimals,
                score,
                score_normalised,
                score_description,
                mint_authority,
                freeze_authority,
                update_authority,
                is_mutable,
                top_10_holders_pct,
                total_supply,
                total_holders,
                total_lp_providers,
                graph_insiders_detected,
                total_market_liquidity,
                total_stable_liquidity,
                creator_balance_pct,
                transfer_fee_pct,
                transfer_fee_max_amount,
                transfer_fee_authority,
                rugged,
                risks,
                top_holders,
                markets,
                security_data_last_fetched_at,
                security_data_first_fetched_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27
             )
             ON CONFLICT(mint) DO UPDATE SET
                token_type = excluded.token_type,
                token_decimals = excluded.token_decimals,
                score = excluded.score,
                score_normalised = excluded.score_normalised,
                score_description = excluded.score_description,
                mint_authority = excluded.mint_authority,
                freeze_authority = excluded.freeze_authority,
                update_authority = excluded.update_authority,
                is_mutable = excluded.is_mutable,
                top_10_holders_pct = excluded.top_10_holders_pct,
                total_supply = excluded.total_supply,
                total_holders = excluded.total_holders,
                total_lp_providers = excluded.total_lp_providers,
                graph_insiders_detected = excluded.graph_insiders_detected,
                total_market_liquidity = excluded.total_market_liquidity,
                total_stable_liquidity = excluded.total_stable_liquidity,
                creator_balance_pct = excluded.creator_balance_pct,
                transfer_fee_pct = excluded.transfer_fee_pct,
                transfer_fee_max_amount = excluded.transfer_fee_max_amount,
                transfer_fee_authority = excluded.transfer_fee_authority,
                rugged = excluded.rugged,
                risks = excluded.risks,
                top_holders = excluded.top_holders,
                markets = excluded.markets,
                security_data_last_fetched_at = excluded.security_data_last_fetched_at",
            params![
                mint,
                &data.token_type,
                data.token_decimals,
                data.score,
                data.score_normalised,
                &data.score_description,
                &data.mint_authority,
                &data.freeze_authority,
                &data.update_authority,
                is_mutable_flag,
                data.top_10_holders_pct,
                &data.total_supply,
                data.total_holders,
                data.total_lp_providers,
                data.graph_insiders_detected,
                data.total_market_liquidity,
                data.total_stable_liquidity,
                data.creator_balance_pct,
                data.transfer_fee_pct,
                data.transfer_fee_max_amount,
                &data.transfer_fee_authority,
                rugged_flag,
                risks_json,
                holders_json,
                markets_json,
                now_ts,
                first_fetched_ts,
            ],
        )
        .map_err(|e| TokenError::Database(format!("Failed to upsert Rugcheck data: {}", e)))?;

        // Update in-memory cache
        store::store_rugcheck(mint, data);

        Ok(())
    }

    pub fn get_rugcheck_data(&self, mint: &str) -> TokenResult<Option<RugcheckData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT
                    token_type,
                    token_decimals,
                    score,
                    score_normalised,
                    score_description,
                    mint_authority,
                    freeze_authority,
                    top_10_holders_pct,
                    total_supply,
                    total_holders,
                    total_lp_providers,
                    graph_insiders_detected,
                    total_market_liquidity,
                    total_stable_liquidity,
                    creator_balance_pct,
                    transfer_fee_pct,
                    transfer_fee_max_amount,
                    transfer_fee_authority,
                    rugged,
                    risks,
                    top_holders,
                    markets,
                    security_data_last_fetched_at,
                    security_data_first_fetched_at,
                    update_authority,
                    is_mutable
                 FROM security_rugcheck WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let risks_json: String = row.get(19)?;
            let holders_json: String = row.get(20)?;
            let markets_json: Option<String> = row.get(21)?;
            let fetched_ts: i64 = row.get(22)?;
            let first_fetched_ts: i64 = row.get(23)?;
            let rugged_flag: Option<i64> = row.get(18)?;
            let is_rugged = rugged_flag.unwrap_or(0) != 0;
            let is_mutable_flag: Option<i64> = row.get(25)?;
            let is_mutable = is_mutable_flag.map(|f| f != 0);

            let risks: Vec<SecurityRisk> = serde_json::from_str(&risks_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let holders: Vec<TokenHolder> = serde_json::from_str(&holders_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let markets = markets_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(RugcheckData {
                token_type: row.get(0)?,
                token_decimals: row.get(1)?,
                score: row.get(2)?,
                score_normalised: row.get(3)?,
                score_description: row.get(4)?,
                mint_authority: row.get(5)?,
                freeze_authority: row.get(6)?,
                update_authority: row.get(24)?,
                is_mutable,
                top_10_holders_pct: row.get(7)?,
                total_supply: row.get(8)?,
                total_holders: row.get(9)?,
                total_lp_providers: row.get(10)?,
                graph_insiders_detected: row.get(11)?,
                total_market_liquidity: row.get(12)?,
                total_stable_liquidity: row.get(13)?,
                creator_balance_pct: row.get(14)?,
                transfer_fee_pct: row.get(15)?,
                transfer_fee_max_amount: row.get(16)?,
                transfer_fee_authority: row.get(17)?,
                rugged: is_rugged,
                risks,
                top_holders: holders,
                markets,
                security_data_last_fetched_at: DateTime::from_timestamp(fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                security_data_first_fetched_at: DateTime::from_timestamp(first_fetched_ts, 0)
                    .unwrap_or_else(|| {
                        DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now())
                    }),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    pub fn get_tokens_without_security_data(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        // Base backoff interval: 2 minutes (120 seconds)
        // Max backoff: 24 hours (86400 seconds)
        // Formula: min(120 * 2^(error_count - 1), 86400)
        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             LEFT JOIN security_rugcheck sr ON t.mint = sr.mint
             LEFT JOIN blacklist b ON t.mint = b.mint
             LEFT JOIN update_tracking ut ON t.mint = ut.mint
             WHERE sr.mint IS NULL
             AND b.mint IS NULL
             AND (
                 -- Never tried
                 ut.security_error_type IS NULL
                 -- Temporary errors with exponential backoff
                 OR (ut.security_error_type = 'temporary' 
                     AND ut.last_security_error_at < ?1 - (120 * (1 << MIN(ut.security_error_count - 1, 10))))
                 -- Permanent errors retry after 7 days
                 OR (ut.security_error_type = 'permanent' 
                     AND ut.last_security_error_at < ?1 - 604800)
             )
             ORDER BY 
                 CASE 
                     -- Priority 1: New tokens (discovered in last 24h, no errors)
                     WHEN ut.security_error_type IS NULL AND t.first_discovered_at > ?1 - 86400 THEN 1
                     -- Priority 2: Tokens without errors
                     WHEN ut.security_error_type IS NULL THEN 2
                     -- Priority 3: Temporary errors (with backoff)
                     WHEN ut.security_error_type = 'temporary' THEN 3
                     -- Priority 4: Permanent errors (very rare retry)
                     ELSE 4
                 END,
                 t.first_discovered_at ASC
             LIMIT ?2",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mints = stmt
            .query_map(params![now, limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    /// Record a failed market update attempt with error type tracking
    ///
    /// Error types:
    /// - "temporary": Transient errors (rate limit, network issues) - retry with backoff
    /// - "permanent": Token not listed on any exchange - stop retrying after threshold
    ///

    pub fn mark_security_data_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                security_data_last_updated_at = ?1,
                security_data_update_count = security_data_update_count + 1,
                last_security_error = NULL,
                last_security_error_at = NULL,
                security_error_type = NULL
             WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| {
            TokenError::Database(format!("Failed to mark security data updated: {}", e))
        })?;

        Ok(())
    }


    pub fn record_security_error(
        &self,
        mint: &str,
        message: &str,
        error_type: &str,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = security_error_count + 1,
                last_security_error = ?1,
                last_security_error_at = ?2,
                security_error_type = ?3
             WHERE mint = ?4",
            params![message, now, error_type, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to record security error: {}", e)))?;

        Ok(())
    }

    /// Clear security error tracking (called after successful fetch)
    pub fn clear_security_error(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = 0,
                last_security_error = NULL,
                last_security_error_at = NULL,
                security_error_type = NULL
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear security error: {}", e)))?;

        Ok(())
    }
}
