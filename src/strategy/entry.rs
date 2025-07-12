use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use crate::configs::ARGS;
use crate::performance;
use super::config::*;
use super::position::can_enter_token_position;
use super::helpers::*;

// Check if debug entry mode is enabled
fn is_debug_entry_enabled() -> bool {
    ARGS.iter().any(|arg| arg == "--debug-entry")
}

// Enhanced debug macro for entry-specific logging
macro_rules! debug_entry {
    ($($arg:tt)*) => {
        if is_debug_entry_enabled() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            println!("🔍 [DEBUG_ENTRY][{}] {}", timestamp, format!($($arg)*));
        }
    };
}

/// 🎯 HIGH SUCCESS RATE ENTRY STRATEGY - VERSION 2.0
///
/// Focus: 100% success rate through smart drop detection and dynamic DCA
/// Approach: Many small profitable trades with smart position sizing
/// Key: Real-time drop detection + historical analysis + dynamic DCA
pub async fn should_buy(
    token: &Token,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> bool {
    debug_entry!("═══ ENTRY ANALYSIS START ═══");

    // Parse token price
    let current_price = match token.price_usd.parse::<f64>() {
        Ok(price) => price,
        Err(_) => {
            debug_entry!("❌ Invalid price format: {}", token.price_usd);
            return false;
        }
    };

    debug_entry!("Token: {} ({}) | Current Price: ${:.8}", token.symbol, token.mint, current_price);

    // ✅ STEP 1: BASIC VALIDATION
    if !validate_basic_requirements(token) {
        return false;
    }

    // ✅ STEP 2: POSITION MANAGEMENT CHECK
    let (can_enter, _cooldown_remaining) = can_enter_token_position(&token.mint).await;
    if !can_enter {
        debug_entry!("❌ Position management check failed");
        return false;
    }

    // ✅ STEP 3: PRICE VALIDATION
    if !is_price_valid(current_price) {
        debug_entry!("❌ Invalid price: {}", current_price);
        return false;
    }

    // ✅ STEP 4: CREATE TOKEN PROFILE
    let token_profile = TokenProfile::from_token(token);
    let trading_config = token_profile.get_trading_config();
    debug_entry!(
        "Token profile: famous={}, holders={:?}",
        token_profile.is_famous,
        token_profile.holder_base_size
    );

    // ✅ STEP 5: SMART DROP DETECTION (REAL-TIME + HISTORICAL)
    let drop_detector = DropDetector::default();

    // Fast drop detection using real-time pool prices
    let fast_drop_signal = drop_detector.detect_fast_drop(token);

    // Historical drop analysis using dataframe
    let historical_drop_signal = drop_detector.analyze_historical_drop(token, dataframe);

    // Choose the best signal
    let drop_signal = match (fast_drop_signal, historical_drop_signal) {
        (Some(fast), Some(hist)) => {
            // Prefer real-time signal if confidence is good
            if fast.confidence >= 0.6 {
                Some(fast)
            } else {
                Some(hist)
            }
        }
        (Some(fast), None) => Some(fast),
        (None, Some(hist)) => Some(hist),
        (None, None) => None,
    };

    if let Some(ref signal) = drop_signal {
        debug_entry!(
            "Drop signal detected: {:.1}% from {:?} (confidence: {:.2})",
            signal.drop_percentage,
            signal.detection_source,
            signal.confidence
        );
    }

    // ✅ STEP 6: CALCULATE OPPORTUNITY SCORE
    let opportunity_score = if let Some(ref signal) = drop_signal {
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        drop_detector.calculate_drop_opportunity_score(signal, token, liquidity_sol)
    } else {
        // No drop signal, calculate basic opportunity
        calculate_basic_opportunity_score(token)
    };

    debug_entry!("Opportunity score: {:.3}", opportunity_score);

    // ✅ STEP 7: POSITION SIZE CALCULATION
    let position_sizer = PositionSizer::default();
    let base_size = if token_profile.is_famous {
        position_sizer.calculate_mooncat_size(token, opportunity_score)
    } else {
        position_sizer.calculate_optimal_size(token, opportunity_score)
    };

    // Apply performance-based adjustments
    let recent_metrics = extract_performance_metrics().await;
    let final_size = position_sizer.calculate_risk_adjusted_size(token, base_size, &recent_metrics);

    debug_entry!("Position size: {:.6} SOL (base: {:.6})", final_size, base_size);

    // ✅ STEP 8: ENTRY DECISION LOGIC
    let should_enter = make_entry_decision(
        token,
        &token_profile,
        &trading_config,
        drop_signal.as_ref(),
        opportunity_score,
        final_size
    );

    if should_enter {
        println!(
            "✅ [ENTRY] {} | Size: {:.6} SOL | Score: {:.3} | Drop: {:?}",
            token.symbol,
            final_size,
            opportunity_score,
            drop_signal
                .as_ref()
                .map(|s| format!("{:.1}%", s.drop_percentage))
                .unwrap_or_else(|| "none".to_string())
        );
    } else {
        debug_entry!("❌ Entry criteria not met");
    }

    should_enter
}

/// Validate basic requirements for trading
fn validate_basic_requirements(token: &Token) -> bool {
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;

    // Basic liquidity check
    if liquidity_sol < MIN_LIQUIDITY_SOL {
        debug_entry!(
            "❌ Insufficient liquidity: {:.1} SOL (min: {})",
            liquidity_sol,
            MIN_LIQUIDITY_SOL
        );
        return false;
    }

    // Basic volume check
    if token.volume.h24 < MIN_VOLUME_24H {
        debug_entry!("❌ Insufficient volume: ${:.0} (min: ${})", token.volume.h24, MIN_VOLUME_24H);
        return false;
    }

    // Activity check (ensure some trading activity)
    if token.txns.h1.buys == 0 {
        debug_entry!("❌ No recent buy activity");
        return false;
    }

    debug_entry!("✅ Basic requirements passed");
    true
}

/// Calculate basic opportunity score when no drop signal is available
fn calculate_basic_opportunity_score(token: &Token) -> f64 {
    let mut score = 0.0;

    // Volume factor (higher volume = better opportunity)
    let volume_factor = (token.volume.h24 / 10000.0).min(1.0);
    score += volume_factor * 0.3;

    // Liquidity factor (higher liquidity = safer)
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let liquidity_factor = (liquidity_sol / 1000.0).min(1.0);
    score += liquidity_factor * 0.3;

    // Activity factor (more buys = better)
    let activity_factor = ((token.txns.h1.buys as f64) / 50.0).min(1.0);
    score += activity_factor * 0.2;

    // Market cap factor (moderate size preferred) - parse from fdv_usd
    if let Ok(mcap) = token.fdv_usd.parse::<f64>() {
        let mcap_factor = if mcap > 100000.0 && mcap < 5000000.0 {
            0.2 // Sweet spot
        } else if mcap > 50000.0 && mcap < 10000000.0 {
            0.1 // Acceptable
        } else {
            0.0 // Too small or too large
        };
        score += mcap_factor;
    }

    score.min(1.0)
}

/// Extract performance metrics from performance system
async fn extract_performance_metrics() -> crate::performance::PerformanceMetrics {
    crate::performance::get_performance_metrics().await
}

/// Make the final entry decision based on all factors
fn make_entry_decision(
    token: &Token,
    profile: &TokenProfile,
    config: &TokenTradingConfig,
    drop_signal: Option<&DropSignal>,
    opportunity_score: f64,
    final_position_size: f64
) -> bool {
    debug_entry!("Making final entry decision...");

    let mut final_score = opportunity_score;

    // Drop signal bonus
    if let Some(signal) = drop_signal {
        if signal.confidence >= 0.8 {
            final_score += 0.4;
            debug_entry!("✅ Good drop signal (+0.4)");
        } else if signal.confidence >= 0.6 {
            final_score += 0.2;
            debug_entry!("✅ Moderate drop signal (+0.2)");
        }
    } else {
        final_score += 0.1; // Small bonus for no drop signal (base strategy)
        debug_entry!("⚠️ No drop signal (+0.1)");
    }

    debug_entry!(
        "✅ Opportunity score: {:.3} (+{:.3})",
        opportunity_score,
        final_score - opportunity_score
    );

    // Holder safety check
    let holder_count = token.rug_check.total_holders;
    if holder_count < MIN_HOLDERS_FOR_ENTRY {
        debug_entry!("❌ Too few holders for safety");
        return false;
    }

    // Holder bonus
    let holder_bonus = if holder_count > PREFERRED_HOLDERS_COUNT {
        0.2
    } else if holder_count > MIN_HOLDERS_FOR_ENTRY * 2 {
        0.1
    } else {
        0.05
    };
    final_score += holder_bonus;
    debug_entry!("✅ Holder bonus: +{:.3}", holder_bonus);

    // Famous token bonus
    if profile.is_famous {
        final_score += FAMOUS_TOKEN_BONUS;
        debug_entry!("✅ Famous token bonus: +{}", FAMOUS_TOKEN_BONUS);
    }

    // Good liquidity bonus
    if token.liquidity.usd > GOOD_LIQUIDITY_THRESHOLD {
        final_score += 0.1;
        debug_entry!("✅ Good liquidity bonus: +0.1");
    }

    let required_score = config.confidence_requirement;
    debug_entry!("Final decision score: {:.3} (required: {:.3})", final_score, required_score);

    let passes = final_score >= required_score;

    // Price validation with cached price for safety
    if passes {
        // Parse current price from token
        let current_price = match token.price_usd.parse::<f64>() {
            Ok(price) => price,
            Err(_) => {
                return false;
            }
        };

        if let Some(cached_price) = get_trading_price(&token.mint) {
            let price_diff = ((current_price - cached_price) / cached_price).abs();
            if price_diff > 0.05 {
                // 5% difference threshold
                debug_entry!(
                    "⚠️ Price validation failed: API=${:.8}, Cached=${:.8} (diff: {:.1}%)",
                    current_price,
                    cached_price,
                    price_diff * 100.0
                );
                false
            } else {
                true
            }
        } else {
            // No cached price available, accept API price
            true
        }
    } else {
        false
    }
}
