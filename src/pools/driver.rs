// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The pricing driver: the chain-owned pipeline that discovers, reads and
//! prices pools, reached only through the chain runtime
//! (`ChainRuntime::pricing_driver`).
//!
//! The pools service initializes and clears each enabled chain's driver; the
//! stage services start one stage loop per enabled chain through
//! [`PricingDriver::start_stage`] and own the returned handles. Status and
//! pool-directory reads go through [`super::api`], which merges every chain in
//! the requested scope.

use std::fmt;
use std::sync::Arc;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::types::PoolDescriptor;
use super::Result;

/// One stage of a chain's pricing pipeline. Each stage runs as one loop per
/// enabled chain, under the service of the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PricingStage {
    Discovery,
    Analysis,
    Fetch,
    Calculation,
}

impl PricingStage {
    /// Stable lowercase name, used in logs and error text.
    pub const fn as_str(self) -> &'static str {
        match self {
            PricingStage::Discovery => "discovery",
            PricingStage::Analysis => "analysis",
            PricingStage::Fetch => "fetch",
            PricingStage::Calculation => "calculation",
        }
    }
}

impl fmt::Display for PricingStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a driver reports once its components are initialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingInit {
    /// The components the driver brought up, in pipeline order.
    pub components: &'static [&'static str],
    /// The RPC providers the driver reads through.
    pub rpc_urls_count: usize,
}

/// Raw counters of one stage loop. Ratios are derived from the counters after
/// they are summed across chains, never averaged per chain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PricingStageMetrics {
    pub operations: u64,
    pub errors: u64,
    /// Stage-specific counters under their metric keys.
    pub counters: Vec<(&'static str, u64)>,
}

/// The pools a driver has analyzed, counted per protocol label.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolDirectoryStatus {
    pub total_pools: usize,
    /// Sorted by count descending, then by label.
    pub pools_by_protocol: Vec<(String, usize)>,
}

/// The account bundles a driver's fetch stage holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountFetchStatus {
    pub total_bundles: usize,
    pub bundles_with_data: usize,
    pub total_accounts_tracked: usize,
}

/// A driver's pipeline state; a part is `None` while its stage is not
/// initialized.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PricingStatus {
    pub directory: Option<PoolDirectoryStatus>,
    pub fetch: Option<AccountFetchStatus>,
}

impl PricingStatus {
    /// Merge another chain's status into this one: counts are summed, and a
    /// part is present when either side has it.
    pub fn merge(self, other: PricingStatus) -> PricingStatus {
        PricingStatus {
            directory: merge_directory(self.directory, other.directory),
            fetch: match (self.fetch, other.fetch) {
                (Some(a), Some(b)) => Some(AccountFetchStatus {
                    total_bundles: a.total_bundles + b.total_bundles,
                    bundles_with_data: a.bundles_with_data + b.bundles_with_data,
                    total_accounts_tracked: a.total_accounts_tracked + b.total_accounts_tracked,
                }),
                (a, b) => a.or(b),
            },
        }
    }
}

fn merge_directory(
    a: Option<PoolDirectoryStatus>,
    b: Option<PoolDirectoryStatus>,
) -> Option<PoolDirectoryStatus> {
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        (a, b) => return a.or(b),
    };
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (label, count) in a.pools_by_protocol.into_iter().chain(b.pools_by_protocol) {
        *counts.entry(label).or_default() += count;
    }
    Some(PoolDirectoryStatus {
        total_pools: a.total_pools + b.total_pools,
        pools_by_protocol: sort_protocol_counts(counts.into_iter().collect()),
    })
}

/// Order protocol counts by count descending, then by label.
pub fn sort_protocol_counts(mut counts: Vec<(String, usize)>) -> Vec<(String, usize)> {
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

/// A chain's pool pricing pipeline.
#[async_trait::async_trait]
pub trait PricingDriver: Send + Sync + 'static {
    /// Build the pipeline components. Spawns nothing: the loops start only
    /// through [`PricingDriver::start_stage`].
    async fn initialize(&self) -> Result<PricingInit>;
    /// Spawn the stage loop under `shutdown` and `monitor` and return its
    /// handle; never awaits the loop. `Ok(None)` means this chain has no such
    /// stage. A stage that is not initialized, or whose loop was already
    /// started since the last initialize, is an error.
    fn start_stage(
        &self,
        stage: PricingStage,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Option<JoinHandle<()>>>;
    /// Whether the stage's component is initialized.
    fn stage_ready(&self, stage: PricingStage) -> bool;
    /// The stage's counters; `None` while it is not initialized.
    fn stage_metrics(&self, stage: PricingStage) -> Option<PricingStageMetrics>;
    /// Release the pipeline components.
    fn clear(&self);
    /// The pools known for `mint`, its selected pricing pool first.
    fn token_pools(&self, mint: &str) -> Vec<PoolDescriptor>;
    /// The stable protocol slug of a known pool of `mint`.
    fn pool_protocol(&self, mint: &str, pool: &str) -> Option<&'static str>;
    /// The pipeline's current state.
    fn status(&self) -> PricingStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(total: usize, counts: &[(&str, usize)]) -> PoolDirectoryStatus {
        PoolDirectoryStatus {
            total_pools: total,
            pools_by_protocol: counts.iter().map(|(l, c)| ((*l).to_owned(), *c)).collect(),
        }
    }

    #[test]
    fn merging_with_an_empty_status_is_the_identity() {
        let status = PricingStatus {
            directory: Some(directory(3, &[("b", 2), ("a", 1)])),
            fetch: Some(AccountFetchStatus {
                total_bundles: 2,
                bundles_with_data: 1,
                total_accounts_tracked: 7,
            }),
        };
        assert_eq!(
            PricingStatus::default().merge(status.clone()),
            status.clone()
        );
        assert_eq!(status.clone().merge(PricingStatus::default()), status);
    }

    #[test]
    fn merging_sums_counts_and_reorders_protocols() {
        let a = PricingStatus {
            directory: Some(directory(3, &[("x", 2), ("y", 1)])),
            fetch: None,
        };
        let b = PricingStatus {
            directory: Some(directory(4, &[("y", 3), ("z", 1)])),
            fetch: None,
        };
        let merged = a.merge(b);
        assert_eq!(
            merged.directory,
            Some(directory(7, &[("y", 4), ("x", 2), ("z", 1)]))
        );
        assert_eq!(merged.fetch, None);
    }
}
