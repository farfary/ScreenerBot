//! Router Registry - Manages all available swap routers
//! Provides router discovery, fallback chains, and global access
//!
//! Chain-neutral: this registry holds `Arc<dyn SwapRouter>` injected by the
//! application composition root (`src/run/services.rs`, via
//! `set_router_factory`) — it never imports `crate::chains::solana`. The
//! concrete Solana router set lives in
//! `crate::chains::solana::swaps::routers::build_routers`.

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

    /// Get all enabled routers
    pub fn enabled_routers(&self) -> Vec<Arc<dyn SwapRouter>> {
        self.routers
            .iter()
            .filter(|r| r.is_enabled())
            .cloned()
            .collect()
    }

    /// Get router by ID
    pub fn get_router(&self, id: &str) -> Option<Arc<dyn SwapRouter>> {
        self.routers.iter().find(|r| r.id() == id).cloned()
    }

    /// Get fallback chain for failed router
    /// Returns routers sorted by priority (excluding failed router)
    pub fn get_fallback_chain(&self, failed_router_id: &str) -> Vec<Arc<dyn SwapRouter>> {
        let mut fallbacks: Vec<_> = self
            .routers
            .iter()
            .filter(|r| r.is_enabled() && r.id() != failed_router_id)
            .cloned()
            .collect();

        fallbacks.sort_by_key(|r| r.priority());
        fallbacks
    }

    /// Check if any router is enabled
    pub fn has_enabled_routers(&self) -> bool {
        self.routers.iter().any(|r| r.is_enabled())
    }

    /// Get primary router (lowest priority number among enabled routers)
    pub fn get_primary_router(&self) -> Option<Arc<dyn SwapRouter>> {
        self.enabled_routers()
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

/// Get global router registry
/// Initializes on first access, using the registered router factory.
pub fn get_registry() -> &'static RouterRegistry {
    REGISTRY.get_or_init(|| {
        let factory = ROUTER_FACTORY
            .get()
            .expect("swaps::registry::set_router_factory must be called before get_registry()");
        RouterRegistry::new(factory())
    })
}
