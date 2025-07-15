use super::types::*;
use anyhow::Result;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct RouteComparison {
    pub route: SwapRoute,
    pub score: f64,
    pub reasons: Vec<String>,
}

pub struct RouteSelector {
    config: SwapConfig,
}

impl RouteSelector {
    pub fn new(config: SwapConfig) -> Self {
        Self { config }
    }

    /// Select the best route from multiple options based on output amount, price impact, and DEX preference
    pub fn select_best_route(&self, routes: Vec<SwapRoute>) -> Result<SwapRoute, SwapError> {
        if routes.is_empty() {
            return Err(SwapError::InvalidRoute("No routes available".to_string()));
        }

        let mut comparisons: Vec<RouteComparison> = routes
            .into_iter()
            .map(|route| self.score_route(route))
            .collect();

        // Sort by score (highest first)
        comparisons.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

        // Log the comparison results
        log::info!("Route comparison results:");
        for (i, comp) in comparisons.iter().enumerate() {
            log::info!(
                "  {}. {} - Score: {:.2} | Output: {} | Impact: {}% | Reasons: {}",
                i + 1,
                comp.route.dex,
                comp.score,
                comp.route.out_amount,
                comp.route.price_impact_pct,
                comp.reasons.join(", ")
            );
        }

        Ok(comparisons.into_iter().next().unwrap().route)
    }

    /// Score a route based on multiple factors
    fn score_route(&self, route: SwapRoute) -> RouteComparison {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // 1. Output amount (40% weight) - Higher output is better
        let output_amount: f64 = route.out_amount.parse().unwrap_or(0.0);
        let output_score = output_amount / 1_000_000.0; // Normalize
        score += output_score * 0.4;
        reasons.push(format!("Output: {:.0}", output_amount));

        // 2. Price impact (30% weight) - Lower impact is better
        let price_impact: f64 = route.price_impact_pct.parse().unwrap_or(100.0);
        let impact_score = (5.0 - price_impact.min(5.0)).max(0.0) / 5.0; // Cap at 5% and invert
        score += impact_score * 0.3;
        reasons.push(format!("Impact: {:.2}%", price_impact));

        // 3. DEX preference (20% weight)
        let dex_preference_score = self.get_dex_preference_score(&route.dex);
        score += dex_preference_score * 0.2;
        reasons.push(format!("DEX pref: {:.1}", dex_preference_score));

        // 4. Route complexity (10% weight) - Fewer hops is better
        let route_complexity = route.route_plan.len() as f64;
        let complexity_score = (5.0 - route_complexity.min(5.0)) / 5.0;
        score += complexity_score * 0.1;
        reasons.push(format!("Hops: {}", route.route_plan.len()));

        // Bonus points for specific features
        if route.dex == DexType::Gmgn && self.config.is_anti_mev {
            score += 0.1;
            reasons.push("Anti-MEV bonus".to_string());
        }

        // Penalty for high slippage
        if route.slippage_bps > 100 { // > 1%
            score -= 0.1;
            reasons.push("High slippage penalty".to_string());
        }

        RouteComparison {
            route,
            score,
            reasons,
        }
    }

    fn get_dex_preference_score(&self, dex: &DexType) -> f64 {
        let dex_str = dex.to_string();
        
        // Find position in preference list (lower index = higher preference)
        for (i, preferred_dex) in self.config.dex_preferences.iter().enumerate() {
            if preferred_dex == &dex_str {
                // Convert position to score (first = 1.0, second = 0.8, etc.)
                return 1.0 - (i as f64 * 0.2);
            }
        }
        
        // Not in preferences list
        0.0
    }

    /// Filter routes by maximum slippage
    pub fn filter_by_slippage(&self, routes: Vec<SwapRoute>) -> Vec<SwapRoute> {
        let max_slippage_bps = (self.config.max_slippage * 10000.0) as u32;
        
        routes
            .into_iter()
            .filter(|route| route.slippage_bps <= max_slippage_bps)
            .collect()
    }

    /// Filter routes by minimum output amount
    pub fn filter_by_minimum_output(&self, routes: Vec<SwapRoute>, min_output: u64) -> Vec<SwapRoute> {
        routes
            .into_iter()
            .filter(|route| {
                route.out_amount.parse::<u64>().unwrap_or(0) >= min_output
            })
            .collect()
    }

    /// Compare two routes directly
    pub fn compare_routes(&self, route1: &SwapRoute, route2: &SwapRoute) -> Ordering {
        let comp1 = self.score_route(route1.clone());
        let comp2 = self.score_route(route2.clone());
        
        comp1.score.partial_cmp(&comp2.score).unwrap_or(Ordering::Equal)
    }

    /// Get route analysis for debugging
    pub fn analyze_route(&self, route: &SwapRoute) -> RouteAnalysis {
        let comparison = self.score_route(route.clone());
        
        RouteAnalysis {
            dex: route.dex.clone(),
            score: comparison.score,
            output_amount: route.out_amount.parse().unwrap_or(0.0),
            price_impact: route.price_impact_pct.parse().unwrap_or(0.0),
            slippage_bps: route.slippage_bps,
            route_hops: route.route_plan.len(),
            estimated_fee: self.estimate_fee(route),
            reasons: comparison.reasons,
        }
    }

    fn estimate_fee(&self, route: &SwapRoute) -> u64 {
        // Estimate transaction fee based on route complexity
        let base_fee = 5000; // Base Solana transaction fee
        let hop_fee = route.route_plan.len() as u64 * 2000; // Additional fee per hop
        
        base_fee + hop_fee
    }
}

#[derive(Debug, Clone)]
pub struct RouteAnalysis {
    pub dex: DexType,
    pub score: f64,
    pub output_amount: f64,
    pub price_impact: f64,
    pub slippage_bps: u32,
    pub route_hops: usize,
    pub estimated_fee: u64,
    pub reasons: Vec<String>,
}

impl std::fmt::Display for RouteAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DEX: {} | Score: {:.2} | Output: {:.0} | Impact: {:.2}% | Slippage: {:.2}% | Hops: {} | Fee: {} | Reasons: {}",
            self.dex,
            self.score,
            self.output_amount,
            self.price_impact,
            self.slippage_bps as f64 / 100.0,
            self.route_hops,
            self.estimated_fee,
            self.reasons.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SwapConfig {
        SwapConfig {
            enabled: true,
            default_dex: "jupiter".to_string(),
            is_anti_mev: false,
            max_slippage: 0.01,
            timeout_seconds: 30,
            retry_attempts: 3,
            dex_preferences: vec!["jupiter".to_string(), "raydium".to_string(), "gmgn".to_string()],
            jupiter: JupiterConfig {
                enabled: true,
                base_url: "https://quote-api.jup.ag/v6".to_string(),
                timeout_seconds: 15,
                max_accounts: 64,
                only_direct_routes: false,
                as_legacy_transaction: false,
            },
            raydium: RaydiumConfig {
                enabled: true,
                base_url: "https://api.raydium.io/v2".to_string(),
                timeout_seconds: 15,
                pool_type: "all".to_string(),
            },
            gmgn: GmgnConfig {
                enabled: true,
                base_url: "https://gmgn.ai/defi/quoterv1".to_string(),
                timeout_seconds: 15,
                api_key: "".to_string(),
                referral_account: "".to_string(),
                referral_fee_bps: 0,
            },
        }
    }

    fn create_test_route(dex: DexType, out_amount: &str, price_impact: &str) -> SwapRoute {
        SwapRoute {
            dex,
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            in_amount: "1000000".to_string(),
            out_amount: out_amount.to_string(),
            other_amount_threshold: (out_amount.parse::<u64>().unwrap_or(0) * 95 / 100).to_string(),
            swap_mode: "ExactIn".to_string(),
            slippage_bps: 50,
            platform_fee: None,
            price_impact_pct: price_impact.to_string(),
            route_plan: vec![],
            context_slot: Some(100),
            time_taken: Some(0.5),
        }
    }

    #[test]
    fn test_route_selection() {
        let config = create_test_config();
        let selector = RouteSelector::new(config);

        let routes = vec![
            create_test_route(DexType::Jupiter, "200000", "0.5"),
            create_test_route(DexType::Raydium, "195000", "0.3"),
            create_test_route(DexType::Gmgn, "198000", "0.4"),
        ];

        let best_route = selector.select_best_route(routes).unwrap();
        
        // Jupiter should win due to highest output and DEX preference
        assert_eq!(best_route.dex, DexType::Jupiter);
        assert_eq!(best_route.out_amount, "200000");
    }

    #[test]
    fn test_slippage_filter() {
        let config = create_test_config();
        let selector = RouteSelector::new(config);

        let mut high_slippage_route = create_test_route(DexType::Jupiter, "200000", "0.5");
        high_slippage_route.slippage_bps = 200; // 2% slippage

        let routes = vec![
            create_test_route(DexType::Raydium, "195000", "0.3"),
            high_slippage_route,
        ];

        let filtered_routes = selector.filter_by_slippage(routes);
        
        // Only the Raydium route should pass (1% max slippage in config)
        assert_eq!(filtered_routes.len(), 1);
        assert_eq!(filtered_routes[0].dex, DexType::Raydium);
    }

    #[test]
    fn test_route_analysis() {
        let config = create_test_config();
        let selector = RouteSelector::new(config);

        let route = create_test_route(DexType::Jupiter, "200000", "0.5");
        let analysis = selector.analyze_route(&route);

        assert_eq!(analysis.dex, DexType::Jupiter);
        assert!(analysis.score > 0.0);
        assert_eq!(analysis.output_amount, 200000.0);
        assert_eq!(analysis.price_impact, 0.5);
        assert!(!analysis.reasons.is_empty());
    }
}
