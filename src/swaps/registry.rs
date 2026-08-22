//! Router Registry - Manages all available swap routers
//! Provides router discovery, fallback chains, and global access
//!
//! Chain-neutral: this registry holds `Arc<dyn SwapRouter>` injected by the
//! application composition root (`src/run/services.rs`, via
//! `set_router_factory`) — it never imports `crate::chains::solana`. The
//! concrete Solana router set lives in
//! `crate::chains::solana::swaps::routers::build_routers`.

use crate::chains::{active_chain, ChainId};
use crate::swaps::router::SwapRouter;
use std::sync::Arc;
use std::sync::OnceLock;

// ============================================================================
// ROUTER REGISTRY
// ============================================================================

/// Global router registry
/// Manages all available swap routers and provides fallback chains
pub struct RouterRegistry {
    routers: Vec<Arc<dyn SwapRouter>>,
}

impl RouterRegistry {
    /// Create a registry from an already-built router set. Chain selection
    /// happens at the call site (the composition root's registered factory),
    /// not here.
    pub fn new(routers: Vec<Arc<dyn SwapRouter>>) -> Self {
        Self { routers }
    }

    /// Get all enabled routers for the currently active chain.
    /// Thin delegate over [`Self::enabled_routers_for`] kept so existing call
    /// sites keep compiling and behaving identically today (single-chain).
    pub fn enabled_routers(&self) -> Vec<Arc<dyn SwapRouter>> {
        self.enabled_routers_for(active_chain())
    }

    /// Get all enabled routers that serve `chain`. A router for a different
    /// chain never appears here, regardless of registration order.
    pub fn enabled_routers_for(&self, chain: ChainId) -> Vec<Arc<dyn SwapRouter>> {
        self.routers
            .iter()
            .filter(|r| r.is_enabled() && r.chain() == chain)
            .cloned()
            .collect()
    }

    /// Get router by ID
    pub fn get_router(&self, id: &str) -> Option<Arc<dyn SwapRouter>> {
        self.routers.iter().find(|r| r.id() == id).cloned()
    }

    /// Get fallback chain for failed router, scoped to the active chain.
    /// Thin delegate over [`Self::get_fallback_chain_for`].
    pub fn get_fallback_chain(&self, failed_router_id: &str) -> Vec<Arc<dyn SwapRouter>> {
        self.get_fallback_chain_for(active_chain(), failed_router_id)
    }

    /// Get fallback routers for `chain`, excluding the failed router.
    /// Returns routers sorted by priority; a router for a different chain
    /// never appears in the fallback chain.
    pub fn get_fallback_chain_for(
        &self,
        chain: ChainId,
        failed_router_id: &str,
    ) -> Vec<Arc<dyn SwapRouter>> {
        let mut fallbacks: Vec<_> = self
            .routers
            .iter()
            .filter(|r| r.is_enabled() && r.chain() == chain && r.id() != failed_router_id)
            .cloned()
            .collect();

        fallbacks.sort_by_key(|r| r.priority());
        fallbacks
    }

    /// Check if any router is enabled for the currently active chain.
    /// Thin delegate over [`Self::has_enabled_routers_for`].
    pub fn has_enabled_routers(&self) -> bool {
        self.has_enabled_routers_for(active_chain())
    }

    /// Check if any router serving `chain` is enabled.
    pub fn has_enabled_routers_for(&self, chain: ChainId) -> bool {
        self.routers
            .iter()
            .any(|r| r.is_enabled() && r.chain() == chain)
    }

    /// Get primary router (lowest priority number among enabled routers) for
    /// the currently active chain. Thin delegate over
    /// [`Self::get_primary_router_for`].
    pub fn get_primary_router(&self) -> Option<Arc<dyn SwapRouter>> {
        self.get_primary_router_for(active_chain())
    }

    /// Get the primary router for `chain` — the lowest-`priority()` ENABLED
    /// router serving that chain, never the first registered.
    pub fn get_primary_router_for(&self, chain: ChainId) -> Option<Arc<dyn SwapRouter>> {
        self.enabled_routers_for(chain)
            .into_iter()
            .min_by_key(|r| r.priority())
    }

    /// Get all routers (enabled and disabled)
    pub fn all_routers(&self) -> &[Arc<dyn SwapRouter>] {
        &self.routers
    }
}

// ============================================================================
// GLOBAL REGISTRY INSTANCE
// ============================================================================

/// Global registry instance (lazy initialized)
static REGISTRY: OnceLock<RouterRegistry> = OnceLock::new();

/// Factory that builds the concrete router set, registered once by the
/// application composition root before any swap activity can occur.
static ROUTER_FACTORY: OnceLock<fn() -> Vec<Arc<dyn SwapRouter>>> = OnceLock::new();

/// Register the chain-owned router factory. Must be called once during boot
/// (see `crate::run::services::register_all_services`) before `get_registry()`
/// is ever called.
pub fn set_router_factory(factory: fn() -> Vec<Arc<dyn SwapRouter>>) {
    let _ = ROUTER_FACTORY.set(factory);
}

/// Fallible global registry access. Returns `None` when the composition root
/// has not registered a router factory — integration tests, library callers,
/// and startup paths that quote before boot. Never panics.
pub fn try_get_registry() -> Option<&'static RouterRegistry> {
    if let Some(registry) = REGISTRY.get() {
        return Some(registry);
    }
    let factory = ROUTER_FACTORY.get()?;
    let _ = REGISTRY.set(RouterRegistry::new(factory()));
    REGISTRY.get()
}

/// Get the global router registry, initializing it from the registered factory
/// on first access. Returns a structured service-init error when the factory
/// has not been registered — never panics.
pub fn get_registry() -> crate::Result<&'static RouterRegistry> {
    try_get_registry().ok_or_else(|| {
        crate::Error::Service(crate::errors::ServiceError::Initialize {
            service: "swaps.registry".to_owned(),
            message: "router factory has not been registered".to_owned(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
    use crate::tokens::Token;
    use crate::Result;
    use async_trait::async_trait;

    struct StubRouter {
        id: &'static str,
        enabled: bool,
        priority: u8,
    }

    #[async_trait]
    impl SwapRouter for StubRouter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            self.id
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn priority(&self) -> u8 {
            self.priority
        }
        fn chain(&self) -> ChainId {
            crate::chains::active_chain()
        }
        async fn get_quote(&self, _request: &QuoteRequest) -> Result<Quote> {
            Err(crate::Error::api_error("stub"))
        }
        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> Result<SwapResult> {
            Err(crate::Error::api_error("stub"))
        }
    }

    fn stub(id: &'static str, enabled: bool, priority: u8) -> Arc<dyn SwapRouter> {
        Arc::new(StubRouter {
            id,
            enabled,
            priority,
        })
    }

    #[test]
    fn primary_router_is_the_enabled_router_with_lowest_priority() {
        let registry = RouterRegistry::new(vec![
            stub("gmgn", true, 1),
            stub("jupiter", true, 0),
            stub("raydium", true, 2),
        ]);
        let primary = registry.get_primary_router().expect("enabled routers");
        assert_eq!(primary.id(), "jupiter");
    }

    #[test]
    fn primary_router_skips_disabled_routers_regardless_of_registration_order() {
        let registry = RouterRegistry::new(vec![
            stub("jupiter", false, 0),
            stub("gmgn", true, 1),
            stub("raydium", false, 2),
        ]);
        let primary = registry.get_primary_router().expect("gmgn enabled");
        assert_eq!(primary.id(), "gmgn");
        let enabled: Vec<_> = registry.enabled_routers().iter().map(|r| r.id()).collect();
        assert_eq!(enabled, vec!["gmgn"]);
    }

    #[test]
    fn primary_selection_is_deterministic_across_shuffled_registration() {
        let orders = [
            vec![
                stub("raydium", true, 2),
                stub("gmgn", true, 1),
                stub("jupiter", true, 0),
            ],
            vec![
                stub("gmgn", true, 1),
                stub("jupiter", true, 0),
                stub("raydium", true, 2),
            ],
            vec![
                stub("jupiter", true, 0),
                stub("raydium", true, 2),
                stub("gmgn", true, 1),
            ],
        ];
        for routers in orders {
            let registry = RouterRegistry::new(routers);
            assert_eq!(
                registry.get_primary_router().expect("enabled").id(),
                "jupiter"
            );
        }
    }

    #[test]
    fn fallback_chain_is_enabled_routers_by_priority_excluding_failed() {
        let registry = RouterRegistry::new(vec![
            stub("jupiter", true, 0),
            stub("gmgn", true, 1),
            stub("raydium", false, 2),
        ]);
        let chain: Vec<_> = registry
            .get_fallback_chain("jupiter")
            .iter()
            .map(|r| r.id())
            .collect();
        assert_eq!(chain, vec!["gmgn"]);
    }
}
