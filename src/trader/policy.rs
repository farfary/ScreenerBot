// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The knobs that decide what happens to an open position, resolved once instead of read
//! from global config inside each evaluator.
//!
//! Every exit rule used to reach into `config::get_*()` on its own, which made them
//! untestable without a config guard and meant one account could run exactly one risk
//! policy. Copy tasks layer explicit optional overrides onto this snapshot so an unset
//! field always means "inherit" rather than silently becoming a second exit stack.

use serde::{Deserialize, Serialize};

use crate::config::TakeProfitLevel;
use crate::positions::{Position, PositionOrigin};
use crate::trader::config;
use crate::trader::evaluators::dca::DcaConfigSnapshot;

/// The stop-loss rule's switch, threshold, minimum hold and partial first sale, read by
/// `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopLossPolicy {
    pub enabled: bool,
    pub threshold_pct: f64,
    pub min_hold_seconds: u64,
    pub allow_partial: bool,
    /// Share of the position a partial stop loss sells, from `trader.stop_loss_partial_pct`.
    pub partial_exit_default_pct: f64,
}

/// The trailing-stop rule's switch, activation and distance, read by `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailingPolicy {
    pub enabled: bool,
    pub activation_pct: f64,
    pub distance_pct: f64,
}

/// The break-even stop's switch, arming profit and kept margin, both in percent from the
/// average entry, read by `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakEvenPolicy {
    pub enabled: bool,
    pub trigger_pct: f64,
    pub offset_pct: f64,
}

/// The take-profit ladder's switch and steps, read by `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeProfitPolicy {
    pub enabled: bool,
    /// Ascending steps; each `sell_pct` is a share of the position as opened.
    pub levels: Vec<TakeProfitLevel>,
}

/// The time-override rule's switch, loss threshold and duration, read by `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimePolicy {
    pub enabled: bool,
    pub loss_threshold_pct: f64,
    pub duration_seconds: f64,
}

/// The stagnation exit's switch, holding time and flat band (percent of profit or loss
/// either side of the average entry), read by `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeStagnationPolicy {
    pub enabled: bool,
    pub duration_seconds: f64,
    pub band_pct: f64,
}

/// Copy tasks inherit `break_even` and `time_stagnation` from the trader config; no
/// task-level override exists for either.
#[derive(Debug, Clone)]
pub struct ExitPolicy {
    pub stop_loss: StopLossPolicy,
    pub trailing: TrailingPolicy,
    pub break_even: BreakEvenPolicy,
    pub take_profit: TakeProfitPolicy,
    pub time: TimePolicy,
    pub time_stagnation: TimeStagnationPolicy,
    pub dca: DcaConfigSnapshot,
}

/// Optional task-level fields. `None` is a durable, user-visible inherit state;
/// setting a value equal to the global value is still an explicit override.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExitPolicyOverrides {
    pub stop_loss: StopLossPolicyOverrides,
    pub trailing: TrailingPolicyOverrides,
    pub roi: RoiPolicyOverrides,
    pub time: TimePolicyOverrides,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StopLossPolicyOverrides {
    pub enabled: Option<bool>,
    pub threshold_pct: Option<f64>,
    pub min_hold_seconds: Option<u64>,
    pub allow_partial: Option<bool>,
    pub partial_exit_default_pct: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrailingPolicyOverrides {
    pub enabled: Option<bool>,
    pub activation_pct: Option<f64>,
    pub distance_pct: Option<f64>,
}

/// The single take-profit target a copy task may set: `target_profit_pct` replaces the
/// ladder with one step that sells everything at that profit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoiPolicyOverrides {
    pub enabled: Option<bool>,
    pub target_profit_pct: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimePolicyOverrides {
    pub enabled: Option<bool>,
    pub loss_threshold_pct: Option<f64>,
    pub duration_seconds: Option<f64>,
}

impl ExitPolicy {
    /// Snapshot the global trader config. One read per position per cycle replaces the
    /// scattered per-evaluator reads.
    pub fn from_config() -> Self {
        Self {
            stop_loss: StopLossPolicy {
                enabled: config::is_stop_loss_enabled(),
                threshold_pct: config::get_stop_loss_threshold_pct(),
                min_hold_seconds: config::get_stop_loss_min_hold_seconds(),
                allow_partial: config::get_stop_loss_allow_partial(),
                partial_exit_default_pct: config::get_stop_loss_partial_pct(),
            },
            trailing: TrailingPolicy {
                enabled: config::is_trailing_stop_enabled(),
                activation_pct: config::get_trailing_stop_activation_pct(),
                distance_pct: config::get_trailing_stop_distance_pct(),
            },
            break_even: BreakEvenPolicy {
                enabled: config::is_break_even_enabled(),
                trigger_pct: config::get_break_even_trigger_pct(),
                offset_pct: config::get_break_even_offset_pct(),
            },
            take_profit: TakeProfitPolicy {
                enabled: config::is_take_profit_enabled(),
                levels: config::get_take_profit_levels(),
            },
            time: TimePolicy {
                enabled: config::is_time_override_enabled(),
                loss_threshold_pct: config::get_time_override_loss_threshold_pct(),
                duration_seconds: config::get_time_override_duration_seconds(),
            },
            time_stagnation: TimeStagnationPolicy {
                enabled: config::is_time_exit_stagnation_enabled(),
                duration_seconds: config::get_time_exit_stagnation_minutes() * 60.0,
                band_pct: config::get_time_exit_stagnation_band_pct(),
            },
            dca: DcaConfigSnapshot {
                enabled: config::is_dca_enabled(),
                levels: config::get_dca_levels(),
                measure_from: config::get_dca_measure_from(),
                cooldown_minutes: config::get_dca_cooldown_minutes(),
                min_hold_minutes: config::get_dca_min_hold_minutes(),
            },
        }
    }

    pub fn apply_overrides(&mut self, overrides: &ExitPolicyOverrides) {
        apply(&mut self.stop_loss.enabled, overrides.stop_loss.enabled);
        apply(
            &mut self.stop_loss.threshold_pct,
            overrides.stop_loss.threshold_pct,
        );
        apply(
            &mut self.stop_loss.min_hold_seconds,
            overrides.stop_loss.min_hold_seconds,
        );
        apply(
            &mut self.stop_loss.allow_partial,
            overrides.stop_loss.allow_partial,
        );
        apply(
            &mut self.stop_loss.partial_exit_default_pct,
            overrides.stop_loss.partial_exit_default_pct,
        );
        apply(&mut self.trailing.enabled, overrides.trailing.enabled);
        apply(
            &mut self.trailing.activation_pct,
            overrides.trailing.activation_pct,
        );
        apply(
            &mut self.trailing.distance_pct,
            overrides.trailing.distance_pct,
        );
        apply(&mut self.take_profit.enabled, overrides.roi.enabled);
        if let Some(profit_pct) = overrides.roi.target_profit_pct {
            self.take_profit.levels = vec![TakeProfitLevel::sell_all_at(profit_pct)];
        }
        apply(&mut self.time.enabled, overrides.time.enabled);
        apply(
            &mut self.time.loss_threshold_pct,
            overrides.time.loss_threshold_pct,
        );
        apply(
            &mut self.time.duration_seconds,
            overrides.time.duration_seconds,
        );
    }
}

impl ExitPolicyOverrides {
    pub fn is_valid(&self) -> bool {
        optional_percent(self.stop_loss.threshold_pct, false)
            && optional_percent(self.stop_loss.partial_exit_default_pct, true)
            && optional_percent(self.trailing.activation_pct, false)
            && optional_percent(self.trailing.distance_pct, false)
            && optional_positive(self.roi.target_profit_pct)
            && self
                .time
                .loss_threshold_pct
                .is_none_or(|value| value.is_finite())
            && optional_positive(self.time.duration_seconds)
    }
}

fn optional_percent(value: Option<f64>, exclusive_maximum: bool) -> bool {
    value.is_none_or(|value| {
        value.is_finite()
            && value > 0.0
            && if exclusive_maximum {
                value < 100.0
            } else {
                value <= 100.0
            }
    })
}

fn optional_positive(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && value > 0.0)
}

fn apply<T: Copy>(target: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *target = value;
    }
}

/// Resolve global policy once, then overlay only the originating copy task's explicit
/// fields. A missing/deleted task inherits global policy so safety evaluation remains
/// available and no parallel exit stack is introduced.
pub async fn resolve_exit_policy(position: &Position) -> ExitPolicy {
    let mut policy = ExitPolicy::from_config();
    let PositionOrigin::Copy { task_id, .. } = &position.origin else {
        return policy;
    };
    let database = match tokio::task::spawn_blocking(|| {
        crate::trader::copy::CopyDatabase::shared(crate::chains::active_chain())
    })
    .await
    {
        Ok(Ok(database)) => database,
        Ok(Err(error)) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("Copy exit-policy database unavailable for task {task_id}: {error}"),
            );
            return policy;
        }
        Err(error) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("Copy exit-policy lookup failed for task {task_id}: {error}"),
            );
            return policy;
        }
    };
    match database.get_task(*task_id).await {
        Ok(Some(task)) => policy.apply_overrides(&task.exit_policy_overrides),
        Ok(None) => {}
        Err(error) => crate::logger::warning(
            crate::logger::LogTag::Trader,
            &format!("Copy exit-policy lookup failed for task {task_id}: {error}"),
        ),
    }
    policy
}
