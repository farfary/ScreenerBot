/// Router Registry - Manages all available swap routers
/// Provides router discovery, fallback chains, and global access
use crate::swaps::router::SwapRouter;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// ============================================================================
// ROUTER REGISTRY
// ============================================================================

/// Global router registry
/// Manages all available swap routers and provides fallback chains
pub struct RouterRegistry {
    routers: Vec<Arc<dyn SwapRouter>>,
}

impl RouterRegistry {
    /// Create registry with all routers
    /// Add new routers here - this is the ONLY place that needs changing
    pub fn new() -> Self {
        use crate::swaps::routers::{GmgnRouter, JupiterRouter, RaydiumRouter};

        Self {
            routers: vec![
                Arc::new(JupiterRouter::new()),
                Arc::new(GmgnRouter::new()),
                Arc::new(RaydiumRouter::new()),
                // Add new routers here - ONLY change needed to add router
            ],
        }
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
static REGISTRY: OnceCell<RouterRegistry> = OnceCell::new();

/// Get global router registry
/// Initializes on first access
pub fn get_registry() -> &'static RouterRegistry {
    REGISTRY.get_or_init(|| RouterRegistry::new())
}

/// Reset registry (for testing only)
#[cfg(test)]
pub fn reset_registry() {
    // Cannot reset OnceCell in production - only for tests
    // This is intentional to prevent runtime registry changes
}
