// Helper modules for improved strategy implementation
pub mod drop_detector;
pub mod dynamic_dca;
pub mod position_sizer;
pub mod profit_calculator;
pub mod token_profiles;

// Re-export main structures for easy access
pub use drop_detector::{ DropDetector, DropSignal, DropSource };
pub use dynamic_dca::{ DynamicDcaCalculator, DcaLevel, DcaDecision };
pub use position_sizer::PositionSizer;
pub use profit_calculator::{
    ProfitTargetCalculator,
    ProfitTarget,
    ProfitUrgency,
    ImmediateProfitDecision,
};
pub use token_profiles::{
    TokenProfile,
    TokenTradingConfig,
    HolderBaseSize,
    VolatilityLevel,
    LiquidityStability,
};
