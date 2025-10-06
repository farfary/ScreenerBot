// Pool manager for multi-pool support and failover

use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::types::{ OhlcvError, OhlcvResult, PoolConfig, PoolMetadata };
use std::sync::Arc;

pub struct PoolManager {
    db: Arc<OhlcvDatabase>,
}

impl PoolManager {
    pub fn new(db: Arc<OhlcvDatabase>) -> Self {
        Self { db }
    }

    /// Register a pool for a token
    pub async fn register_pool(
        &self,
        mint: &str,
        pool_address: &str,
        dex: &str,
        liquidity: f64
    ) -> OhlcvResult<()> {
        let pool = PoolConfig::new(pool_address.to_string(), dex.to_string(), liquidity);
        self.db.upsert_pool(mint, &pool)?;
        Ok(())
    }

    /// Get all pools for a token
    pub async fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        self.db.get_pools(mint)
    }

    /// Get the default pool for a token
    pub async fn get_default_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;
        Ok(pools.into_iter().find(|p| p.is_default))
    }

    /// Get the best available pool (highest liquidity, healthy)
    pub async fn get_best_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;

        // Find highest liquidity pool that's healthy
        let best = pools
            .into_iter()
            .filter(|p| p.is_healthy())
            .max_by(|a, b| {
                a.liquidity.partial_cmp(&b.liquidity).unwrap_or(std::cmp::Ordering::Equal)
            });

        Ok(best)
    }

    /// Set a pool as default
    pub async fn set_default_pool(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let mut pools = self.db.get_pools(mint)?;

        for pool in &mut pools {
            pool.is_default = pool.address == pool_address;
            self.db.upsert_pool(mint, pool)?;
        }

        Ok(())
    }

    /// Mark a pool as failed
    pub async fn mark_failure(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_failure(mint, pool_address)?;

        // Check if we need to switch default
        let default_pool = self.get_default_pool(mint).await?;
        if let Some(pool) = default_pool {
            if pool.address == pool_address && !pool.is_healthy() {
                // Switch to best alternative
                if let Some(best) = self.get_best_pool(mint).await? {
                    self.set_default_pool(mint, &best.address).await?;
                }
            }
        }

        Ok(())
    }

    /// Mark a pool as successful
    pub async fn mark_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_success(mint, pool_address)
    }

    /// Select the best pool for fetching
    /// Returns (pool_address, should_set_as_default)
    pub async fn select_pool_for_fetch(&self, mint: &str) -> OhlcvResult<Option<(String, bool)>> {
        // Try default pool first
        if let Some(default) = self.get_default_pool(mint).await? {
            if default.is_healthy() {
                return Ok(Some((default.address, false)));
            }
        }

        // Fall back to best pool
        if let Some(best) = self.get_best_pool(mint).await? {
            return Ok(Some((best.address, true))); // Should set as new default
        }

        Ok(None)
    }

    /// Discover and register pools from DexScreener
    pub async fn discover_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        // This would integrate with DexScreener API to find pools
        // For now, return empty - implement when DexScreener integration is ready
        Ok(Vec::new())
    }

    /// Get pool metadata for API responses
    pub async fn get_pool_metadata(&self, mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
        let pools = self.get_pools(mint).await?;
        Ok(pools.iter().map(PoolMetadata::from).collect())
    }

    /// Health check all pools for a token
    pub async fn check_pool_health(&self, mint: &str) -> OhlcvResult<Vec<(String, bool)>> {
        let pools = self.get_pools(mint).await?;
        Ok(
            pools
                .into_iter()
                .map(|p| {
                    let address = p.address.clone();
                    (address, p.is_healthy())
                })
                .collect()
        )
    }

    /// Reset failure count for a pool (for manual recovery)
    pub async fn reset_pool_failures(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_success(mint, pool_address)
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self, mint: &str) -> OhlcvResult<PoolStats> {
        let pools = self.get_pools(mint).await?;

        let total_pools = pools.len();
        let healthy_pools = pools
            .iter()
            .filter(|p| p.is_healthy())
            .count();
        let total_liquidity: f64 = pools
            .iter()
            .map(|p| p.liquidity)
            .sum();
        let has_default = pools.iter().any(|p| p.is_default);

        Ok(PoolStats {
            total_pools,
            healthy_pools,
            total_liquidity,
            has_default,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_pools: usize,
    pub healthy_pools: usize,
    pub total_liquidity: f64,
    pub has_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ohlcvs::database::OhlcvDatabase;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_pool_registration() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Arc::new(OhlcvDatabase::new(temp_file.path()).unwrap());
        let manager = PoolManager::new(db);

        manager.register_pool("mint1", "pool1", "raydium", 10000.0).await.unwrap();

        let pools = manager.get_pools("mint1").await.unwrap();
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].address, "pool1");
    }

    #[tokio::test]
    async fn test_best_pool_selection() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Arc::new(OhlcvDatabase::new(temp_file.path()).unwrap());
        let manager = PoolManager::new(db);

        manager.register_pool("mint1", "pool1", "raydium", 5000.0).await.unwrap();
        manager.register_pool("mint1", "pool2", "orca", 10000.0).await.unwrap();

        let best = manager.get_best_pool("mint1").await.unwrap();
        assert!(best.is_some());
        assert_eq!(best.unwrap().address, "pool2"); // Higher liquidity
    }
}
