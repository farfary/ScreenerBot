/// Advanced ATH and Trend Analysis System for ScreenerBot
///
/// This module provides intelligent ATH detection and trend analysis using real-time API data
/// and enhanced OHLCV technical analysis instead of relying on bot's limited runtime price history.
/// Implements multi-timeframe analysis and liquidity-based dynamic thresholds.

use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_trader_enabled, is_debug_entry_enabled };
use crate::ohlcv_analysis::{ AthDangerLevel };

// =============================================================================
// DYNAMIC LIQUIDITY-BASED THRESHOLDS
// =============================================================================

/// Liquidity tiers for dynamic threshold calculation (24 tiers from $1 to $5M+)
/// EXPANDED FOR GEM HUNTING: Added ultra-micro tiers to catch moonshot tokens
#[derive(Debug, Clone, PartialEq)]
pub enum LiquidityTier {
    // Ultra High Liquidity Tiers ($2M - $5M)
    UltraWhale, // $5M+ liquidity
    MegaWhale, // $3.5M-$5M liquidity
    SuperWhale, // $2M-$3.5M liquidity

    // High Liquidity Tiers ($500K - $2M)
    Whale, // $1.5M-$2M liquidity
    LargeWhale, // $1M-$1.5M liquidity
    MediumWhale, // $750K-$1M liquidity
    SmallWhale, // $500K-$750K liquidity

    // Medium Liquidity Tiers ($50K - $500K)
    Massive, // $300K-$500K liquidity
    Large, // $200K-$300K liquidity
    MediumLarge, // $150K-$200K liquidity
    Medium, // $100K-$150K liquidity
    MediumSmall, // $75K-$100K liquidity
    Small, // $50K-$75K liquidity

    // Low Liquidity Tiers ($1K - $50K)
    Micro, // $25K-$50K liquidity
    MiniMicro, // $10K-$25K liquidity
    Tiny, // $5K-$10K liquidity

    // Ultra Low Liquidity Tiers ($100 - $5K) - GEM TERRITORY!
    Nano, // $1K-$5K liquidity
    Pico, // $500-$1K liquidity

    // NEW: ULTRA-MICRO GEM HUNTING TIERS ($1 - $500) - MOONSHOT POTENTIAL!
    UltraPico, // $100-$500 liquidity - TRUE GEMS!
    Femto, // $25-$100 liquidity - ULTRA GEMS!
    Atto, // $5-$25 liquidity - MEGA GEMS!
    Yocto, // $1-$5 liquidity - LEGENDARY GEMS!
}

impl LiquidityTier {
    /// Determine liquidity tier from USD liquidity amount
    pub fn from_liquidity(liquidity_usd: f64) -> Self {
        match liquidity_usd {
            x if x >= 5_000_000.0 => LiquidityTier::UltraWhale,
            x if x >= 3_500_000.0 => LiquidityTier::MegaWhale,
            x if x >= 2_000_000.0 => LiquidityTier::SuperWhale,
            x if x >= 1_500_000.0 => LiquidityTier::Whale,
            x if x >= 1_000_000.0 => LiquidityTier::LargeWhale,
            x if x >= 750_000.0 => LiquidityTier::MediumWhale,
            x if x >= 500_000.0 => LiquidityTier::SmallWhale,
            x if x >= 300_000.0 => LiquidityTier::Massive,
            x if x >= 200_000.0 => LiquidityTier::Large,
            x if x >= 150_000.0 => LiquidityTier::MediumLarge,
            x if x >= 100_000.0 => LiquidityTier::Medium,
            x if x >= 75_000.0 => LiquidityTier::MediumSmall,
            x if x >= 50_000.0 => LiquidityTier::Small,
            x if x >= 25_000.0 => LiquidityTier::Micro,
            x if x >= 10_000.0 => LiquidityTier::MiniMicro,
            x if x >= 5_000.0 => LiquidityTier::Tiny,
            x if x >= 1_000.0 => LiquidityTier::Nano,
            x if x >= 500.0 => LiquidityTier::Pico,
            // NEW GEM HUNTING TIERS - These are where 1000% gains happen!
            x if x >= 100.0 => LiquidityTier::UltraPico, // $100-$500 - TRUE GEMS!
            x if x >= 25.0 => LiquidityTier::Femto, // $25-$100 - ULTRA GEMS!
            x if x >= 5.0 => LiquidityTier::Atto, // $5-$25 - MEGA GEMS!
            _ => LiquidityTier::Yocto, // <$5 - LEGENDARY STATUS!
        }
    }

    /// Get dynamic dip threshold based on liquidity tier
    /// ULTRA AGGRESSIVE FOR GEM HUNTING: Expanded to catch massive moves (500-1000%)
    /// Range: 0.3% (ultra stable) to 25% (legendary gems) - MOONSHOT HUNTER MODE!
    pub fn get_dip_threshold(&self) -> f64 {
        match self {
            // Ultra High Liquidity: Very stable, tiny dips are profitable
            LiquidityTier::UltraWhale => 0.3, // $5M+: 0.3% dip (AGGRESSIVE)
            LiquidityTier::MegaWhale => 0.4, // $3.5M-$5M: 0.4% dip
            LiquidityTier::SuperWhale => 0.5, // $2M-$3.5M: 0.5% dip

            // High Liquidity: Small moves generate good profits
            LiquidityTier::Whale => 0.6, // $1.5M-$2M: 0.6% dip
            LiquidityTier::LargeWhale => 0.8, // $1M-$1.5M: 0.8% dip
            LiquidityTier::MediumWhale => 1.0, // $750K-$1M: 1.0% dip
            LiquidityTier::SmallWhale => 1.2, // $500K-$750K: 1.2% dip

            // Medium Liquidity: Still very tradeable
            LiquidityTier::Massive => 1.5, // $300K-$500K: 1.5% dip
            LiquidityTier::Large => 1.8, // $200K-$300K: 1.8% dip
            LiquidityTier::MediumLarge => 2.0, // $150K-$200K: 2.0% dip
            LiquidityTier::Medium => 2.2, // $100K-$150K: 2.2% dip
            LiquidityTier::MediumSmall => 2.5, // $75K-$100K: 2.5% dip
            LiquidityTier::Small => 3.0, // $50K-$75K: 3.0% dip

            // Low Liquidity: Still profitable with moderate drops
            LiquidityTier::Micro => 3.5, // $25K-$50K: 3.5% dip
            LiquidityTier::MiniMicro => 4.0, // $10K-$25K: 4.0% dip
            LiquidityTier::Tiny => 5.0, // $5K-$10K: 5.0% dip

            // Ultra Low Liquidity: Higher moves but still reasonable
            LiquidityTier::Nano => 6.0, // $1K-$5K: 6.0% dip
            LiquidityTier::Pico => 8.0, // $500-$1K: 8.0% dip

            // 🚀 NEW GEM HUNTING ULTRA-AGGRESSIVE THRESHOLDS 🚀
            // These are where 500-1000% moves happen!
            LiquidityTier::UltraPico => 12.0, // $100-$500: 12% dip - TRUE GEMS!
            LiquidityTier::Femto => 15.0, // $25-$100: 15% dip - ULTRA GEMS!
            LiquidityTier::Atto => 20.0, // $5-$25: 20% dip - MEGA GEMS!
            LiquidityTier::Yocto => 25.0, // <$5: 25% dip - LEGENDARY MOONSHOTS!
        }
    }

    /// Get profit target range based on liquidity tier
    /// ULTRA AGGRESSIVE FOR MOONSHOTS: Extended to capture 500-5000% gains
    /// Range from conservative (5%-20%) to LEGENDARY (1000%-10000%+)
    pub fn get_profit_target_range(&self) -> (f64, f64) {
        match self {
            // Ultra High Liquidity: Conservative, stable returns
            LiquidityTier::UltraWhale => (5.0, 20.0), // $5M+: 5%-20%
            LiquidityTier::MegaWhale => (6.0, 25.0), // $3.5M-$5M: 6%-25%
            LiquidityTier::SuperWhale => (8.0, 30.0), // $2M-$3.5M: 8%-30%

            // High Liquidity: Moderate returns with good stability
            LiquidityTier::Whale => (10.0, 35.0), // $1.5M-$2M: 10%-35%
            LiquidityTier::LargeWhale => (12.0, 40.0), // $1M-$1.5M: 12%-40%
            LiquidityTier::MediumWhale => (15.0, 50.0), // $750K-$1M: 15%-50%
            LiquidityTier::SmallWhale => (18.0, 60.0), // $500K-$750K: 18%-60%

            // Medium Liquidity: Higher returns with moderate risk
            LiquidityTier::Massive => (20.0, 75.0), // $300K-$500K: 20%-75%
            LiquidityTier::Large => (25.0, 100.0), // $200K-$300K: 25%-100%
            LiquidityTier::MediumLarge => (30.0, 125.0), // $150K-$200K: 30%-125%
            LiquidityTier::Medium => (35.0, 150.0), // $100K-$150K: 35%-150%
            LiquidityTier::MediumSmall => (40.0, 200.0), // $75K-$100K: 40%-200%
            LiquidityTier::Small => (50.0, 250.0), // $50K-$75K: 50%-250%

            // Low Liquidity: High volatility, high reward potential
            LiquidityTier::Micro => (60.0, 350.0), // $25K-$50K: 60%-350%
            LiquidityTier::MiniMicro => (75.0, 500.0), // $10K-$25K: 75%-500%
            LiquidityTier::Tiny => (100.0, 750.0), // $5K-$10K: 100%-750%

            // Ultra Low Liquidity: Extreme volatility, moonshot potential
            LiquidityTier::Nano => (150.0, 1000.0), // $1K-$5K: 150%-1000%
            LiquidityTier::Pico => (200.0, 1500.0), // $500-$1K: 200%-1500%

            // 🚀 NEW GEM HUNTING ULTRA-MOONSHOT TARGETS 🚀
            // These are where LEGENDARY gains happen! (REALISTIC TARGETS)
            LiquidityTier::UltraPico => (50.0, 500.0), // $100-$500: 50%-500% - TRUE GEMS!
            LiquidityTier::Femto => (75.0, 1000.0), // $25-$100: 75%-1000% - ULTRA GEMS!
            LiquidityTier::Atto => (100.0, 2000.0), // $5-$25: 100%-2000% - MEGA GEMS!
            LiquidityTier::Yocto => (150.0, 2000.0), // <$5: 150%-2000% - LEGENDARY MOONSHOTS!
        }
    }
}

// =============================================================================
// MULTI-TIMEFRAME TREND ANALYSIS
// =============================================================================

/// Trend direction for different timeframes
#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    StrongUp, // >+5% change
    ModerateUp, // +2% to +5% change
    Sideways, // -2% to +2% change
    ModerateDown, // -5% to -2% change
    StrongDown, // <-5% change
}

impl TrendDirection {
    fn from_price_change(change_percent: f64) -> Self {
        match change_percent {
            x if x > 5.0 => TrendDirection::StrongUp,
            x if x > 2.0 => TrendDirection::ModerateUp,
            x if x > -2.0 => TrendDirection::Sideways,
            x if x > -5.0 => TrendDirection::ModerateDown,
            _ => TrendDirection::StrongDown,
        }
    }

    /// Check if trend is bullish (up or sideways)
    pub fn is_bullish(&self) -> bool {
        matches!(
            self,
            TrendDirection::StrongUp | TrendDirection::ModerateUp | TrendDirection::Sideways
        )
    }

    /// Check if trend is bearish (down)
    pub fn is_bearish(&self) -> bool {
        matches!(self, TrendDirection::ModerateDown | TrendDirection::StrongDown)
    }
}

/// Multi-timeframe trend analysis result
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub m5_trend: TrendDirection, // 5-minute trend
    pub h1_trend: TrendDirection, // 1-hour trend
    pub h6_trend: TrendDirection, // 6-hour trend
    pub h24_trend: TrendDirection, // 24-hour trend
    pub overall_sentiment: f64, // -1.0 (very bearish) to +1.0 (very bullish)
    pub momentum_score: f64, // 0.0 to 1.0 momentum strength
    pub is_safe_for_entry: bool, // True if trends allow entry
}

impl TrendAnalysis {
    /// Create trend analysis from token price change data
    pub fn from_token(token: &Token) -> Self {
        let price_change = token.price_change.as_ref();

        let m5_change = price_change.and_then(|pc| pc.m5).unwrap_or(0.0);
        let h1_change = price_change.and_then(|pc| pc.h1).unwrap_or(0.0);
        let h6_change = price_change.and_then(|pc| pc.h6).unwrap_or(0.0);
        let h24_change = price_change.and_then(|pc| pc.h24).unwrap_or(0.0);

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "TREND_CHANGES",
                &format!(
                    "📈 {} Price Changes: 5m={:.2}% | 1h={:.2}% | 6h={:.2}% | 24h={:.2}%",
                    token.symbol,
                    m5_change,
                    h1_change,
                    h6_change,
                    h24_change
                )
            );
        }

        let m5_trend = TrendDirection::from_price_change(m5_change);
        let h1_trend = TrendDirection::from_price_change(h1_change);
        let h6_trend = TrendDirection::from_price_change(h6_change);
        let h24_trend = TrendDirection::from_price_change(h24_change);

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "TREND_DIRECTIONS",
                &format!(
                    "🧭 {} Trend Directions: 5m={:?} | 1h={:?} | 6h={:?} | 24h={:?}",
                    token.symbol,
                    m5_trend,
                    h1_trend,
                    h6_trend,
                    h24_trend
                )
            );
        }

        // Calculate overall sentiment score (-1.0 to +1.0)
        let sentiment_scores = [
            Self::trend_to_score(&m5_trend) * 0.4, // 5min gets 40% weight (immediate)
            Self::trend_to_score(&h1_trend) * 0.3, // 1h gets 30% weight
            Self::trend_to_score(&h6_trend) * 0.2, // 6h gets 20% weight
            Self::trend_to_score(&h24_trend) * 0.1, // 24h gets 10% weight
        ];
        let overall_sentiment = sentiment_scores.iter().sum();

        // Calculate momentum score based on trend alignment
        let momentum_score = Self::calculate_momentum_score(
            &[&m5_trend, &h1_trend, &h6_trend, &h24_trend]
        );

        // Determine if safe for entry (no bearish short-term trends)
        let is_safe_for_entry = !m5_trend.is_bearish() && !h1_trend.is_bearish();

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "TREND_SCORES",
                &format!(
                    "⚡ {} Trend Scores: Sentiment={:.2} | Momentum={:.2} | Safe={} (no bearish 5m+1h)",
                    token.symbol,
                    overall_sentiment,
                    momentum_score,
                    is_safe_for_entry
                )
            );
        }

        Self {
            m5_trend,
            h1_trend,
            h6_trend,
            h24_trend,
            overall_sentiment,
            momentum_score,
            is_safe_for_entry,
        }
    }

    /// Convert trend direction to numeric score
    fn trend_to_score(trend: &TrendDirection) -> f64 {
        match trend {
            TrendDirection::StrongUp => 1.0,
            TrendDirection::ModerateUp => 0.5,
            TrendDirection::Sideways => 0.0,
            TrendDirection::ModerateDown => -0.5,
            TrendDirection::StrongDown => -1.0,
        }
    }

    /// Calculate momentum score based on trend alignment
    fn calculate_momentum_score(trends: &[&TrendDirection]) -> f64 {
        let bullish_count = trends
            .iter()
            .filter(|t| t.is_bullish())
            .count() as f64;
        let total_count = trends.len() as f64;
        bullish_count / total_count
    }
}

// =============================================================================
// SMART ATH DETECTION USING API DATA
// =============================================================================

/// ATH analysis using real-time price change data instead of runtime history
#[derive(Debug, Clone)]
pub struct SmartAthAnalysis {
    pub current_price: f64,
    pub estimated_24h_high: f64, // Estimated from current price + 24h change
    pub estimated_6h_high: f64, // Estimated from current price + 6h change
    pub estimated_1h_high: f64, // Estimated from current price + 1h change
    pub is_near_24h_ath: bool, // Within 25% of estimated 24h high (lenient for dip buying)
    pub is_near_6h_ath: bool, // Within 30% of estimated 6h high (lenient for dip buying)
    pub is_near_1h_ath: bool, // Within 35% of estimated 1h high (lenient for dip buying)
    pub ath_proximity_score: f64, // 0.0 (far from ATH) to 1.0 (at ATH)
    pub ath_danger_level: AthDangerLevel,
}

impl SmartAthAnalysis {
    /// Create ATH analysis from token data
    pub fn from_token(token: &Token) -> Self {
        let current_price = token.price_dexscreener_sol.unwrap_or(0.0);
        let price_change = token.price_change.as_ref();

        // Estimate recent highs from price change data
        let h24_change = price_change.and_then(|pc| pc.h24).unwrap_or(0.0);
        let h6_change = price_change.and_then(|pc| pc.h6).unwrap_or(0.0);
        let h1_change = price_change.and_then(|pc| pc.h1).unwrap_or(0.0);

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_INPUT_DATA",
                &format!(
                    "🏔️ {} ATH Input: Current={:.10} | Changes: 24h={:.2}% | 6h={:.2}% | 1h={:.2}%",
                    token.symbol,
                    current_price,
                    h24_change,
                    h6_change,
                    h1_change
                )
            );
        }

        // Calculate estimated highs (assuming price was higher if change is negative)
        let estimated_24h_high = if h24_change < 0.0 {
            current_price / (1.0 + h24_change / 100.0)
        } else {
            current_price
        };

        let estimated_6h_high = if h6_change < 0.0 {
            current_price / (1.0 + h6_change / 100.0)
        } else {
            current_price
        };

        let estimated_1h_high = if h1_change < 0.0 {
            current_price / (1.0 + h1_change / 100.0)
        } else {
            current_price
        };

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_ESTIMATES",
                &format!(
                    "🔢 {} Estimated Highs: 24h={:.10} | 6h={:.10} | 1h={:.10}",
                    token.symbol,
                    estimated_24h_high,
                    estimated_6h_high,
                    estimated_1h_high
                )
            );
        }

        // Check ATH proximity with more lenient thresholds for dip buying and micro-caps
        let is_near_24h_ath = current_price >= estimated_24h_high * 0.6; // Within 40% (more lenient for gems)
        let is_near_6h_ath = current_price >= estimated_6h_high * 0.55; // Within 45% (more lenient for gems)
        let is_near_1h_ath = current_price >= estimated_1h_high * 0.5; // Within 50% (more lenient for gems)

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_PROXIMITY_CHECKS",
                &format!(
                    "🚨 {} ATH Proximity: 24h: {} ({:.1}%) | 6h: {} ({:.1}%) | 1h: {} ({:.1}%)",
                    token.symbol,
                    is_near_24h_ath,
                    (current_price / estimated_24h_high) * 100.0,
                    is_near_6h_ath,
                    (current_price / estimated_6h_high) * 100.0,
                    is_near_1h_ath,
                    (current_price / estimated_1h_high) * 100.0
                )
            );
        }

        // Calculate overall ATH proximity score
        let proximity_scores = [
            (current_price / estimated_24h_high).min(1.0) * 0.5, // 24h gets 50% weight
            (current_price / estimated_6h_high).min(1.0) * 0.3, // 6h gets 30% weight
            (current_price / estimated_1h_high).min(1.0) * 0.2, // 1h gets 20% weight
        ];
        let ath_proximity_score = proximity_scores.iter().sum::<f64>().min(1.0);

        // Determine danger level
        let ath_danger_level = if is_near_24h_ath && is_near_6h_ath && is_near_1h_ath {
            AthDangerLevel::Danger
        } else if (is_near_24h_ath && is_near_6h_ath) || (is_near_6h_ath && is_near_1h_ath) {
            AthDangerLevel::Warning
        } else if is_near_24h_ath || is_near_6h_ath || is_near_1h_ath {
            AthDangerLevel::Caution
        } else {
            AthDangerLevel::Safe
        };

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_FINAL",
                &format!(
                    "⚠️ {} ATH Final: Proximity Score={:.2} | Danger Level={:?} | Safe={}",
                    token.symbol,
                    ath_proximity_score,
                    ath_danger_level,
                    matches!(ath_danger_level, AthDangerLevel::Safe | AthDangerLevel::Caution)
                )
            );
        }

        Self {
            current_price,
            estimated_24h_high,
            estimated_6h_high,
            estimated_1h_high,
            is_near_24h_ath,
            is_near_6h_ath,
            is_near_1h_ath,
            ath_proximity_score,
            ath_danger_level,
        }
    }

    /// Check if token is safe for entry based on ATH analysis (more lenient for dip buying)
    pub fn is_safe_for_entry(&self) -> bool {
        matches!(
            self.ath_danger_level,
            AthDangerLevel::Safe | AthDangerLevel::Caution | AthDangerLevel::Warning
        )
    }
}

// =============================================================================
// COMPREHENSIVE ENTRY ANALYSIS
// =============================================================================

/// Complete entry analysis combining ATH, trend, and liquidity analysis
#[derive(Debug, Clone)]
pub struct SmartEntryAnalysis {
    pub liquidity_tier: LiquidityTier,
    pub trend_analysis: TrendAnalysis,
    pub ath_analysis: SmartAthAnalysis,
    pub dynamic_dip_threshold: f64,
    pub profit_target_range: (f64, f64),
    pub is_safe_for_entry: bool,
    pub entry_confidence: f64, // 0.0 to 1.0
    pub recommended_action: EntryAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryAction {
    BuyNow, // Strong buy signal
    BuyOnDip, // Wait for dip
    Monitor, // Watch but don't enter
    Avoid, // Skip this token
}

impl SmartEntryAnalysis {
    /// Create comprehensive entry analysis from token
    pub fn analyze_token(token: &Token) -> Self {
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        let liquidity_tier = LiquidityTier::from_liquidity(liquidity_usd);
        let trend_analysis = TrendAnalysis::from_token(token);
        let ath_analysis = SmartAthAnalysis::from_token(token);

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_STEP1",
                &format!(
                    "🪙 {} Liquidity Analysis: ${:.0} USD → Tier: {:?}",
                    token.symbol,
                    liquidity_usd,
                    liquidity_tier
                )
            );
        }

        let dynamic_dip_threshold = liquidity_tier.get_dip_threshold();
        let profit_target_range = liquidity_tier.get_profit_target_range();

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_STEP2",
                &format!(
                    "🎯 {} Dynamic Thresholds: Dip: {:.1}% | Profit: {:.1}%-{:.1}%",
                    token.symbol,
                    dynamic_dip_threshold,
                    profit_target_range.0,
                    profit_target_range.1
                )
            );
        }

        // Calculate entry safety
        let trend_safe = trend_analysis.is_safe_for_entry;
        let ath_safe = ath_analysis.is_safe_for_entry();
        let is_safe_for_entry = trend_safe && ath_safe;

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_STEP3",
                &format!(
                    "🛡️ {} Safety Checks: Trend Safe: {} | ATH Safe: {} | Combined: {}",
                    token.symbol,
                    trend_safe,
                    ath_safe,
                    is_safe_for_entry
                )
            );
        }

        // Calculate entry confidence (0.0 to 1.0)
        let entry_confidence = Self::calculate_entry_confidence(
            &trend_analysis,
            &ath_analysis,
            &liquidity_tier
        );

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_STEP4",
                &format!(
                    "📊 {} Confidence Calculation: {:.2} (based on trend momentum, ATH safety, liquidity)",
                    token.symbol,
                    entry_confidence
                )
            );
        }

        // Determine recommended action
        let recommended_action = Self::determine_action(
            is_safe_for_entry,
            entry_confidence,
            &trend_analysis,
            &ath_analysis
        );

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_STEP5",
                &format!(
                    "🎬 {} Final Action: {:?} (based on safety={}, confidence={:.2}, sentiment={:.2})",
                    token.symbol,
                    recommended_action,
                    is_safe_for_entry,
                    entry_confidence,
                    trend_analysis.overall_sentiment
                )
            );
        }

        Self {
            liquidity_tier,
            trend_analysis,
            ath_analysis,
            dynamic_dip_threshold,
            profit_target_range,
            is_safe_for_entry,
            entry_confidence,
            recommended_action,
        }
    }

    /// Calculate entry confidence score
    fn calculate_entry_confidence(
        trend: &TrendAnalysis,
        ath: &SmartAthAnalysis,
        liquidity: &LiquidityTier
    ) -> f64 {
        let mut confidence = 0.0;

        // Trend confidence (40% weight)
        confidence += trend.momentum_score * 0.4;

        // ATH safety confidence (30% weight)
        let ath_confidence = match ath.ath_danger_level {
            AthDangerLevel::Safe => 1.0,
            AthDangerLevel::Caution => 0.7,
            AthDangerLevel::Warning => 0.3,
            AthDangerLevel::Danger => 0.0,
        };
        confidence += ath_confidence * 0.3;

        // Liquidity confidence (20% weight) - More granular scaling with GEM HUNTING confidence
        let liquidity_confidence = match liquidity {
            // Ultra High Liquidity: Maximum confidence
            LiquidityTier::UltraWhale | LiquidityTier::MegaWhale | LiquidityTier::SuperWhale => 1.0,

            // High Liquidity: Very high confidence
            LiquidityTier::Whale | LiquidityTier::LargeWhale => 0.95,
            LiquidityTier::MediumWhale | LiquidityTier::SmallWhale => 0.9,

            // Medium Liquidity: Good confidence
            LiquidityTier::Massive | LiquidityTier::Large => 0.8,
            LiquidityTier::MediumLarge | LiquidityTier::Medium => 0.7,
            LiquidityTier::MediumSmall | LiquidityTier::Small => 0.6,

            // Low Liquidity: Moderate confidence
            LiquidityTier::Micro | LiquidityTier::MiniMicro => 0.4,
            LiquidityTier::Tiny => 0.3,

            // Ultra Low Liquidity: Lower confidence but still tradeable
            LiquidityTier::Nano => 0.2,
            LiquidityTier::Pico => 0.15,

            // 🚀 GEM HUNTING ULTRA-MICRO TIERS: BOOSTED CONFIDENCE FOR MOONSHOTS! 🚀
            // Higher confidence because these gems have massive upside potential
            LiquidityTier::UltraPico => 0.7, // $100-$500: High confidence - gems can be stable
            LiquidityTier::Femto => 0.6, // $25-$100: Good confidence with huge upside
            LiquidityTier::Atto => 0.5, // $5-$25: Moderate confidence but legendary potential
            LiquidityTier::Yocto => 0.4, // <$5: Lower confidence but god-tier potential
        };
        confidence += liquidity_confidence * 0.2;

        // Overall sentiment bonus (10% weight)
        let sentiment_bonus = ((trend.overall_sentiment + 1.0) / 2.0).max(0.0);
        confidence += sentiment_bonus * 0.1;

        confidence.min(1.0)
    }

    /// Determine recommended action
    fn determine_action(
        is_safe: bool,
        confidence: f64,
        trend: &TrendAnalysis,
        ath: &SmartAthAnalysis
    ) -> EntryAction {
        if !is_safe {
            return EntryAction::Avoid;
        }

        // More aggressive conditions for BuyNow to enable more immediate trades
        // LOWERED THRESHOLDS for moonshot hunting
        if confidence >= 0.4 && trend.overall_sentiment > 0.0 {
            EntryAction::BuyNow
        } else if confidence >= 0.3 && trend.overall_sentiment > -0.2 {
            EntryAction::BuyOnDip
        } else if confidence >= 0.2 {
            EntryAction::Monitor
        } else {
            EntryAction::Avoid
        }
    }
}

// =============================================================================
// PUBLIC INTERFACE FUNCTIONS
// =============================================================================

/// Check if token price action shows a valid dip based on liquidity-adjusted thresholds
pub fn is_valid_dip_for_liquidity(token: &Token, price_drop_percent: f64) -> bool {
    let analysis = SmartEntryAnalysis::analyze_token(token);
    price_drop_percent >= analysis.dynamic_dip_threshold
}

/// Get recommended profit target for token based on comprehensive analysis
pub fn get_smart_profit_target(token: &Token) -> (f64, f64) {
    let analysis = SmartEntryAnalysis::analyze_token(token);
    analysis.profit_target_range
}

/// Check if token is safe for entry using smart multi-timeframe analysis (Enhanced with OHLCV)
pub async fn is_token_safe_for_smart_entry_enhanced(token: &Token) -> (bool, SmartEntryAnalysis) {
    let analysis = analyze_token_enhanced(token).await;
    // ULTRA AGGRESSIVE: Lower confidence requirement to 0.3 for moonshot hunting
    let is_safe = analysis.is_safe_for_entry && analysis.entry_confidence >= 0.3;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "ENHANCED_SMART_ENTRY_RESULT",
            &format!(
                "🔬 Enhanced Smart Entry for {}: Safe={}, Confidence={:.2}, Action={:?}, Dip_Threshold={:.1}%, Profit_Target={:.1}%-{:.1}%",
                token.symbol.as_str(),
                is_safe,
                analysis.entry_confidence,
                analysis.recommended_action,
                analysis.dynamic_dip_threshold,
                analysis.profit_target_range.0,
                analysis.profit_target_range.1
            )
        );
    } else if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "ENHANCED_SMART_ENTRY_BRIEF",
            &format!(
                "Enhanced entry analysis for {}: safe={}, confidence={:.2}, action={:?}",
                token.symbol.as_str(),
                is_safe,
                analysis.entry_confidence,
                analysis.recommended_action
            )
        );
    }

    (is_safe, analysis)
}

/// Check if token is safe for entry using smart multi-timeframe analysis (Original)
pub fn is_token_safe_for_smart_entry(token: &Token) -> (bool, SmartEntryAnalysis) {
    let analysis = SmartEntryAnalysis::analyze_token(token);
    // ULTRA AGGRESSIVE: Lower confidence requirement to 0.3 for moonshot hunting
    let is_safe = analysis.is_safe_for_entry && analysis.entry_confidence >= 0.3;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "ENTRY_ANALYSIS",
            &format!("🔍 SMART ENTRY ANALYSIS for {}: Final Result={}", token.symbol, if is_safe {
                "✅ SAFE"
            } else {
                "❌ UNSAFE"
            })
        );

        log(
            LogTag::Trader,
            "ENTRY_DETAILS",
            &format!(
                "   📊 Confidence: {:.2} | Action: {:?} | ATH Danger: {:?} | Trend Safe: {}",
                analysis.entry_confidence,
                analysis.recommended_action,
                analysis.ath_analysis.ath_danger_level,
                analysis.trend_analysis.is_safe_for_entry
            )
        );

        log(
            LogTag::Trader,
            "ENTRY_THRESHOLDS",
            &format!(
                "   🎯 Liquidity Tier: {:?} | Dip Threshold: {:.1}% | Profit Target: {:.1}%-{:.1}%",
                analysis.liquidity_tier,
                analysis.dynamic_dip_threshold,
                analysis.profit_target_range.0,
                analysis.profit_target_range.1
            )
        );

        log(
            LogTag::Trader,
            "ENTRY_TRENDS",
            &format!(
                "   📈 Trends: 5m={:?} | 1h={:?} | 6h={:?} | 24h={:?} | Sentiment: {:.2}",
                analysis.trend_analysis.m5_trend,
                analysis.trend_analysis.h1_trend,
                analysis.trend_analysis.h6_trend,
                analysis.trend_analysis.h24_trend,
                analysis.trend_analysis.overall_sentiment
            )
        );

        log(
            LogTag::Trader,
            "ENTRY_ATH",
            &format!(
                "   🏔️ ATH Analysis: Current: {:.10} | 24h High: {:.10} | 6h High: {:.10} | 1h High: {:.10}",
                analysis.ath_analysis.current_price,
                analysis.ath_analysis.estimated_24h_high,
                analysis.ath_analysis.estimated_6h_high,
                analysis.ath_analysis.estimated_1h_high
            )
        );

        log(
            LogTag::Trader,
            "ENTRY_ATH_PROXIMITY",
            &format!(
                "   🚨 ATH Proximity: Near 24h: {} | Near 6h: {} | Near 1h: {} | Score: {:.2}",
                analysis.ath_analysis.is_near_24h_ath,
                analysis.ath_analysis.is_near_6h_ath,
                analysis.ath_analysis.is_near_1h_ath,
                analysis.ath_analysis.ath_proximity_score
            )
        );
    } else if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "SMART_ENTRY",
            &format!(
                "🧠 Smart entry analysis for {}: Safe={}, Confidence={:.2}, Action={:?}, Dip Threshold={:.1}%, Profit Target={:.1}%-{:.1}%",
                token.symbol,
                is_safe,
                analysis.entry_confidence,
                analysis.recommended_action,
                analysis.dynamic_dip_threshold,
                analysis.profit_target_range.0,
                analysis.profit_target_range.1
            )
        );
    }

    (is_safe, analysis)
}

/// Check if current price action indicates we're in the deepest part of a dip
pub fn is_deepest_dip_moment(token: &Token) -> bool {
    let trend_analysis = TrendAnalysis::from_token(token);

    // We're in deepest dip if:
    // 1. 5-minute trend is not strongly down (bottoming out)
    // 2. Longer timeframes show recent decline (confirming dip)
    // 3. Overall sentiment is recovering

    let m5_not_crashing = !matches!(trend_analysis.m5_trend, TrendDirection::StrongDown);
    let recent_decline =
        trend_analysis.h1_trend.is_bearish() || trend_analysis.h6_trend.is_bearish();
    let sentiment_recovering = trend_analysis.overall_sentiment > -0.5;

    m5_not_crashing && recent_decline && sentiment_recovering
}

// =============================================================================
// ENHANCED OHLCV INTEGRATION FOR ATH ANALYSIS
// =============================================================================

/// Enhanced ATH analysis that combines traditional price change analysis with OHLCV data
#[derive(Debug, Clone)]
pub struct EnhancedAthAnalysis {
    pub traditional_ath: SmartAthAnalysis,
    pub ohlcv_ath_available: bool,
    pub combined_ath_danger_level: AthDangerLevel,
    pub is_safe_for_entry: bool,
    pub confidence_score: f64, // 0.0 to 1.0
}

/// Perform enhanced ATH analysis combining traditional and OHLCV approaches
pub async fn analyze_ath_enhanced(token: &Token) -> EnhancedAthAnalysis {
    // Start with traditional analysis
    let traditional_ath = SmartAthAnalysis::from_token(token);
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    // Try to get OHLCV-based ATH analysis
    let ohlcv_ath = if current_price > 0.0 {
        match crate::ohlcv_analysis::analyze_ath_with_ohlcv(&token.mint, current_price).await {
            Some(analysis) => Some(analysis),
            None => {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Trader,
                        "ATH_OHLCV_UNAVAILABLE",
                        &format!("OHLCV ATH analysis unavailable for {}", token.symbol.as_str())
                    );
                }
                None
            }
        }
    } else {
        None
    };

    let ohlcv_ath_available = ohlcv_ath.is_some();

    // Determine combined danger level and safety
    let (combined_ath_danger_level, is_safe_for_entry, confidence_score) = if
        let Some(ohlcv) = ohlcv_ath
    {
        // OHLCV data available - use it as primary with traditional as backup
        let ohlcv_confidence = ohlcv.ath_analysis_confidence;
        let traditional_confidence = 0.6; // Fixed confidence for traditional analysis

        // Weighted combination based on confidence
        let is_ohlcv_safe = ohlcv.is_safe_for_entry;
        let is_traditional_safe = traditional_ath.is_safe_for_entry();

        let combined_safety = if ohlcv_confidence > 0.7 {
            // High OHLCV confidence - trust it primarily
            is_ohlcv_safe && is_traditional_safe
        } else if ohlcv_confidence > 0.4 {
            // Medium OHLCV confidence - require both to agree for safety
            is_ohlcv_safe && is_traditional_safe
        } else {
            // Low OHLCV confidence - fall back to traditional
            is_traditional_safe
        };

        let combined_danger = if !combined_safety {
            // If not safe, take the more conservative (dangerous) assessment
            match (ohlcv.overall_ath_danger, &traditional_ath.ath_danger_level) {
                (d1, d2) if d1 == AthDangerLevel::Danger || *d2 == AthDangerLevel::Danger =>
                    AthDangerLevel::Danger,
                (d1, d2) if d1 == AthDangerLevel::Warning || *d2 == AthDangerLevel::Warning =>
                    AthDangerLevel::Warning,
                (d1, d2) if d1 == AthDangerLevel::Caution || *d2 == AthDangerLevel::Caution =>
                    AthDangerLevel::Caution,
                _ => AthDangerLevel::Safe,
            }
        } else {
            AthDangerLevel::Safe
        };

        let final_confidence = (ohlcv_confidence * 0.7 + traditional_confidence * 0.3).min(1.0);

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_ENHANCED_ANALYSIS",
                &format!(
                    "🔬 Enhanced ATH for {}: OHLCV_safe={}, Traditional_safe={}, Combined_safe={}, Danger={:?}, Confidence={:.2}",
                    token.symbol.as_str(),
                    is_ohlcv_safe,
                    is_traditional_safe,
                    combined_safety,
                    combined_danger,
                    final_confidence
                )
            );
        }

        (combined_danger, combined_safety, final_confidence)
    } else {
        // No OHLCV data - use traditional analysis only
        let traditional_safety = traditional_ath.is_safe_for_entry();
        let traditional_confidence = 0.6;

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ATH_TRADITIONAL_ONLY",
                &format!(
                    "📊 Traditional ATH only for {}: safe={}, danger={:?}, confidence={:.2}",
                    token.symbol.as_str(),
                    traditional_safety,
                    traditional_ath.ath_danger_level,
                    traditional_confidence
                )
            );
        }

        (traditional_ath.ath_danger_level.clone(), traditional_safety, traditional_confidence)
    };

    EnhancedAthAnalysis {
        traditional_ath,
        ohlcv_ath_available,
        combined_ath_danger_level,
        is_safe_for_entry,
        confidence_score,
    }
}

/// Enhanced comprehensive entry analysis that integrates OHLCV ATH analysis
pub async fn analyze_token_enhanced(token: &Token) -> SmartEntryAnalysis {
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    let liquidity_tier = LiquidityTier::from_liquidity(liquidity_usd);
    let trend_analysis = TrendAnalysis::from_token(token);

    // Use enhanced ATH analysis
    let enhanced_ath = analyze_ath_enhanced(token).await;

    // Use the traditional ATH structure but with enhanced safety decision
    let ath_analysis = enhanced_ath.traditional_ath.clone();

    let dynamic_dip_threshold = liquidity_tier.get_dip_threshold();
    let profit_target_range = liquidity_tier.get_profit_target_range();

    // Enhanced safety check combining trend and enhanced ATH analysis
    let is_safe_for_entry = trend_analysis.is_safe_for_entry && enhanced_ath.is_safe_for_entry;

    // Enhanced confidence calculation
    let trend_confidence = trend_analysis.momentum_score;
    let ath_confidence = enhanced_ath.confidence_score;
    let liquidity_confidence = match liquidity_tier {
        LiquidityTier::UltraWhale | LiquidityTier::MegaWhale | LiquidityTier::SuperWhale => 0.9,
        LiquidityTier::Whale | LiquidityTier::LargeWhale => 0.8,
        LiquidityTier::MediumWhale | LiquidityTier::SmallWhale => 0.7,
        LiquidityTier::Massive | LiquidityTier::Large => 0.6,
        LiquidityTier::MediumLarge | LiquidityTier::Medium => 0.5,
        _ => 0.3,
    };

    let entry_confidence = (
        trend_confidence * 0.3 +
        ath_confidence * 0.4 +
        liquidity_confidence * 0.3
    ).min(1.0);

    let recommended_action = if !is_safe_for_entry {
        EntryAction::Avoid
    } else if
        enhanced_ath.combined_ath_danger_level == AthDangerLevel::Safe &&
        trend_analysis.overall_sentiment > 0.2
    {
        EntryAction::BuyNow
    } else if enhanced_ath.combined_ath_danger_level == AthDangerLevel::Caution {
        EntryAction::BuyOnDip
    } else {
        EntryAction::Monitor
    };

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "ENHANCED_ENTRY_ANALYSIS",
            &format!(
                "🎯 Enhanced Entry Analysis for {}: Safe={}, Action={:?}, Confidence={:.2}, ATH_Enhanced={}",
                token.symbol.as_str(),
                is_safe_for_entry,
                recommended_action,
                entry_confidence,
                enhanced_ath.ohlcv_ath_available
            )
        );
    }

    SmartEntryAnalysis {
        liquidity_tier,
        trend_analysis,
        ath_analysis,
        dynamic_dip_threshold,
        profit_target_range,
        is_safe_for_entry,
        entry_confidence,
        recommended_action,
    }
}

// =============================================================================
// 🚀 NEW ULTRA-AGGRESSIVE GEM HUNTING STRATEGIES 🚀
// =============================================================================

/// Detect early momentum patterns that indicate potential moonshots
/// Returns confidence score 0.0-1.0 if token shows early pump signals
pub fn detect_early_momentum_signals(token: &Token) -> f64 {
    let mut momentum_score = 0.0;

    // Volume surge detection (based on 24h data)
    if let Some(volume) = &token.volume {
        if let Some(volume_24h) = volume.h24 {
            // If 24h volume is substantial relative to market cap, it's getting attention
            if let Some(market_cap) = token.market_cap {
                let volume_to_mc_ratio = volume_24h / market_cap;
                if volume_to_mc_ratio > 0.1 {
                    // 10%+ volume/MC ratio
                    momentum_score += 0.3;
                }
                if volume_to_mc_ratio > 0.5 {
                    // 50%+ volume/MC ratio = hot token
                    momentum_score += 0.4;
                }
            }
        }
    }

    // Price change momentum (consistent upward movement)
    if let Some(price_changes) = &token.price_change {
        let mut positive_periods = 0;
        let mut total_periods = 0;

        // Check multiple timeframes for consistent gains
        if let Some(m5) = price_changes.m5 {
            total_periods += 1;
            if m5 > 0.0 {
                positive_periods += 1;
            }
        }
        if let Some(h1) = price_changes.h1 {
            total_periods += 1;
            if h1 > 0.0 {
                positive_periods += 1;
            }
        }
        if let Some(h6) = price_changes.h6 {
            total_periods += 1;
            if h6 > 0.0 {
                positive_periods += 1;
            }
        }

        if total_periods > 0 {
            let positive_ratio = (positive_periods as f64) / (total_periods as f64);
            momentum_score += positive_ratio * 0.3;
        }
    }

    momentum_score.min(1.0)
}

/// Detect volume spikes that indicate unusual attention
/// Returns true if token shows unusual volume patterns
pub fn detect_volume_spike_patterns(token: &Token) -> bool {
    if let Some(volume) = &token.volume {
        if let Some(h24) = volume.h24 {
            if let Some(h6) = volume.h6 {
                // 6h volume should be substantial portion of 24h volume for recent spike
                let recent_volume_ratio = h6 / h24;
                if recent_volume_ratio > 0.5 {
                    // 50%+ of daily volume in last 6h
                    return true;
                }
            }
        }
    }
    false
}

/// Check if token is in "fresh gem" category - recently created but showing promise
/// Returns true if token meets gem criteria
pub fn is_fresh_gem_candidate(token: &Token) -> bool {
    // Check token age (prefer newer tokens for gem potential)
    if let Some(created_at) = &token.created_at {
        let age_hours = chrono::Utc::now().signed_duration_since(*created_at).num_hours();

        // Gems are typically fresh (less than 48 hours) but not brand new
        if age_hours < 1 || age_hours > 48 {
            return false;
        }
    }

    // Must have some basic legitimacy indicators
    let has_basic_info =
        token.logo_url.is_some() || token.website.is_some() || !token.name.is_empty();

    if !has_basic_info {
        return false;
    }

    // Must have some liquidity but not too much (gems start small)
    if let Some(liquidity) = &token.liquidity {
        if let Some(usd) = liquidity.usd {
            // Sweet spot: $25 - $10,000 for gem hunting
            return usd >= 25.0 && usd <= 10_000.0;
        }
    }

    false
}

/// Ultra-aggressive dip detection for micro-cap gems
/// Returns urgency score 0.0-2.0 for extreme dip opportunities
pub fn detect_ultra_aggressive_dip_signals(token: &Token, price_drop_percent: f64) -> f64 {
    let liquidity_tier = LiquidityTier::from_liquidity(
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    );

    // Base urgency from liquidity-adjusted thresholds
    let base_threshold = liquidity_tier.get_dip_threshold();
    let mut urgency = if price_drop_percent >= base_threshold {
        (price_drop_percent / base_threshold).min(2.0)
    } else {
        0.0
    };

    // ULTRA AGGRESSIVE BONUSES for micro gems
    match liquidity_tier {
        | LiquidityTier::UltraPico
        | LiquidityTier::Femto
        | LiquidityTier::Atto
        | LiquidityTier::Yocto => {
            // Massive dip bonus for ultra-micro tokens
            if price_drop_percent >= 20.0 {
                urgency += 0.8; // Big bonus for 20%+ dips
            }
            if price_drop_percent >= 30.0 {
                urgency += 0.5; // Even bigger bonus for 30%+ dips
            }

            // Fresh gem bonus
            if is_fresh_gem_candidate(token) {
                urgency += 0.3;
            }

            // Momentum bonus
            let momentum = detect_early_momentum_signals(token);
            urgency += momentum * 0.4;

            // Volume spike bonus
            if detect_volume_spike_patterns(token) {
                urgency += 0.2;
            }
        }
        _ => {
            // Standard bonuses for larger tokens
            if price_drop_percent >= 15.0 {
                urgency += 0.3;
            }
        }
    }

    urgency.min(2.0)
}

/// Check if token shows "moonshot potential" - combination of factors indicating 500-1000%+ potential
/// Returns (is_moonshot_candidate: bool, confidence: f64)
pub fn analyze_moonshot_potential(token: &Token) -> (bool, f64) {
    let mut confidence = 0.0;

    // 1. Must be small enough to moon (liquidity check)
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    if liquidity_usd > 50_000.0 {
        return (false, 0.0); // Too big to be a moonshot gem
    }

    if liquidity_usd >= 100.0 && liquidity_usd <= 10_000.0 {
        confidence += 0.4; // Sweet spot for gems
    }

    // 2. Fresh but not too fresh
    if is_fresh_gem_candidate(token) {
        confidence += 0.3;
    }

    // 3. Early momentum signals
    let momentum = detect_early_momentum_signals(token);
    confidence += momentum * 0.3;

    // 4. Volume activity
    if detect_volume_spike_patterns(token) {
        confidence += 0.2;
    }

    // 5. Basic legitimacy (prevents complete scams)
    let legitimacy_score =
        (if token.logo_url.is_some() { 0.1 } else { 0.0 }) +
        (if token.website.is_some() { 0.1 } else { 0.0 }) +
        (if !token.name.is_empty() && token.name.len() > 2 { 0.1 } else { 0.0 });

    confidence += legitimacy_score;

    let is_candidate = confidence >= 0.6; // Need at least 60% confidence
    (is_candidate, confidence)
}
