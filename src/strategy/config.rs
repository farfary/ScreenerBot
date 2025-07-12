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
pub const TOKEN_DISCOVERY_CHECK_TIME_SEC: u64 = 300; // DexScreener data refresh interval (new tokens)
pub const PRICE_MONITORING_CHECK_TIME_SEC: u64 = 30; // Price updates for discovered tokens (waiting for entry)

// ─── OPTIMIZED EXIT TIMING (ADDRESSING LONG HOLD INEFFICIENCY) ───
// Analysis shows long holds (>3h) miss 6.8% profit on average
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print every 10 seconds
pub const ENTRY_COOLDOWN_MINUTES: i64 = 15; // Faster re-entry
pub const DCA_COOLDOWN_MINUTES: i64 = 30; // Faster DCA attempts

// ─── OPTIMIZED TRADING SIZE PARAMETERS (SCALED BY LIQUIDITY AND MARKET CAP) ───
// Dynamic scaling from $1K to $1.2B market cap and $1K to $5M liquidity
pub const MIN_TRADE_SIZE_SOL: f64 = 0.001; // Minimum trade size for small tokens
pub const MAX_TRADE_SIZE_SOL: f64 = 0.005; // Maximum trade size for high liquidity tokens
pub const MIN_LIQUIDITY_FOR_MIN_SIZE: f64 = 10.0; // Start scaling from 5 SOL liquidity
pub const MAX_LIQUIDITY_FOR_MAX_SIZE: f64 = 30000.0; // Scale up to 20K SOL (~$5M liquidity)

// Market cap based scaling
pub const MIN_MARKET_CAP_USD: f64 = 10000.0; // $1K minimum market cap
pub const MAX_MARKET_CAP_USD: f64 = 2500000000.0; // $1.2B maximum market cap
pub const MARKET_CAP_SCALING_FACTOR: f64 = 0.5; // 50% weight for market cap scaling

// Liquidity impact thresholds (prevent whale anger)
pub const MAX_TRADE_PCT_OF_LIQUIDITY: f64 = 0.1; // Max 0.5% of total liquidity per trade
pub const WHALE_ANGER_THRESHOLD_PCT: f64 = 1.0; // Avoid trades >1% of liquidity

// ─── ENHANCED POSITION MANAGEMENT ───
pub const MAX_TOKENS: usize = 100;
pub const MAX_OPEN_POSITIONS: usize = 35; // Increased for more opportunities
pub const MAX_DCA_COUNT: u8 = 2; // Allow 2 DCA rounds for better averaging
pub const DCA_SIZE_FACTOR: f64 = 1.2; // Slightly larger DCA for better averaging
pub const DCA_BASE_TRIGGER_PCT: f64 = -15.0; // More aggressive DCA trigger (was -20%)

// ─── OPTIMIZED DCA STRATEGY (ADDRESSING EFFICIENCY ISSUE) ───
pub const DCA_QUICK_TRIGGER_PCT: f64 = -8.0; // Quick DCA on 8% drops with strong whale signals
pub const DCA_WHALE_CONFIRMATION_THRESHOLD: f64 = 200.0; // USD whale volume for quick DCA
pub const DCA_MAX_HOLD_TIME_MINUTES: i64 = 180; // 3 hours max for DCA positions (was 2h)
pub const DCA_PROFIT_TARGET: f64 = 2.5; // Lower profit target for quicker exits
pub const DCA_SELL_MULTIPLIER: f64 = 1.5; // Reduced sell pressure multiplier

// ─── TRADING COSTS ───
pub const TRANSACTION_FEE_SOL: f64 = 0.000015; // Transaction fee
pub const SLIPPAGE_BPS: f64 = 1.0; // Slightly higher slippage for execution

// ─── ENTRY FILTERS - FUNDAMENTAL REQUIREMENTS (RELAXED FOR MORE OPPORTUNITIES) ───
pub const MIN_VOLUME_USD: f64 = 500.0; // Reduced from 1500 for more opportunities
pub const MIN_LIQUIDITY_SOL: f64 = 2.0; // Reduced from 5 SOL for smaller/newer tokens
pub const MIN_ACTIVITY_BUYS_1H: u64 = 1; // Reduced from 2 for more flexibility
pub const MIN_HOLDER_COUNT: u64 = 100; // Reduced from 8 for newer tokens

// ─── ENHANCED UPTREND DETECTION AND ENTRY OPTIMIZATION ───
pub const UPTREND_MOMENTUM_THRESHOLD: f64 = 3.0; // Enter uptrends above 3% momentum
pub const UPTREND_VOLUME_CONFIRMATION: f64 = 1.3; // Volume should be 1.3x average
pub const DOWNTREND_DIP_OPPORTUNITY: f64 = -5.0; // Buy dips below -5% in downtrends
pub const CONSOLIDATION_RANGE: f64 = 2.0; // +/- 2% considered consolidation

// ─── WHALE DETECTION THRESHOLDS ───
pub const WHALE_BUY_THRESHOLD_SOL: f64 = 0.5; // Minimum SOL for whale trade

// ─── WHALE ACTIVITY SCORING ───
pub const STRONG_WHALE_ACCUMULATION_USD: f64 = 500.0; // Strong whale net flow
pub const MODERATE_WHALE_ACCUMULATION_USD: f64 = 100.0; // Moderate whale net flow
pub const LARGE_TRADE_THRESHOLD_USD: f64 = 100.0; // Large trade detection
pub const MEDIUM_TRADE_THRESHOLD_USD: f64 = 50.0; // Medium trade detection
pub const SMALL_TRADE_THRESHOLD_USD: f64 = 10.0; // Small/bot trade detection

// ─── ENTRY SCORING THRESHOLDS (RELAXED FOR MORE BUYS) ───
pub const MIN_WHALE_SCORE: f64 = 0.2; // Reduced from 0.4
pub const MIN_TRADES_SCORE: f64 = 0.2; // Reduced from 0.3
pub const MAX_BOT_SCORE: f64 = 0.8; // Increased from 0.6 (more tolerant)
pub const MIN_BUY_RATIO: f64 = 0.4; // Reduced from 0.5
pub const LIQUIDITY_MULTIPLIER: f64 = 1.2; // Reduced threshold (was 1.5)

// ─── ADAPTIVE ENTRY THRESHOLDS (DYNAMIC BASED ON MARKET CONDITIONS) ───
pub const BASE_ENTRY_THRESHOLD: f64 = 0.3; // Reduced from 0.6 for more opportunities
pub const UPTREND_ENTRY_THRESHOLD: f64 = 0.25; // Lower threshold for uptrend entries
pub const DOWNTREND_ENTRY_THRESHOLD: f64 = 0.2; // Even lower for downtrend dip buys
pub const HIGH_VOLUME_BONUS: f64 = 0.15; // Increased bonus for high volume conditions
pub const REAL_TIME_PRICE_BONUS: f64 = 0.2; // Higher bonus for real-time pool prices

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
pub const MOMENTUM_REVERSAL_THRESHOLD: f64 = 1.0; // 1m price change for reversal signal

// ─── DANGER SIGNAL ANALYSIS ───
pub const MAX_DANGER_RATIO: f64 = 0.6; // Maximum danger signal ratio before rejection
pub const PANIC_SELLING_THRESHOLD: u64 = 15; // Small sells indicating panic
pub const VOLUME_ACCUMULATION_MULTIPLIER: f64 = 2.0; // Volume spike during dip threshold

// ─── ENHANCED COOLDOWN SYSTEM ───
pub const SAME_TOKEN_ENTRY_COOLDOWN_HOURS: i64 = 2; // Reduced from 6 hours
pub const PROFITABLE_EXIT_COOLDOWN_HOURS: i64 = 4; // Reduced from 12 hours
pub const LOSS_EXIT_COOLDOWN_HOURS: i64 = 1; // Reduced from 2 hours

// ─── PROFITABLE EXIT RE-ENTRY CONTROLS ───
pub const MIN_PRICE_DROP_AFTER_PROFIT_PCT: f64 = 3.0; // Reduced from 8% for faster re-entries
pub const MAX_RECENT_EXITS_LOOKBACK_HOURS: i64 = 24; // Reduced from 72 hours
pub const MIN_PROFIT_EXIT_THRESHOLD_PCT: f64 = 5.0; // Only count bigger profits (was 3%)

// ─── LOSS CONTROL POLICY PARAMETERS ───
pub const FORBIDDEN_LOSS_ZONE_MIN: f64 = 0.0; // Upper bound of forbidden zone (breakeven)
pub const FORBIDDEN_LOSS_ZONE_MAX: f64 = -50.0; // Lower bound of forbidden zone
pub const CATASTROPHIC_LOSS_THRESHOLD: f64 = -50.0; // Threshold for catastrophic loss sales
pub const EMERGENCY_RUG_LOSS_THRESHOLD: f64 = -30.0; // Min loss for emergency rug override
pub const CATASTROPHIC_TIME_LIMIT_MINUTES: i64 = 4320; // 3 days max hold for catastrophic losses
