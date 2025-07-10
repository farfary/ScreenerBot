// ═══════════════════════════════════════════════════════════════════════════════
// STRATEGY MODULE - ENHANCED ANTI-BOT WHALE-FOLLOWING TRADING STRATEGY V2.0
// ═══════════════════════════════════════════════════════════════════════════════

pub mod config;
pub mod pump_analysis;
pub mod price_analysis;
pub mod entry;
pub mod dca;
pub mod exit;
pub mod position;

// Re-export main public APIs to maintain backward compatibility
pub use config::*;
pub use pump_analysis::{
    PumpIntensity,
    detect_pump_intensity,
    detect_momentum_deceleration,
    detect_pump_distribution,
};
pub use price_analysis::{
    PriceAnalysis,
    get_realtime_price_analysis,
    get_price_change_with_fallback,
    calculate_trade_size_sol,
};
pub use entry::should_buy;
pub use dca::should_dca;
pub use exit::should_sell;
pub use position::{
    PositionAction,
    evaluate_position,
    can_enter_token_position,
    should_update_peak,
    get_profit_bucket,
    calculate_dca_size,
};
