//! Swap router, slippage, and DEX aggregator configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// SWAPS CONFIGURATION
// ============================================================================

config_struct! {
    /// Jupiter router configuration
    pub struct JupiterConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Router,
        })]
        enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        dynamic_compute_unit_limit: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fees,
        })]
        priority_fee_micro_lamports: u64 = 50_000,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Routing,
        })]
        default_swap_mode: String = "ExactIn".to_owned(),
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Api,
        })]
        api_key: String = String::new(),
    }
}

config_struct! {
    /// Direct pool-swap engine configuration.
    ///
    /// Direct swaps build the DEX instruction themselves instead of routing
    /// through an aggregator. They apply to every venue the engine supports, not
    /// to one DEX -- which is why this section is named for the mechanism.
    pub struct DirectSwapConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Router,
        })]
        enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fees,
        })]
        priority_fee_micro_lamports: u64 = 50_000,
        #[metadata(field_metadata! {
            min: 10,
            max: 180,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Safety,
        })]
        confirmation_timeout_secs: u64 = 60,
        #[metadata(field_metadata! {
            min: 0.1,
            max: 50,
            step: 0.1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Risk,
        })]
        max_price_impact_pct: f64 = 10.0,
    }
}

config_struct! {
    /// Raptor aggregator router configuration.
    ///
    /// Raptor (Solana Tracker) is a second aggregator that competes with Jupiter
    /// on every quote. It needs no API key and publishes no rate limit, but it is
    /// served from a beta host, which is why it ships disabled.
    pub struct RaptorConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Router,
        })]
        enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fees,
        })]
        priority_fee_micro_lamports: u64 = 50_000,
        #[metadata(field_metadata! {
            min: 1,
            max: 4,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Routing,
        })]
        max_hops: u8 = 4,
    }
}

config_struct! {
    /// Guard against a built swap spending SOL on anything but the trade.
    ///
    /// Slippage protects the OUTPUT: it guarantees a minimum number of tokens.
    /// It says nothing about lamports that leave the wallet alongside the swap,
    /// so a route through a venue that makes the trader pay rent for one of its
    /// own accounts passes every slippage check ever written. This guard
    /// simulates the built transaction and refuses it when that cost is out of
    /// proportion to the trade.
    pub struct SwapCostGuardConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Safety,
        })]
        enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 0.1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Safety,
        })]
        max_extra_cost_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000000,
            step: 10000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Safety,
        })]
        always_allow_below_lamports: u64 = 100_000,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Safety,
        })]
        retry_excluding_venue: bool = true,
    }
}

config_struct! {
    /// Slippage configuration
    pub struct SlippageConfig {
        #[metadata(field_metadata! {
            min: 0.1,
            max: 25,
            step: 0.1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Quote,
        })]
        quote_default_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 50,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Exit,
        })]
        exit_profit_shortfall_pct: f64 = 3.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 50,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Exit,
        })]
        exit_loss_shortfall_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Exit,
        })]
        exit_retry_steps_pct: Vec<f64> = vec![3.0, 10.0, 25.0],
    }
}

config_struct! {
    /// Swap router configuration
    pub struct SwapsConfig {
        /// Jupiter router configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Routers,
        })]
        jupiter: JupiterConfig = JupiterConfig::default(),

        /// Direct pool-swap engine configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Routers,
        })]
        direct: DirectSwapConfig = DirectSwapConfig::default(),

        /// Raptor router configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Routers,
        })]
        raptor: RaptorConfig = RaptorConfig::default(),

        /// Cost guard configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Safety,
        })]
        cost_guard: SwapCostGuardConfig = SwapCostGuardConfig::default(),

        /// Slippage configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Slippage,
        })]
        slippage: SlippageConfig = SlippageConfig::default(),
    }
}
