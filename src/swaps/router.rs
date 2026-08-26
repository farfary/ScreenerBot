//! Router trait — defines the unified interface for all DEX swap routers.

use crate::chains::ChainId;
use crate::swaps::error::QuoteResult;
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use async_trait::async_trait;

// ============================================================================
// CORE TRAIT
// ============================================================================

/// Unified swap router interface
/// All routers must implement this trait to participate in the swap system
#[async_trait]
pub trait SwapRouter: Send + Sync {
    /// Router identifier (e.g., "jupiter", "raydium")
    fn id(&self) -> &'static str;

    /// Display name for logging/UI (e.g., "Jupiter", "Raydium")
    fn name(&self) -> &'static str;

    /// Check if router is enabled in config
    fn is_enabled(&self) -> bool;

    /// Fallback priority (lower = higher priority, 0 = primary)
    /// Used to determine fallback order when primary fails
    fn priority(&self) -> u8;

    /// The chain this router serves. Selection is scoped to this chain —
    /// see `RouterRegistry::enabled_routers_for` and friends.
    fn chain(&self) -> ChainId;

    /// Get quote from this router.
    ///
    /// The router classifies its own failure into [`QuoteError`] here, while it
    /// still holds the HTTP status and the decoded provider body. Everything
    /// downstream — whether to blacklist the token, whether to back off, what
    /// the trade dialog says — reads the variant, never the message.
    async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote>;

    /// Execute swap using quote from this router
    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult>;

    /// Refuse a quote produced by a different router. Router-specific
    /// `execution_data` must never be submitted through another adapter.
    fn accept_own_quote(&self, quote: &Quote) -> Result<()> {
        if quote.router_id != self.id() {
            Err(Error::internal_error(format!(
                "Quote from router '{}' cannot be executed by '{}'",
                quote.router_id,
                self.id()
            )))
        } else {
            Ok(())
        }
    }

    /// Refuse a request for a chain this router does not serve. A router must
    /// never quote or execute for a chain other than its own, even if it was
    /// (incorrectly) handed the request.
    fn accept_own_chain(&self, request: &QuoteRequest) -> Result<()> {
        if request.chain != self.chain() {
            Err(Error::internal_error(format!(
                "Request for chain '{}' cannot be routed to '{}' which serves '{}'",
                request.chain,
                self.id(),
                self.chain()
            )))
        } else {
            Ok(())
        }
    }

    /// Execute this router's own quote, signing for a specific wallet id.
    ///
    /// Chain adapters resolve and sign inside `src/chains/solana`. Routers
    /// that cannot do wallet-scoped execution must return
    /// [`Error::unsupported_capability`] and must not submit anything, and
    /// must never fall back to another router.
    async fn execute_swap_for_wallet(&self, quote: &Quote, _wallet_id: i64) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        Err(Error::unsupported_capability(
            "wallet_scoped_execution",
            self.id(),
        ))
    }
}
