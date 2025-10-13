use super::{AggregatorConfig, DataSource, UnifiedTokenInfo};

pub fn calculate_deviation(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        return 100.0;
    }
    ((a - b).abs() / a.abs()) * 100.0
}

impl super::MultiSourceAggregator {
    pub fn check_inter_source_agreement(&self, unified: &UnifiedTokenInfo) -> bool {
        let max_dev = self.config.max_inter_source_deviation;
        if unified.prices.len() < 2 {
            return false;
        }
        for i in 0..unified.prices.len() {
            for j in (i + 1)..unified.prices.len() {
                let d =
                    calculate_deviation(unified.prices[i].price_sol, unified.prices[j].price_sol);
                if d > max_dev {
                    return false;
                }
            }
        }
        true
    }
}
