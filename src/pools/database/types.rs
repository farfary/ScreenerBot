/// Database structures and conversion utilities
use super::super::types::PriceResult;
use chrono::{DateTime, Utc};
use rusqlite::Row;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// =============================================================================
// DATABASE STRUCTURES
// =============================================================================

/// Database representation of a price result for storage
#[derive(Debug, Clone)]
pub struct DbPriceResult {
    pub id: Option<i64>,
    pub mint: String,
    pub pool_address: String,
    pub price_usd: f64,
    pub price_sol: f64,
    pub confidence: f32,
    pub slot: u64,
    pub timestamp_unix: i64,
    pub sol_reserves: f64,
    pub token_reserves: f64,
    pub source_pool: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DbPriceResult {
    /// Create from PriceResult
    pub fn from_price_result(price: &PriceResult) -> Self {
        let timestamp_unix = Self::approximate_unix_timestamp(price.timestamp);

        Self {
            id: None,
            mint: price.mint.clone(),
            pool_address: price.pool_address.clone(),
            price_usd: price.price_usd,
            price_sol: price.price_sol,
            confidence: price.confidence,
            slot: price.slot,
            timestamp_unix,
            sol_reserves: price.sol_reserves,
            token_reserves: price.token_reserves,
            source_pool: price.source_pool.clone(),
            created_at: Utc::now(),
        }
    }

    /// Convert to PriceResult
    pub fn to_price_result(&self) -> PriceResult {
        PriceResult {
            mint: self.mint.clone(),
            price_usd: self.price_usd,
            price_sol: self.price_sol,
            confidence: self.confidence,
            source_pool: self.source_pool.clone(),
            pool_address: self.pool_address.clone(),
            slot: self.slot,
            timestamp: Self::instant_from_unix_timestamp(self.timestamp_unix),
            sol_reserves: self.sol_reserves,
            token_reserves: self.token_reserves,
        }
    }

    /// Create from database row
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let created_at_str: String = row.get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "created_at".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(Self {
            id: Some(row.get("id")?),
            mint: row.get("mint")?,
            pool_address: row.get("pool_address")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            confidence: row.get("confidence")?,
            slot: row.get("slot")?,
            timestamp_unix: row.get("timestamp_unix")?,
            sol_reserves: row.get("sol_reserves")?,
            token_reserves: row.get("token_reserves")?,
            source_pool: row.get("source_pool")?,
            created_at,
        })
    }

    /// Convert an Instant to an approximate unix timestamp (seconds precision)
    fn approximate_unix_timestamp(instant: std::time::Instant) -> i64 {
        let now_system = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        match now_system.checked_sub(instant.elapsed()) {
            Some(ts) => ts.as_secs() as i64,
            None => 0,
        }
    }

    /// Recreate an Instant from a unix timestamp (seconds precision)
    fn instant_from_unix_timestamp(timestamp_unix: i64) -> std::time::Instant {
        if timestamp_unix <= 0 {
            return std::time::Instant::now();
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let diff = if timestamp_unix >= now {
            0
        } else {
            (now - timestamp_unix) as u64
        };
        let duration = Duration::from_secs(diff);

        std::time::Instant::now()
            .checked_sub(duration)
            .unwrap_or_else(std::time::Instant::now)
    }
}

#[derive(Debug, Clone)]
pub struct BlacklistedAccountRecord {
    pub account_pubkey: String,
    pub reason: String,
    pub source: Option<String>,
    pub pool_id: Option<String>,
    pub token_mint: Option<String>,
    pub error_count: i64,
    pub first_failed_at: i64,
    pub last_failed_at: i64,
    pub added_at: i64,
}

#[derive(Debug, Clone)]
pub struct BlacklistedPoolRecord {
    pub pool_id: String,
    pub reason: String,
    pub token_mint: Option<String>,
    pub program_id: Option<String>,
    pub error_count: i64,
    pub first_failed_at: i64,
    pub last_failed_at: i64,
    pub added_at: i64,
}
