/// Raydium Router Implementation (Stub)
/// Placeholder for future Raydium direct swap support
use crate::config::with_config;
use crate::errors::ScreenerBotError;
use crate::swaps::router::{Quote, QuoteRequest, SwapResult, SwapRouter};
use crate::tokens::Token;
use async_trait::async_trait;

// ============================================================================
// RAYDIUM ROUTER (DISABLED)
// ============================================================================

pub struct RaydiumRouter;

impl RaydiumRouter {
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

    async fn get_quote(&self, _request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        Err(ScreenerBotError::internal_error(
            "Raydium router not implemented yet",
        ))
    }

    async fn execute_swap(
        &self,
        _token: &Token,
        _quote: &Quote,
    ) -> Result<SwapResult, ScreenerBotError> {
        Err(ScreenerBotError::internal_error(
            "Raydium router not implemented yet",
        ))
    }
}
