use crate::global::*;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

/// Represents price movement velocity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceVelocityAnalysis {
    pub velocity_5m: f64, // Price change rate in last 5 minutes
    pub velocity_1h: f64, // Price change rate in last 1 hour
    pub velocity_deceleration: f64, // How much velocity is slowing (negative = slowing)
    pub profit_momentum_score: f64, // 0.0-1.0, how strong is profit momentum
    pub loss_momentum_score: f64, // 0.0-1.0, how strong is loss momentum
    pub is_momentum_fading: bool, // Is upward momentum clearly fading
    pub is_freefall: bool, // Is downward momentum accelerating dangerously
    pub is_fast_spike: bool, // >25% jump detected in <15 minutes
    pub spike_magnitude: f64, // Size of the spike in percentage
    pub spike_sustainability_score: f64, // How likely the spike is to hold (0.0-1.0)
}

/// Represents the analysis of how much a position has declined
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceDeclineAnalysis {
    pub entry_price: f64,
    pub current_price: f64,
    pub lowest_since_entry: f64,
    pub decline_from_entry_percent: f64,
    pub decline_from_peak_percent: f64,
    pub max_drawdown_percent: f64,
}

/// Analyzes price movement velocity to detect momentum changes
pub fn analyze_price_velocity(
    token: &Token,
    current_price: f64,
    position: &Position
) -> PriceVelocityAnalysis {
    let mut velocity_5m = 0.0;
    let mut velocity_1h = 0.0;
    let mut profit_momentum_score = 0.0;
    let mut loss_momentum_score = 0.0;
    let mut is_fast_spike = false;
    let mut spike_magnitude = 0.0;
    let mut spike_sustainability_score = 0.5;

    // Calculate velocity from price changes (% change per unit time)
    if let Some(price_changes) = &token.price_change {
        velocity_5m = price_changes.m5.unwrap_or(0.0) / 5.0; // % per minute
        velocity_1h = price_changes.h1.unwrap_or(0.0) / 60.0; // % per minute

        // FAST SPIKE DETECTION - >25% in 15 minutes or less
        let change_5m = price_changes.m5.unwrap_or(0.0);
        let change_1h = price_changes.h1.unwrap_or(0.0);

        // Detect fast spike: significant 5-minute change that's much larger than hourly average
        if change_5m > 25.0 {
            // Direct >25% in 5 minutes - definitely a fast spike
            is_fast_spike = true;
            spike_magnitude = change_5m;
        } else if change_5m > 15.0 && change_1h > 25.0 {
            // Strong 5-minute change combined with >25% hourly suggests fast spike within 15 min
            is_fast_spike = true;
            spike_magnitude = change_1h;
        }

        // Calculate spike sustainability based on volume, liquidity, and momentum consistency
        if is_fast_spike {
            spike_sustainability_score = calculate_spike_sustainability(
                token,
                change_5m,
                change_1h
            );
        }

        // Detect if we're in profit or loss territory
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let current_pnl_percent = ((current_price - entry_price) / entry_price) * 100.0;

        if current_pnl_percent > 0.0 {
            // In profit - check if momentum is slowing
            if velocity_5m > 0.0 && velocity_1h > 0.0 {
                // Both positive, check if recent is stronger
                profit_momentum_score = if velocity_5m > velocity_1h {
                    0.8 // Strong recent momentum
                } else {
                    0.3 // Momentum fading
                };
            } else if velocity_5m > 0.0 {
                profit_momentum_score = 0.5; // Only recent positive
            } else {
                profit_momentum_score = 0.1; // No positive momentum
            }
        } else {
            // In loss - check if momentum is accelerating downward
            if velocity_5m < 0.0 && velocity_1h < 0.0 {
                // Both negative, check if recent is worse
                loss_momentum_score = if velocity_5m < velocity_1h {
                    0.9 // Accelerating downward - danger
                } else {
                    0.4 // Slowing down
                };
            } else if velocity_5m < 0.0 {
                loss_momentum_score = 0.6; // Recent negative trend
            } else {
                loss_momentum_score = 0.2; // Improving
            }
        }
    }

    // Calculate deceleration (positive = accelerating, negative = decelerating)
    let velocity_deceleration = velocity_5m - velocity_1h;

    // Determine key conditions
    let is_momentum_fading = profit_momentum_score > 0.0 && velocity_deceleration < -0.1;
    let is_freefall = loss_momentum_score > 0.7 && velocity_deceleration < -0.2;

    PriceVelocityAnalysis {
        velocity_5m,
        velocity_1h,
        velocity_deceleration,
        profit_momentum_score,
        loss_momentum_score,
        is_momentum_fading,
        is_freefall,
        is_fast_spike,
        spike_magnitude,
        spike_sustainability_score,
    }
}

/// Calculate spike sustainability - how likely a fast spike is to hold vs dump immediately
/// Considers volume surge, liquidity depth, momentum consistency, and market conditions
fn calculate_spike_sustainability(token: &Token, change_5m: f64, change_1h: f64) -> f64 {
    let mut sustainability_score: f64 = 0.5; // Start neutral

    // Volume analysis - spikes with volume surge are more sustainable
    if let Some(volume) = &token.volume {
        if let (Some(vol_5m), Some(vol_1h)) = (volume.m5, volume.h1) {
            let expected_5m_volume = vol_1h / 12.0; // Expected if consistent
            let volume_surge_ratio = vol_5m / expected_5m_volume;

            if volume_surge_ratio > 5.0 {
                sustainability_score += 0.3; // Strong volume support - very bullish
            } else if volume_surge_ratio > 3.0 {
                sustainability_score += 0.2; // Good volume support
            } else if volume_surge_ratio > 1.5 {
                sustainability_score += 0.1; // Some volume support
            } else if volume_surge_ratio < 0.8 {
                sustainability_score -= 0.2; // Weak volume - concerning for spike
            }
        }
    }

    // Liquidity depth analysis - deeper liquidity supports price stability
    if let Some(liquidity) = &token.liquidity {
        if let Some(usd_liquidity) = liquidity.usd {
            if usd_liquidity > 500000.0 {
                sustainability_score += 0.25; // Deep liquidity pool - can absorb sells
            } else if usd_liquidity > 200000.0 {
                sustainability_score += 0.15; // Good liquidity
            } else if usd_liquidity > 100000.0 {
                sustainability_score += 0.05; // Moderate liquidity
            } else if usd_liquidity < 50000.0 {
                sustainability_score -= 0.15; // Shallow liquidity - spike likely to dump
            }
        }
    }

    // Momentum consistency analysis - gradual buildup vs sudden spike
    let momentum_consistency = if change_1h > 0.0 {
        (change_5m / change_1h).min(2.0) // How much of hourly gain is in last 5 minutes
    } else {
        0.0
    };

    if momentum_consistency > 1.5 {
        // Most gains in last 5 minutes - possible pump and dump
        sustainability_score -= 0.2;
    } else if momentum_consistency > 0.8 && momentum_consistency <= 1.2 {
        // Consistent momentum buildup - more sustainable
        sustainability_score += 0.15;
    }

    // Spike magnitude risk - larger spikes are harder to sustain
    if change_5m > 100.0 {
        sustainability_score -= 0.3; // Extreme spikes often unsustainable
    } else if change_5m > 50.0 {
        sustainability_score -= 0.2; // Large spikes risky
    } else if change_5m > 35.0 {
        sustainability_score -= 0.1; // Moderate spike risk
    }

    sustainability_score.max(0.0).min(1.0)
}

/// SMART PROFIT SYSTEM - Main decision engine
/// This system ONLY handles profit taking - never sells at loss
/// Loss management is handled by hardcoded -99% stop loss only
pub fn should_sell_smart_system(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    let (_, current_pnl_percent) = calculate_position_pnl(position, Some(current_price));

    // === NEVER SELL AT LOSS ===
    // Only the hardcoded -99% stop loss can trigger loss sales
    if current_pnl_percent <= -99.0 {
        return (1.0, "EMERGENCY: -99% stop loss triggered".to_string());
    }

    // Never sell at any other loss - always hold for recovery
    if current_pnl_percent < 0.0 {
        return (0.0, format!("HOLD: At {:.1}% loss - waiting for recovery", current_pnl_percent));
    }

    // === PROFIT-ONLY LOGIC BELOW ===

    // Analyze market conditions for profit taking
    let velocity_analysis = analyze_price_velocity(token, current_price, position);

    // === FAST SPIKE DETECTION - >25% jump in <15 minutes ===

    if velocity_analysis.is_fast_spike && current_pnl_percent > 15.0 {
        // Fast spike detected with meaningful profit

        let time_minutes = time_held_seconds / 60.0;

        // Time-based urgency - faster spikes need faster exits
        let time_urgency: f64 = if time_minutes < 5.0 {
            0.9 // Very recent spike - high urgency
        } else if time_minutes < 10.0 {
            0.8 // Recent spike - high urgency
        } else if time_minutes < 20.0 {
            0.7 // Moderate time urgency
        } else {
            0.6 // Lower time urgency but still significant
        };

        // Sustainability-based adjustment
        let sustainability_adjustment: f64 = if velocity_analysis.spike_sustainability_score > 0.7 {
            -0.15 // High sustainability - reduce urgency slightly
        } else if velocity_analysis.spike_sustainability_score > 0.5 {
            -0.05 // Moderate sustainability - small reduction
        } else if velocity_analysis.spike_sustainability_score < 0.3 {
            0.1 // Low sustainability - increase urgency
        } else {
            0.0 // Neutral
        };

        // Profit magnitude consideration - higher profits deserve more caution
        let profit_adjustment: f64 = if current_pnl_percent > 100.0 {
            0.05 // Extreme profits - more urgent to secure
        } else if current_pnl_percent > 50.0 {
            0.03 // High profits - slightly more urgent
        } else {
            0.0
        };

        let final_urgency: f64 = (time_urgency + sustainability_adjustment + profit_adjustment)
            .max(0.6) // Minimum 60% urgency for fast spikes
            .min(0.98); // Cap at 98%

        return (
            final_urgency,
            format!(
                "FAST SPIKE: +{:.1}% spike detected ({:.1}% profit) - sustainability {:.0}%",
                velocity_analysis.spike_magnitude,
                current_pnl_percent,
                velocity_analysis.spike_sustainability_score * 100.0
            ),
        );
    }

    // === PROFIT MOMENTUM SYSTEM - Fast profit taking ===

    if current_pnl_percent > 5.0 {
        // In meaningful profit

        // Momentum fading while profitable - SELL FAST
        if velocity_analysis.is_momentum_fading {
            let urgency = 0.85 + (current_pnl_percent / 100.0).min(0.1); // Higher profit = more urgent
            return (urgency, format!("Profit momentum fading at +{:.1}%", current_pnl_percent));
        }

        // Strong profit but low momentum score - momentum dying
        if current_pnl_percent > 15.0 && velocity_analysis.profit_momentum_score < 0.3 {
            return (0.9, format!("Strong profit +{:.1}% but momentum dying", current_pnl_percent));
        }

        // Very high profit with any momentum concerns
        if current_pnl_percent > 30.0 && velocity_analysis.profit_momentum_score < 0.6 {
            return (0.95, format!("Very high profit +{:.1}% - secure gains", current_pnl_percent));
        }

        // Time-based profit taking - longer held = lower expectations
        let time_hours = time_held_seconds / 3600.0;
        if time_hours > 1.0 {
            let time_decay_urgency = (time_hours / 6.0).min(0.4); // Max 40% urgency from time
            let profit_urgency = (current_pnl_percent / 100.0).min(0.3); // Max 30% from profit

            if time_decay_urgency + profit_urgency > 0.5 {
                return (
                    0.6 + time_decay_urgency,
                    format!(
                        "Time decay: {:.1}h held with +{:.1}% profit",
                        time_hours,
                        current_pnl_percent
                    ),
                );
            }
        }
    }

    // === DEFAULT: HOLD (Only for profitable positions) ===

    // For profitable positions, calculate small base urgency for time pressure
    if current_pnl_percent > 0.0 {
        let time_hours = time_held_seconds / 3600.0;
        let base_urgency = (time_hours / 12.0).min(0.15); // Very gradual time pressure

        return (
            base_urgency,
            format!("Hold: +{:.1}% profit with good momentum", current_pnl_percent),
        );
    }

    // For positions at break-even or small profits
    if current_pnl_percent >= -1.0 {
        return (0.0, format!("Hold: Near break-even at {:.1}%", current_pnl_percent));
    }

    // All other cases (losses) - always hold except -99% stop loss
    (0.0, format!("Hold: At {:.1}% loss - waiting for recovery", current_pnl_percent))
}

/// Legacy compatibility functions - these wrap the new smart system

/// Analyzes how much the price has declined since position entry
pub fn analyze_price_decline(position: &Position, current_price: f64) -> PriceDeclineAnalysis {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let decline_from_entry = ((current_price - entry_price) / entry_price) * 100.0;
    let decline_from_peak =
        ((current_price - position.price_highest) / position.price_highest) * 100.0;

    // Calculate maximum drawdown (worst point since entry)
    let max_drawdown = ((position.price_lowest - entry_price) / entry_price) * 100.0;

    PriceDeclineAnalysis {
        entry_price,
        current_price,
        lowest_since_entry: position.price_lowest,
        decline_from_entry_percent: decline_from_entry,
        decline_from_peak_percent: decline_from_peak,
        max_drawdown_percent: max_drawdown,
    }
}

/// Legacy function - wraps the new smart system
pub fn should_sell_dynamic(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    should_sell_smart_system(position, token, current_price, time_held_seconds)
}
