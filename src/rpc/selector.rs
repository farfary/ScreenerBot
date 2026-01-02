//! Provider selection strategies
//!
//! Provides different algorithms for selecting the next RPC provider:
//! - RoundRobin: Distribute load evenly across providers
//! - Priority: Always use highest priority available provider
//! - LatencyBased: Route to lowest latency provider
//! - Adaptive: Combine health, latency, and error rate

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::rpc::provider::ProviderConfig;
use crate::rpc::types::{ProviderState, SelectionStrategy};

// ============================================================
// Provider Selector Trait
// ============================================================

/// Trait for provider selection algorithms
pub trait ProviderSelector: Send + Sync {
    /// Select a provider from available options
    ///
    /// # Arguments
    /// * `providers` - Available provider configurations
    /// * `states` - Current runtime states of providers
    /// * `excluded` - Provider IDs to exclude (already tried)
    ///
    /// # Returns
    /// Reference to selected provider config, or None if no providers available
    fn select<'a>(
        &self,
        providers: &'a [ProviderConfig],
        states: &HashMap<String, ProviderState>,
        excluded: &[String],
    ) -> Option<&'a ProviderConfig>;

    /// Get the strategy type
    fn strategy(&self) -> SelectionStrategy;
}

// ============================================================
// Round Robin Selector
// ============================================================

/// Round-robin selector distributes load evenly across healthy providers
pub struct RoundRobinSelector {
    index: AtomicUsize,
}

impl RoundRobinSelector {
    /// Create new round-robin selector
    pub fn new() -> Self {
        Self {
            index: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSelector for RoundRobinSelector {
    fn select<'a>(
        &self,
        providers: &'a [ProviderConfig],
        states: &HashMap<String, ProviderState>,
        excluded: &[String],
    ) -> Option<&'a ProviderConfig> {
        let available: Vec<_> = providers
            .iter()
            .filter(|p| {
                p.enabled
                    && !excluded.contains(&p.id)
                    && states.get(&p.id).map(|s| s.is_healthy()).unwrap_or(true)
            })
            .collect();

        if available.is_empty() {
            // Fallback: try any enabled provider not in excluded list
            return providers
                .iter()
                .find(|p| p.enabled && !excluded.contains(&p.id));
        }

        let idx = self.index.fetch_add(1, Ordering::SeqCst) % available.len();
        available.get(idx).copied()
    }

    fn strategy(&self) -> SelectionStrategy {
        SelectionStrategy::RoundRobin
    }
}

// ============================================================
// Priority Selector
// ============================================================

/// Priority-based selector always uses the highest priority healthy provider
pub struct PrioritySelector;

impl PrioritySelector {
    /// Create new priority selector
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrioritySelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSelector for PrioritySelector {
    fn select<'a>(
        &self,
        providers: &'a [ProviderConfig],
        states: &HashMap<String, ProviderState>,
        excluded: &[String],
    ) -> Option<&'a ProviderConfig> {
        // Filter to healthy, enabled, non-excluded providers
        providers
            .iter()
            .filter(|p| {
                p.enabled
                    && !excluded.contains(&p.id)
                    && states.get(&p.id).map(|s| s.is_healthy()).unwrap_or(true)
            })
            .min_by_key(|p| p.priority)
    }

    fn strategy(&self) -> SelectionStrategy {
        SelectionStrategy::Priority
    }
}

// ============================================================
// Latency Selector
// ============================================================

/// Latency-based selector routes to the lowest latency healthy provider
pub struct LatencySelector;

impl LatencySelector {
    /// Create new latency selector
    pub fn new() -> Self {
        Self
    }
}

impl Default for LatencySelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSelector for LatencySelector {
    fn select<'a>(
        &self,
        providers: &'a [ProviderConfig],
        states: &HashMap<String, ProviderState>,
        excluded: &[String],
    ) -> Option<&'a ProviderConfig> {
        providers
            .iter()
            .filter(|p| {
                p.enabled
                    && !excluded.contains(&p.id)
                    && states.get(&p.id).map(|s| s.is_healthy()).unwrap_or(true)
            })
            .min_by(|a, b| {
                let lat_a = states
                    .get(&a.id)
                    .map(|s| s.avg_latency_ms)
                    .unwrap_or(f64::MAX);
                let lat_b = states
                    .get(&b.id)
                    .map(|s| s.avg_latency_ms)
                    .unwrap_or(f64::MAX);
                lat_a
                    .partial_cmp(&lat_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    fn strategy(&self) -> SelectionStrategy {
        SelectionStrategy::LatencyBased
    }
}

// ============================================================
// Adaptive Selector
// ============================================================

/// Adaptive selector combines multiple factors for intelligent routing
///
/// Scoring:
/// - Success rate: 0-70 points (70% weight)
/// - Latency: 0-20 points (inverse, 20% weight)
/// - Priority: 0-10 points (lower priority = higher score, 10% weight)
pub struct AdaptiveSelector;

impl AdaptiveSelector {
    /// Create new adaptive selector
    pub fn new() -> Self {
        Self
    }

    /// Calculate provider score for selection
    ///
    /// Higher score = better provider
    fn score(state: Option<&ProviderState>, priority: u8) -> f64 {
        let state = match state {
            Some(s) => s,
            None => return 50.0, // Default score for unknown providers
        };

        // Success rate component: 0-70 points
        let success_score = state.success_rate() * 0.7;

        // Latency component: 0-20 points (inverse - lower latency = higher score)
        let latency_score = if state.avg_latency_ms > 0.0 {
            (1000.0 / state.avg_latency_ms).min(20.0)
        } else {
            20.0 // Max score if no latency data yet
        };

        // Priority component: 0-10 points (lower priority number = higher score)
        // Priority 0 -> 10 points, Priority 255 -> 0 points
        let priority_score = 10.0 - (priority as f64 / 25.5);

        success_score + latency_score + priority_score
    }
}

impl Default for AdaptiveSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSelector for AdaptiveSelector {
    fn select<'a>(
        &self,
        providers: &'a [ProviderConfig],
        states: &HashMap<String, ProviderState>,
        excluded: &[String],
    ) -> Option<&'a ProviderConfig> {
        providers
            .iter()
            .filter(|p| {
                p.enabled
                    && !excluded.contains(&p.id)
                    && states.get(&p.id).map(|s| s.is_healthy()).unwrap_or(true)
            })
            .max_by(|a, b| {
                let score_a = Self::score(states.get(&a.id), a.priority);
                let score_b = Self::score(states.get(&b.id), b.priority);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    fn strategy(&self) -> SelectionStrategy {
        SelectionStrategy::Adaptive
    }
}

// ============================================================
// Factory Function
// ============================================================

/// Create selector from strategy enum
pub fn create_selector(strategy: SelectionStrategy) -> Box<dyn ProviderSelector> {
    match strategy {
        SelectionStrategy::RoundRobin => Box::new(RoundRobinSelector::new()),
        SelectionStrategy::Priority => Box::new(PrioritySelector::new()),
        SelectionStrategy::LatencyBased => Box::new(LatencySelector::new()),
        SelectionStrategy::Adaptive => Box::new(AdaptiveSelector::new()),
    }
}
