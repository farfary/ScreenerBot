/// Trading system configuration
use crate::config_struct;
use crate::field_metadata;
use serde::{Deserialize, Serialize};

/// Time unit for duration configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl Default for TimeUnit {
    fn default() -> Self {
        TimeUnit::Hours
    }
}

impl TimeUnit {
    /// Convert duration to seconds
    pub fn to_seconds(&self, value: f64) -> f64 {
        match self {
            TimeUnit::Seconds => value,
            TimeUnit::Minutes => value * 60.0,
            TimeUnit::Hours => value * 3600.0,
            TimeUnit::Days => value * 86400.0,
        }
    }

    /// Convert from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "seconds" | "s" | "sec" => Some(TimeUnit::Seconds),
            "minutes" | "m" | "min" => Some(TimeUnit::Minutes),
            "hours" | "h" | "hr" => Some(TimeUnit::Hours),
            "days" | "d" | "day" => Some(TimeUnit::Days),
            _ => None,
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        match self {
            TimeUnit::Seconds => "seconds".to_string(),
            TimeUnit::Minutes => "minutes".to_string(),
            TimeUnit::Hours => "hours".to_string(),
            TimeUnit::Days => "days".to_string(),
        }
    }
}

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
        #[metadata(field_metadata! {
            label: "Entry Sizes",
            hint: "Preset SOL amounts for manual trades [0.005, 0.01, 0.02, 0.05]",
            impact: "medium",
            category: "Core Trading",
        })]
        entry_sizes: Vec<f64> = vec![0.005, 0.01, 0.02, 0.05],

        // ==================== ROI EXIT CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable ROI Exit",
            hint: "Enable automatic exit when profit target is reached",
            impact: "high",
            category: "ROI Exit",
        })]
        roi_exit_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "ROI Target %",
            hint: "Exit when profit reaches this % (20 = exit at +20%)",
            min: 1,
            max: 1000,
            step: 1,
            unit: "%",
            impact: "high",
            category: "ROI Exit",
        })]
        roi_target_percent: f64 = 20.0,

        // ==================== TIME OVERRIDE CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable Time Override",
            hint: "Enable automatic exit for positions held too long at a loss",
            impact: "high",
            category: "Time Override",
        })]
        time_override_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Time Override Duration",
            hint: "Duration before forced exit (168 hours = 7 days, 30 minutes, etc)",
            min: 1,
            max: 43200,
            step: 1,
            impact: "critical",
            category: "Time Override",
        })]
        time_override_duration: f64 = 168.0,
        #[metadata(field_metadata! {
            label: "Time Override Unit",
            hint: "Time unit: seconds, minutes, hours, days",
            impact: "critical",
            category: "Time Override",
        })]
        time_override_unit: String = "hours".to_string(),
        #[metadata(field_metadata! {
            label: "Time Override Loss %",
            hint: "Loss % to trigger time override (-40 = exit if down 40%)",
            min: -100,
            max: 0,
            step: 1,
            unit: "%",
            impact: "medium",
            category: "Time Override",
        })]
        time_override_loss_threshold_percent: f64 = -40.0,

        // ==================== STOP LOSS CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable Stop Loss",
            hint: "Enable automatic exit when loss exceeds threshold (exits immediately, unlike time override)",
            impact: "high",
            category: "Stop Loss",
        })]
        stop_loss_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Stop Loss Threshold %",
            hint: "Exit when loss exceeds this % (50 = exit at -50%)",
            min: 1,
            max: 100,
            step: 1,
            unit: "%",
            impact: "critical",
            category: "Stop Loss",
        })]
        stop_loss_threshold_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            label: "Allow Partial Exit",
            hint: "Allow partial exits for stop loss instead of full position close",
            impact: "medium",
            category: "Stop Loss",
        })]
        stop_loss_allow_partial: bool = false,
        #[metadata(field_metadata! {
            label: "Min Hold Time",
            hint: "Minimum seconds to hold before stop loss can trigger (0 = immediate)",
            min: 0,
            max: 86400,
            step: 1,
            unit: "seconds",
            impact: "medium",
            category: "Stop Loss",
        })]
        stop_loss_min_hold_seconds: u64 = 0,

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

        // ==================== DCA CONFIGURATION ====================
        #[metadata(field_metadata! {
            label: "Enable DCA",
            hint: "Enable Dollar Cost Averaging for positions",
            impact: "high",
            category: "DCA",
        })]
        dca_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "DCA Threshold %",
            hint: "Enter DCA when position down by this % (-10 = DCA at -10%)",
            min: -100,
            max: 0,
            step: 1,
            unit: "%",
            impact: "high",
            category: "DCA",
        })]
        dca_threshold_pct: f64 = -10.0,
        #[metadata(field_metadata! {
            label: "Max DCA Count",
            hint: "Maximum number of additional DCA entries per position",
            min: 1,
            max: 5,
            step: 1,
            unit: "entries",
            impact: "critical",
            category: "DCA",
        })]
        dca_max_count: usize = 2,
        #[metadata(field_metadata! {
            label: "DCA Size %",
            hint: "Size of each DCA entry as % of initial position size",
            min: 10,
            max: 200,
            step: 10,
            unit: "%",
            impact: "high",
            category: "DCA",
        })]
        dca_size_percentage: f64 = 50.0,
        #[metadata(field_metadata! {
            label: "DCA Cooldown",
            hint: "Minimum minutes between DCA entries",
            min: 1,
            max: 1440,
            step: 5,
            unit: "minutes",
            impact: "medium",
            category: "DCA",
        })]
        dca_cooldown_minutes: i64 = 30,
    }
}
