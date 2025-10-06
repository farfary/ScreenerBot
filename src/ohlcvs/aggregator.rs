// Timeframe aggregation logic

use crate::ohlcvs::types::{ OhlcvDataPoint, OhlcvError, OhlcvResult, Timeframe };
use std::collections::HashMap;

pub struct OhlcvAggregator;

impl OhlcvAggregator {
    /// Aggregate 1-minute data to a higher timeframe
    pub fn aggregate(
        data: &[OhlcvDataPoint],
        target_timeframe: Timeframe
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // 1-minute data doesn't need aggregation
        if target_timeframe == Timeframe::Minute1 {
            return Ok(data.to_vec());
        }

        let bucket_size = target_timeframe.to_seconds();

        // Group data points by bucket
        let mut buckets: HashMap<i64, Vec<&OhlcvDataPoint>> = HashMap::new();

        for point in data {
            let bucket_start = (point.timestamp / bucket_size) * bucket_size;
            buckets.entry(bucket_start).or_default().push(point);
        }

        // Aggregate each bucket
        let mut aggregated: Vec<OhlcvDataPoint> = buckets
            .into_iter()
            .filter_map(|(timestamp, points)| Self::aggregate_bucket(timestamp, &points))
            .collect();

        // Sort by timestamp
        aggregated.sort_by_key(|p| p.timestamp);

        Ok(aggregated)
    }

    /// Aggregate multiple data points into a single candle
    fn aggregate_bucket(timestamp: i64, points: &[&OhlcvDataPoint]) -> Option<OhlcvDataPoint> {
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
        let volume: f64 = sorted_points
            .iter()
            .map(|p| p.volume)
            .sum();

        Some(OhlcvDataPoint {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        })
    }

    /// Validate aggregated data
    pub fn validate_aggregated(data: &[OhlcvDataPoint]) -> bool {
        data.iter().all(|point| point.is_valid())
    }

    /// Calculate expected candle count for a time range
    pub fn expected_candles(from_timestamp: i64, to_timestamp: i64, timeframe: Timeframe) -> usize {
        let duration = to_timestamp - from_timestamp;
        let candle_duration = timeframe.to_seconds();

        if candle_duration == 0 {
            return 0;
        }

        (duration / candle_duration) as usize
    }

    /// Check if data has gaps
    pub fn detect_gaps(data: &[OhlcvDataPoint], timeframe: Timeframe) -> Vec<(i64, i64)> {
        if data.len() < 2 {
            return Vec::new();
        }

        let mut gaps = Vec::new();
        let candle_duration = timeframe.to_seconds();

        for i in 1..data.len() {
            let prev_timestamp = data[i - 1].timestamp;
            let curr_timestamp = data[i].timestamp;
            let expected_next = prev_timestamp + candle_duration;

            if curr_timestamp > expected_next {
                // Gap detected
                gaps.push((expected_next, curr_timestamp - candle_duration));
            }
        }

        gaps
    }

    /// Interpolate missing candles (simple forward fill)
    pub fn interpolate_gaps(data: &[OhlcvDataPoint], timeframe: Timeframe) -> Vec<OhlcvDataPoint> {
        if data.len() < 2 {
            return data.to_vec();
        }

        let mut result = Vec::new();
        let candle_duration = timeframe.to_seconds();

        for i in 0..data.len() {
            result.push(data[i].clone());

            if i < data.len() - 1 {
                let curr_timestamp = data[i].timestamp;
                let next_timestamp = data[i + 1].timestamp;
                let expected_next = curr_timestamp + candle_duration;

                // Fill gaps with forward-filled data
                let mut fill_timestamp = expected_next;
                while fill_timestamp < next_timestamp {
                    result.push(OhlcvDataPoint {
                        timestamp: fill_timestamp,
                        open: data[i].close,
                        high: data[i].close,
                        low: data[i].close,
                        close: data[i].close,
                        volume: 0.0,
                    });

                    fill_timestamp += candle_duration;
                }
            }
        }

        result
    }

    /// Resample data to a different timeframe (downsample only)
    pub fn resample(
        data: &[OhlcvDataPoint],
        from_timeframe: Timeframe,
        to_timeframe: Timeframe
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        // Can only downsample (smaller -> larger timeframe)
        if to_timeframe.to_seconds() < from_timeframe.to_seconds() {
            return Err(
                OhlcvError::InvalidTimeframe(
                    "Cannot upsample data, only downsample supported".to_string()
                )
            );
        }

        if from_timeframe == to_timeframe {
            return Ok(data.to_vec());
        }

        Self::aggregate(data, to_timeframe)
    }

    /// Calculate volume-weighted average price (VWAP) for a bucket
    pub fn calculate_vwap(data: &[OhlcvDataPoint]) -> Option<f64> {
        if data.is_empty() {
            return None;
        }

        let total_volume: f64 = data
            .iter()
            .map(|p| p.volume)
            .sum();
        if total_volume == 0.0 {
            return None;
        }

        let vwap: f64 =
            data
                .iter()
                .map(|p| {
                    let typical_price = (p.high + p.low + p.close) / 3.0;
                    typical_price * p.volume
                })
                .sum::<f64>() / total_volume;

        Some(vwap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_1m_to_5m() {
        let data = vec![
            OhlcvDataPoint::new(0, 100.0, 105.0, 95.0, 102.0, 1000.0),
            OhlcvDataPoint::new(60, 102.0, 110.0, 100.0, 108.0, 1500.0),
            OhlcvDataPoint::new(120, 108.0, 115.0, 105.0, 110.0, 2000.0),
            OhlcvDataPoint::new(180, 110.0, 120.0, 108.0, 115.0, 2500.0),
            OhlcvDataPoint::new(240, 115.0, 118.0, 112.0, 116.0, 1800.0)
        ];

        let aggregated = OhlcvAggregator::aggregate(&data, Timeframe::Minute5).unwrap();

        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated[0].timestamp, 0);
        assert_eq!(aggregated[0].open, 100.0);
        assert_eq!(aggregated[0].close, 116.0);
        assert_eq!(aggregated[0].high, 120.0);
        assert_eq!(aggregated[0].low, 95.0);
        assert_eq!(aggregated[0].volume, 8800.0);
    }

    #[test]
    fn test_gap_detection() {
        let data = vec![
            OhlcvDataPoint::new(0, 100.0, 105.0, 95.0, 102.0, 1000.0),
            OhlcvDataPoint::new(60, 102.0, 110.0, 100.0, 108.0, 1500.0),
            // Gap here - missing 120
            OhlcvDataPoint::new(180, 110.0, 120.0, 108.0, 115.0, 2500.0)
        ];

        let gaps = OhlcvAggregator::detect_gaps(&data, Timeframe::Minute1);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0], (120, 120));
    }

    #[test]
    fn test_interpolation() {
        let data = vec![
            OhlcvDataPoint::new(0, 100.0, 105.0, 95.0, 102.0, 1000.0),
            // Gap
            OhlcvDataPoint::new(180, 110.0, 120.0, 108.0, 115.0, 2500.0)
        ];

        let interpolated = OhlcvAggregator::interpolate_gaps(&data, Timeframe::Minute1);

        assert_eq!(interpolated.len(), 4); // Original 2 + 2 filled
        assert_eq!(interpolated[1].timestamp, 60);
        assert_eq!(interpolated[1].close, 102.0); // Forward filled
        assert_eq!(interpolated[1].volume, 0.0); // No volume for filled
    }

    #[test]
    fn test_vwap_calculation() {
        let data = vec![
            OhlcvDataPoint::new(0, 100.0, 105.0, 95.0, 100.0, 1000.0),
            OhlcvDataPoint::new(60, 100.0, 110.0, 100.0, 110.0, 2000.0)
        ];

        let vwap = OhlcvAggregator::calculate_vwap(&data).unwrap();

        // Typical prices: (105+95+100)/3 = 100, (110+100+110)/3 = 106.67
        // VWAP = (100 * 1000 + 106.67 * 2000) / 3000 = 104.44
        assert!((vwap - 104.44).abs() < 0.5);
    }
}
