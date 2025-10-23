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

        // ==================== PARTIAL EXIT CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable Partial Exits",
            hint: "Allow selling portions of a position instead of all-or-nothing",
            impact: "high",
            category: "Partial Exit",
        })]
        partial_exit_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Default Partial Exit %",
            hint: "Default percentage to sell on partial exits",
            min: 10,
            max: 90,
            step: 5,
            unit: "%",
            impact: "medium",
            category: "Partial Exit",
        })]
        partial_exit_default_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            label: "Min Partial Exit %",
            hint: "Minimum percentage allowed for partial exits",
            min: 1,
            max: 50,
            step: 1,
            unit: "%",
            impact: "low",
            category: "Partial Exit",
        })]
        partial_exit_min_pct: f64 = 10.0,
        #[metadata(field_metadata! {
            label: "Max Partial Exit %",
            hint: "Maximum percentage allowed for partial exits",
            min: 50,
            max: 99,
            step: 1,
            unit: "%",
            impact: "low",
            category: "Partial Exit",
        })]
        partial_exit_max_pct: f64 = 90.0,

        // ==================== TRAILING STOP CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable Trailing Stop",
            hint: "Enable trailing stop-loss after profit threshold",
            impact: "high",
            category: "Trailing Stop",
        })]
        trailing_stop_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Trailing Stop Activation %",
            hint: "Activate trailing stop after this profit % (10 = activate at +10%)",
            min: 0,
            max: 100,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Trailing Stop",
        })]
        trailing_stop_activation_pct: f64 = 10.0,
        #[metadata(field_metadata! {
            label: "Trailing Stop Distance %",
            hint: "How far to trail below peak (5 = exit if price drops 5% from peak)",
            min: 1,
            max: 50,
            step: 1,
            unit: "%",
            impact: "critical",
            category: "Trailing Stop",
        })]
        trailing_stop_distance_pct: f64 = 5.0,
    }
}
