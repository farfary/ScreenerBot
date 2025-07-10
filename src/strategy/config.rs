// ═══════════════════════════════════════════════════════════════════════════════
// ENHANCED ANTI-BOT WHALE-FOLLOWING MEME COIN STRATEGY V2.0
// ═══════════════════════════════════════════════════════════════════════════════
//
// ⚠️  OPTIMIZED FOR SOLANA MEME TRADING WITH HEAVY BOT MANIPULATION
//
// 🎯 CORE OBJECTIVES:
// • Follow whale accumulation patterns while avoiding bot front-running
// • Use historical performance data to adapt strategy parameters
// • Take quick profits to offset inevitable rug pull losses
// • Minimize bot detection through unpredictable entry timing
// • Never sell at loss - hold losers until recovery or rug
//
// 🤖 ENHANCED ANTI-BOT MEASURES:
// • Transaction pattern analysis to detect bot vs whale activity
// • Entry timing randomization to avoid predictable patterns
// • Whale/retail ratio analysis using average transaction size
// • Volume spike detection to avoid pump schemes
// • Multiple confirmation signals before entry
//
// 🐋 IMPROVED WHALE DETECTION:
// • Large transaction monitoring (>2 SOL threshold)
// • Accumulation phase identification (low volatility + whale buys)
// • Distribution phase avoidance (high sell pressure from large holders)
// • Smart money following vs retail FOMO detection
//
// 💰 AGGRESSIVE PROFIT OPTIMIZATION:
// • Quick profit targets: 0.5%, 1%, 2%, 4%, 8%, 15%+
// • Take profits on ANY negative momentum when profitable
// • Faster exits to capture more winning trades
// • Historical win rate tracking for strategy adaptation
//
// 🔄 ADAPTIVE RISK MANAGEMENT:
// • Performance-based position sizing (reduce after losses)
// • Token blacklisting after failed trades
// • DCA only during confirmed whale accumulation
// • Emergency exits on bot flood detection
//
// 📊 TARGET METRICS:
// • Win rate: 65-75% (more small wins, fewer big losses)
// • Average win: 1-8% (quick scalps preferred)
// • Risk/reward: 2:1 minimum (2% avg win vs 1% avg loss)
// • Rug loss offset: 10+ small wins per rug loss
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// 🔧 CONFIGURATION PARAMETERS - ADJUST THESE TO CUSTOMIZE STRATEGY
// ═══════════════════════════════════════════════════════════════════════════════

// ─── TIMING PARAMETERS ───
pub const POSITIONS_CHECK_TIME_SEC: u64 = 30; // Normal position check interval
pub const POSITIONS_FREQUENT_CHECK_TIME_SEC: u64 = 5; // Frequent check for profitable positions (>2%)
pub const TOKEN_DISCOVERY_CHECK_TIME_SEC: u64 = 30;
pub const WATCHLIST_CHECK_TIME_SEC: u64 = 10; // Check watchlist tokens more frequently
pub const NEW_TOKEN_DISCOVERY_CHECK_TIME_SEC: u64 = 60; // Check new tokens less frequently
// ─── OPTIMIZED EXIT TIMING (ADDRESSING LONG HOLD INEFFICIENCY) ───
// Analysis shows long holds (>3h) miss 6.8% profit on average
pub const MIN_HOLD_TIME_SECONDS: i64 = 30; // Faster exits allowed
pub const MAX_HOLD_TIME_SECONDS: i64 = 7200; // Reduced to 2 hours max (was 6h)
pub const PROFITABLE_MAX_HOLD_MINUTES: i64 = 90; // Even shorter for profitable positions
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print every 10 seconds
pub const ENTRY_COOLDOWN_MINUTES: i64 = 15; // Faster re-entry
pub const DCA_COOLDOWN_MINUTES: i64 = 30; // Faster DCA attempts

// ─── TIME-BASED EXIT URGENCY MULTIPLIERS ───
pub const TIME_BASED_SELL_MULTIPLIER_1H: f64 = 1.1; // 10% more aggressive after 1h
pub const TIME_BASED_SELL_MULTIPLIER_2H: f64 = 1.3; // 30% more aggressive after 2h
pub const TIME_BASED_SELL_MULTIPLIER_3H: f64 = 2.0; // 100% more aggressive after 3h

// ─── OPTIMIZED TRADING SIZE PARAMETERS (BASED ON PERFORMANCE ANALYSIS) ───
// Analysis shows medium trades (0.0015-0.005 SOL) have best performance (9.02% avg profit)
pub const MIN_TRADE_SIZE_SOL: f64 = 0.0015; // Increased from 0.001 (sweet spot analysis)
pub const MAX_TRADE_SIZE_SOL: f64 = 0.008; // Reduced from 0.01 (focus on performing range)
pub const MIN_LIQUIDITY_FOR_MIN_SIZE: f64 = 15.0; // Increased for safety
pub const MAX_LIQUIDITY_FOR_MAX_SIZE: f64 = 8000.0; // Reduced to prevent oversizing

// ─── ENHANCED POSITION MANAGEMENT ───
pub const MAX_TOKENS: usize = 100;
pub const MAX_OPEN_POSITIONS: usize = 20; // Reduced for better management
pub const MAX_DCA_COUNT: u8 = 1; // Only 1 DCA round to limit risk
pub const DCA_SIZE_FACTOR: f64 = 1.0; // Same size DCA as initial
pub const DCA_BASE_TRIGGER_PCT: f64 = -20.0; // More conservative DCA trigger (was -15%)

// ─── DCA OPTIMIZATION (ADDRESSING 42% EFFICIENCY ISSUE) ───
pub const DCA_AGGRESSIVE_EXIT_THRESHOLD: f64 = -2.0; // Exit DCA on 2% negative momentum
pub const DCA_MAX_HOLD_TIME_MINUTES: i64 = 120; // Maximum 2 hours for DCA positions
pub const DCA_PROFIT_TARGET: f64 = 3.0; // Take profits at 3% for DCA positions
pub const DCA_SELL_MULTIPLIER: f64 = 2.0; // Double sell pressure for DCA positions
pub const DCA_MOMENTUM_MULTIPLIER: f64 = 1.5; // 1.5x more sensitive to momentum

// ─── TRADING COSTS ───
pub const TRANSACTION_FEE_SOL: f64 = 0.000015; // Transaction fee
pub const SLIPPAGE_BPS: f64 = 1.0; // Slightly higher slippage for execution

// ─── ENTRY FILTERS - FUNDAMENTAL REQUIREMENTS ───
pub const MIN_VOLUME_USD: f64 = 3000.0; // Minimum 24h volume
pub const MIN_LIQUIDITY_SOL: f64 = 8.0; // Minimum liquidity pool size
pub const MIN_ACTIVITY_BUYS_1H: u64 = 3; // Minimum buying activity per hour
pub const MIN_HOLDER_COUNT: u64 = 10; // Minimum unique holders

// ─── WHALE DETECTION THRESHOLDS ───
pub const WHALE_BUY_THRESHOLD_SOL: f64 = 2.0; // Minimum SOL for whale trade
pub const LARGE_WHALE_MULTIPLIER: f64 = 2.0; // 4+ SOL for large whale
pub const MEDIUM_WHALE_MULTIPLIER: f64 = 0.5; // 1+ SOL for medium whale

// ─── RISK MANAGEMENT ───
pub const BIG_DUMP_THRESHOLD: f64 = -25.0; // Avoid tokens with major dumps
pub const ACCUMULATION_PATIENCE_THRESHOLD: f64 = 12.0; // Allow moderate pump before entry
pub const MAX_PRICE_DIFFERENCE_PCT: f64 = 10.0; // Max price difference between sources
pub const HIGH_VOLATILITY_THRESHOLD: f64 = 15.0; // High volatility warning

// ─── WHALE ACTIVITY SCORING ───
pub const STRONG_WHALE_ACCUMULATION_USD: f64 = 500.0; // Strong whale net flow
pub const MODERATE_WHALE_ACCUMULATION_USD: f64 = 100.0; // Moderate whale net flow
pub const LARGE_TRADE_THRESHOLD_USD: f64 = 100.0; // Large trade detection
pub const MEDIUM_TRADE_THRESHOLD_USD: f64 = 50.0; // Medium trade detection
pub const SMALL_TRADE_THRESHOLD_USD: f64 = 10.0; // Small/bot trade detection

// ─── BOT DETECTION PARAMETERS ───
pub const HIGH_BOT_ACTIVITY_AVG_SIZE: f64 = 0.5; // SOL - very small avg trade
pub const HIGH_BOT_ACTIVITY_COUNT: u64 = 100; // Many small transactions
pub const MEDIUM_BOT_ACTIVITY_AVG_SIZE: f64 = 1.0; // SOL - small avg trade
pub const MEDIUM_BOT_ACTIVITY_COUNT: u64 = 50; // Moderate transaction count
pub const LOW_BOT_ACTIVITY_AVG_SIZE: f64 = 1.5; // SOL - reasonable avg trade
pub const LOW_BOT_ACTIVITY_COUNT: u64 = 20; // Low transaction count

// ─── ENTRY SCORING WEIGHTS ───
pub const WHALE_SCORE_WEIGHT: f64 = 0.3; // Weight for whale activity
pub const TRADES_SCORE_WEIGHT: f64 = 0.4; // Weight for trades data (higher)
pub const BOT_SCORE_WEIGHT: f64 = 0.2; // Weight for anti-bot scoring
pub const BUY_RATIO_WEIGHT: f64 = 0.15; // Weight for buy/sell ratio
pub const PRICE_MOMENTUM_WEIGHT: f64 = 0.15; // Weight for price momentum
pub const LIQUIDITY_BONUS_WEIGHT: f64 = 0.1; // Weight for extra liquidity
pub const VOLUME_MOMENTUM_WEIGHT: f64 = 0.1; // Weight for volume momentum

// ─── TECHNICAL ANALYSIS PARAMETERS ───
pub const VOLUME_SURGE_MULTIPLIER: f64 = 1.5; // Recent vs older volume
pub const POSITIVE_MOMENTUM_THRESHOLD: f64 = 2.0; // Price change %
pub const NEGATIVE_MOMENTUM_THRESHOLD: f64 = -3.0; // Price decline %
pub const VWAP_BULLISH_THRESHOLD: f64 = 1.02; // Price above VWAP
pub const VWAP_BEARISH_THRESHOLD: f64 = 0.98; // Price below VWAP
pub const VOLATILITY_MULTIPLIER: f64 = 1.5; // Increase caution in volatile markets

// ─── ENTRY SCORING THRESHOLDS ───
pub const MIN_WHALE_SCORE: f64 = 0.6; // Minimum whale activity
pub const MIN_TRADES_SCORE: f64 = 0.5; // Minimum trades score
pub const MAX_BOT_SCORE: f64 = 0.4; // Maximum bot activity
pub const MIN_BUY_RATIO: f64 = 0.6; // Minimum buy ratio
pub const ACCUMULATION_RANGE_MIN: f64 = -10.0; // Price change range
pub const LIQUIDITY_MULTIPLIER: f64 = 2.0; // Liquidity bonus threshold

// ─── SELL STRATEGY PARAMETERS ───
pub const WHALE_DISTRIBUTION_THRESHOLD: f64 = -200.0; // Heavy whale selling
pub const MODERATE_SELLING_THRESHOLD: f64 = -50.0; // Moderate selling pressure
pub const RECENT_MOMENTUM_THRESHOLD: f64 = -1.0; // Bearish momentum
pub const RESISTANCE_DISTANCE_THRESHOLD: f64 = 1.0; // Distance from resistance
pub const VOLUME_DECLINE_MULTIPLIER: f64 = 0.7; // Volume decline indicator
pub const PROFITABLE_VWAP_THRESHOLD: f64 = 1.05; // Extended above VWAP
pub const MIN_PROFIT_FOR_VWAP_SELL: f64 = 1.0; // Min profit for VWAP sell

// ─── SELL MULTIPLIERS ───
pub const WHALE_DISTRIBUTION_MULTIPLIER: f64 = 1.5; // Aggressive on whale distribution
pub const MODERATE_SELLING_MULTIPLIER: f64 = 1.2; // Moderate selling pressure
pub const MOMENTUM_MULTIPLIER: f64 = 1.3; // Bearish momentum
pub const RESISTANCE_MULTIPLIER: f64 = 1.2; // At resistance level
pub const VWAP_EXTENDED_MULTIPLIER: f64 = 1.15; // Extended above VWAP

// ─── OPTIMIZED PROFIT TAKING THRESHOLDS ───
// Tightened based on analysis - currently missing 2.07% profit on average
pub const WEAK_SELL_THRESHOLD: f64 = -1.5; // Tightened from -2.0
pub const MEDIUM_SELL_THRESHOLD: f64 = -2.5; // Tightened from -3.0
pub const STRONG_SELL_THRESHOLD: f64 = -4.0; // Tightened from -5.0
pub const EMERGENCY_EXIT_MIN_PROFIT: f64 = 0.3; // Min profit for emergency exit

// ─── PROFIT-LEVEL BASED TRAILING STOPS ───
pub const QUICK_PROFIT_TRAILING_STOP: f64 = 3.0; // 0.5-3% profits: 3% stop
pub const SMALL_PROFIT_TRAILING_STOP: f64 = 5.0; // 3-10% profits: 5% stop
pub const MEDIUM_PROFIT_TRAILING_STOP: f64 = 8.0; // 10-25% profits: 8% stop
pub const LARGE_PROFIT_TRAILING_STOP: f64 = 12.0; // 25%+ profits: 12% stop

// ─── PARTIAL PROFIT TAKING SYSTEM ───
pub const PARTIAL_PROFIT_LEVELS: [f64; 3] = [5.0, 10.0, 20.0]; // Take profits at these levels
pub const PARTIAL_PROFIT_PERCENTAGE: f64 = 25.0; // Sell 25% at each level

// ═══════════════════════════════════════════════════════════════════════════════
// 🚀 FAST PUMP DETECTION & VELOCITY-BASED PROFIT TAKING
// ═══════════════════════════════════════════════════════════════════════════════

// ─── FAST PUMP DETECTION PARAMETERS ───
pub const FAST_PUMP_VELOCITY_5M: f64 = 8.0; // 8%+ in 5 minutes = fast pump
pub const VERY_FAST_PUMP_VELOCITY_5M: f64 = 15.0; // 15%+ in 5 minutes = very fast pump
pub const EXTREME_PUMP_VELOCITY_5M: f64 = 25.0; // 25%+ in 5 minutes = extreme pump

pub const FAST_PUMP_VELOCITY_1M: f64 = 3.0; // 3%+ in 1 minute = fast pump
pub const VERY_FAST_PUMP_VELOCITY_1M: f64 = 6.0; // 6%+ in 1 minute = very fast pump

// ─── PUMP MOMENTUM DECELERATION DETECTION ───
pub const MOMENTUM_DECELERATION_THRESHOLD: f64 = 0.5; // 50% momentum loss = danger
pub const VELOCITY_LOSS_WARNING: f64 = 0.3; // 30% velocity loss = warning

// ─── FAST PUMP PROFIT-TAKING MULTIPLIERS ───
pub const FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.6; // Tighten trailing stops to 60% during fast pumps
pub const VERY_FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.4; // Tighten to 40% during very fast pumps
pub const EXTREME_PUMP_TRAILING_MULTIPLIER: f64 = 0.25; // Tighten to 25% during extreme pumps

pub const FAST_PUMP_MOMENTUM_MULTIPLIER: f64 = 2.0; // 2x more sensitive to momentum during pumps
pub const VELOCITY_EXIT_MULTIPLIER: f64 = 3.0; // 3x more aggressive on velocity loss

// ─── VOLUME-VELOCITY CORRELATION THRESHOLDS ───
pub const PUMP_VOLUME_DECLINE_THRESHOLD: f64 = 0.6; // Volume drops to 60% during pump = distribution
pub const HIGH_VELOCITY_LOW_VOLUME_MULTIPLIER: f64 = 2.5; // Extra aggressive when volume doesn't match pump

// ─── DYNAMIC PROFIT TARGETS FOR FAST PUMPS ───
pub const FAST_PUMP_QUICK_EXIT_PCT: f64 = 1.5; // Take 1.5%+ profits immediately in fast pumps
pub const VELOCITY_BASED_MIN_PROFIT: f64 = 0.8; // Minimum 0.8% to consider velocity exits

// ═══════════════════════════════════════════════════════════════════════════════
// 📉 SMART DIP BUYING & SWING TRADING PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

// ─── DIP BUYING THRESHOLDS ───
pub const HEALTHY_DIP_MIN: f64 = -3.0; // Minimum dip for consideration
pub const HEALTHY_DIP_MAX: f64 = -15.0; // Maximum healthy dip (beyond = dangerous)
pub const DANGEROUS_DUMP_THRESHOLD: f64 = -25.0; // Major dump threshold
pub const EXTREME_DUMP_THRESHOLD: f64 = -40.0; // Extreme dump threshold

// ─── SWING TRADING SCORING ───
pub const MIN_SWING_SCORE_THRESHOLD: f64 = 0.3; // Minimum swing score for consideration
pub const STRONG_SWING_SCORE_THRESHOLD: f64 = 0.5; // Strong swing opportunity
pub const MOMENTUM_REVERSAL_THRESHOLD: f64 = 1.0; // 1m price change for reversal signal

// ─── ADAPTIVE THRESHOLD ADJUSTMENTS ───
pub const SWING_THRESHOLD_REDUCTION_STRONG: f64 = 0.15; // Strong swing opportunity adjustment
pub const SWING_THRESHOLD_REDUCTION_MODERATE: f64 = 0.1; // Moderate swing opportunity adjustment
pub const WHALE_CONTRARIAN_THRESHOLD_REDUCTION: f64 = 0.1; // Whale accumulation during weakness
pub const REALTIME_DATA_THRESHOLD_REDUCTION: f64 = 0.05; // Real-time data advantage
pub const MIN_ADAPTIVE_THRESHOLD: f64 = 0.3; // Minimum threshold floor

// ─── DANGER SIGNAL ANALYSIS ───
pub const MAX_DANGER_RATIO: f64 = 0.6; // Maximum danger signal ratio before rejection
pub const PANIC_SELLING_THRESHOLD: u64 = 15; // Small sells indicating panic
pub const VOLUME_ACCUMULATION_MULTIPLIER: f64 = 2.0; // Volume spike during dip threshold
