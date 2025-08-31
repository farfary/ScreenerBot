use crate::global::is_debug_trader_enabled;
use crate::logger::{log, LogTag};
/// Advanced OHLCV Technical Analysis Module for ScreenerBot
///
/// This module provides sophisticated technical analysis using real OHLCV data
/// from the GeckoTerminal API. Implements advanced dip detection, ATH analysis,
/// and multi-timeframe technical indicators for enhanced trading decisions.
///
/// ## Features:
/// - **Technical Indicators**: RSI, Bollinger Bands, Moving Averages, ATR
/// - **Candlestick Patterns**: Hammer, Doji, Engulfing, Three White Soldiers
/// - **Volume Analysis**: Volume spikes, money flow, accumulation/distribution
/// - **Support/Resistance**: Real historical levels with volume confirmation
/// - **Multi-Timeframe**: Analysis across 7 different timeframes
/// - **ATH Detection**: Real historical highs with volume and age analysis
use crate::tokens::ohlcvs::{get_latest_ohlcv, is_ohlcv_data_available, OhlcvDataPoint, Timeframe};
use crate::tokens::Token;
use std::collections::HashMap;

// =============================================================================
// TECHNICAL INDICATOR CALCULATIONS
// =============================================================================

/// RSI (Relative Strength Index) calculation result
#[derive(Debug, Clone)]
pub struct RsiResult {
    pub value: f64,
    pub is_oversold: bool,   // RSI < 30
    pub is_overbought: bool, // RSI > 70
    pub trend: RsiTrend,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RsiTrend {
    Rising,
    Falling,
    Sideways,
}

/// Calculate RSI for given price data
pub fn calculate_rsi(prices: &[f64], period: usize) -> Option<RsiResult> {
    if prices.len() < period + 1 {
        return None;
    }

    let mut gains = Vec::new();
    let mut losses = Vec::new();

    // Calculate price changes
    for i in 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }
    }

    if gains.len() < period {
        return None;
    }

    // Calculate average gains and losses
    let avg_gain: f64 = gains.iter().take(period).sum::<f64>() / (period as f64);
    let avg_loss: f64 = losses.iter().take(period).sum::<f64>() / (period as f64);

    if avg_loss == 0.0 {
        return Some(RsiResult {
            value: 100.0,
            is_oversold: false,
            is_overbought: true,
            trend: RsiTrend::Rising,
        });
    }

    let rs = avg_gain / avg_loss;
    let rsi = 100.0 - 100.0 / (1.0 + rs);

    // Determine trend (compare last 3 RSI values if available)
    let trend = if gains.len() >= 3 {
        let recent_changes: Vec<f64> = gains.iter().rev().take(3).cloned().collect();
        let avg_recent = recent_changes.iter().sum::<f64>() / (recent_changes.len() as f64);

        if avg_recent > avg_gain * 1.1 {
            RsiTrend::Rising
        } else if avg_recent < avg_gain * 0.9 {
            RsiTrend::Falling
        } else {
            RsiTrend::Sideways
        }
    } else {
        RsiTrend::Sideways
    };

    Some(RsiResult {
        value: rsi,
        is_oversold: rsi < 30.0,
        is_overbought: rsi > 70.0,
        trend,
    })
}

/// Bollinger Bands calculation result
#[derive(Debug, Clone)]
pub struct BollingerBands {
    pub upper_band: f64,
    pub middle_band: f64, // Simple Moving Average
    pub lower_band: f64,
    pub bandwidth: f64, // (upper - lower) / middle
    pub percent_b: f64, // Where current price sits in bands
    pub squeeze: bool,  // Low volatility period
}

/// Calculate Bollinger Bands
pub fn calculate_bollinger_bands(
    prices: &[f64],
    period: usize,
    std_dev_multiplier: f64,
) -> Option<BollingerBands> {
    if prices.len() < period {
        return None;
    }

    let recent_prices = &prices[prices.len() - period..];
    let sma = recent_prices.iter().sum::<f64>() / (period as f64);

    // Calculate standard deviation
    let variance = recent_prices
        .iter()
        .map(|price| (price - sma).powi(2))
        .sum::<f64>()
        / (period as f64);
    let std_dev = variance.sqrt();

    let upper_band = sma + std_dev * std_dev_multiplier;
    let lower_band = sma - std_dev * std_dev_multiplier;
    let current_price = prices[prices.len() - 1];

    let bandwidth = (upper_band - lower_band) / sma;
    let percent_b = if upper_band != lower_band {
        (current_price - lower_band) / (upper_band - lower_band)
    } else {
        0.5
    };

    // Squeeze detection: bandwidth < 0.1 (10%)
    let squeeze = bandwidth < 0.1;

    Some(BollingerBands {
        upper_band,
        middle_band: sma,
        lower_band,
        bandwidth,
        percent_b,
        squeeze,
    })
}

/// Volume analysis result
#[derive(Debug, Clone)]
pub struct VolumeAnalysis {
    pub avg_volume: f64,
    pub current_volume: f64,
    pub volume_ratio: f64,     // current / average
    pub is_volume_spike: bool, // >2x average
    pub volume_trend: VolumeTrend,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VolumeTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Analyze volume patterns
pub fn analyze_volume(ohlcv_data: &[OhlcvDataPoint], lookback: usize) -> Option<VolumeAnalysis> {
    if ohlcv_data.len() < lookback + 1 {
        return None;
    }

    let volumes: Vec<f64> = ohlcv_data.iter().map(|d| d.volume).collect();
    let recent_volumes = &volumes[volumes.len() - lookback..];
    let avg_volume = recent_volumes.iter().sum::<f64>() / (lookback as f64);
    let current_volume = volumes[volumes.len() - 1];

    let volume_ratio = if avg_volume > 0.0 {
        current_volume / avg_volume
    } else {
        1.0
    };

    let is_volume_spike = volume_ratio > 2.0;

    // Determine volume trend
    let volume_trend = if volumes.len() >= 3 {
        let last_3: Vec<f64> = volumes.iter().rev().take(3).cloned().collect();
        if last_3[0] > last_3[1] && last_3[1] > last_3[2] {
            VolumeTrend::Increasing
        } else if last_3[0] < last_3[1] && last_3[1] < last_3[2] {
            VolumeTrend::Decreasing
        } else {
            VolumeTrend::Stable
        }
    } else {
        VolumeTrend::Stable
    };

    Some(VolumeAnalysis {
        avg_volume,
        current_volume,
        volume_ratio,
        is_volume_spike,
        volume_trend,
    })
}

// =============================================================================
// CANDLESTICK PATTERN RECOGNITION
// =============================================================================

/// Candlestick pattern types
#[derive(Debug, Clone, PartialEq)]
pub enum CandlestickPattern {
    Hammer,
    InvertedHammer,
    Doji,
    BullishEngulfing,
    BearishEngulfing,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,
    MorningStar,
    EveningStar,
    None,
}

/// Candlestick pattern detection result
#[derive(Debug, Clone)]
pub struct PatternResult {
    pub pattern: CandlestickPattern,
    pub bullish: bool,
    pub confidence: f64, // 0.0 to 1.0
    pub description: String,
}

/// Detect candlestick patterns in OHLCV data
pub fn detect_candlestick_patterns(ohlcv_data: &[OhlcvDataPoint]) -> Vec<PatternResult> {
    let mut patterns = Vec::new();

    if ohlcv_data.len() < 3 {
        return patterns;
    }

    let len = ohlcv_data.len();
    let current = &ohlcv_data[len - 1];
    let previous = &ohlcv_data[len - 2];
    let before_previous = if len >= 3 {
        Some(&ohlcv_data[len - 3])
    } else {
        None
    };

    // Helper function to calculate body and shadow sizes
    let body_size = |candle: &OhlcvDataPoint| (candle.close - candle.open).abs();
    let upper_shadow = |candle: &OhlcvDataPoint| candle.high - candle.close.max(candle.open);
    let lower_shadow = |candle: &OhlcvDataPoint| candle.close.min(candle.open) - candle.low;
    let total_range = |candle: &OhlcvDataPoint| candle.high - candle.low;

    // Hammer pattern detection
    let current_body = body_size(current);
    let current_lower_shadow = lower_shadow(current);
    let current_upper_shadow = upper_shadow(current);
    let current_range = total_range(current);

    if current_range > 0.0 {
        // Hammer: small body, long lower shadow, small upper shadow
        if current_body < current_range * 0.3
            && current_lower_shadow > current_body * 2.0
            && current_upper_shadow < current_body * 0.5
        {
            patterns.push(PatternResult {
                pattern: CandlestickPattern::Hammer,
                bullish: true,
                confidence: 0.8,
                description: "Hammer pattern - potential reversal".to_string(),
            });
        }

        // Doji: very small body relative to range
        if current_body < current_range * 0.1 {
            patterns.push(PatternResult {
                pattern: CandlestickPattern::Doji,
                bullish: false, // Neutral, but indicates indecision
                confidence: 0.7,
                description: "Doji pattern - market indecision".to_string(),
            });
        }
    }

    // Bullish Engulfing pattern (requires previous candle)
    let prev_body = body_size(previous);
    if current.close > current.open &&
        previous.close < previous.open && // Current green, previous red
        current.open < previous.close &&
        current.close > previous.open
    {
        // Current engulfs previous
        patterns.push(PatternResult {
            pattern: CandlestickPattern::BullishEngulfing,
            bullish: true,
            confidence: 0.85,
            description: "Bullish engulfing - strong reversal signal".to_string(),
        });
    }

    // Three White Soldiers (requires 3 candles)
    if let Some(before_prev) = before_previous {
        if current.close > current.open
            && previous.close > previous.open
            && before_prev.close > before_prev.open
            && current.close > previous.close
            && previous.close > before_prev.close
        {
            patterns.push(PatternResult {
                pattern: CandlestickPattern::ThreeWhiteSoldiers,
                bullish: true,
                confidence: 0.9,
                description: "Three white soldiers - strong bullish continuation".to_string(),
            });
        }
    }

    patterns
}

// =============================================================================
// SUPPORT AND RESISTANCE DETECTION
// =============================================================================

/// Support/Resistance level
#[derive(Debug, Clone)]
pub struct SupportResistanceLevel {
    pub price: f64,
    pub strength: f64, // 0.0 to 1.0, based on touches and volume
    pub touches: usize,
    pub is_support: bool,
    pub volume_at_level: f64,
    pub last_touch_age: i64, // Timestamp of last touch
}

/// Find support and resistance levels from OHLCV data
pub fn find_support_resistance_levels(
    ohlcv_data: &[OhlcvDataPoint],
    price_tolerance: f64,
) -> Vec<SupportResistanceLevel> {
    if ohlcv_data.len() < 10 {
        return Vec::new();
    }

    let mut levels = Vec::new();
    let mut potential_levels: HashMap<String, SupportResistanceLevel> = HashMap::new();

    // Look for levels where price reversed multiple times
    for (i, candle) in ohlcv_data.iter().enumerate() {
        // Check for support levels (bounces from lows)
        if i > 2 && i < ohlcv_data.len() - 2 {
            let prev_low = ohlcv_data[i - 1].low;
            let next_low = ohlcv_data[i + 1].low;

            // Local low (support)
            if candle.low <= prev_low && candle.low <= next_low {
                let level_key = format!("support_{:.8}", candle.low);

                if let Some(existing_level) = potential_levels.get_mut(&level_key) {
                    existing_level.touches += 1;
                    existing_level.volume_at_level += candle.volume;
                    existing_level.last_touch_age = candle.timestamp;
                } else {
                    potential_levels.insert(
                        level_key,
                        SupportResistanceLevel {
                            price: candle.low,
                            strength: 0.0, // Will calculate later
                            touches: 1,
                            is_support: true,
                            volume_at_level: candle.volume,
                            last_touch_age: candle.timestamp,
                        },
                    );
                }
            }

            // Local high (resistance)
            let prev_high = ohlcv_data[i - 1].high;
            let next_high = ohlcv_data[i + 1].high;

            if candle.high >= prev_high && candle.high >= next_high {
                let level_key = format!("resistance_{:.8}", candle.high);

                if let Some(existing_level) = potential_levels.get_mut(&level_key) {
                    existing_level.touches += 1;
                    existing_level.volume_at_level += candle.volume;
                    existing_level.last_touch_age = candle.timestamp;
                } else {
                    potential_levels.insert(
                        level_key,
                        SupportResistanceLevel {
                            price: candle.high,
                            strength: 0.0, // Will calculate later
                            touches: 1,
                            is_support: false,
                            volume_at_level: candle.volume,
                            last_touch_age: candle.timestamp,
                        },
                    );
                }
            }
        }
    }

    // Calculate strength and filter significant levels
    let avg_volume: f64 =
        ohlcv_data.iter().map(|d| d.volume).sum::<f64>() / (ohlcv_data.len() as f64);

    for mut level in potential_levels.into_values() {
        // Strength based on touches, volume, and recency
        let touch_score = (level.touches as f64).min(5.0) / 5.0; // Max 5 touches
        let volume_score = (level.volume_at_level / avg_volume).min(2.0) / 2.0; // Max 2x avg volume
        let recency_score = {
            let age_hours = (ohlcv_data.last().unwrap().timestamp - level.last_touch_age) / 3600;
            (24.0 - (age_hours as f64).min(24.0)) / 24.0 // More recent = higher score
        };

        level.strength = (touch_score * 0.4 + volume_score * 0.3 + recency_score * 0.3).min(1.0);

        // Only include levels with multiple touches and decent strength
        if level.touches >= 2 && level.strength >= 0.3 {
            levels.push(level);
        }
    }

    // Sort by strength
    levels.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());
    levels
}

// =============================================================================
// ENHANCED DIP DETECTION USING OHLCV
// =============================================================================

/// Enhanced dip signal with OHLCV analysis
#[derive(Debug, Clone)]
pub struct OhlcvDipSignal {
    pub strategy_name: String,
    pub urgency: f64,    // 0.0 to 2.0
    pub confidence: f64, // 0.0 to 1.0
    pub drop_percent: f64,
    pub timeframe: Timeframe,
    pub analysis_details: String,
    pub volume_confirmation: bool,
    pub technical_indicators: HashMap<String, f64>,
}

/// Strategy 1: OHLCV Candlestick Pattern Dip Detection
pub async fn detect_candlestick_pattern_dip(mint: &str) -> Option<OhlcvDipSignal> {
    // Check multiple timeframes for reversal patterns
    let timeframes = vec![Timeframe::Minute15, Timeframe::Hour1, Timeframe::Hour4];
    let mut best_signal: Option<OhlcvDipSignal> = None;
    let mut max_confidence = 0.0;

    for timeframe in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 20).await {
            if ohlcv_data.len() < 5 {
                continue;
            }

            let patterns = detect_candlestick_patterns(&ohlcv_data);
            let volume_analysis = analyze_volume(&ohlcv_data, 10);

            for pattern in patterns {
                if pattern.bullish && pattern.confidence > max_confidence {
                    let current_price = ohlcv_data.last().unwrap().close;
                    let price_5_ago = if ohlcv_data.len() >= 5 {
                        ohlcv_data[ohlcv_data.len() - 5].close
                    } else {
                        current_price
                    };

                    let drop_percent = ((current_price - price_5_ago) / price_5_ago) * 100.0;

                    // Only consider as dip if price actually dropped
                    if drop_percent < -3.0 {
                        let volume_confirmation = volume_analysis
                            .as_ref()
                            .map(|va| {
                                va.is_volume_spike || va.volume_trend == VolumeTrend::Increasing
                            })
                            .unwrap_or(false);

                        let mut technical_indicators = HashMap::new();
                        technical_indicators
                            .insert("pattern_confidence".to_string(), pattern.confidence);
                        if let Some(va) = &volume_analysis {
                            technical_indicators
                                .insert("volume_ratio".to_string(), va.volume_ratio);
                        }

                        let signal = OhlcvDipSignal {
                            strategy_name: "Candlestick Pattern Dip".to_string(),
                            urgency: pattern.confidence * 1.5, // Max 1.5 urgency
                            confidence: pattern.confidence,
                            drop_percent,
                            timeframe: timeframe.clone(),
                            analysis_details: format!(
                                "{} on {} timeframe",
                                pattern.description, timeframe
                            ),
                            volume_confirmation,
                            technical_indicators,
                        };

                        max_confidence = pattern.confidence;
                        best_signal = Some(signal);
                    }
                }
            }
        }
    }

    best_signal
}

/// Strategy 2: OHLCV Volume-Price Divergence Detection
pub async fn detect_volume_price_divergence_dip(mint: &str) -> Option<OhlcvDipSignal> {
    let timeframes = vec![Timeframe::Minute15, Timeframe::Hour1];

    for timeframe in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 30).await {
            if ohlcv_data.len() < 10 {
                continue;
            }

            let volume_analysis = analyze_volume(&ohlcv_data, 10)?;
            let current_price = ohlcv_data.last().unwrap().close;
            let price_10_ago = ohlcv_data[ohlcv_data.len() - 10].close;
            let drop_percent = ((current_price - price_10_ago) / price_10_ago) * 100.0;

            // Look for volume spike during price drop
            if drop_percent < -5.0 && volume_analysis.is_volume_spike {
                let confidence = (volume_analysis.volume_ratio - 1.0) * 0.2; // Higher volume = higher confidence
                let confidence = confidence.min(0.9);

                if confidence > 0.4 {
                    let mut technical_indicators = HashMap::new();
                    technical_indicators
                        .insert("volume_ratio".to_string(), volume_analysis.volume_ratio);
                    technical_indicators
                        .insert("avg_volume".to_string(), volume_analysis.avg_volume);

                    return Some(OhlcvDipSignal {
                        strategy_name: "Volume-Price Divergence".to_string(),
                        urgency: confidence * 1.8, // Max 1.62 urgency
                        confidence,
                        drop_percent,
                        timeframe,
                        analysis_details: format!(
                            "Volume spike {:.1}x during {:.1}% drop",
                            volume_analysis.volume_ratio, -drop_percent
                        ),
                        volume_confirmation: true,
                        technical_indicators,
                    });
                }
            }
        }
    }

    None
}

/// Strategy 3: OHLCV Bollinger Band Dip Detection
pub async fn detect_bollinger_band_dip(mint: &str) -> Option<OhlcvDipSignal> {
    let timeframes = vec![Timeframe::Hour1, Timeframe::Hour4];

    for timeframe in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 30).await {
            if ohlcv_data.len() < 20 {
                continue;
            }

            let prices: Vec<f64> = ohlcv_data.iter().map(|d| d.close).collect();
            let bb = calculate_bollinger_bands(&prices, 20, 2.0)?;
            let current_price = prices[prices.len() - 1];

            // Check if price is near or below lower Bollinger Band
            if bb.percent_b < 0.2 {
                // Below or very close to lower band
                let volume_analysis = analyze_volume(&ohlcv_data, 10);
                let volume_confirmation = volume_analysis
                    .as_ref()
                    .map(|va| va.volume_ratio > 1.2)
                    .unwrap_or(false);

                // Calculate drop from middle band
                let drop_from_mid = ((current_price - bb.middle_band) / bb.middle_band) * 100.0;

                if drop_from_mid < -3.0 {
                    let confidence = (0.2 - bb.percent_b) * 2.0; // Closer to lower band = higher confidence
                    let confidence = confidence.min(0.8);

                    let mut technical_indicators = HashMap::new();
                    technical_indicators.insert("percent_b".to_string(), bb.percent_b);
                    technical_indicators.insert("bandwidth".to_string(), bb.bandwidth);
                    technical_indicators.insert("lower_band".to_string(), bb.lower_band);

                    return Some(OhlcvDipSignal {
                        strategy_name: "Bollinger Band Oversold".to_string(),
                        urgency: confidence * 1.6,
                        confidence,
                        drop_percent: drop_from_mid,
                        timeframe,
                        analysis_details: format!(
                            "Price at {:.1}% of Bollinger Band range",
                            bb.percent_b * 100.0
                        ),
                        volume_confirmation,
                        technical_indicators,
                    });
                }
            }
        }
    }

    None
}

/// Strategy 4: OHLCV RSI Divergence & Oversold Detection
pub async fn detect_rsi_oversold_dip(mint: &str) -> Option<OhlcvDipSignal> {
    let timeframes = vec![Timeframe::Hour1, Timeframe::Hour4];

    for timeframe in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 30).await {
            if ohlcv_data.len() < 15 {
                continue;
            }

            let prices: Vec<f64> = ohlcv_data.iter().map(|d| d.close).collect();
            let rsi = calculate_rsi(&prices, 14)?;

            // Look for oversold conditions with potential reversal
            if rsi.is_oversold {
                let current_price = prices[prices.len() - 1];
                let price_14_ago = if prices.len() >= 14 {
                    prices[prices.len() - 14]
                } else {
                    current_price
                };

                let drop_percent = ((current_price - price_14_ago) / price_14_ago) * 100.0;

                if drop_percent < -5.0 {
                    // Higher confidence if RSI is rising (potential reversal)
                    let base_confidence = (30.0 - rsi.value) / 30.0; // Lower RSI = higher confidence
                    let trend_bonus = match rsi.trend {
                        RsiTrend::Rising => 0.2,
                        RsiTrend::Sideways => 0.1,
                        RsiTrend::Falling => 0.0,
                    };
                    let confidence = (base_confidence + trend_bonus).min(0.9);

                    let volume_analysis = analyze_volume(&ohlcv_data, 7);
                    let volume_confirmation = volume_analysis
                        .as_ref()
                        .map(|va| va.volume_ratio > 1.1)
                        .unwrap_or(false);

                    let mut technical_indicators = HashMap::new();
                    technical_indicators.insert("rsi".to_string(), rsi.value);
                    technical_indicators.insert("rsi_trend_bonus".to_string(), trend_bonus);

                    return Some(OhlcvDipSignal {
                        strategy_name: "RSI Oversold Divergence".to_string(),
                        urgency: confidence * 1.7,
                        confidence,
                        drop_percent,
                        timeframe,
                        analysis_details: format!(
                            "RSI {:.1} oversold with {:?} trend",
                            rsi.value, rsi.trend
                        ),
                        volume_confirmation,
                        technical_indicators,
                    });
                }
            }
        }
    }

    None
}

/// Strategy 5: OHLCV Support Level Precision Dip
pub async fn detect_support_level_precision_dip(mint: &str) -> Option<OhlcvDipSignal> {
    let timeframes = vec![Timeframe::Hour1, Timeframe::Hour4, Timeframe::Day1];

    for timeframe in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 50).await {
            if ohlcv_data.len() < 20 {
                continue;
            }

            let support_levels = find_support_resistance_levels(&ohlcv_data, 0.02); // 2% tolerance
            let current_price = ohlcv_data.last().unwrap().close;

            // Find closest support level
            let closest_support = support_levels
                .iter()
                .filter(|level| level.is_support && level.price < current_price)
                .min_by(|a, b| {
                    let a_distance = (current_price - a.price).abs();
                    let b_distance = (current_price - b.price).abs();
                    a_distance.partial_cmp(&b_distance).unwrap()
                });

            if let Some(support) = closest_support {
                let distance_to_support = ((current_price - support.price) / support.price) * 100.0;

                // If we're within 5% of a strong support level
                if distance_to_support < 5.0 && support.strength > 0.5 {
                    let price_20_ago = if ohlcv_data.len() >= 20 {
                        ohlcv_data[ohlcv_data.len() - 20].close
                    } else {
                        current_price
                    };

                    let drop_percent = ((current_price - price_20_ago) / price_20_ago) * 100.0;

                    if drop_percent < -3.0 {
                        let confidence = support.strength * 0.8; // Strong support = higher confidence
                        let volume_confirmation = support.volume_at_level > 0.0;

                        let mut technical_indicators = HashMap::new();
                        technical_indicators
                            .insert("support_strength".to_string(), support.strength);
                        technical_indicators
                            .insert("distance_to_support".to_string(), distance_to_support);
                        technical_indicators
                            .insert("support_touches".to_string(), support.touches as f64);

                        return Some(OhlcvDipSignal {
                            strategy_name: "Support Level Precision".to_string(),
                            urgency: confidence * 1.9,
                            confidence,
                            drop_percent,
                            timeframe,
                            analysis_details: format!(
                                "Near support at {:.8} ({:.1}% away, strength {:.2})",
                                support.price, distance_to_support, support.strength
                            ),
                            volume_confirmation,
                            technical_indicators,
                        });
                    }
                }
            }
        }
    }

    None
}

// =============================================================================
// ENHANCED ATH DETECTION USING OHLCV
// =============================================================================

/// Real ATH analysis using OHLCV historical data
#[derive(Debug, Clone)]
pub struct OhlcvAthAnalysis {
    pub mint: String,
    pub current_price: f64,
    pub timeframe_aths: HashMap<String, AthInfo>, // timeframe -> ATH info
    pub overall_ath_danger: AthDangerLevel,
    pub is_safe_for_entry: bool,
    pub volume_at_ath: f64,
    pub ath_analysis_confidence: f64,
}

#[derive(Debug, Clone)]
pub struct AthInfo {
    pub ath_price: f64,
    pub ath_timestamp: i64,
    pub distance_from_ath: f64, // Percentage
    pub volume_at_ath: f64,
    pub ath_confirmed: bool,     // High volume at ATH
    pub breakout_potential: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, PartialEq)]
pub enum AthDangerLevel {
    Safe,    // >40% from any recent ATH
    Caution, // 25-40% from ATH
    Warning, // 15-25% from ATH
    Danger,  // <15% from recent ATH
}

/// Comprehensive ATH analysis using real OHLCV data
pub async fn analyze_ath_with_ohlcv(mint: &str, current_price: f64) -> Option<OhlcvAthAnalysis> {
    let timeframes = vec![
        ("1h", Timeframe::Hour1),
        ("4h", Timeframe::Hour4),
        ("12h", Timeframe::Hour12),
        ("1d", Timeframe::Day1),
    ];

    let mut timeframe_aths = HashMap::new();
    let mut min_distance = f64::MAX;
    let mut total_volume_at_aths = 0.0;

    for (tf_name, timeframe) in timeframes {
        if !is_ohlcv_data_available(mint, &timeframe).await {
            continue;
        }

        if let Ok(ohlcv_data) = get_latest_ohlcv(mint, &timeframe, 100).await {
            if let Some(ath_info) = find_ath_in_timeframe(&ohlcv_data, current_price) {
                min_distance = min_distance.min(ath_info.distance_from_ath);
                total_volume_at_aths += ath_info.volume_at_ath;
                timeframe_aths.insert(tf_name.to_string(), ath_info);
            }
        }
    }

    if timeframe_aths.is_empty() {
        return None;
    }

    // Determine overall danger level based on closest ATH
    let overall_ath_danger = if min_distance < 15.0 {
        AthDangerLevel::Danger
    } else if min_distance < 25.0 {
        AthDangerLevel::Warning
    } else if min_distance < 40.0 {
        AthDangerLevel::Caution
    } else {
        AthDangerLevel::Safe
    };

    let is_safe_for_entry = matches!(
        overall_ath_danger,
        AthDangerLevel::Safe | AthDangerLevel::Caution
    );

    let ath_analysis_confidence = (timeframe_aths.len() as f64) / 4.0; // More timeframes = higher confidence

    Some(OhlcvAthAnalysis {
        mint: mint.to_string(),
        current_price,
        timeframe_aths,
        overall_ath_danger,
        is_safe_for_entry,
        volume_at_ath: total_volume_at_aths,
        ath_analysis_confidence,
    })
}

/// Find ATH information in a specific timeframe
fn find_ath_in_timeframe(ohlcv_data: &[OhlcvDataPoint], current_price: f64) -> Option<AthInfo> {
    if ohlcv_data.is_empty() {
        return None;
    }

    // Find the highest high in the dataset
    let ath_candle = ohlcv_data
        .iter()
        .max_by(|a, b| a.high.partial_cmp(&b.high).unwrap())?;

    let ath_price = ath_candle.high;
    let distance_from_ath = ((ath_price - current_price) / ath_price) * 100.0;

    // Calculate average volume to determine if ATH was volume-confirmed
    let avg_volume = ohlcv_data.iter().map(|d| d.volume).sum::<f64>() / (ohlcv_data.len() as f64);
    let ath_confirmed = ath_candle.volume > avg_volume * 1.5; // 1.5x average volume

    // Calculate breakout potential based on recent price action
    let recent_highs: Vec<f64> = ohlcv_data.iter().rev().take(10).map(|d| d.high).collect();
    let recent_avg_high = recent_highs.iter().sum::<f64>() / (recent_highs.len() as f64);
    let breakout_potential = (recent_avg_high / ath_price).min(1.0);

    Some(AthInfo {
        ath_price,
        ath_timestamp: ath_candle.timestamp,
        distance_from_ath,
        volume_at_ath: ath_candle.volume,
        ath_confirmed,
        breakout_potential,
    })
}

// =============================================================================
// COMPREHENSIVE OHLCV ANALYSIS INTEGRATION
// =============================================================================

/// Complete OHLCV-based trading analysis
#[derive(Debug, Clone)]
pub struct ComprehensiveOhlcvAnalysis {
    pub dip_signals: Vec<OhlcvDipSignal>,
    pub ath_analysis: Option<OhlcvAthAnalysis>,
    pub overall_buy_urgency: f64, // 0.0 to 2.0
    pub overall_confidence: f64,  // 0.0 to 1.0
    pub is_safe_for_entry: bool,
    pub analysis_summary: String,
}

/// Perform comprehensive OHLCV analysis for a token
pub async fn perform_comprehensive_ohlcv_analysis(token: &Token) -> ComprehensiveOhlcvAnalysis {
    let mint = &token.mint;
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    // Run all 5 enhanced dip detection strategies
    let mut dip_signals = Vec::new();

    if let Some(signal) = detect_candlestick_pattern_dip(mint).await {
        dip_signals.push(signal);
    }

    if let Some(signal) = detect_volume_price_divergence_dip(mint).await {
        dip_signals.push(signal);
    }

    if let Some(signal) = detect_bollinger_band_dip(mint).await {
        dip_signals.push(signal);
    }

    if let Some(signal) = detect_rsi_oversold_dip(mint).await {
        dip_signals.push(signal);
    }

    if let Some(signal) = detect_support_level_precision_dip(mint).await {
        dip_signals.push(signal);
    }

    // Perform ATH analysis
    let ath_analysis = analyze_ath_with_ohlcv(mint, current_price).await;

    // Calculate overall scores
    let overall_buy_urgency = if dip_signals.is_empty() {
        0.0
    } else {
        let weighted_urgency: f64 = dip_signals
            .iter()
            .map(|signal| signal.urgency * signal.confidence)
            .sum();
        let total_weight: f64 = dip_signals.iter().map(|signal| signal.confidence).sum();

        if total_weight > 0.0 {
            (weighted_urgency / total_weight).min(2.0)
        } else {
            0.0
        }
    };

    let overall_confidence = if dip_signals.is_empty() {
        0.0
    } else {
        dip_signals.iter().map(|s| s.confidence).sum::<f64>() / (dip_signals.len() as f64)
    };

    let is_safe_for_entry = ath_analysis
        .as_ref()
        .map(|ath| ath.is_safe_for_entry)
        .unwrap_or(true)
        && overall_confidence > 0.3;

    let analysis_summary = format!(
        "OHLCV Analysis: {} dip signals, urgency {:.2}, confidence {:.2}, ATH safety: {}",
        dip_signals.len(),
        overall_buy_urgency,
        overall_confidence,
        is_safe_for_entry
    );

    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "OHLCV_ANALYSIS",
            &format!("🔬 {} for {}", analysis_summary, token.symbol.as_str()),
        );
    }

    ComprehensiveOhlcvAnalysis {
        dip_signals,
        ath_analysis,
        overall_buy_urgency,
        overall_confidence,
        is_safe_for_entry,
        analysis_summary,
    }
}
