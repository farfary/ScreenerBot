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
            hint: "Jupiter routes through every major DEX. Turning it off leaves only Direct Pool and Raptor, which cannot trade every token.",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Dynamic CU Limit",
            hint: "Let Jupiter size the compute-unit limit by simulating the swap. The priority fee is charged on the limit, so a sized limit costs less.",
            impact: "medium",
            category: "Performance",
        })]
        dynamic_compute_unit_limit: bool = true,
        #[metadata(field_metadata! {
            label: "Priority Fee",
            hint: "Compute-unit price in micro-lamports, the same model Direct Pool and Raptor use. Higher lands faster in a busy block.",
            min: 0,
            max: 10000000,
            step: 1000,
            unit: "micro-lamports/CU",
            impact: "medium",
            category: "Fees",
        })]
        priority_fee_micro_lamports: u64 = 50_000,
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
            hint: "Quote the pool directly alongside the other enabled routers and take whichever returns the most after network fees.",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = false,
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
        #[metadata(field_metadata! {
            label: "Max Price Impact",
            hint: "Refuse a direct quote that moves the pool more than this -- an aggregator that splits the order is the safer choice above it",
            min: 0.1,
            max: 50,
            step: 0.1,
            unit: "%",
            impact: "high",
            category: "Risk",
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
            label: "Enabled",
            hint: "Quote Raptor alongside the other enabled routers and take whichever returns the most after network fees.",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = false,
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
            label: "Max Hops",
            hint: "Longest route Raptor may build. More hops can find a better price but cost more compute.",
            min: 1,
            max: 4,
            step: 1,
            impact: "low",
            category: "Routing",
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
            label: "Enabled",
            hint: "Simulate every aggregator transaction before sending it and refuse one that parks SOL in an account the wallet cannot close. Off means a route may lock rent without warning.",
            impact: "high",
            category: "Safety",
        })]
        enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Max Extra Cost",
            hint: "Share of the trade that may be spent on accounts the wallet will never get back. A venue that charges a one-off deposit is accepted only when the trade is large enough to justify it.",
            min: 0,
            max: 100,
            step: 0.1,
            unit: "% of trade",
            impact: "high",
            category: "Safety",
        })]
        max_extra_cost_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            label: "Always Allow Below",
            hint: "Extra cost small enough to ignore whatever the trade size, so ordinary rounding and dust never block a swap.",
            min: 0,
            max: 100000000,
            step: 10000,
            unit: "lamports",
            impact: "medium",
            category: "Safety",
        })]
        always_allow_below_lamports: u64 = 100_000,
        #[metadata(field_metadata! {
            label: "Retry Without The Venue",
            hint: "When a venue is refused, ask the same aggregator again with that venue excluded before falling back to another router.",
            impact: "medium",
            category: "Safety",
        })]
        retry_excluding_venue: bool = true,
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
            hint: "Build the DEX instruction ourselves instead of routing through an aggregator",
            impact: "high",
            category: "Routers",
        })]
        direct: DirectSwapConfig = DirectSwapConfig::default(),

        /// Raptor router configuration
        #[metadata(field_metadata! {
            label: "Raptor",
            hint: "Solana Tracker's aggregator, quoted alongside the other routers",
            impact: "high",
            category: "Routers",
        })]
        raptor: RaptorConfig = RaptorConfig::default(),

        /// Cost guard configuration
        #[metadata(field_metadata! {
            label: "Cost Guard",
            hint: "Refuse a swap that would spend SOL on accounts the wallet cannot reclaim, whichever router built it",
            impact: "high",
            category: "Safety",
        })]
        cost_guard: SwapCostGuardConfig = SwapCostGuardConfig::default(),

        /// Slippage configuration
        #[metadata(field_metadata! {
            label: "Slippage",
            hint: "Slippage tolerance settings",
            impact: "critical",
            category: "Slippage",
        })]
        slippage: SlippageConfig = SlippageConfig::default(),
    }
}
