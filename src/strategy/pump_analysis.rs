use crate::prelude::*;
use super::config::*;

/// Detect fast pump conditions and return pump intensity level
#[derive(Debug, Clone, Copy)]
pub enum PumpIntensity {
    Normal,
    Fast,
    VeryFast,
    Extreme,
}

impl PumpIntensity {
    pub fn get_trailing_multiplier(&self) -> f64 {
        match self {
            PumpIntensity::Normal => 1.0,
            PumpIntensity::Fast => FAST_PUMP_TRAILING_MULTIPLIER,
            PumpIntensity::VeryFast => VERY_FAST_PUMP_TRAILING_MULTIPLIER,
            PumpIntensity::Extreme => EXTREME_PUMP_TRAILING_MULTIPLIER,
        }
    }
}

/// Detect pump velocity and intensity
pub fn detect_pump_intensity(price_analysis: &PriceAnalysis) -> (PumpIntensity, String) {
    let change_5m = price_analysis.change_5m;

    if change_5m >= EXTREME_PUMP_VELOCITY_5M {
        (
            PumpIntensity::Extreme,
            format!("extreme_pump_{}%_5m{}", change_5m.round(), if price_analysis.is_5m_realtime {
                "_RT"
            } else {
                "_DX"
            }),
        )
    } else if change_5m >= VERY_FAST_PUMP_VELOCITY_5M {
        (
            PumpIntensity::VeryFast,
            format!("very_fast_pump_{}%_5m{}", change_5m.round(), if price_analysis.is_5m_realtime {
                "_RT"
            } else {
                "_DX"
            }),
        )
    } else if change_5m >= FAST_PUMP_VELOCITY_5M {
        (
            PumpIntensity::Fast,
            format!("fast_pump_{}%_5m{}", change_5m.round(), if price_analysis.is_5m_realtime {
                "_RT"
            } else {
                "_DX"
            }),
        )
    } else {
        (PumpIntensity::Normal, "normal_momentum".to_string())
    }
}

/// Detect momentum deceleration within a pump
pub fn detect_momentum_deceleration(
    token: &Token,
    price_analysis: &PriceAnalysis,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> (bool, f64, String) {
    // Compare recent momentum vs slightly older momentum to detect deceleration
    let current_5m = price_analysis.change_5m;
    let current_1h = price_analysis.change_1h;

    // If we have OHLCV data, use more granular analysis
    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Get momentum over different periods
        if
            let (Some(momentum_3_periods), Some(momentum_10_periods)) = (
                primary_timeframe.price_change_over_period(3),
                primary_timeframe.price_change_over_period(10),
            )
        {
            // Calculate momentum deceleration ratio
            let momentum_ratio = if momentum_10_periods > 1.0 {
                momentum_3_periods / momentum_10_periods
            } else {
                1.0
            };

            let is_decelerating = momentum_ratio < MOMENTUM_DECELERATION_THRESHOLD;

            if is_decelerating {
                return (true, momentum_ratio, format!("momentum_decel_{:.1}x", momentum_ratio));
            }
        }
    }

    // Fallback to basic deceleration detection using 5m vs 1h
    if current_1h > 5.0 {
        // Only check deceleration if we're in a significant pump
        let velocity_ratio = if current_1h > 0.0 {
            (current_5m * 12.0) / current_1h // Normalize 5m to hourly rate
        } else {
            1.0
        };

        let is_decelerating = velocity_ratio < VELOCITY_LOSS_WARNING;

        if is_decelerating {
            return (true, velocity_ratio, format!("velocity_loss_{:.1}x", velocity_ratio));
        }
    }

    (false, 1.0, "momentum_stable".to_string())
}

/// Check volume-velocity correlation for distribution detection
pub fn detect_pump_distribution(
    token: &Token,
    pump_intensity: PumpIntensity,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> (bool, String) {
    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Check if volume is declining during pump
        let recent_avg_volume = primary_timeframe.average_volume(3).unwrap_or(0.0);
        let older_avg_volume = primary_timeframe.average_volume(10).unwrap_or(0.0);

        if recent_avg_volume > 0.0 && older_avg_volume > 0.0 {
            let volume_ratio = recent_avg_volume / older_avg_volume;

            // During fast pumps, declining volume is very suspicious
            match pump_intensity {
                PumpIntensity::Fast | PumpIntensity::VeryFast | PumpIntensity::Extreme => {
                    if volume_ratio < PUMP_VOLUME_DECLINE_THRESHOLD {
                        return (true, format!("pump_vol_decline_{:.1}x", volume_ratio));
                    }
                }
                PumpIntensity::Normal => {
                    // Normal conditions - less strict
                    if volume_ratio < 0.4 {
                        return (true, format!("vol_decline_{:.1}x", volume_ratio));
                    }
                }
            }
        }
    }

    (false, "volume_normal".to_string())
}
