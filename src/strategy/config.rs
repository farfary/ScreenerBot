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
pub const WATCHLIST_CHECK_TIME_SEC: u64 = 10; // Check watchlist tokens more frequently
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print every 10 seconds
pub const ENTRY_COOLDOWN_MINUTES: i64 = 15; // Faster re-entry
pub const DCA_COOLDOWN_MINUTES: i64 = 30; // Faster DCA attempts


// ─── OPTIMIZED TRADING SIZE PARAMETERS (BASED ON PERFORMANCE ANALYSIS) ───
// Analysis shows medium trades (0.0015-0.005 SOL) have best performance (9.02% avg profit)
pub const MIN_TRADE_SIZE_SOL: f64 = 0.001; // Increased from 0.001 (sweet spot analysis)
pub const MAX_TRADE_SIZE_SOL: f64 = 0.003; // Reduced from 0.01 (focus on performing range)
pub const MIN_LIQUIDITY_FOR_MIN_SIZE: f64 = 15.0; // Increased for safety
pub const MAX_LIQUIDITY_FOR_MAX_SIZE: f64 = 8000.0; // Reduced to prevent oversizing

// ─── ENHANCED POSITION MANAGEMENT ───
pub const MAX_TOKENS: usize = 100;
pub const MAX_OPEN_POSITIONS: usize = 20; // Reduced for better management
pub const MAX_DCA_COUNT: u8 = 1; // Only 1 DCA round to limit risk
pub const DCA_BASE_TRIGGER_PCT: f64 = -20.0; // More conservative DCA trigger (was -15%)

// ─── DCA OPTIMIZATION (ADDRESSING 42% EFFICIENCY ISSUE) ───
pub const DCA_AGGRESSIVE_EXIT_THRESHOLD: f64 = -2.0; // Exit DCA on 2% negative momentum
pub const DCA_MAX_HOLD_TIME_MINUTES: i64 = 120; // Maximum 2 hours for DCA positions
pub const DCA_PROFIT_TARGET: f64 = 3.0; // Take profits at 3% for DCA positions
pub const DCA_SELL_MULTIPLIER: f64 = 2.0; // Double sell pressure for DCA positions

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

// ─── RISK MANAGEMENT ───
pub const ACCUMULATION_PATIENCE_THRESHOLD: f64 = 12.0; // Allow moderate pump before entry


// ─── WHALE ACTIVITY SCORING ───
pub const STRONG_WHALE_ACCUMULATION_USD: f64 = 500.0; // Strong whale net flow
pub const MODERATE_WHALE_ACCUMULATION_USD: f64 = 100.0; // Moderate whale net flow
pub const LARGE_TRADE_THRESHOLD_USD: f64 = 100.0; // Large trade detection
pub const MEDIUM_TRADE_THRESHOLD_USD: f64 = 50.0; // Medium trade detection
pub const SMALL_TRADE_THRESHOLD_USD: f64 = 10.0; // Small/bot trade detection



// ═══════════════════════════════════════════════════════════════════════════════
// 🚀 FAST PUMP DETECTION & VELOCITY-BASED PROFIT TAKING
// ═══════════════════════════════════════════════════════════════════════════════

// ─── FAST PUMP DETECTION PARAMETERS ───
pub const FAST_PUMP_VELOCITY_5M: f64 = 8.0; // 8%+ in 5 minutes = fast pump
pub const VERY_FAST_PUMP_VELOCITY_5M: f64 = 15.0; // 15%+ in 5 minutes = very fast pump
pub const EXTREME_PUMP_VELOCITY_5M: f64 = 25.0; // 25%+ in 5 minutes = extreme pump

// ─── PUMP MOMENTUM DECELERATION DETECTION ───
pub const MOMENTUM_DECELERATION_THRESHOLD: f64 = 0.5; // 50% momentum loss = danger
pub const VELOCITY_LOSS_WARNING: f64 = 0.3; // 30% velocity loss = warning

// ─── FAST PUMP PROFIT-TAKING MULTIPLIERS ───
pub const FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.6; // Tighten trailing stops to 60% during fast pumps
pub const VERY_FAST_PUMP_TRAILING_MULTIPLIER: f64 = 0.4; // Tighten to 40% during very fast pumps
pub const EXTREME_PUMP_TRAILING_MULTIPLIER: f64 = 0.25; // Tighten to 25% during extreme pumps

// ─── VOLUME-VELOCITY CORRELATION THRESHOLDS ───
pub const PUMP_VOLUME_DECLINE_THRESHOLD: f64 = 0.6; // Volume drops to 60% during pump = distribution

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

// ═══════════════════════════════════════════════════════════════════════════════
// 🚫 ENHANCED ENTRY CONTROLS - PREVENT BUYING ON TOPS & AFTER PROFIT EXITS
// ═══════════════════════════════════════════════════════════════════════════════


// ─── ENHANCED COOLDOWN SYSTEM ───
pub const SAME_TOKEN_ENTRY_COOLDOWN_HOURS: i64 = 6; // Minimum 6 hours between entries for same token
pub const PROFITABLE_EXIT_COOLDOWN_HOURS: i64 = 12; // Longer cooldown after profitable exits
pub const LOSS_EXIT_COOLDOWN_HOURS: i64 = 2; // Shorter cooldown after losses (for DCA opportunities)

// ─── PROFITABLE EXIT RE-ENTRY CONTROLS ───
pub const MIN_PRICE_DROP_AFTER_PROFIT_PCT: f64 = 8.0; // Must drop 8% from profitable exit price
pub const MAX_RECENT_EXITS_LOOKBACK_HOURS: i64 = 72; // Check exits within last 72 hours
pub const MIN_PROFIT_EXIT_THRESHOLD_PCT: f64 = 3.0; // Consider exits >3% profit as "profitable"


// ─── LOSS CONTROL POLICY PARAMETERS ───
pub const FORBIDDEN_LOSS_ZONE_MIN: f64 = 0.0; // Upper bound of forbidden zone (breakeven)
pub const CATASTROPHIC_LOSS_THRESHOLD: f64 = -50.0; // Threshold for catastrophic loss sales
pub const EMERGENCY_RUG_LOSS_THRESHOLD: f64 = -30.0; // Min loss for emergency rug override
pub const CATASTROPHIC_TIME_LIMIT_MINUTES: i64 = 4320; // 3 days max hold for catastrophic losses
