// ═══════════════════════════════════════════════════════════════════════════════
// HIGH-SUCCESS RATE TRADING STRATEGY V3.0 - OPTIMIZED FOR PROFIT
// ═══════════════════════════════════════════════════════════════════════════════
//
// ⚡ OPTIMIZED FOR MAXIMUM SUCCESS RATE WITH SMART DROP DETECTION
//
// 🎯 CORE OBJECTIVES:
// • 100% success rate through smart DCA and dynamic position sizing
// • Profit from many small trades (millions per month) with small size each
// • Smart drop detection using real-time pool prices (seconds response)
// • Dynamic DCA based on token characteristics and liquidity
// • Always wait for profit - never sell at loss
// • Handle MOONCAT and other famous tokens with more data
//
// 🚀 KEY INNOVATIONS:
// • Real-time drop detection (2-10 seconds) vs API data (2+ minutes)
// • Dynamic DCA percentage per token based on liquidity/volatility
// • Token-specific trading profiles (MOONCAT gets special treatment)
// • Smart position sizing to always be a winner
// • Fast trading on ALL tokens without getting stuck on API delays
//
// 💰 PROFIT STRATEGY:
// • Many small profitable trades vs few large ones
// • Quick profit taking (0.5% to 20%+)
// • Conservative position sizing for 100% success rate
// • Dynamic DCA to handle drops and whales selling
// • Rug protection through liquidity monitoring
//
// � TARGET METRICS:
// • Success rate: 95%+ (through smart DCA)
// • Millions of trades per month with small sizes
// • Always profitable through patience and DCA
// • Fast execution on drop detection
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// 🔧 CORE TRADING PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

// ─── POSITION SIZING (CONSERVATIVE FOR 100% SUCCESS) ───
pub const MIN_TRADE_SIZE_SOL: f64 = 0.001; // Minimum trade size
pub const MAX_TRADE_SIZE_SOL: f64 = 0.015; // Maximum trade size (conservative)
pub const MAX_TRADE_PCT_OF_LIQUIDITY: f64 = 0.3; // Max 0.3% of liquidity per trade

// ─── DROP DETECTION THRESHOLDS ───
pub const FAST_DROP_THRESHOLD: f64 = -3.0; // Fast drop detection at -3%
pub const DCA_TRIGGER_THRESHOLD: f64 = -8.0; // Start DCA at -8% drop
pub const MAX_DCA_COUNT: u8 = 5; // Maximum 5 DCA levels
pub const DCA_SPACING_BASE: f64 = 0.6; // Base spacing between DCA levels

// ─── PROFIT TARGETS (AGGRESSIVE PROFIT TAKING) ───
pub const MIN_PROFIT_TARGET: f64 = 0.3; // Minimum profit to consider
pub const QUICK_PROFIT_TARGET: f64 = 1.5; // Quick profit target
pub const MAIN_PROFIT_TARGET: f64 = 4.0; // Main profit target
pub const BIG_PROFIT_TARGET: f64 = 12.0; // Big profit target

// ─── TIMING PARAMETERS ───
pub const ENTRY_COOLDOWN_MINUTES: i64 = 5; // Wait 5 minutes between entries
pub const MAX_POSITION_HOLD_HOURS: u64 = 48; // Maximum hold time (patient for profit)
pub const SIGNAL_MAX_AGE_SECONDS: u64 = 300; // Signal valid for 5 minutes

// ─── SAFETY PARAMETERS ───
pub const MIN_LIQUIDITY_SOL: f64 = 2.0; // Minimum liquidity required
pub const MIN_VOLUME_24H: f64 = 500.0; // Minimum 24h volume
pub const MIN_HOLDERS_FOR_SAFETY: u64 = 5; // Minimum holders for basic safety
pub const PREFERRED_HOLDERS: u64 = 100; // Preferred holder count

// ─── FAMOUS TOKEN BONUSES ───
pub const MOONCAT_SIZE_MULTIPLIER: f64 = 1.5; // MOONCAT gets larger positions
pub const FAMOUS_TOKEN_CONFIDENCE_BONUS: f64 = 0.2; // Famous tokens need less confidence

// ─── LIQUIDITY THRESHOLDS ───
pub const MIN_LIQUIDITY_FOR_MIN_SIZE: f64 = 10.0; // Liquidity for minimum size
pub const MAX_LIQUIDITY_FOR_MAX_SIZE: f64 = 2000.0; // Liquidity for maximum size

// ─── RUG PROTECTION ───
pub const MAX_DANGER_RATIO: f64 = 0.6; // Max 60% danger signals allowed
pub const EXTREME_DROP_THRESHOLD: f64 = -30.0; // Extreme drop threshold
pub const DANGEROUS_DROP_THRESHOLD: f64 = -20.0; // Dangerous drop threshold
pub const HEALTHY_DIP_MAX: f64 = -15.0; // Maximum healthy dip

// ═══════════════════════════════════════════════════════════════════════════════
// 🎯 DYNAMIC TRADING CONFIGURATIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Dynamic configuration that adapts to market conditions
#[derive(Debug, Clone)]
pub struct DynamicConfig {
    pub drop_detection_sensitivity: f64,
    pub dca_aggressiveness: f64,
    pub profit_taking_speed: f64,
    pub position_size_factor: f64,
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            drop_detection_sensitivity: 1.0, // Normal sensitivity
            dca_aggressiveness: 1.0, // Normal DCA aggressiveness
            profit_taking_speed: 1.0, // Normal profit taking speed
            position_size_factor: 1.0, // Normal position sizing
        }
    }
}

impl DynamicConfig {
    /// Create config optimized for high liquidity tokens
    pub fn high_liquidity() -> Self {
        Self {
            drop_detection_sensitivity: 0.8, // Less sensitive for stable tokens
            dca_aggressiveness: 1.2, // More aggressive DCA
            profit_taking_speed: 0.9, // Slower profit taking
            position_size_factor: 1.3, // Larger positions
        }
    }

    /// Create config optimized for low liquidity tokens
    pub fn low_liquidity() -> Self {
        Self {
            drop_detection_sensitivity: 1.3, // More sensitive detection
            dca_aggressiveness: 0.7, // Conservative DCA
            profit_taking_speed: 1.4, // Faster profit taking
            position_size_factor: 0.6, // Smaller positions
        }
    }

    /// Create config for famous tokens like MOONCAT
    pub fn famous_token() -> Self {
        Self {
            drop_detection_sensitivity: 0.9, // Slightly less sensitive
            dca_aggressiveness: 1.1, // Slightly more aggressive
            profit_taking_speed: 0.8, // Let winners run longer
            position_size_factor: 1.5, // Larger positions due to more data
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 🔍 SAFETY AND VALIDATION CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

pub const LARGE_TRADE_THRESHOLD_USD: f64 = 100.0; // Large trade threshold for whale detection
pub const VOLUME_ACCUMULATION_THRESHOLD: f64 = 2.0; // Volume accumulation threshold

// ═══════════════════════════════════════════════════════════════════════════════
// 🎮 FEATURE FLAGS
// ═══════════════════════════════════════════════════════════════════════════════

pub const ENABLE_REALTIME_DROP_DETECTION: bool = true; // Enable fast drop detection
pub const ENABLE_DYNAMIC_DCA: bool = true; // Enable dynamic DCA
pub const ENABLE_TOKEN_PROFILES: bool = true; // Enable token-specific configs
pub const ENABLE_SMART_PROFIT_TAKING: bool = true; // Enable smart profit targets

// ═══════════════════════════════════════════════════════════════════════════════
// 📈 MARKET CAP AND VOLUME SCALING
// ═══════════════════════════════════════════════════════════════════════════════

pub const MIN_MARKET_CAP_USD: f64 = 50000.0; // Minimum market cap for trading
pub const MAX_MARKET_CAP_USD: f64 = 10000000.0; // Market cap for maximum position

// ═══════════════════════════════════════════════════════════════════════════════
// ⚡ MARKET CAP SCALING FACTORS
// ═══════════════════════════════════════════════════════════════════════════════

pub const MARKET_CAP_SCALING_FACTOR: f64 = 0.000001; // Market cap scaling factor for calculations

// ═══════════════════════════════════════════════════════════════════════════════
// 📊 TIMING AND OPERATIONAL CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

pub const POSITIONS_CHECK_TIME_SEC: u64 = 30; // Normal position check interval
pub const POSITIONS_FREQUENT_CHECK_TIME_SEC: u64 = 5; // Frequent check for profitable positions
pub const TOKEN_DISCOVERY_CHECK_TIME_SEC: u64 = 300; // DexScreener data refresh interval
pub const PRICE_MONITORING_CHECK_TIME_SEC: u64 = 30; // Price updates for discovered tokens
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print positions every 10 seconds

// ═══════════════════════════════════════════════════════════════════════════════
// 💰 TRADING COSTS AND SLIPPAGE
// ═══════════════════════════════════════════════════════════════════════════════

pub const TRANSACTION_FEE_SOL: f64 = 0.000015; // Transaction fee in SOL
pub const SLIPPAGE_BPS: f64 = 1.0; // Slippage in basis points

// ═══════════════════════════════════════════════════════════════════════════════
// 📋 PORTFOLIO MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

pub const MAX_TOKENS: usize = 100; // Maximum tokens to track
pub const MAX_OPEN_POSITIONS: usize = 35; // Maximum open positions

// ═══════════════════════════════════════════════════════════════════════════════
// ⏰ COOLDOWN AND TIMING CONTROLS
// ═══════════════════════════════════════════════════════════════════════════════

pub const SAME_TOKEN_ENTRY_COOLDOWN_HOURS: i64 = 2; // Cooldown between entries on same token
pub const PROFITABLE_EXIT_COOLDOWN_HOURS: i64 = 4; // Cooldown after profitable exit
pub const LOSS_EXIT_COOLDOWN_HOURS: i64 = 1; // Cooldown after loss exit
pub const MIN_PROFIT_EXIT_THRESHOLD_PCT: f64 = 5.0; // Minimum profit to count as profitable exit

// ═══════════════════════════════════════════════════════════════════════════════
// 🔧 DCA CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

pub const DCA_SIZE_FACTOR: f64 = 1.2; // DCA size multiplier

// ═══════════════════════════════════════════════════════════════════════════════
// 🚀 PUMP DETECTION PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

pub const FAST_PUMP_VELOCITY_5M: f64 = 8.0; // 8%+ in 5 minutes = fast pump
pub const VERY_FAST_PUMP_VELOCITY_5M: f64 = 15.0; // 15%+ in 5 minutes = very fast pump
pub const EXTREME_PUMP_VELOCITY_5M: f64 = 25.0; // 25%+ in 5 minutes = extreme pump

pub const MOMENTUM_DECELERATION_THRESHOLD: f64 = 0.5; // 50% momentum loss = danger
pub const VELOCITY_LOSS_WARNING: f64 = 0.3; // 30% velocity loss = warning

pub const FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.6; // Tighten trailing stops during fast pumps
pub const VERY_FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.4; // Tighten stops during very fast pumps
pub const EXTREME_PUMP_TRAILING_MULTIPLIER: f64 = 0.25; // Tighten stops during extreme pumps

pub const PUMP_VOLUME_DECLINE_THRESHOLD: f64 = 0.6; // Volume drops to 60% during pump = distribution

// ═══════════════════════════════════════════════════════════════════════════════
// 📈 TREND DETECTION PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

pub const UPTREND_MOMENTUM_THRESHOLD: f64 = 3.0; // Enter uptrends above 3% momentum
pub const UPTREND_VOLUME_CONFIRMATION: f64 = 1.3; // Volume should be 1.3x average
pub const DOWNTREND_DIP_OPPORTUNITY: f64 = -5.0; // Buy dips below -5% in downtrends
pub const CONSOLIDATION_RANGE: f64 = 2.0; // +/- 2% considered consolidation

pub const HIGH_VOLUME_BONUS: f64 = 0.15; // Bonus for high volume conditions
pub const REAL_TIME_PRICE_BONUS: f64 = 0.2; // Bonus for real-time pool prices

// ═══════════════════════════════════════════════════════════════════════════════
// 💎 PRICE VALIDATION PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

pub const PRICE_VALIDATION_TOLERANCE: f64 = 0.05; // 5% tolerance for price validation

// ═══════════════════════════════════════════════════════════════════════════════
// 👥 HOLDER AND FAME PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

pub const MIN_HOLDERS_FOR_ENTRY: u64 = 5; // Minimum holders to enter
pub const PREFERRED_HOLDERS_COUNT: u64 = 100; // Preferred holder count
pub const FAMOUS_TOKEN_BONUS: f64 = 0.3; // Bonus for famous tokens
pub const GOOD_LIQUIDITY_THRESHOLD: f64 = 50000.0; // Threshold for good liquidity bonus
