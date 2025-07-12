use crate::prelude::*;
use crate::price_validation::{is_price_valid, get_trading_price};
use super::config::*;

/// Simplified DCA function for testing
pub fn should_dca(
    token: &Token,
    pos: &Position,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> bool {
    // Basic price validation
    if !is_price_valid(current_price) {
        return false;
    }

    // Check if we're down enough to DCA
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    
    // Simple DCA logic: DCA if down more than 20% and haven't reached max DCA count
    if drop_pct < -20.0 && pos.dca_count < MAX_DCA_COUNT {
        println!("🔄 [DCA] {} | Drop: {:.1}% | DCA: {}/{}", 
                token.symbol, drop_pct, pos.dca_count, MAX_DCA_COUNT);
        true
    } else {
        false
    }
}

/// Test function
pub fn test_dca_compiles() -> bool {
    true
}
