//! Swap router, slippage, and DEX aggregator configuration.

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// SWAPS CONFIGURATION
// ============================================================================

config_struct! {
    /// Jupiter router configuration
    pub struct JupiterConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable Jupiter router (finds best routes across DEXes)",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Dynamic CU Limit",
            hint: "Let Jupiter calculate compute units",
            impact: "medium",
            category: "Performance",
        })]
        dynamic_compute_unit_limit: bool = false,
        #[metadata(field_metadata! {
            label: "Default Priority Fee",
            hint: "1000 lamports = 0.000001 SOL, higher = faster",
            min: 0,
            max: 1000000,
            step: 100,
            unit: "lamports",
            impact: "medium",
            category: "Fees",
        })]
        default_priority_fee: u64 = 1000,
        #[metadata(field_metadata! {
            label: "Default Swap Mode",
            hint: "ExactIn or ExactOut",
            impact: "low",
            category: "Routing",
        })]
        default_swap_mode: String = "ExactIn".to_owned(),
        #[metadata(field_metadata! {
            label: "API Key",
            hint: "Optional. Leave empty to use Jupiter's free endpoint (lite-api.jup.ag). Add a key from portal.jup.ag only if you hit rate limits and want higher throughput (api.jup.ag). Does NOT affect swap fees or the referral revenue share.",
            impact: "low",
            category: "API",
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
            label: "Enabled",
            hint: "Swap straight against the pool instead of through an aggregator",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Default Slippage (BPS)",
            hint: "Slippage floor written into the swap instruction (100 = 1%)",
            min: 10,
            max: 5000,
            step: 10,
            unit: "bps",
            impact: "high",
            category: "Risk",
        })]
        default_slippage_bps: u16 = 100,
        #[metadata(field_metadata! {
            label: "Priority Fee",
            hint: "Compute-unit price in micro-lamports. Higher lands faster in a busy block.",
            min: 0,
            max: 10000000,
            step: 1000,
            unit: "micro-lamports/CU",
            impact: "medium",
            category: "Fees",
        })]
        priority_fee_micro_lamports: u64 = 50_000,
        #[metadata(field_metadata! {
            label: "Simulate Before Send",
            hint: "Run the swap against a node first so a broken build fails for free",
            impact: "high",
            category: "Safety",
        })]
        simulate_before_send: bool = true,
        #[metadata(field_metadata! {
            label: "Confirmation Timeout",
            hint: "How long to wait for the swap to confirm before reporting an unknown outcome",
            min: 10,
            max: 180,
            step: 5,
            unit: "seconds",
            impact: "medium",
            category: "Safety",
        })]
        confirmation_timeout_secs: u64 = 60,
    }
}

config_struct! {
    /// Slippage configuration
    pub struct SlippageConfig {
        #[metadata(field_metadata! {
            label: "Default Slippage",
            hint: "1% tight, 3-5% for volatile",
            min: 0.1,
            max: 25,
            step: 0.1,
            unit: "%",
            impact: "high",
            category: "Quote",
        })]
        quote_default_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            label: "Profit Exit Slippage",
            hint: "Higher ensures exits succeed",
            min: 0,
            max: 50,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Exit",
        })]
        exit_profit_shortfall_pct: f64 = 3.0,
        #[metadata(field_metadata! {
            label: "Loss Exit Slippage",
            hint: "Even higher to exit bad positions",
            min: 0,
            max: 50,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Exit",
        })]
        exit_loss_shortfall_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            label: "Exit Retry Steps",
            hint: "Comma-separated slippage for retries",
            unit: "%",
            impact: "medium",
            category: "Exit",
        })]
        exit_retry_steps_pct: Vec<f64> = vec![3.0, 10.0, 25.0],
    }
}

config_struct! {
    /// Swap router configuration
    pub struct SwapsConfig {
        /// Jupiter router configuration
        #[metadata(field_metadata! {
            label: "Jupiter",
            hint: "Jupiter aggregator router",
            impact: "high",
            category: "Routers",
        })]
        jupiter: JupiterConfig = JupiterConfig::default(),

        /// Direct pool-swap engine configuration
        #[metadata(field_metadata! {
            label: "Direct Pool Swaps",
            hint: "Build DEX instructions directly against the pool",
            impact: "high",
            category: "Routers",
        })]
        direct: DirectSwapConfig = DirectSwapConfig::default(),

        /// Slippage configuration
        #[metadata(field_metadata! {
            label: "Slippage",
            hint: "Slippage tolerance settings",
            impact: "critical",
            category: "Risk",
        })]
        slippage: SlippageConfig = SlippageConfig::default(),
    }
}
