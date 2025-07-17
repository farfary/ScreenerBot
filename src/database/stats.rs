use crate::database::models::DatabaseResult;
use crate::database::connection::Database;
use crate::types::DiscoveryStats;
use anyhow::Result;
use chrono::{ DateTime, Utc };
use rusqlite::params;

impl Database {
    /// Save discovery statistics
    pub fn save_discovery_stats(&self, stats: &DiscoveryStats) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO discovery_stats 
            (total_tokens_discovered, active_tokens, last_discovery_run, 
             discovery_rate_per_hour, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                stats.total_tokens_discovered,
                stats.active_tokens,
                stats.last_discovery_run.to_rfc3339(),
                stats.discovery_rate_per_hour,
                Utc::now().to_rfc3339()
            ]
        )?;
        Ok(())
    }

    /// Get latest discovery statistics
    pub fn get_latest_discovery_stats(&self) -> Result<Option<DiscoveryStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM discovery_stats ORDER BY created_at DESC LIMIT 1"
        )?;

        let mut stats_iter = stmt.query_map([], |row| {
            Ok(DiscoveryStats {
                total_tokens_discovered: row.get(1)?,
                active_tokens: row.get(2)?,
                last_discovery_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                discovery_rate_per_hour: row.get(4)?,
            })
        })?;

        if let Some(stats) = stats_iter.next() {
            return Ok(Some(stats?));
        }

        Ok(None)
    }

    /// Get discovery statistics history
    pub async fn get_discovery_stats_history(
        &self,
        limit: u32
    ) -> DatabaseResult<Vec<DiscoveryStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM discovery_stats ORDER BY created_at DESC LIMIT ?1"
        )?;

        let stats_iter = stmt.query_map([limit], |row| {
            Ok(DiscoveryStats {
                total_tokens_discovered: row.get(1)?,
                active_tokens: row.get(2)?,
                last_discovery_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                discovery_rate_per_hour: row.get(4)?,
            })
        })?;

        let mut stats = Vec::new();
        for stat in stats_iter {
            stats.push(stat?);
        }

        Ok(stats)
    }

    /// Get discovery stats summary
    pub async fn get_discovery_stats_summary(&self) -> DatabaseResult<DiscoveryStatsSummary> {
        let conn = self.conn.lock().unwrap();

        let total_discoveries: u64 = conn.query_row(
            "SELECT COUNT(*) FROM discovery_stats",
            [],
            |row| row.get(0)
        )?;

        let avg_discovery_rate: f64 = conn.query_row(
            "SELECT COALESCE(AVG(discovery_rate_per_hour), 0) FROM discovery_stats",
            [],
            |row| row.get(0)
        )?;

        let max_discovery_rate: f64 = conn.query_row(
            "SELECT COALESCE(MAX(discovery_rate_per_hour), 0) FROM discovery_stats",
            [],
            |row| row.get(0)
        )?;

        let min_discovery_rate: f64 = conn.query_row(
            "SELECT COALESCE(MIN(discovery_rate_per_hour), 0) FROM discovery_stats",
            [],
            |row| row.get(0)
        )?;

        let latest_total_tokens: u64 = conn
            .query_row(
                "SELECT COALESCE(total_tokens_discovered, 0) FROM discovery_stats ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0)
            )
            .unwrap_or(0);

        let latest_active_tokens: u64 = conn
            .query_row(
                "SELECT COALESCE(active_tokens, 0) FROM discovery_stats ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0)
            )
            .unwrap_or(0);

        Ok(DiscoveryStatsSummary {
            total_discoveries,
            avg_discovery_rate,
            max_discovery_rate,
            min_discovery_rate,
            latest_total_tokens,
            latest_active_tokens,
        })
    }

    /// Clean up old discovery stats
    pub async fn cleanup_old_discovery_stats(&self, max_age_days: u64) -> DatabaseResult<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);

        let rows_affected = conn.execute(
            "DELETE FROM discovery_stats WHERE created_at < ?1",
            params![cutoff_date.to_rfc3339()]
        )?;

        Ok(rows_affected as u64)
    }

    /// Get discovery trend analysis
    pub async fn get_discovery_trend(&self, hours: u32) -> DatabaseResult<Option<DiscoveryTrend>> {
        let conn = self.conn.lock().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);

        let mut stmt = conn.prepare(
            "SELECT 
                MIN(total_tokens_discovered) as min_tokens,
                MAX(total_tokens_discovered) as max_tokens,
                AVG(discovery_rate_per_hour) as avg_rate,
                COUNT(*) as data_points
             FROM discovery_stats 
             WHERE created_at >= ?1"
        )?;

        let mut trend_iter = stmt.query_map([cutoff_time.to_rfc3339()], |row| {
            Ok(DiscoveryTrend {
                min_tokens: row.get(0)?,
                max_tokens: row.get(1)?,
                avg_rate: row.get(2)?,
                data_points: row.get(3)?,
                growth_rate: 0.0, // Calculate below
            })
        })?;

        if let Some(trend_result) = trend_iter.next() {
            let mut trend = trend_result?;

            // Calculate growth rate
            if trend.min_tokens > 0 {
                trend.growth_rate =
                    (((trend.max_tokens as f64) - (trend.min_tokens as f64)) /
                        (trend.min_tokens as f64)) *
                    100.0;
            }

            return Ok(Some(trend));
        }

        Ok(None)
    }
}

/// Discovery statistics summary
#[derive(Debug, Clone)]
pub struct DiscoveryStatsSummary {
    pub total_discoveries: u64,
    pub avg_discovery_rate: f64,
    pub max_discovery_rate: f64,
    pub min_discovery_rate: f64,
    pub latest_total_tokens: u64,
    pub latest_active_tokens: u64,
}

/// Discovery trend analysis
#[derive(Debug, Clone)]
pub struct DiscoveryTrend {
    pub min_tokens: u64,
    pub max_tokens: u64,
    pub avg_rate: f64,
    pub data_points: u32,
    pub growth_rate: f64, // Percentage growth
}
