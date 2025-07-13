// Trading constants
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// Default trading parameters
pub const DEFAULT_SLIPPAGE_BPS: u16 = 50; // 0.5%
pub const DEFAULT_PRIORITY_FEE: u64 = 1000; // microlamports
pub const DEFAULT_ENTRY_AMOUNT: f64 = 0.001; // SOL
pub const MAX_POSITIONS: u32 = 50;

// Cache settings
pub const DEFAULT_CACHE_TTL_HOURS: u64 = 6;
pub const TOKEN_METADATA_TTL_HOURS: u64 = 24;
pub const TRANSACTION_CACHE_TTL_HOURS: u64 = 48;
pub const PRICE_CACHE_TTL_MINUTES: u64 = 5;

// API rate limits
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: u32 = 300;
pub const GECKOTERMINAL_RATE_LIMIT_PER_MINUTE: u32 = 30;
pub const HELIUS_RATE_LIMIT_PER_SECOND: u32 = 10;

// Portfolio settings
pub const MIN_POSITION_VALUE_USD: f64 = 0.1;
pub const MAX_POSITION_AGE_DAYS: u32 = 30;

// Screener settings
pub const MIN_LIQUIDITY_USD: f64 = 5000.0;
pub const MIN_VOLUME_24H_USD: f64 = 1000.0;
pub const MAX_TOKEN_AGE_HOURS: u64 = 24;

// Risk management
pub const MAX_SLIPPAGE_PERCENT: f64 = 10.0;
pub const MIN_SUCCESS_RATE_PERCENT: f64 = 60.0;
pub const MAX_DRAWDOWN_PERCENT: f64 = 20.0;

// Database settings
pub const DB_CONNECTION_POOL_SIZE: u32 = 10;
pub const DB_QUERY_TIMEOUT_SECONDS: u64 = 30;

// Retry settings
pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_SECONDS: u64 = 5;
pub const EXPONENTIAL_BACKOFF_MULTIPLIER: f64 = 2.0;

// Display settings
pub const PORTFOLIO_DISPLAY_PRECISION: usize = 6;
pub const PRICE_DISPLAY_PRECISION: usize = 8;
pub const PERCENTAGE_DISPLAY_PRECISION: usize = 2;

// API endpoints
pub const DEXSCREENER_API_BASE: &str = "https://api.dexscreener.com/latest";
pub const GECKOTERMINAL_API_BASE: &str = "https://api.geckoterminal.com/api/v2";
pub const RUGCHECK_API_BASE: &str = "https://api.rugcheck.xyz/v1";
pub const RAYDIUM_API_BASE: &str = "https://api.raydium.io/v2";

// Console display colors
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_BLUE: &str = "\x1b[34m";
pub const COLOR_CYAN: &str = "\x1b[36m";
pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";

// Unicode symbols for display
pub const SYMBOL_CHECK: &str = "✅";
pub const SYMBOL_CROSS: &str = "❌";
pub const SYMBOL_WARNING: &str = "⚠️";
pub const SYMBOL_INFO: &str = "ℹ️";
pub const SYMBOL_ROCKET: &str = "🚀";
pub const SYMBOL_MONEY: &str = "💰";
pub const SYMBOL_CHART: &str = "📈";
pub const SYMBOL_FIRE: &str = "🔥";
pub const SYMBOL_EYES: &str = "👀";
pub const SYMBOL_ROBOT: &str = "🤖";
