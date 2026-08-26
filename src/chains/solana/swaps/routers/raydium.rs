//! Raydium Router Implementation (Stub)
//! Placeholder for future Raydium direct swap support

use crate::config::with_config;
use crate::swaps::error::{QuoteError, QuoteResult};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use async_trait::async_trait;

// ============================================================================
// RAYDIUM ROUTER (DISABLED)
// ============================================================================

pub struct RaydiumRouter;

impl RaydiumRouter {
    /// Create a new Raydium swap router instance
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SwapRouter for RaydiumRouter {
    fn id(&self) -> &'static str {
        "raydium"
    }

    fn name(&self) -> &'static str {
        "Raydium"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.raydium.enabled)
    }

    fn priority(&self) -> u8 {
        2 // Tertiary priority (after Jupiter and GMGN)
    }

    fn chain(&self) -> crate::chains::ChainId {
        crate::chains::ChainId::Solana
    }

    async fn get_quote(&self, _request: &QuoteRequest) -> QuoteResult<Quote> {
        // Not implemented, so it can say nothing about the token — this must
        // stay a router-level fault and never count towards retiring a mint.
        Err(QuoteError::Unavailable {
            router: self.name().to_owned(),
            detail: "Raydium router not implemented yet".to_owned(),
        })
    }

    async fn execute_swap(&self, _token: &Token, quote: &Quote) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        Err(Error::internal_error("Raydium router not implemented yet"))
    }
}
