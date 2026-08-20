//! OHLCV aggregator — combines candles from multiple timeframes and sources.

use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use serde_json::json;
use std::collections::HashMap;

pub struct OhlcvAggregator;

impl OhlcvAggregator {
    /// Combine a finer series into a coarser one.
    ///
    /// `from_timeframe` declares what `data` actually holds and is enforced: a request to
    /// synthesize finer candles from coarser ones is an error rather than a silently
    /// mis-bucketed series, and an equal pair returns the input (de-duplicated) untouched.
    ///
    /// Bucket boundaries come from the TARGET timeframe and the canonical UTC floor
    /// (`(ts / bucket) * bucket`), never from the first candle seen, so two batches of the
    /// same token always land on the same grid.
    ///
    /// The newest bucket is emitted as an ordinary candle even when it is still forming —
    /// deliberate, because the chart and the indicators both want the in-progress bar —
    /// so a caller that needs only closed candles must drop the last one itself.
    pub fn aggregate(
        data: &[Candle],
        from_timeframe: Timeframe,
        to_timeframe: Timeframe,
    ) -> OhlcvResult<Vec<Candle>> {
        if from_timeframe.to_seconds() > to_timeframe.to_seconds() {
            return Err(OhlcvError::InvalidTimeframe(format!(
                "cannot aggregate {from_timeframe} candles into finer {to_timeframe} candles"
            )));
        }

        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Already at the requested granularity - nothing to combine.
        if from_timeframe == to_timeframe {
            return Ok(Self::one_candle_per_timestamp(data));
        }

        let bucket_size = to_timeframe.to_seconds();

        // Group data points into buckets
        let mut buckets: HashMap<i64, Vec<Candle>> = HashMap::new();

        for point in Self::one_candle_per_timestamp(data) {
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

    /// One candle per source timestamp, ascending, latest write winning.
    ///
    /// A repeated timestamp is the same period delivered twice (two providers, or a
    /// retried write). Summed into a bucket it doubles that bucket's volume while leaving
    /// its prices untouched — exactly the shape a VolumeSpike entry buys on — so the
    /// duplicate is dropped here rather than trusted to hold in every caller.
    fn one_candle_per_timestamp(data: &[Candle]) -> Vec<Candle> {
        let mut sorted = data.to_vec();
        sorted.sort_by_key(|c| c.timestamp);

        let mut unique: Vec<Candle> = Vec::with_capacity(sorted.len());
        for candle in sorted {
            match unique.last_mut() {
                Some(last) if last.timestamp == candle.timestamp => *last = candle,
                _ => unique.push(candle),
            }
        }

        unique
    }

    /// Aggregate multiple data points into a single candle
    fn aggregate_bucket(timestamp: i64, points: &[Candle]) -> Option<Candle> {
        // OHLCV aggregation rules:
        // - Open: first candle's open
        // - High: maximum high
        // - Low: minimum low
        // - Close: last candle's close
        // - Volume: sum of all volumes
        //
        // `points` is ascending and duplicate-free: its only builder is `aggregate`,
        // which fills the buckets from `one_candle_per_timestamp`.
        let open = points.first()?.open;
        let close = points.last()?.close;
        let high = points
            .iter()
            .map(|p| p.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = points.iter().map(|p| p.low).fold(f64::INFINITY, f64::min);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A timestamp divisible by every timeframe bucket up to 1d, so a fixture's bucket
    /// boundaries are obvious by inspection.
    const BASE: i64 = 1_800_000_000 - (1_800_000_000 % 86_400);

    fn c(offset_minutes: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle::new(BASE + offset_minutes * 60, open, high, low, close, volume)
    }

    /// Ten one-minute candles: closes walk 100 -> 109, one distinctive high and low per
    /// five-minute bucket, one unit of volume each.
    fn ten_minutes() -> Vec<Candle> {
        (0..10)
            .map(|i| {
                let base = 100.0 + i as f64;
                c(i, base, base + 5.0, base - 5.0, base + 0.5, 1.0)
            })
            .collect()
    }

    #[test]
    fn a_bucket_takes_its_open_from_the_first_candle_and_its_close_from_the_last() {
        // The rule every higher timeframe depends on. Getting `open`/`close` from the
        // wrong end silently inverts candle colour, which is what `ConsecutiveCandles`
        // and `CandleSize` decide on.
        let out =
            OhlcvAggregator::aggregate(&ten_minutes(), Timeframe::Minute1, Timeframe::Minute5)
                .expect("aggregation succeeds");

        assert_eq!(
            out.len(),
            2,
            "ten one-minute candles are two five-minute ones"
        );

        assert_eq!(out[0].timestamp, BASE, "buckets are stamped at their floor");
        assert_eq!(out[0].open, 100.0); // first candle's open
        assert_eq!(out[0].close, 104.5); // last candle's close
        assert_eq!(out[0].high, 109.0); // max high in the bucket
        assert_eq!(out[0].low, 95.0); // min low in the bucket
        assert_eq!(out[0].volume, 5.0); // sum, never an average

        assert_eq!(out[1].timestamp, BASE + 300);
        assert_eq!(out[1].open, 105.0);
        assert_eq!(out[1].close, 109.5);
    }

    #[test]
    fn buckets_are_floored_to_the_utc_grid_not_to_the_first_candle_seen() {
        // Anchoring on the first candle instead of the UTC grid is what once interleaved
        // a 12h series from two providers. A batch that starts mid-bucket must still land
        // on the canonical boundary, and must not merge two grid buckets into one.
        let mid_bucket: Vec<Candle> = (3..8).map(|i| c(i, 1.0, 1.0, 1.0, 1.0, 1.0)).collect();
        let out = OhlcvAggregator::aggregate(&mid_bucket, Timeframe::Minute1, Timeframe::Minute5)
            .expect("aggregation succeeds");

        assert_eq!(
            out.iter().map(|k| k.timestamp).collect::<Vec<_>>(),
            vec![BASE, BASE + 300]
        );
        assert_eq!(out[0].volume, 2.0, "minutes 3-4 belong to the first bucket");
        assert_eq!(out[1].volume, 3.0, "minutes 5-7 belong to the second");
    }

    #[test]
    fn a_partial_bucket_is_emitted_as_though_it_were_finished() {
        // The newest bucket is always incomplete in live data — the current five-minute
        // candle is two minutes old. Aggregation emits it anyway, with no marker: the
        // chart and the indicators both want the forming bar, so a caller that needs
        // only closed candles has to drop the last one itself.
        let two_minutes: Vec<Candle> = (0..2).map(|i| c(i, 10.0, 10.0, 10.0, 10.0, 1.0)).collect();
        let out = OhlcvAggregator::aggregate(&two_minutes, Timeframe::Minute1, Timeframe::Minute5)
            .expect("aggregation succeeds");

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].volume, 2.0, "two minutes of a five-minute candle");
    }

    #[test]
    fn an_unordered_batch_aggregates_to_the_same_series_as_a_sorted_one() {
        // Database reads are ordered, but gap fills and merges are not. Order must not
        // change a single value, or the same history would produce two different charts.
        let sorted = ten_minutes();
        let mut shuffled = sorted.clone();
        shuffled.reverse();
        shuffled.swap(0, 4);

        let from_sorted =
            OhlcvAggregator::aggregate(&sorted, Timeframe::Minute1, Timeframe::Minute5).unwrap();
        let from_shuffled =
            OhlcvAggregator::aggregate(&shuffled, Timeframe::Minute1, Timeframe::Minute5).unwrap();

        for (a, b) in from_sorted.iter().zip(from_shuffled.iter()) {
            assert_eq!(
                (a.timestamp, a.open, a.high, a.low, a.close, a.volume),
                (b.timestamp, b.open, b.high, b.low, b.close, b.volume)
            );
        }
        assert_eq!(from_sorted.len(), from_shuffled.len());
    }

    #[test]
    fn a_repeated_timestamp_is_one_candle_not_two_helpings_of_volume() {
        // A duplicate row is the same minute delivered twice. Summed into its bucket it
        // inflates the volume while the price stays put, which is precisely the shape
        // `VolumeSpike` treats as a signal to buy — so the last write wins and the
        // repeat contributes nothing.
        let mut with_duplicate = ten_minutes();
        with_duplicate.push(with_duplicate[0].clone());

        let out =
            OhlcvAggregator::aggregate(&with_duplicate, Timeframe::Minute1, Timeframe::Minute5)
                .unwrap();

        assert_eq!(out.len(), 2, "and it creates no extra candle");
        assert_eq!(out[0].volume, 5.0, "the repeated minute is counted once");
    }

    #[test]
    fn a_later_write_of_the_same_minute_replaces_the_earlier_one() {
        // Two providers disagreeing about one minute must not average or double: batch
        // order decides, and the value that arrived last is the one kept.
        let mut revised = ten_minutes();
        revised.push(c(0, 100.0, 200.0, 50.0, 150.0, 9.0));

        let out =
            OhlcvAggregator::aggregate(&revised, Timeframe::Minute1, Timeframe::Minute5).unwrap();

        assert_eq!(out[0].high, 200.0, "the revision's high is used");
        assert_eq!(out[0].volume, 13.0, "9.0 replaces the original 1.0");
    }

    #[test]
    fn a_target_finer_than_the_source_is_refused() {
        // Hourly candles cannot be cut into minutes. This used to return the input
        // unchanged whenever the TARGET was 1m, so a caller could "normalise" unknown
        // data into a series labelled 1m that held nothing of the sort.
        let hourly = vec![
            Candle::new(BASE, 1.0, 2.0, 0.5, 1.5, 10.0),
            Candle::new(BASE + 3_600, 1.5, 3.0, 1.0, 2.5, 20.0),
        ];

        let err = OhlcvAggregator::aggregate(&hourly, Timeframe::Hour1, Timeframe::Minute1)
            .expect_err("a finer target is not aggregatable");
        assert!(
            matches!(err, OhlcvError::InvalidTimeframe(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn a_source_label_that_contradicts_the_target_is_an_error_not_a_silent_answer() {
        // `from_timeframe` declares what the input holds and is now enforced against the
        // target. It used to be accepted and never read, so a caller that mislabelled its
        // source got no error and no different answer.
        let data = ten_minutes();
        OhlcvAggregator::aggregate(&data, Timeframe::Minute1, Timeframe::Minute5)
            .expect("1m into 5m is the ordinary case");
        assert!(
            OhlcvAggregator::aggregate(&data, Timeframe::Day1, Timeframe::Minute5).is_err(),
            "daily candles cannot produce five-minute ones"
        );
    }

    #[test]
    fn an_equal_pair_returns_the_series_at_its_own_granularity() {
        // The no-op path: same timeframe in and out, still ordered and de-duplicated.
        let mut hourly = vec![
            Candle::new(BASE + 3_600, 1.5, 3.0, 1.0, 2.5, 20.0),
            Candle::new(BASE, 1.0, 2.0, 0.5, 1.5, 10.0),
            Candle::new(BASE, 1.0, 2.0, 0.5, 1.5, 11.0),
        ];
        let out = OhlcvAggregator::aggregate(&hourly, Timeframe::Hour1, Timeframe::Hour1).unwrap();

        assert_eq!(
            out.iter().map(|k| k.timestamp).collect::<Vec<_>>(),
            vec![BASE, BASE + 3_600]
        );
        assert_eq!(out[0].volume, 11.0, "latest write wins");

        hourly.clear();
        assert!(
            OhlcvAggregator::aggregate(&hourly, Timeframe::Hour1, Timeframe::Hour1)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn an_empty_batch_aggregates_to_an_empty_series() {
        assert!(
            OhlcvAggregator::aggregate(&[], Timeframe::Minute1, Timeframe::Hour1)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn expected_candles_counts_both_ends_of_the_range() {
        // Used to score data completeness, so an off-by-one here reports healthy series
        // as gappy (or the reverse) and drives needless refetching.
        assert_eq!(
            OhlcvAggregator::expected_candles(BASE, BASE, Timeframe::Minute1),
            1
        );
        assert_eq!(
            OhlcvAggregator::expected_candles(BASE, BASE + 60, Timeframe::Minute1),
            2
        );
        assert_eq!(
            OhlcvAggregator::expected_candles(BASE, BASE + 3_600, Timeframe::Minute5),
            13
        );
        // A partial trailing interval is not counted as a whole candle.
        assert_eq!(
            OhlcvAggregator::expected_candles(BASE, BASE + 359, Timeframe::Minute5),
            2
        );
        // A reversed range is nonsense, not a negative count.
        assert_eq!(
            OhlcvAggregator::expected_candles(BASE + 60, BASE, Timeframe::Minute1),
            0
        );
    }

    #[test]
    fn a_gap_is_reported_as_the_missing_span_between_two_known_candles() {
        // The pair returned is the FIRST and LAST missing candle, inclusive — it is fed
        // straight to the gap filler as a fetch range, so an off-by-one either refetches
        // a candle already held or leaves a hole that can never close.
        let data = vec![
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 180, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        assert_eq!(
            OhlcvAggregator::detect_gaps(&data, Timeframe::Minute1),
            vec![(BASE + 60, BASE + 120)]
        );
    }

    #[test]
    fn consecutive_candles_and_repeated_timestamps_are_not_gaps() {
        let contiguous = vec![
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 60, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 120, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        assert!(OhlcvAggregator::detect_gaps(&contiguous, Timeframe::Minute1).is_empty());

        let repeated = vec![
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        assert!(
            OhlcvAggregator::detect_gaps(&repeated, Timeframe::Minute1).is_empty(),
            "a duplicate is a storage problem, never a hole to backfill"
        );
    }

    #[test]
    fn gap_detection_sorts_before_comparing_so_an_unordered_batch_invents_no_holes() {
        let mut data = vec![
            Candle::new(BASE + 120, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 60, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        assert!(OhlcvAggregator::detect_gaps(&data, Timeframe::Minute1).is_empty());

        // And a single candle cannot describe a gap at all.
        data.truncate(1);
        assert!(OhlcvAggregator::detect_gaps(&data, Timeframe::Minute1).is_empty());
    }

    #[test]
    fn every_gap_in_a_series_is_reported_not_just_the_first() {
        let data = vec![
            Candle::new(BASE, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 180, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 240, 1.0, 1.0, 1.0, 1.0, 1.0),
            Candle::new(BASE + 600, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        assert_eq!(
            OhlcvAggregator::detect_gaps(&data, Timeframe::Minute1),
            vec![(BASE + 60, BASE + 120), (BASE + 300, BASE + 540)]
        );
    }
}
