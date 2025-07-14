use crate::core::{ BotResult, ScreenerFilters, TokenOpportunity };

/// Filters for screening token opportunities
#[derive(Debug)]
pub struct OpportunityFilter {
    filters: ScreenerFilters,
}

impl OpportunityFilter {
    pub fn new(filters: &ScreenerFilters) -> Self {
        Self {
            filters: filters.clone(),
        }
    }

    /// Apply all filters to a list of opportunities
    pub async fn apply_filters(
        &self,
        opportunities: Vec<TokenOpportunity>
    ) -> BotResult<Vec<TokenOpportunity>> {
        let mut filtered = Vec::new();

        for opportunity in opportunities {
            if self.passes_all_filters(&opportunity).await? {
                filtered.push(opportunity);
            }
        }

        log::debug!("🔍 Filtered {} opportunities", filtered.len());
        Ok(filtered)
    }

    /// Check if an opportunity passes all filters
    async fn passes_all_filters(&self, opportunity: &TokenOpportunity) -> BotResult<bool> {
        // Volume filter
        if opportunity.metrics.volume_24h < self.filters.min_volume_24h {
            log::debug!(
                "❌ {} failed volume filter: {} < {}",
                opportunity.symbol,
                opportunity.metrics.volume_24h,
                self.filters.min_volume_24h
            );
            return Ok(false);
        }

        // Liquidity filter
        if opportunity.metrics.liquidity_usd < self.filters.min_liquidity {
            log::debug!(
                "❌ {} failed liquidity filter: {} < {}",
                opportunity.symbol,
                opportunity.metrics.liquidity_usd,
                self.filters.min_liquidity
            );
            return Ok(false);
        }

        // Age filter
        if opportunity.metrics.age_hours > (self.filters.max_age_hours as f64) {
            log::debug!(
                "❌ {} failed age filter: {} > {}",
                opportunity.symbol,
                opportunity.metrics.age_hours,
                self.filters.max_age_hours
            );
            return Ok(false);
        }

        // Verification filter
        if self.filters.require_verified && !opportunity.verification_status.is_verified {
            log::debug!("❌ {} failed verification filter", opportunity.symbol);
            return Ok(false);
        }

        // Profile filter
        if self.filters.require_profile && !opportunity.verification_status.has_profile {
            log::debug!("❌ {} failed profile filter", opportunity.symbol);
            return Ok(false);
        }

        // Boosted filter (if boosted not allowed)
        if !self.filters.allow_boosted && opportunity.verification_status.is_boosted {
            log::debug!("❌ {} failed boosted filter", opportunity.symbol);
            return Ok(false);
        }

        // Risk score filter (implicit high risk rejection)
        if opportunity.risk_score > 0.8 {
            log::debug!("❌ {} failed risk filter: {}", opportunity.symbol, opportunity.risk_score);
            return Ok(false);
        }

        // Security flags filter
        if opportunity.verification_status.security_flags.len() > 2 {
            log::debug!(
                "❌ {} failed security flags filter: {} flags",
                opportunity.symbol,
                opportunity.verification_status.security_flags.len()
            );
            return Ok(false);
        }

        log::debug!("✅ {} passed all filters", opportunity.symbol);
        Ok(true)
    }

    /// Filter by minimum confidence score
    pub fn filter_by_confidence(
        &self,
        opportunities: Vec<TokenOpportunity>,
        min_confidence: f64
    ) -> Vec<TokenOpportunity> {
        opportunities
            .into_iter()
            .filter(|opp| opp.confidence_score >= min_confidence)
            .collect()
    }

    /// Filter by maximum risk score
    pub fn filter_by_risk(
        &self,
        opportunities: Vec<TokenOpportunity>,
        max_risk: f64
    ) -> Vec<TokenOpportunity> {
        opportunities
            .into_iter()
            .filter(|opp| opp.risk_score <= max_risk)
            .collect()
    }

    /// Sort opportunities by score (confidence - risk)
    pub fn sort_by_score(&self, mut opportunities: Vec<TokenOpportunity>) -> Vec<TokenOpportunity> {
        opportunities.sort_by(|a, b| {
            let score_a = a.confidence_score - a.risk_score;
            let score_b = b.confidence_score - b.risk_score;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        opportunities
    }

    /// Apply additional custom filters
    pub fn apply_custom_filters(
        &self,
        opportunities: Vec<TokenOpportunity>
    ) -> Vec<TokenOpportunity> {
        opportunities
            .into_iter()
            .filter(|opp| {
                // Custom filter: prefer tokens with some market cap
                if let Some(mcap) = opp.metrics.market_cap {
                    mcap > 10000.0 && mcap < 10000000.0 // Between 10K and 10M
                } else {
                    true // Allow if no market cap data
                }
            })
            .filter(|opp| {
                // Custom filter: avoid extreme price changes
                if let Some(change) = opp.metrics.price_change_24h {
                    change.abs() < 500.0 // Less than 500% change
                } else {
                    true
                }
            })
            .collect()
    }
}
