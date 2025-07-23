use crate::global::*;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

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

/// Represents the volatility profile of a token based on historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVolatilityProfile {
    pub avg_volatility_5m: f64,
    pub avg_volatility_1h: f64,
    pub avg_volatility_6h: f64,
    pub avg_volatility_24h: f64,
    pub recovery_probability: f64,
    pub momentum_score: f64,
    pub volume_trend_score: f64,
}

/// Represents the dynamic profit target calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicProfitTarget {
    pub base_target_percent: f64,
    pub time_decay_multiplier: f64,
    pub volatility_adjustment: f64,
    pub recovery_adjustment: f64,
    pub decline_adjustment: f64,
    pub final_target_percent: f64,
}

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

/// Analyzes token volatility and recovery probability based on available data
pub fn analyze_token_volatility(token: &Token) -> TokenVolatilityProfile {
    let mut profile = TokenVolatilityProfile {
        avg_volatility_5m: 0.0,
        avg_volatility_1h: 0.0,
        avg_volatility_6h: 0.0,
        avg_volatility_24h: 0.0,
        recovery_probability: 0.5, // Default neutral
        momentum_score: 0.5, // Default neutral
        volume_trend_score: 0.5, // Default neutral
    };

    // Calculate volatility from price changes if available
    if let Some(price_changes) = &token.price_change {
        profile.avg_volatility_5m = price_changes.m5.unwrap_or(0.0).abs();
        profile.avg_volatility_1h = price_changes.h1.unwrap_or(0.0).abs();
        profile.avg_volatility_6h = price_changes.h6.unwrap_or(0.0).abs();
        profile.avg_volatility_24h = price_changes.h24.unwrap_or(0.0).abs();

        // Recovery probability based on recent positive movements
        let positive_movements = [
            price_changes.m5.unwrap_or(0.0) > 0.0,
            price_changes.h1.unwrap_or(0.0) > 0.0,
            price_changes.h6.unwrap_or(0.0) > 0.0,
        ]
            .iter()
            .filter(|&&x| x)
            .count();

        profile.recovery_probability = (positive_movements as f64) / 3.0;

        // If we have 24h data, use it for longer-term recovery assessment
        if let Some(h24_change) = price_changes.h24 {
            if h24_change > 0.0 {
                profile.recovery_probability = (profile.recovery_probability + 0.3).min(1.0);
            }
        }
    }

    // Momentum score based on transaction activity (buy pressure)
    if let Some(txns) = &token.txns {
        profile.momentum_score = calculate_buy_pressure(txns);
    }

    // Volume trend analysis
    if let Some(volume) = &token.volume {
        profile.volume_trend_score = calculate_volume_trend(volume);
    }

    profile
}

/// Calculates buy pressure from transaction data
fn calculate_buy_pressure(txns: &TxnStats) -> f64 {
    let mut total_buys = 0.0;
    let mut total_sells = 0.0;
    let mut timeframe_count = 0;

    // Weight recent activity more heavily
    if let Some(ref m5) = txns.m5 {
        let buys = m5.buys.unwrap_or(0) as f64;
        let sells = m5.sells.unwrap_or(0) as f64;
        total_buys += buys * 4.0; // 4x weight for 5m data
        total_sells += sells * 4.0;
        timeframe_count += 1;
    }

    if let Some(ref h1) = txns.h1 {
        let buys = h1.buys.unwrap_or(0) as f64;
        let sells = h1.sells.unwrap_or(0) as f64;
        total_buys += buys * 2.0; // 2x weight for 1h data
        total_sells += sells * 2.0;
        timeframe_count += 1;
    }

    if let Some(ref h6) = txns.h6 {
        let buys = h6.buys.unwrap_or(0) as f64;
        let sells = h6.sells.unwrap_or(0) as f64;
        total_buys += buys; // 1x weight for 6h data
        total_sells += sells;
        timeframe_count += 1;
    }

    if total_buys + total_sells > 0.0 {
        total_buys / (total_buys + total_sells)
    } else {
        0.5 // Neutral if no transaction data
    }
}

/// Calculates volume trend score (increasing volume is bullish)
fn calculate_volume_trend(volume: &VolumeStats) -> f64 {
    let mut score: f64 = 0.5; // Default neutral

    // Compare recent volume to longer timeframes
    if let (Some(m5), Some(h1)) = (volume.m5, volume.h1) {
        if m5 > h1 * 0.083 {
            // 5min should be ~1/12 of hourly if consistent
            score += 0.2; // Recent volume spike
        }
    }

    if let (Some(h1), Some(h6)) = (volume.h1, volume.h6) {
        if h1 > h6 * 0.167 {
            // 1h should be ~1/6 of 6h if consistent
            score += 0.2; // Hourly volume increasing
        }
    }

    if let (Some(h6), Some(h24)) = (volume.h6, volume.h24) {
        if h6 > h24 * 0.25 {
            // 6h should be ~1/4 of 24h if consistent
            score += 0.1; // Daily volume trend up
        }
    }

    score.max(0.0).min(1.0)
}

/// Calculates dynamic profit target based on time, volatility, and position performance
pub fn calculate_dynamic_profit_target(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> DynamicProfitTarget {
    let volatility = analyze_token_volatility(token);
    let decline = analyze_price_decline(position, current_price);

    // Base target starts very high (exponential decay model from chart)
    let base_target = 500.0; // 500% initial target like the green line

    // Exponential time decay (steeper than original chart for faster convergence)
    let time_decay = ((-0.15 * time_held_seconds) / 3600.0).exp(); // Decay over hours

    // Volatility adjustment - more volatile tokens get higher targets
    let avg_volatility = (volatility.avg_volatility_1h + volatility.avg_volatility_6h) / 2.0;
    let volatility_multiplier = if avg_volatility > 50.0 {
        1.5 // Very volatile tokens
    } else if avg_volatility > 20.0 {
        1.2 // Moderately volatile
    } else {
        1.0 // Low volatility
    };

    // Recovery adjustment - if token shows signs of recovery, be more patient
    let recovery_multiplier = if volatility.recovery_probability > 0.7 {
        1.4 // High recovery probability
    } else if volatility.recovery_probability > 0.5 {
        1.1 // Moderate recovery probability
    } else if volatility.recovery_probability < 0.3 {
        0.7 // Low recovery probability - exit faster
    } else {
        1.0 // Neutral
    };

    // Decline adjustment - if we're significantly down, lower expectations dramatically
    let decline_adjustment = if decline.decline_from_entry_percent < -30.0 {
        0.3 // Heavily underwater - very low targets
    } else if decline.decline_from_entry_percent < -20.0 {
        0.5 // Significantly down - lower targets
    } else if decline.decline_from_entry_percent < -10.0 {
        0.8 // Moderately down - slightly lower targets
    } else {
        1.0 // Profitable or small loss - normal targets
    };

    // Combine all factors
    let final_target =
        base_target * time_decay * volatility_multiplier * recovery_multiplier * decline_adjustment;

    // Set reasonable bounds
    let bounded_target = final_target.max(3.0).min(1000.0); // Between 3% and 1000%

    DynamicProfitTarget {
        base_target_percent: base_target,
        time_decay_multiplier: time_decay,
        volatility_adjustment: volatility_multiplier,
        recovery_adjustment: recovery_multiplier,
        decline_adjustment,
        final_target_percent: bounded_target,
    }
}

/// Enhanced should sell logic using dynamic profit calculation
pub fn should_sell_dynamic(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    let (_, current_pnl_percent) = calculate_position_pnl(position, Some(current_price));

    // Get analysis components
    let profit_target = calculate_dynamic_profit_target(
        position,
        token,
        current_price,
        time_held_seconds
    );
    let volatility = analyze_token_volatility(token);
    let decline = analyze_price_decline(position, current_price);

    let mut urgency: f64 = 0.0;
    let mut reasons = Vec::new();

    // Emergency stop loss - immediate exit
    if current_pnl_percent <= -50.0 {
        return (1.0, "Emergency stop loss: -50%".to_string());
    }

    // Catastrophic decline with low recovery probability
    if current_pnl_percent <= -30.0 && volatility.recovery_probability < 0.3 {
        return (
            0.95,
            format!("Catastrophic decline {}% with low recovery probability", current_pnl_percent),
        );
    }

    // Dynamic profit target reached
    if current_pnl_percent >= profit_target.final_target_percent {
        urgency = 0.9;
        reasons.push(format!("Profit target {:.1}% reached", profit_target.final_target_percent));
    }

    // Time-based urgency (exponential decay like the chart)
    if time_held_seconds > 1800.0 {
        // After 30 minutes
        let time_urgency = 1.0 - (-time_held_seconds / 7200.0).exp(); // 2-hour exponential decay
        urgency = urgency.max(time_urgency * 0.6);
        reasons.push(format!("Time decay: {:.1}%", time_urgency * 100.0));
    }

    // Recovery probability adjustment
    if volatility.recovery_probability < 0.3 && current_pnl_percent < -5.0 {
        urgency += 0.25;
        reasons.push("Low recovery probability".to_string());
    }

    // Momentum-based adjustment
    if volatility.momentum_score < 0.4 {
        // More sells than buys
        urgency += 0.2;
        reasons.push("Negative momentum".to_string());
    }

    // Volume trend consideration
    if volatility.volume_trend_score < 0.4 && current_pnl_percent < 0.0 {
        urgency += 0.15;
        reasons.push("Declining volume".to_string());
    }

    // Maximum drawdown consideration
    if decline.max_drawdown_percent < -40.0 && current_pnl_percent < -20.0 {
        urgency += 0.3;
        reasons.push("Severe drawdown history".to_string());
    }

    // Volatility spike protection (if very volatile but declining, exit faster)
    if volatility.avg_volatility_1h > 30.0 && current_pnl_percent < -15.0 {
        urgency += 0.2;
        reasons.push("High volatility with loss".to_string());
    }

    // Ensure urgency is within bounds
    urgency = urgency.max(0.0).min(1.0);

    let reason = if reasons.is_empty() { "Hold".to_string() } else { reasons.join(" + ") };

    // Log detailed analysis for debugging
    if urgency > 0.3 {
        log(
            LogTag::Trader,
            "ANALYSIS",
            &format!(
                "{}: P&L {:.1}% | Target {:.1}% | Recovery {:.1}% | Momentum {:.1}% | Urgency {:.1}% | {}",
                position.symbol,
                current_pnl_percent,
                profit_target.final_target_percent,
                volatility.recovery_probability * 100.0,
                volatility.momentum_score * 100.0,
                urgency * 100.0,
                reason
            )
        );
    }

    (urgency, reason)
}

/// Quick helper to get basic sell decision without full analysis (for compatibility)
pub fn should_sell_simple(position: &Position, current_price: f64, time_held_seconds: f64) -> f64 {
    // Create a minimal token for basic analysis
    let minimal_token = Token {
        mint: position.mint.clone(),
        symbol: position.symbol.clone(),
        name: position.name.clone(),
        decimals: 6, // Default
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: Vec::new(),
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: Some(current_price),
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: Vec::new(),
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: Vec::new(),
        fdv: None,
        market_cap: None,
        txns: None, // No transaction data
        volume: None, // No volume data
        price_change: None, // No price change data
        liquidity: None,
        info: None,
        boosts: None,
    };

    let (urgency, _) = should_sell_dynamic(
        position,
        &minimal_token,
        current_price,
        time_held_seconds
    );
    urgency
}
