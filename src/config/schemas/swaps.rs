use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// SWAPS CONFIGURATION
// ============================================================================

config_struct! {
    /// GMGN router configuration
    pub struct GmgnConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable GMGN router (provides MEV protection)",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Partner",
            hint: "Partner identifier for GMGN",
            impact: "low",
            category: "API",
        })]
        partner: String = "screenerbot".to_string(),
        #[metadata(field_metadata! {
            label: "Anti-MEV",
            hint: "Enable GMGN MEV protection",
            impact: "medium",
            category: "Protection",
        })]
        anti_mev: bool = false,
        #[metadata(field_metadata! {
            label: "Fee",
            hint: "Usually 0, check GMGN docs",
            min: 0,
            max: 0.1,
            step: 0.001,
            unit: "SOL",
            impact: "low",
            category: "Fees",
        })]
        fee_sol: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Default Swap Mode",
            hint: "ExactIn or ExactOut",
            impact: "low",
            category: "Routing",
        })]
        default_swap_mode: String = "ExactIn".to_string(),
    }
}

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
        default_swap_mode: String = "ExactIn".to_string(),
        #[metadata(field_metadata! {
            label: "API Key",
            hint: "Optional. Get from portal.jup.ag for higher rate limits. Free tier: 1 req/sec. Does NOT affect swap fees.",
            impact: "medium",
            category: "API",
        })]
        api_key: String = String::new(),
    }
}

config_struct! {
    /// Raydium direct swap configuration
    pub struct RaydiumConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable direct Raydium swaps (bypass aggregators)",
            impact: "medium",
            category: "Router",
        })]
        enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Default Slippage (BPS)",
            hint: "Default slippage for direct pool swaps in basis points (100 = 1%)",
            min: 10,
            max: 2500,
            step: 10,
            unit: "bps",
            impact: "high",
            category: "Risk",
        })]
        default_slippage_bps: u16 = 100,
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
        /// GMGN router configuration
        #[serde(skip_serializing)]
        #[metadata(field_metadata! {
            label: "GMGN",
            hint: "GMGN router with MEV protection",
            impact: "high",
            category: "Routers",
            hidden: true,
        })]
        gmgn: GmgnConfig = GmgnConfig::default(),

        /// Jupiter router configuration
        #[metadata(field_metadata! {
            label: "Jupiter",
            hint: "Jupiter aggregator router",
            impact: "high",
            category: "Routers",
        })]
        jupiter: JupiterConfig = JupiterConfig::default(),

        /// Raydium direct swap configuration
        #[serde(skip_serializing)]
        #[metadata(field_metadata! {
            label: "Raydium",
            hint: "Direct Raydium swaps",
            impact: "medium",
            category: "Routers",
            hidden: true,
        })]
        raydium: RaydiumConfig = RaydiumConfig::default(),

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
