mod types;
pub use types::*;

mod validator;
pub use validator::*;

mod dexscreener_adapter;
mod geckoterminal_adapter;

use crate::config::with_config;
use crate::logger::{log, LogTag};
use chrono::Utc;
use std::collections::HashMap;

use crate::tokens::types::Token;

/// Aggregator config fetched from global config
#[derive(Clone, Debug)]
pub struct AggregatorConfig {
    pub enable_multi_source: bool,
    pub min_sources: usize,
    pub max_inter_source_deviation: f64,
    pub enable_dexscreener: bool,
    pub enable_geckoterminal: bool,
}

impl AggregatorConfig {
    pub fn from_global() -> Self {
        with_config(|cfg| AggregatorConfig {
            enable_multi_source: cfg.tokens.sources.enable_multi_source,
            min_sources: cfg.tokens.sources.min_sources,
            max_inter_source_deviation: cfg.tokens.sources.max_inter_source_deviation,
            enable_dexscreener: cfg.tokens.sources.dexscreener.enabled,
            enable_geckoterminal: cfg.tokens.sources.geckoterminal.enabled,
        })
    }
}

pub struct MultiSourceAggregator {
    pub config: AggregatorConfig,
}

impl Default for MultiSourceAggregator {
    fn default() -> Self {
        Self {
            config: AggregatorConfig::from_global(),
        }
    }
}

impl MultiSourceAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_source_enabled(&self, src: DataSource) -> bool {
        match src {
            DataSource::DexScreener => self.config.enable_dexscreener,
            DataSource::GeckoTerminal => self.config.enable_geckoterminal,
        }
    }

    /// Fetch unified token info honoring per-source enable flags.
    pub async fn get_token_info(&self, mint: &str) -> Result<UnifiedTokenInfo, String> {
        self.build_unified_info(mint, None).await
    }

    /// Get merged pools from enabled sources, deduplicated and ranked by liquidity
    pub async fn get_token_pools(&self, mint: &str) -> Result<Vec<SourcedPool>, String> {
        if !self.config.enable_multi_source {
            return Err("Multi-source is disabled via config".to_string());
        }
        let mut all: Vec<SourcedPool> = Vec::new();
        if self.is_source_enabled(DataSource::DexScreener) {
            if let Ok(info) = dexscreener_adapter::fetch_token_info_from_dexscreener(mint).await {
                all.extend(info.pools);
            }
        }
        if self.is_source_enabled(DataSource::GeckoTerminal) {
            if let Ok(info) = geckoterminal_adapter::fetch_token_info_from_geckoterminal(mint).await
            {
                all.extend(info.pools);
            }
        }
        // Deduplicate by pool_address, keep highest liquidity
        let mut by_addr: HashMap<String, SourcedPool> = HashMap::new();
        for p in all.into_iter() {
            by_addr
                .entry(p.pool_address.clone())
                .and_modify(|existing| {
                    let e_liq = existing.liquidity_usd.unwrap_or(0.0);
                    let p_liq = p.liquidity_usd.unwrap_or(0.0);
                    if p_liq > e_liq {
                        *existing = p.clone();
                    }
                })
                .or_insert(p);
        }
        let mut deduped: Vec<SourcedPool> = by_addr.into_values().collect();
        deduped.sort_by(|a, b| {
            b.liquidity_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.liquidity_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(deduped)
    }

    /// Validate a price update using multi-source consensus (no single-source fallback)
    pub async fn validate_price_update(&self, mint: &str) -> Result<ValidationResult, String> {
        if !self.config.enable_multi_source {
            return Ok(ValidationResult {
                is_valid: false,
                consensus_price: None,
                used_sources: vec![],
                issues: vec![ValidationIssue::NoSourcesEnabled],
            });
        }

        let info = self.build_unified_info(mint, None).await?;
        let result = self.build_validation_result(&info);
        Ok(result)
    }

    /// Validate using a DexScreener token that was already fetched by the caller.
    pub async fn validate_prefetched(&self, token: &Token) -> Result<ValidationResult, String> {
        if !self.config.enable_multi_source {
            return Ok(ValidationResult {
                is_valid: false,
                consensus_price: None,
                used_sources: vec![],
                issues: vec![ValidationIssue::NoSourcesEnabled],
            });
        }

        let info = self.build_unified_info(&token.mint, Some(token)).await?;
        let result = self.build_validation_result(&info);
        Ok(result)
    }

    async fn build_unified_info(
        &self,
        mint: &str,
        prefetched: Option<&Token>,
    ) -> Result<UnifiedTokenInfo, String> {
        if !self.config.enable_multi_source {
            return Err("Multi-source is disabled via config".to_string());
        }

        let mut unified_opt: Option<UnifiedTokenInfo> = None;

        if self.is_source_enabled(DataSource::DexScreener) {
            let dex_result = match prefetched {
                Some(token) => dexscreener_adapter::unify_prefetched_token(token).await,
                None => dexscreener_adapter::fetch_token_info_from_dexscreener(mint).await,
            };

            match dex_result {
                Ok(info) => unified_opt = Some(info),
                Err(e) => log(LogTag::Api, "DEXSCREENER_ADAPTER_ERROR", &format!("{}", e)),
            }
        }

        if self.is_source_enabled(DataSource::GeckoTerminal) {
            match geckoterminal_adapter::fetch_token_info_from_geckoterminal(mint).await {
                Ok(info) => {
                    if let Some(mut base) = unified_opt {
                        let mut pools = base.pools;
                        pools.extend(info.pools);
                        base.pools = pools;

                        let mut prices = base.prices;
                        prices.extend(info.prices);
                        base.prices = prices;

                        let mut sources = base.sources;
                        sources.extend(info.sources);
                        base.sources = sources;
                        unified_opt = Some(base);
                    } else {
                        unified_opt = Some(info);
                    }
                }
                Err(e) => log(
                    LogTag::Api,
                    "GECKOTERMINAL_ADAPTER_ERROR",
                    &format!("{}", e),
                ),
            }
        }

        if let Some(mut unified) = unified_opt {
            self.finalize_unified(&mut unified);
            return Ok(unified);
        }

        Err("No sources available or all failed".to_string())
    }

    fn finalize_unified(&self, unified: &mut UnifiedTokenInfo) {
        let mut pool_to_dex: HashMap<String, String> = HashMap::new();
        let mut primary_pool: Option<(String, f64)> = None;

        for pool in &unified.pools {
            if !pool.pool_address.is_empty() {
                pool_to_dex.insert(pool.pool_address.clone(), pool.dex_id.clone());
            }
            if let Some(liq) = pool.liquidity_usd {
                match &mut primary_pool {
                    Some((_addr, best_liq)) if liq > *best_liq => {
                        *best_liq = liq;
                        primary_pool = Some((pool.pool_address.clone(), liq));
                    }
                    None => primary_pool = Some((pool.pool_address.clone(), liq)),
                    _ => {}
                }
            }
        }

        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, price) in unified.prices.iter().enumerate() {
            let key = price
                .pool_address
                .as_ref()
                .and_then(|addr| pool_to_dex.get(addr))
                .cloned()
                .unwrap_or_else(|| price.source.to_string());
            groups.entry(key).or_default().push(idx);
        }

        let mut best_group_price: Option<f64> = None;
        let mut best_group_liq: f64 = 0.0;
        for indices in groups.values() {
            let mut sum = 0.0f64;
            let mut count = 0usize;
            let mut liq_weighted_sum = 0.0f64;
            let mut liq_total = 0.0f64;

            for &i in indices {
                let p = &unified.prices[i];
                if !p.price_sol.is_finite() {
                    continue;
                }
                sum += p.price_sol;
                count += 1;
                if let Some(liq) = p.liquidity_usd {
                    if liq.is_finite() && liq > 0.0 {
                        liq_weighted_sum += p.price_sol * liq;
                        liq_total += liq;
                    }
                }
            }

            if count == 0 {
                continue;
            }

            let group_price = if liq_total > 0.0 {
                liq_weighted_sum / liq_total
            } else {
                sum / count as f64
            };

            let mut group_liq = 0.0f64;
            for &i in indices {
                if let Some(addr) = &unified.prices[i].pool_address {
                    if let Some(pool) = unified.pools.iter().find(|pp| &pp.pool_address == addr) {
                        group_liq += pool.liquidity_usd.unwrap_or(0.0);
                    }
                }
            }

            if group_liq > best_group_liq {
                best_group_liq = group_liq;
                best_group_price = Some(group_price);
            }
        }

        unified.consensus_price_sol = best_group_price;
        unified.price_confidence = if self.check_inter_source_agreement(unified) {
            0.9
        } else {
            0.3
        };

        if let Some((addr, _)) = primary_pool {
            unified.primary_pool = Some(addr);
        }
    }

    fn build_validation_result(&self, info: &UnifiedTokenInfo) -> ValidationResult {
        let mut issues = Vec::new();
        let sources_count = info.sources.len();
        if sources_count < self.config.min_sources {
            issues.push(ValidationIssue::NotEnoughSources {
                available: sources_count,
                required: self.config.min_sources,
            });
        }

        let agree = self.check_inter_source_agreement(info);
        if !agree {
            issues.push(ValidationIssue::SourcesDisagree {
                max_deviation_pct: self.config.max_inter_source_deviation,
            });
        }

        if info.consensus_price_sol.is_none() {
            issues.push(ValidationIssue::NoConsensusPrice);
        }

        let mut used_sources = info.sources.clone();
        used_sources.sort_by_key(|s| match s {
            DataSource::DexScreener => 0,
            DataSource::GeckoTerminal => 1,
        });
        used_sources.dedup();

        let is_valid =
            sources_count >= self.config.min_sources && agree && info.consensus_price_sol.is_some();
        ValidationResult {
            is_valid,
            consensus_price: info.consensus_price_sol,
            used_sources,
            issues,
        }
    }
}
