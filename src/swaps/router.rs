use crate::errors::ScreenerBotError;
use crate::tokens::Token;
/// Router Trait - Unified swap router interface
/// All swap routers (Jupiter, GMGN, Raydium) must implement this trait
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================================
// CORE TRAIT
// ============================================================================

/// Unified swap router interface
/// All routers must implement this trait to participate in the swap system
#[async_trait]
pub trait SwapRouter: Send + Sync {
    /// Router identifier (e.g., "jupiter", "gmgn", "raydium")
    fn id(&self) -> &'static str;

    /// Display name for logging/UI (e.g., "Jupiter", "GMGN", "Raydium")
    fn name(&self) -> &'static str;

    /// Check if router is enabled in config
    fn is_enabled(&self) -> bool;

    /// Fallback priority (lower = higher priority, 0 = primary)
    /// Used to determine fallback order when primary fails
    fn priority(&self) -> u8;

    /// Get quote from this router
    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError>;

    /// Execute swap using quote from this router
    async fn execute_swap(
        &self,
        token: &Token,
        quote: &Quote,
    ) -> Result<SwapResult, ScreenerBotError>;
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Quote request parameters (immutable, passed to all routers)
#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub wallet_address: String,
    pub slippage_pct: f64,
    pub swap_mode: SwapMode,
}

/// Swap mode enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

impl SwapMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwapMode::ExactIn => "ExactIn",
            SwapMode::ExactOut => "ExactOut",
        }
    }
}

/// Unified quote response (router-agnostic)
#[derive(Debug, Clone)]
pub struct Quote {
    pub router_id: String,
    pub router_name: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub swap_mode: SwapMode,
    pub wallet_address: String,
    pub execution_data: Vec<u8>, // Serialized router-specific data
}

/// Swap execution result (router-agnostic)
#[derive(Debug)]
pub struct SwapResult {
    pub success: bool,
    pub router_id: String,
    pub router_name: String,
    pub transaction_signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub execution_time_ms: u64,
    pub effective_price_sol: Option<f64>,
}

impl SwapResult {
    /// Create a failed swap result
    pub fn failed(router_id: String, router_name: String, error: String) -> Self {
        Self {
            success: false,
            router_id,
            router_name,
            transaction_signature: String::new(),
            input_amount: 0,
            output_amount: 0,
            price_impact_pct: 0.0,
            fee_lamports: 0,
            execution_time_ms: 0,
            effective_price_sol: None,
        }
    }
}
