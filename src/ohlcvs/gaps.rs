// Gap detection and filling system

use crate::config::with_config;
use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::types::{OhlcvDataPoint, OhlcvError, OhlcvResult, Priority, Timeframe};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct GapManager {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
}

impl GapManager {
    pub fn new(db: Arc<OhlcvDatabase>, fetcher: Arc<OhlcvFetcher>) -> Self {
        Self { db, fetcher }
    }

    /// Detect gaps in stored data for a token
    pub async fn detect_gaps(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<(i64, i64)>> {
        let retention_days = with_config(|cfg| cfg.ohlcv.retention_days).max(1);
        let timeframe_seconds = timeframe.to_seconds().max(1);
        let window_seconds = (retention_days as i64).saturating_mul(86_400);
        let lookback_start =
            (Utc::now().timestamp() - window_seconds - timeframe_seconds * 2).max(0);

        let estimated_points = ((window_seconds / 60).max(1) as usize).saturating_add(512);
        let limit = estimated_points.min(200_000);

        // DEBUG: Record gap detection start
        record_ohlcv_event(
            "gap_detection_start",
            Severity::Debug,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "lookback_start": lookback_start,
                "limit": limit,
            }),
        )
        .await;

        // Get existing data sliced to the retention window in ascending order
        let mut data = self.db.get_1m_data_range_asc(
            mint,
            pool_address,
            Some(lookback_start),
            None,
            Some(limit),
        )?;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Normalize data for requested timeframe.
        let normalized = if timeframe == Timeframe::Minute1 {
            data
        } else {
            OhlcvAggregator::aggregate(&data, timeframe)?
        };

        // Detect gaps using aggregator
        let gaps = OhlcvAggregator::detect_gaps(&normalized, timeframe);

        // Store detected gaps in database
        for (start, end) in &gaps {
            self.db
                .insert_gap(mint, pool_address, timeframe, *start, *end)?;
        }

        // INFO: Record gap detection completion
        record_ohlcv_event(
            "gap_detection_complete",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "gaps_found": gaps.len(),
                "data_points_analyzed": normalized.len(),
            }),
        )
        .await;

        Ok(gaps)
    }

    /// Fill a specific gap
    pub async fn fill_gap(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
        priority: Priority,
    ) -> OhlcvResult<usize> {
        // DEBUG: Record gap fill start
        record_ohlcv_event(
            "gap_fill_start",
            Severity::Debug,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "start_timestamp": start_timestamp,
                "end_timestamp": end_timestamp,
                "priority": format!("{:?}", priority),
            }),
        )
        .await;

        // Always fetch 1m base data (API constraint)
        let data_1m = self
            .fetcher
            .fetch_historical(
                pool_address,
                Timeframe::Minute1,
                start_timestamp,
                end_timestamp,
            )
            .await?;

        if data_1m.is_empty() {
            return Ok(0);
        }

        // Store 1m data
        let inserted = self.db.insert_1m_data(mint, pool_address, &data_1m)?;

        // If higher timeframe, aggregate and cache
        if timeframe != Timeframe::Minute1 {
            if let Ok(aggregated) =
                crate::ohlcvs::aggregator::OhlcvAggregator::aggregate(&data_1m, timeframe)
            {
                let _ = self
                    .db
                    .cache_aggregated_data(mint, pool_address, timeframe, &aggregated);
            }
        }

        // Mark gap as filled if we got data
        if inserted > 0 {
            self.db.mark_gap_filled(
                mint,
                pool_address,
                timeframe,
                start_timestamp,
                end_timestamp,
            )?;

            // INFO: Record successful gap fill
            record_ohlcv_event(
                "gap_fill_complete",
                Severity::Info,
                Some(mint),
                Some(pool_address),
                json!({
                    "mint": mint,
                    "pool_address": pool_address,
                    "timeframe": timeframe.to_string(),
                    "start_timestamp": start_timestamp,
                    "end_timestamp": end_timestamp,
                    "data_points_inserted": inserted,
                }),
            )
            .await;
        }

        Ok(inserted)
    }

    /// Get unfilled gaps for a token
    pub async fn get_unfilled_gaps(
        &self,
        mint: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<Gap>> {
        let gap_tuples = self.db.get_unfilled_gaps(mint, timeframe)?;

        Ok(gap_tuples
            .into_iter()
            .map(|(pool_address, start, end)| Gap {
                mint: mint.to_string(),
                pool_address,
                timeframe,
                start_timestamp: start,
                end_timestamp: end,
            })
            .collect())
    }

    /// Fill all gaps for a token with priority-based strategy
    pub async fn fill_all_gaps(&self, mint: &str, priority: Priority) -> OhlcvResult<GapFillStats> {
        let mut stats = GapFillStats::default();

        // Get gaps for all timeframes
        for timeframe in Timeframe::all() {
            let gaps = self.get_unfilled_gaps(mint, timeframe).await?;
            stats.total_gaps += gaps.len();

            // Prioritize gaps
            let prioritized_gaps = self.prioritize_gaps(gaps);

            for gap in prioritized_gaps {
                // Add delay between requests to respect rate limits
                sleep(Duration::from_millis(500)).await;

                match self
                    .fill_gap(
                        &gap.mint,
                        &gap.pool_address,
                        gap.timeframe,
                        gap.start_timestamp,
                        gap.end_timestamp,
                        priority,
                    )
                    .await
                {
                    Ok(inserted) => {
                        stats.filled_gaps += 1;
                        stats.data_points_added += inserted;
                    }
                    Err(e) => {
                        stats.failed_gaps += 1;
                        logger::error(LogTag::Ohlcv, &format!("Failed to fill gap: {}", e));

                        // ERROR: Record gap fill failure
                        record_ohlcv_event(
                            "gap_fill_error",
                            Severity::Error,
                            Some(&gap.mint),
                            Some(&gap.pool_address),
                            json!({
                                "mint": gap.mint,
                                "pool_address": gap.pool_address,
                                "timeframe": gap.timeframe.to_string(),
                                "start_timestamp": gap.start_timestamp,
                                "end_timestamp": gap.end_timestamp,
                                "error": e.to_string(),
                            }),
                        )
                        .await;
                    }
                }
            }
        }

        // INFO: Record fill_all_gaps summary
        record_ohlcv_event(
            "fill_all_gaps_complete",
            Severity::Info,
            Some(mint),
            None,
            json!({
                "mint": mint,
                "total_gaps": stats.total_gaps,
                "filled_gaps": stats.filled_gaps,
                "failed_gaps": stats.failed_gaps,
                "data_points_added": stats.data_points_added,
            }),
        )
        .await;

        Ok(stats)
    }

    /// Prioritize gaps based on recency and importance
    fn prioritize_gaps(&self, mut gaps: Vec<Gap>) -> Vec<Gap> {
        gaps.sort_by(|a, b| {
            // More recent gaps first
            b.start_timestamp.cmp(&a.start_timestamp)
        });

        gaps
    }

    /// Check data quality and detect potential issues
    pub async fn check_data_quality(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<DataQualityReport> {
        let mut data = self
            .db
            .get_1m_data(mint, Some(pool_address), None, None, 10000)?;

        let total_candles = data.len();

        // Sort to ASC for accurate gap detection
        data.sort_by_key(|d| d.timestamp);

        let gaps = OhlcvAggregator::detect_gaps(&data, timeframe);
        let gap_count = gaps.len();

        let invalid_candles = data.iter().filter(|d| !d.is_valid()).count();

        // Calculate expected candles
        let expected = if let (Some(first), Some(last)) = (data.first(), data.last()) {
            OhlcvAggregator::expected_candles(first.timestamp, last.timestamp, timeframe)
        } else {
            0
        };

        let completeness = if expected > 0 {
            ((total_candles as f64) / (expected as f64)) * 100.0
        } else {
            100.0
        };

        Ok(DataQualityReport {
            total_candles,
            expected_candles: expected,
            gap_count,
            invalid_candles,
            completeness_percent: completeness,
            has_issues: gap_count > 0 || invalid_candles > 0,
        })
    }

    /// Auto-fill recent gaps (last 24 hours)
    pub async fn auto_fill_recent_gaps(&self, mint: &str) -> OhlcvResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let yesterday = now - 86400; // 24 hours ago

        let mut total_filled = 0;

        for timeframe in Timeframe::all() {
            let gaps = self.get_unfilled_gaps(mint, timeframe).await?;

            // Filter to recent gaps only
            let recent_gaps: Vec<Gap> = gaps
                .into_iter()
                .filter(|g| g.start_timestamp >= yesterday)
                .collect();

            for gap in recent_gaps {
                if let Ok(filled) = self
                    .fill_gap(
                        &gap.mint,
                        &gap.pool_address,
                        gap.timeframe,
                        gap.start_timestamp,
                        gap.end_timestamp,
                        Priority::High,
                    )
                    .await
                {
                    total_filled += filled;
                }

                // Rate limit protection
                sleep(Duration::from_millis(300)).await;
            }
        }

        Ok(total_filled)
    }

    /// Estimate time to fill all gaps
    pub async fn estimate_fill_time(&self, mint: &str) -> OhlcvResult<Duration> {
        let mut total_gaps = 0;

        for timeframe in Timeframe::all() {
            let gaps = self.get_unfilled_gaps(mint, timeframe).await?;
            total_gaps += gaps.len();
        }

        // Estimate: 2 seconds per gap (includes rate limiting)
        Ok(Duration::from_secs((total_gaps as u64) * 2))
    }
}

#[derive(Debug, Clone)]
pub struct Gap {
    pub mint: String,
    pub pool_address: String,
    pub timeframe: Timeframe,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

impl Gap {
    pub fn duration_seconds(&self) -> i64 {
        self.end_timestamp - self.start_timestamp
    }

    pub fn candle_count(&self) -> usize {
        let duration = self.duration_seconds();
        let candle_duration = self.timeframe.to_seconds();

        if candle_duration == 0 {
            return 0;
        }

        (duration / candle_duration) as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct GapFillStats {
    pub total_gaps: usize,
    pub filled_gaps: usize,
    pub failed_gaps: usize,
    pub data_points_added: usize,
}

#[derive(Debug, Clone)]
pub struct DataQualityReport {
    pub total_candles: usize,
    pub expected_candles: usize,
    pub gap_count: usize,
    pub invalid_candles: usize,
    pub completeness_percent: f64,
    pub has_issues: bool,
}
