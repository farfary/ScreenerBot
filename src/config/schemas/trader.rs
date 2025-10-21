/// Trading system configuration
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Trading system configuration
    pub struct TraderConfig {
        // Trader control
        enabled: bool = true,

        // Core trading parameters
        #[metadata(field_metadata! {
            label: "Max Open Positions",
            hint: "Max simultaneous positions (2-5 conservative)",
            min: 1,
            max: 100,
            unit: "positions",
            impact: "critical",
            category: "Core Trading",
        })]
        max_open_positions: usize = 2,
        #[metadata(field_metadata! {
            label: "Trade Size",
            hint: "SOL per position (0.005-0.01 for testing)",
            min: 0.001,
            max: 10,
            step: 0.001,
            unit: "SOL",
            impact: "critical",
            category: "Core Trading",
        })]
        trade_size_sol: f64 = 0.005,

        // Profit thresholds
        #[metadata(field_metadata! {
            label: "Enable Profit Threshold",
            hint: "Require minimum profit before exit",
            impact: "high",
            category: "Profit Management",
        })]
        min_profit_threshold_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min Profit %",
            hint: "2-5% typical for volatile tokens",
            min: 0,
            max: 100,
            step: 0.1,
            unit: "%",
            impact: "high",
            category: "Profit Management",
        })]
        min_profit_threshold_percent: f64 = 2.0,

        // Time-based overrides
        #[metadata(field_metadata! {
            label: "Time Override Duration",
            hint: "Hours before forced exit (168=1 week)",
            min: 1,
            max: 720,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Time Overrides",
        })]
        time_override_duration_hours: f64 = 168.0,
        #[metadata(field_metadata! {
            label: "Time Override Loss %",
            hint: "Loss % to trigger time override (-40 = exit if down 40%)",
            min: -100,
            max: 0,
            step: 1,
            unit: "%",
            impact: "medium",
            category: "Time Overrides",
        })]
        time_override_loss_threshold_percent: f64 = -40.0,

        // Position timing
        #[metadata(field_metadata! {
            label: "Close Cooldown",
            hint: "Minutes before reopening same token",
            min: 0,
            max: 1440,
            step: 5,
            unit: "minutes",
            impact: "critical",
            category: "Timing",
        })]
        position_close_cooldown_minutes: i64 = 15,

        // Performance settings
        #[metadata(field_metadata! {
            label: "Entry Check Concurrency",
            hint: "Tokens to check concurrently (higher = faster but more CPU)",
            min: 1,
            max: 50,
            step: 1,
            unit: "concurrent",
            impact: "medium",
            category: "Performance",
        })]
        entry_check_concurrency: usize = 10,

        // Dry run mode
        dry_run_mode: bool = false,

        // Sell concurrency
        sell_concurrency: usize = 5,
    }
}
