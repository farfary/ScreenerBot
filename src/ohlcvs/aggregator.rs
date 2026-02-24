//! OHLCV aggregator — combines candles from multiple timeframes and sources.

use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use serde_json::json;
use std::collections::HashMap;

pub struct OhlcvAggregator;

impl OhlcvAggregator {
    /// Aggregate 1-minute data to a higher timeframe
    pub fn aggregate(
        data: &[Candle],
        from_timeframe: Timeframe,
        to_timeframe: Timeframe,
    ) -> OhlcvResult<Vec<Candle>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // 1-minute data doesn't need aggregation
        if to_timeframe == Timeframe::Minute1 {
            return Ok(data.to_vec());
        }

        let bucket_size = to_timeframe.to_seconds();

        // Group data points into buckets
        let mut buckets: HashMap<i64, Vec<&Candle>> = HashMap::new();

        for point in data {
            let bucket_start = (point.timestamp / bucket_size) * bucket_size;
            buckets.entry(bucket_start).or_default().push(point);
        }

        // Aggregate each bucket
        let mut aggregated: Vec<Candle> = buckets
            .into_iter()
            .filter_map(|(timestamp, points)| Self::aggregate_bucket(timestamp, &points))
            .collect();

        // Sort by timestamp
        aggregated.sort_by_key(|p| p.timestamp);

        // DEBUG: Record large aggregation operations
        if data.len() >= 1000 {
            let input_len = data.len();
            let output_len = aggregated.len();
            let to_timeframe_str = to_timeframe.to_string();
            tokio::spawn(async move {
                record_ohlcv_event(
                    "large_aggregation",
                    Severity::Debug,
                    None,
                    None,
                    json!({
                        "input_points": input_len,
                        "output_points": output_len,
                        "target_timeframe": to_timeframe_str,
                    }),
                )
                .await
            });
        }

        Ok(aggregated)
    }

    /// Aggregate multiple data points into a single candle
    fn aggregate_bucket(timestamp: i64, points: &[&Candle]) -> Option<Candle> {
        if points.is_empty() {
            return None;
        }

        // Sort points by timestamp within bucket
        let mut sorted_points = points.to_vec();
        sorted_points.sort_by_key(|p| p.timestamp);

        // OHLCV aggregation rules:
        // - Open: first candle's open
        // - High: maximum high
        // - Low: minimum low
        // - Close: last candle's close
        // - Volume: sum of all volumes

        let open = sorted_points.first()?.open;
        let close = sorted_points.last()?.close;
        let high = sorted_points
            .iter()
            .map(|p| p.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = sorted_points
            .iter()
            .map(|p| p.low)
            .fold(f64::INFINITY, f64::min);
        let volume: f64 = points.iter().map(|p| p.volume).sum();

        Some(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        })
    }

    /// Calculate expected candle count for a time range
    pub fn expected_candles(from_timestamp: i64, to_timestamp: i64, timeframe: Timeframe) -> usize {
        if to_timestamp < from_timestamp {
            return 0;
        }

        let duration = to_timestamp - from_timestamp;
        let candle_duration = timeframe.to_seconds();

        if candle_duration == 0 {
            return 0;
        }

        ((duration / candle_duration) as usize).saturating_add(1)
    }

    /// Check if data has gaps
    pub fn detect_gaps(data: &[Candle], timeframe: Timeframe) -> Vec<(i64, i64)> {
        if data.len() < 2 {
            return Vec::new();
        }
        // Ensure ascending order to avoid false gap detection
        let mut sorted = data.to_vec();
        sorted.sort_by_key(|p| p.timestamp);

        let mut gaps = Vec::new();
        let candle_duration = timeframe.to_seconds();

        for i in 1..sorted.len() {
            let prev_timestamp = sorted[i - 1].timestamp;
            let curr_timestamp = sorted[i].timestamp;
            let expected_next = prev_timestamp + candle_duration;

            if curr_timestamp > expected_next {
                // Gap detected
                gaps.push((expected_next, curr_timestamp - candle_duration));
            }
        }

        gaps
    }
}
