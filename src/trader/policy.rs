//! The knobs that decide what happens to an open position, resolved once instead of read
//! from global config inside each evaluator.
//!
//! Every exit rule used to reach into `config::get_*()` on its own, which made them
//! untestable without a config guard and meant one account could run exactly one risk
//! policy. Resolving a value here is what lets a future per-source risk profile be data
//! rather than a second exit stack. There are no overrides yet — this is a snapshot.

use crate::trader::config;
use crate::trader::evaluators::dca::DcaConfigSnapshot;
use crate::trader::evaluators::exit_stop_loss;

/// The four config values `exit_stop_loss::check_stop_loss` reads today.
#[derive(Debug, Clone, PartialEq)]
pub struct StopLossPolicy {
    pub enabled: bool,
    pub threshold_pct: f64,
    pub min_hold_seconds: u64,
    pub allow_partial: bool,
    pub partial_exit_default_pct: f64,
}

/// The config values `exit_trailing::check_trailing_stop` reads today.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailingPolicy {
    pub enabled: bool,
    pub activation_pct: f64,
    pub distance_pct: f64,
}

/// The config values `exit_roi::check_roi_exit` reads today.
#[derive(Debug, Clone, PartialEq)]
pub struct RoiPolicy {
    pub enabled: bool,
    pub target_profit_pct: f64,
}

/// The config values `exit_time::check_time_override` reads today.
#[derive(Debug, Clone, PartialEq)]
pub struct TimePolicy {
    pub enabled: bool,
    pub loss_threshold_pct: f64,
    pub duration_seconds: f64,
}

pub struct ExitPolicy {
    pub stop_loss: StopLossPolicy,
    pub trailing: TrailingPolicy,
    pub roi: RoiPolicy,
    pub time: TimePolicy,
    pub dca: DcaConfigSnapshot,
}

impl ExitPolicy {
    /// Snapshot the global trader config. One read per position per cycle replaces the
    /// scattered per-evaluator reads.
    pub fn from_config() -> Self {
        Self {
            stop_loss: StopLossPolicy {
                enabled: exit_stop_loss::is_stop_loss_enabled(),
                threshold_pct: exit_stop_loss::get_stop_loss_threshold_pct(),
                min_hold_seconds: exit_stop_loss::get_stop_loss_min_hold_seconds(),
                allow_partial: exit_stop_loss::get_stop_loss_allow_partial(),
                partial_exit_default_pct: config::get_partial_exit_default_pct(),
            },
            trailing: TrailingPolicy {
                enabled: config::is_trailing_stop_enabled(),
                activation_pct: config::get_trailing_stop_activation_pct(),
                distance_pct: config::get_trailing_stop_distance_pct(),
            },
            roi: RoiPolicy {
                enabled: config::is_roi_exit_enabled(),
                target_profit_pct: config::get_target_profit_pct(),
            },
            time: TimePolicy {
                enabled: config::is_time_override_enabled(),
                loss_threshold_pct: config::get_time_override_loss_threshold_pct(),
                duration_seconds: config::get_time_override_duration_seconds(),
            },
            dca: DcaConfigSnapshot {
                enabled: config::is_dca_enabled(),
                max_count: config::get_dca_max_count() as u32,
                cooldown_minutes: config::get_dca_cooldown_minutes(),
                threshold_pct: config::get_dca_threshold_pct(),
                size_percentage: config::get_dca_size_percentage(),
            },
        }
    }
}
