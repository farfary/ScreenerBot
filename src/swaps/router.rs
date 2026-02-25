//! Router trait — defines the unified interface for all DEX swap routers.

use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::Result;
use async_trait::async_trait;

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
    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote>;

    /// Execute swap using quote from this router
    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult>;
}
