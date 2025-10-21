/// Position management configuration
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Position management configuration
    pub struct PositionsConfig {
        /// Extra SOL needed for profit calculations (accounts for priority fees, etc.)
        #[metadata(field_metadata! {
            label: "Profit Extra Buffer",
            hint: "Extra SOL needed for profit calculations (priority fees)",
            min: 0,
            max: 0.01,
            step: 0.0001,
            unit: "SOL",
            impact: "high",
            category: "Profit",
        })]
        profit_extra_needed_sol: f64 = 0.0002,

        /// Global cooldown between opening ANY positions (prevents rapid bursts)
        #[metadata(field_metadata! {
            label: "Position Open Cooldown",
            hint: "Seconds between opening any positions (prevents rapid bursts)",
            min: 1,
            max: 30,
            step: 1,
            unit: "seconds",
            impact: "medium",
            category: "Timing",
        })]
        position_open_cooldown_secs: i64 = 5,
    }
}
