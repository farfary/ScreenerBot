// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure indicator math over candle series, shared by every condition that reads candles.
//!
//! Each function takes plain values in time order (oldest first) and answers nothing
//! rather than a fabricated number when the series is too short: the condition that asked
//! owns the `InsufficientCandles` error, because only it knows which indicator to name.

/// Simple moving average series of `values`; element `j` averages `values[j..j + period]`
/// and so belongs to index `j + period - 1`. Empty when `period` is zero or longer than
/// the series.
pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }
    let period_f = period as f64;
    values
        .windows(period)
        .map(|window| window.iter().sum::<f64>() / period_f)
        .collect()
}

/// Exponential moving average series of `values`, seeded with the simple average of the
/// first `period` values; element `j` belongs to index `j + period - 1`. Empty when
/// `period` is zero or longer than the series.
pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }
    let period_f = period as f64;
    let k = 2.0 / (period_f + 1.0);
    let mut average = values[..period].iter().sum::<f64>() / period_f;
    let mut series = Vec::with_capacity(values.len() - period + 1);
    series.push(average);
    for value in &values[period..] {
        average += k * (value - average);
        series.push(average);
    }
    series
}

/// Wilder's smoothing of `values`: the first `period` values seed a simple mean, every
/// later value is folded in as `(previous × (period − 1) + value) / period`. Returns the
/// final smoothed value, or `None` when `period` is zero or longer than the series.
pub fn wilder_smooth(values: &[f64], period: usize) -> Option<f64> {
    if period == 0 || values.len() < period {
        return None;
    }
    let period_f = period as f64;
    let seed = values[..period].iter().fold(0.0, |sum, value| sum + value) / period_f;
    Some(values[period..].iter().fold(seed, |smoothed, value| {
        (smoothed * (period_f - 1.0) + value) / period_f
    }))
}

/// Wilder's relative strength index of `closes`: the gains and losses of consecutive
/// closes, each smoothed with [`wilder_smooth`]. A series with neither gains nor losses
/// is neutral (50); one with gains and no losses is 100. `None` when there are fewer
/// than `period + 1` closes.
pub fn rsi(closes: &[f64], period: usize) -> Option<f64> {
    let changes: Vec<f64> = closes.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let gains: Vec<f64> = changes.iter().map(|change| change.max(0.0)).collect();
    let losses: Vec<f64> = changes.iter().map(|change| (-change).max(0.0)).collect();
    let gain = wilder_smooth(&gains, period)?;
    let loss = wilder_smooth(&losses, period)?;
    if loss == 0.0 {
        return Some(if gain == 0.0 { 50.0 } else { 100.0 });
    }
    Some(100.0 - 100.0 / (1.0 + gain / loss))
}

/// True range of one candle: the widest of its own range and its gaps from the previous
/// close, so a candle that opens away from the last one still counts the jump.
pub fn true_range(high: f64, low: f64, previous_close: f64) -> f64 {
    (high - low)
        .max((high - previous_close).abs())
        .max((low - previous_close).abs())
}

/// Wilder's average true range of a candle series given as parallel `highs`, `lows` and
/// `closes`: the true range of every candle after the first, smoothed with
/// [`wilder_smooth`]. `None` when the series differ in length or hold fewer than
/// `period + 1` candles.
pub fn atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64> {
    if highs.len() != lows.len() || highs.len() != closes.len() {
        return None;
    }
    let ranges: Vec<f64> = (1..closes.len())
        .map(|index| true_range(highs[index], lows[index], closes[index - 1]))
        .collect();
    wilder_smooth(&ranges, period)
}

/// Population standard deviation of `values` (divided by `n`, not `n − 1`): the series is
/// the whole window being described, not a sample of a larger one. `None` when empty.
pub fn stddev_population(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let count = values.len() as f64;
    let mean = values.iter().sum::<f64>() / count;
    let variance = values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .sum::<f64>()
        / count;
    Some(variance.sqrt())
}

/// Volume-weighted average of `prices`: `Σ(price × volume) / Σ volume`. `None` when the
/// series differ in length, are empty, or the total volume is not a positive finite
/// number, because a window with no traded volume has no volume-weighted price.
pub fn vwap(prices: &[f64], volumes: &[f64]) -> Option<f64> {
    if prices.len() != volumes.len() || prices.is_empty() {
        return None;
    }
    let total_volume: f64 = volumes.iter().sum();
    if !total_volume.is_finite() || total_volume <= 0.0 {
        return None;
    }
    let weighted: f64 = prices
        .iter()
        .zip(volumes)
        .map(|(price, volume)| price * volume)
        .sum();
    Some(weighted / total_volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded close series: a pump, a fade, a flat stretch, a zero print and a spike, so
    /// every branch of the averages and of the gain/loss split is exercised.
    const RECORDED: [&[f64]; 4] = [
        &[
            0.000_012_4,
            0.000_012_9,
            0.000_013_7,
            0.000_015_2,
            0.000_014_8,
            0.000_014_1,
            0.000_014_1,
            0.000_013_3,
            0.000_012_0,
            0.000_012_6,
            0.000_013_9,
            0.000_016_4,
            0.000_019_8,
            0.000_018_1,
            0.000_017_5,
            0.000_017_5,
            0.000_016_2,
            0.000_015_0,
            0.000_015_9,
            0.000_014_7,
            0.000_013_8,
            0.000_014_4,
            0.000_015_1,
            0.000_014_9,
            0.000_014_2,
        ],
        &[
            101.5, 102.25, 99.75, 98.0, 103.5, 107.25, 106.0, 104.5, 104.5, 109.75, 112.0, 108.25,
            103.0, 101.75, 100.0, 97.5, 99.25, 102.0, 105.5, 104.0, 106.75, 108.0,
        ],
        &[
            1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 2.5, 2.5, 2.25, 2.0, 1.75, 1.75, 1.5, 3.0,
        ],
        &[
            0.42, 0.41, 0.40, 0.39, 0.38, 0.37, 0.36, 0.35, 0.34, 0.33, 0.32, 0.31, 0.30, 0.29,
            0.28, 0.27,
        ],
    ];

    /// The moving average the cross condition computed before the shared math existed,
    /// kept verbatim as the reference the shared functions must reproduce bit for bit.
    fn reference_moving_average(closes: &[f64], period: usize, exponential: bool) -> Vec<f64> {
        let period_f = period as f64;
        if !exponential {
            return closes
                .windows(period)
                .map(|window| window.iter().sum::<f64>() / period_f)
                .collect();
        }
        let k = 2.0 / (period_f + 1.0);
        let mut average = closes[..period].iter().sum::<f64>() / period_f;
        let mut series = vec![average];
        for close in &closes[period..] {
            average += k * (close - average);
            series.push(average);
        }
        series
    }

    /// The moving average the price-to-MA condition computed inline: the mean of the last
    /// `period` closes.
    fn reference_latest_sma(closes: &[f64], period: usize) -> f64 {
        closes[closes.len() - period..].iter().sum::<f64>() / period as f64
    }

    /// The RSI the RSI condition computed before the shared math existed.
    fn reference_wilder_rsi(closes: &[f64], period: usize) -> f64 {
        let period_f = period as f64;
        let mut changes = closes.windows(2).map(|pair| pair[1] - pair[0]);
        let (mut gain, mut loss) = changes
            .by_ref()
            .take(period)
            .fold((0.0, 0.0), |(gain, loss), change: f64| {
                (gain + change.max(0.0), loss + (-change).max(0.0))
            });
        gain /= period_f;
        loss /= period_f;
        for change in changes {
            gain = (gain * (period_f - 1.0) + change.max(0.0)) / period_f;
            loss = (loss * (period_f - 1.0) + (-change).max(0.0)) / period_f;
        }
        if loss == 0.0 {
            return if gain == 0.0 { 50.0 } else { 100.0 };
        }
        100.0 - 100.0 / (1.0 + gain / loss)
    }

    fn bits(series: &[f64]) -> Vec<u64> {
        series.iter().map(|value| value.to_bits()).collect()
    }

    #[test]
    fn sma_and_ema_reproduce_the_cross_condition_averages() {
        for closes in RECORDED {
            for period in 1..=closes.len() {
                assert_eq!(
                    bits(&sma(closes, period)),
                    bits(&reference_moving_average(closes, period, false)),
                    "sma period {period}"
                );
                assert_eq!(
                    bits(&ema(closes, period)),
                    bits(&reference_moving_average(closes, period, true)),
                    "ema period {period}"
                );
            }
        }
    }

    #[test]
    fn the_last_sma_reproduces_the_price_to_ma_average() {
        for closes in RECORDED {
            for period in 1..=closes.len() {
                let latest = sma(&closes[closes.len() - period..], period);
                assert_eq!(latest.len(), 1);
                assert_eq!(
                    latest[0].to_bits(),
                    reference_latest_sma(closes, period).to_bits(),
                    "period {period}"
                );
            }
        }
    }

    #[test]
    fn rsi_reproduces_the_rsi_condition() {
        for closes in RECORDED {
            for period in 1..closes.len() {
                let shared = rsi(closes, period).expect("period + 1 closes");
                let reference = reference_wilder_rsi(closes, period);
                assert_eq!(shared.to_bits(), reference.to_bits(), "period {period}");
            }
        }
    }

    #[test]
    fn a_series_too_short_answers_nothing() {
        let closes = [1.0, 2.0, 3.0];
        assert!(sma(&closes, 4).is_empty());
        assert!(sma(&closes, 0).is_empty());
        assert!(ema(&closes, 4).is_empty());
        assert!(ema(&closes, 0).is_empty());
        assert_eq!(wilder_smooth(&closes, 4), None);
        assert_eq!(wilder_smooth(&closes, 0), None);
        assert_eq!(rsi(&closes, 3), None);
        assert!(rsi(&closes, 2).is_some());
    }

    #[test]
    fn true_range_counts_the_gap_from_the_previous_close() {
        assert_eq!(true_range(12.0, 10.0, 11.0), 2.0);
        // Gap up: the candle range is 1, the jump from the last close is 5.
        assert_eq!(true_range(15.0, 14.0, 10.0), 5.0);
        // Gap down mirrors it.
        assert_eq!(true_range(6.0, 5.0, 10.0), 5.0);
    }

    #[test]
    fn atr_is_the_wilder_smoothed_true_range() {
        let highs = [10.0, 12.0, 13.0, 15.0, 14.0];
        let lows = [9.0, 10.0, 11.0, 12.0, 11.0];
        let closes = [9.0, 11.0, 12.5, 14.0, 12.0];
        let ranges = [3.0, 2.0, 3.0, 3.0];
        for (index, expected) in ranges.iter().enumerate() {
            let at = index + 1;
            assert_eq!(
                true_range(highs[at], lows[at], closes[at - 1]),
                *expected,
                "candle {at}"
            );
        }
        assert_eq!(
            atr(&highs, &lows, &closes, 2),
            wilder_smooth(&ranges, 2),
            "ATR shares the RSI smoothing"
        );
        // Seed (3 + 2) / 2 = 2.5, then (2.5 + 3) / 2 = 2.75, then (2.75 + 3) / 2.
        assert_eq!(atr(&highs, &lows, &closes, 2), Some(2.875));
        assert_eq!(atr(&highs, &lows, &closes, 4), Some(2.75));
        assert_eq!(
            atr(&highs, &lows, &closes, 5),
            None,
            "needs period + 1 candles"
        );
        assert_eq!(
            atr(&highs[..4], &lows, &closes, 2),
            None,
            "parallel series only"
        );
    }

    #[test]
    fn stddev_divides_by_the_whole_window() {
        assert_eq!(
            stddev_population(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
            Some(2.0)
        );
        assert_eq!(stddev_population(&[3.0, 3.0, 3.0]), Some(0.0));
        assert_eq!(stddev_population(&[]), None);
    }

    #[test]
    fn vwap_weights_each_price_by_its_volume() {
        assert_eq!(vwap(&[10.0, 20.0], &[3.0, 1.0]), Some(12.5));
        assert_eq!(vwap(&[10.0, 20.0], &[1.0, 1.0]), Some(15.0));
        assert_eq!(vwap(&[10.0, 20.0], &[0.0, 0.0]), None, "no volume");
        assert_eq!(vwap(&[10.0], &[1.0, 1.0]), None, "parallel series only");
        assert_eq!(vwap(&[], &[]), None);
    }

    #[test]
    fn known_values() {
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0], 2), vec![1.5, 2.5, 3.5]);
        assert_eq!(ema(&[2.0, 4.0, 6.0], 2), vec![3.0, 3.0 + 2.0 / 3.0 * 3.0]);
        assert_eq!(wilder_smooth(&[2.0, 4.0, 8.0], 2), Some((3.0 + 8.0) / 2.0));
        assert_eq!(rsi(&[5.0, 5.0, 5.0, 5.0], 3), Some(50.0));
        assert_eq!(rsi(&[1.0, 2.0, 3.0, 4.0], 3), Some(100.0));
        assert_eq!(rsi(&[4.0, 3.0, 2.0, 1.0], 3), Some(0.0));
    }
}
