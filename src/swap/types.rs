use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

/// Standard swap request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub token_in_address: String,
    pub token_out_address: String,
    pub amount_in: u64,
    pub from_address: String,
    pub slippage_bps: u16,
    pub swap_type: SwapType,
    pub priority_fee_lamports: u64,
    pub is_anti_mev: bool,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of swap operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwapType {
    Buy, // SOL -> Token
    Sell, // Token -> SOL
}

/// Quote response from a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub provider_id: String,
    pub request_id: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub in_decimals: u8,
    pub out_decimals: u8,
    pub price_impact_bps: u16,
    pub fee_amount: u64,
    pub minimum_out_amount: u64,
    pub route_info: RouteInfo,
    pub estimated_gas: u64,
    pub valid_until: DateTime<Utc>,
    pub quote_time_ms: u64,
}

/// Information about the swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub route_steps: Vec<RouteStep>,
    pub total_fee_bps: u16,
    pub price_impact_pct: f64,
    pub liquidity_sources: Vec<String>,
}

/// Individual step in a swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    pub amm_id: String,
    pub amm_label: String,
    pub percent: u8,
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee_amount: u64,
}

/// Complete result of a swap operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub request: SwapRequest,
    pub quote: Option<SwapQuote>,
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub final_status: TransactionStatus,
    pub error_message: Option<String>,
    pub provider_id: String,

    // Execution metrics
    pub execution_time_ms: u64,
    pub confirmation_time_ms: Option<u64>,
    pub gas_used: Option<u64>,

    // On-chain results
    pub actual_amount_in: Option<u64>,
    pub actual_amount_out: Option<u64>,
    pub effective_price: Option<f64>,
    pub actual_price_impact_bps: Option<u16>,

    // Provider-specific data
    pub provider_data: HashMap<String, serde_json::Value>,
}

/// Status of a transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Expired,
    Unknown,
}

/// Error types for swap operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapError {
    InvalidTokenPair,
    InsufficientBalance,
    SlippageTooHigh,
    AmountTooSmall,
    AmountTooLarge,
    NetworkError(String),
    ProviderError(String),
    TransactionFailed(String),
    Timeout,
    Unknown(String),
}

impl SwapRequest {
    pub fn new_buy(
        token_out_address: &str,
        amount_in_lamports: u64,
        from_address: &str,
        slippage_bps: u16,
        priority_fee_lamports: u64
    ) -> Self {
        Self {
            token_in_address: "So11111111111111111111111111111111111111112".to_string(), // WSOL
            token_out_address: token_out_address.to_string(),
            amount_in: amount_in_lamports,
            from_address: from_address.to_string(),
            slippage_bps,
            swap_type: SwapType::Buy,
            priority_fee_lamports,
            is_anti_mev: false,
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn new_sell(
        token_in_address: &str,
        amount_in_tokens: u64,
        from_address: &str,
        slippage_bps: u16,
        priority_fee_lamports: u64
    ) -> Self {
        Self {
            token_in_address: token_in_address.to_string(),
            token_out_address: "So11111111111111111111111111111111111111112".to_string(), // WSOL
            amount_in: amount_in_tokens,
            from_address: from_address.to_string(),
            slippage_bps,
            swap_type: SwapType::Sell,
            priority_fee_lamports,
            is_anti_mev: false,
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }
}

impl SwapResult {
    pub fn new_success(
        request: SwapRequest,
        quote: SwapQuote,
        transaction_signature: String,
        provider_id: String,
        execution_time_ms: u64
    ) -> Self {
        Self {
            request,
            quote: Some(quote),
            success: true,
            transaction_signature: Some(transaction_signature),
            final_status: TransactionStatus::Pending,
            error_message: None,
            provider_id,
            execution_time_ms,
            confirmation_time_ms: None,
            gas_used: None,
            actual_amount_in: None,
            actual_amount_out: None,
            effective_price: None,
            actual_price_impact_bps: None,
            provider_data: HashMap::new(),
        }
    }

    pub fn new_error(
        request: SwapRequest,
        error: SwapError,
        provider_id: String,
        execution_time_ms: u64
    ) -> Self {
        Self {
            request,
            quote: None,
            success: false,
            transaction_signature: None,
            final_status: TransactionStatus::Failed,
            error_message: Some(format!("{:?}", error)),
            provider_id,
            execution_time_ms,
            confirmation_time_ms: None,
            gas_used: None,
            actual_amount_in: None,
            actual_amount_out: None,
            effective_price: None,
            actual_price_impact_bps: None,
            provider_data: HashMap::new(),
        }
    }

    pub fn set_confirmed(&mut self, confirmation_time_ms: u64) {
        self.final_status = TransactionStatus::Confirmed;
        self.confirmation_time_ms = Some(confirmation_time_ms);
    }

    pub fn set_failed(&mut self, error_message: String) {
        self.final_status = TransactionStatus::Failed;
        self.error_message = Some(error_message);
        self.success = false;
    }

    pub fn set_on_chain_results(
        &mut self,
        amount_in: u64,
        amount_out: u64,
        effective_price: f64,
        price_impact_bps: u16
    ) {
        self.actual_amount_in = Some(amount_in);
        self.actual_amount_out = Some(amount_out);
        self.effective_price = Some(effective_price);
        self.actual_price_impact_bps = Some(price_impact_bps);
    }

    pub fn print_summary(&self) {
        println!("🔄 SWAP SUMMARY");
        println!("  Type: {:?}", self.request.swap_type);
        println!("  Provider: {}", self.provider_id);
        println!("  Success: {}", self.success);
        println!("  Status: {:?}", self.final_status);

        if let Some(signature) = &self.transaction_signature {
            println!("  Signature: {}", signature);
        }

        if let Some(error) = &self.error_message {
            println!("  Error: {}", error);
        }

        if let Some(quote) = &self.quote {
            println!("  Quote Details:");
            println!("    In Amount: {} (decimals: {})", quote.in_amount, quote.in_decimals);
            println!("    Out Amount: {} (decimals: {})", quote.out_amount, quote.out_decimals);
            println!("    Price Impact: {:.2}%", quote.route_info.price_impact_pct);
            println!("    Route: {}", quote.route_info.liquidity_sources.join(" -> "));
        }

        if let Some(effective_price) = self.effective_price {
            println!("  Effective Price: {:.9} SOL per token", effective_price);
        }

        println!("  Execution Time: {}ms", self.execution_time_ms);
        if let Some(confirm_time) = self.confirmation_time_ms {
            println!("  Confirmation Time: {}ms", confirm_time);
        }
    }
}
