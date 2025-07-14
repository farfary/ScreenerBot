use solana_sdk::pubkey::Pubkey;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use chrono::{ DateTime, Utc };

// Liquidity provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiquidityProvider {
    Raydium,
    Orca,
    Jupiter,
    Meteora,
    PumpFun,
    Other(String),
}

/// Risk level enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: f64,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub price_usd: Option<f64>,
    pub value_usd: Option<f64>,
    pub last_updated: DateTime<Utc>,
}

/// Wallet transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub signature: String,
    pub block_time: Option<i64>,
    pub slot: u64,
    pub transaction_type: TransactionType,
    pub tokens_involved: Vec<Pubkey>,
    pub sol_change: i64,
    pub token_changes: HashMap<Pubkey, i64>,
    pub fees: u64,
    pub status: TransactionStatus,
    pub parsed_data: Option<ParsedTransactionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Buy,
    Sell,
    Transfer,
    Swap,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failed,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTransactionData {
    pub input_token: Option<Pubkey>,
    pub output_token: Option<Pubkey>,
    pub input_amount: Option<u64>,
    pub output_amount: Option<u64>,
    pub price_per_token: Option<f64>,
    pub pool_address: Option<Pubkey>,
    pub dex: Option<String>,
}

/// Token opportunity from screener
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenOpportunity {
    pub mint: Pubkey,
    pub token: TokenInfo, // Added for backward compatibility
    pub symbol: String,
    pub name: String,
    pub source: ScreenerSource,
    pub discovery_time: DateTime<Utc>,
    pub metrics: TokenMetrics,
    pub verification_status: VerificationStatus,
    pub risk_score: f64,
    pub confidence_score: f64,
    pub liquidity_provider: LiquidityProvider,
    pub social_metrics: Option<SocialMetrics>,
    pub risk_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScreenerSource {
    DexScreener,
    GeckoTerminal,
    Raydium,
    RugCheck,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub price_usd: f64,
    pub volume_24h: f64,
    pub liquidity_usd: f64,
    pub market_cap: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub age_hours: f64,
    pub holder_count: Option<u64>,
    pub top_10_holder_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStatus {
    pub is_verified: bool,
    pub has_profile: bool,
    pub is_boosted: bool,
    pub rugcheck_score: Option<f64>,
    pub security_flags: Vec<String>,
    pub has_socials: bool,
    pub contract_verified: bool,
}

/// Trading signal generated from analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub token: Pubkey,
    pub signal_type: SignalType,
    pub strength: f64,
    pub recommended_amount: f64,
    pub max_slippage: f64,
    pub generated_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub analysis_data: TradeAnalysis,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SignalType {
    Buy,
    Sell,
    DCA,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAnalysis {
    pub entry_reason: String,
    pub technical_indicators: HashMap<String, f64>,
    pub fundamental_score: f64,
    pub risk_assessment: RiskAssessment,
    pub expected_return: f64,
    pub time_horizon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub liquidity_risk: RiskLevel,
    pub volatility_risk: RiskLevel,
    pub concentration_risk: RiskLevel,
    pub smart_money_risk: RiskLevel,
}

/// Portfolio position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token: Pubkey,
    pub symbol: String,
    pub total_amount: u64,
    pub average_entry_price: f64,
    pub current_price: f64,
    pub total_invested_sol: f64,
    pub current_value_sol: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_percentage: f64,
    pub first_buy_time: DateTime<Utc>,
    pub last_buy_time: DateTime<Utc>,
    pub trade_count: u32,
    pub dca_opportunities: u32,
}

/// Complete portfolio summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub total_value_sol: f64,
    pub total_invested_sol: f64,
    pub total_unrealized_pnl: f64,
    pub total_unrealized_pnl_percentage: f64,
    pub sol_balance: f64,
    pub positions: Vec<Position>,
    pub performance_metrics: PerformanceMetrics,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub win_rate: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub best_trade_pnl: f64,
    pub worst_trade_pnl: f64,
    pub average_trade_duration_hours: f64,
}

/// Trade execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub transaction_id: String,
    pub trade_type: SignalType,
    pub token: Pubkey,
    pub amount_sol: f64,
    pub amount_token: u64,
    pub price_per_token: f64,
    pub slippage_actual: f64,
    pub fees_paid: u64,
    pub executed_at: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
    pub gas_used: u64,
    pub pool_used: Option<Pubkey>,
}

impl Default for TradeResult {
    fn default() -> Self {
        Self {
            transaction_id: String::new(),
            trade_type: SignalType::Buy,
            token: Pubkey::default(),
            amount_sol: 0.0,
            amount_token: 0,
            price_per_token: 0.0,
            slippage_actual: 0.0,
            fees_paid: 0,
            executed_at: Utc::now(),
            success: false,
            error_message: None,
            gas_used: 0,
            pool_used: None,
        }
    }
}

/// Market data for tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub mint: Pubkey,
    pub symbol: String,
    pub price_usd: f64,
    pub volume_24h: f64,
    pub liquidity: f64,
    pub market_cap: Option<f64>,
    pub price_change_1h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub price_change_7d: Option<f64>,
    pub all_time_high: Option<f64>,
    pub all_time_low: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub data_source: String,
}

/// Cache entry for storing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl_hours: u64) -> Self {
        let now = Utc::now();
        Self {
            data,
            cached_at: now,
            expires_at: now + chrono::Duration::hours(ttl_hours as i64),
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn access(&mut self) -> &T {
        self.access_count += 1;
        self.last_accessed = Utc::now();
        &self.data
    }
}

/// Portfolio health and rebalancing types
#[derive(Debug, Clone)]
pub struct PortfolioHealth {
    pub total_value_sol: f64,
    pub total_invested_sol: f64,
    pub total_unrealized_pnl: f64,
    pub total_pnl_percentage: f64,
    pub positions_count: usize,
    pub profitable_positions: usize,
    pub losing_positions: usize,
    pub largest_position_percentage: f64,
    pub portfolio_concentration_risk: String,
    pub health_score: u8,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RebalanceRecommendation {
    pub token: Pubkey,
    pub symbol: String,
    pub action: RebalanceAction,
    pub reason: String,
    pub current_percentage: f64,
    pub target_percentage: f64,
    pub amount_sol: f64,
    pub priority: String,
}

#[derive(Debug, Clone)]
pub enum RebalanceAction {
    DCA,
    TakeProfit,
    Reduce,
    Increase,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMetrics {
    pub twitter_followers: Option<u64>,
    pub telegram_members: Option<u64>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub telegram_url: Option<String>,
}
