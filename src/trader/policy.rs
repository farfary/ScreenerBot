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

use crate::config::{
    with_config, TakeProfitLevel, TraderConfig, TrailingStopBand, TRAILING_STOP_MAX_BANDS,
};
use crate::i18n::{ids, UiArg, UiText};
use crate::positions::{Position, PositionOrigin};
use crate::trader::evaluators::dca::DcaConfigSnapshot;

/// Why an exit policy cannot act as configured: the typed reason behind
/// `trader::Error::InvalidExitPolicy`, a refused trader config and a copy task whose
/// stored overrides no longer fit the global policy.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ExitPolicyFault {
    #[error("stop loss threshold must be greater than 0 and at most 100, got {value}")]
    StopLossThreshold { value: f64 },
    #[error("partial stop loss share must be between 0 and 100 (exclusive), got {value}")]
    PartialShare { value: f64 },
    #[error("take-profit target must be greater than 0, got {value}")]
    TakeProfitTarget { value: f64 },
    #[error("trailing stop activation must be greater than 0 and at most 100, got {value}")]
    TrailingActivation { value: f64 },
    #[error("trailing stop distance must be greater than 0 and at most 100, got {value}")]
    TrailingDistance { value: f64 },
    #[error("trailing stop distance ({distance:.1}%) must be less than its activation ({activation:.1}%)")]
    TrailingDistanceNotBelowActivation { distance: f64, activation: f64 },
    #[error("trailing stop holds {count} bands, at most {maximum} are allowed", maximum = TRAILING_STOP_MAX_BANDS)]
    TrailingBandCount { count: usize },
    #[error("trailing stop band at {profit}% must sit above the activation and the band before it ({floor}%)")]
    TrailingBandOrder { profit: f64, floor: f64 },
    #[error("trailing stop band at {profit}% must trail by more than 0 and less than its threshold, got {distance}%")]
    TrailingBandDistance { profit: f64, distance: f64 },
    #[error("time override duration must be greater than 0, got {seconds} seconds")]
    TimeDuration { seconds: f64 },
    #[error("time override loss threshold must be between -100 and 0, got {value}")]
    TimeLossThreshold { value: f64 },
}

impl ExitPolicyFault {
    /// The catalog text a copy task shows for this fault: the task editor's own wording
    /// for a rule of one field, `copy-exit-issue-*` for a rule that relates two values.
    pub fn ui_text(&self) -> UiText {
        let number = UiArg::Number;
        match *self {
            Self::StopLossThreshold { .. } => UiText::new(ids::COPY_VALIDATE_STOP_LOSS),
            Self::PartialShare { .. } => UiText::new(ids::COPY_VALIDATE_PARTIAL_SIZE),
            Self::TakeProfitTarget { .. } => UiText::new(ids::COPY_VALIDATE_TAKE_PROFIT),
            Self::TrailingActivation { .. } => UiText::new(ids::COPY_VALIDATE_TRAILING_ACTIVATION),
            Self::TrailingDistance { .. } => UiText::new(ids::COPY_VALIDATE_TRAILING_DISTANCE),
            Self::TrailingDistanceNotBelowActivation {
                distance,
                activation,
            } => UiText::new(ids::COPY_EXIT_ISSUE_TRAILING_DISTANCE_ABOVE_ACTIVATION)
                .arg("distance", number(distance))
                .arg("activation", number(activation)),
            Self::TrailingBandCount { .. } => UiText::new(ids::COPY_EXIT_ISSUE_TRAILING_BAND_COUNT)
                .arg("maximum", number(TRAILING_STOP_MAX_BANDS as f64)),
            Self::TrailingBandOrder { profit, floor } => {
                UiText::new(ids::COPY_EXIT_ISSUE_TRAILING_BAND_ORDER)
                    .arg("profit", number(profit))
                    .arg("floor", number(floor))
            }
            Self::TrailingBandDistance { profit, distance } => {
                UiText::new(ids::COPY_EXIT_ISSUE_TRAILING_BAND_DISTANCE)
                    .arg("profit", number(profit))
                    .arg("distance", number(distance))
            }
            Self::TimeDuration { .. } => UiText::new(ids::COPY_VALIDATE_TIME_DURATION),
            Self::TimeLossThreshold { value } => {
                UiText::new(ids::COPY_EXIT_ISSUE_TIME_LOSS_THRESHOLD).arg("value", number(value))
            }
        }
    }
}

impl From<ExitPolicyFault> for crate::trader::Error {
    fn from(fault: ExitPolicyFault) -> Self {
        Self::InvalidExitPolicy {
            detail: fault.to_string(),
        }
    }
}

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

/// The trailing-stop rule's switch, activation, base distance and distance bands, read by
/// `exit_rules`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailingPolicy {
    pub enabled: bool,
    pub activation_pct: f64,
    pub distance_pct: f64,
    /// Distances that replace the base distance once the basis peak reaches each band's
    /// profit; thresholds ascend and sit above the activation. Empty: the base distance at
    /// every profit.
    #[serde(default)]
    pub bands: Vec<TrailingStopBand>,
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
        with_config(|cfg| Self::from_trader_config(&cfg.trader))
    }

    /// The policy a trader config section describes; [`Self::from_config`] reads the
    /// global one, config validation reads the section being saved.
    pub fn from_trader_config(trader: &TraderConfig) -> Self {
        Self {
            stop_loss: StopLossPolicy {
                enabled: trader.stop_loss_enabled,
                threshold_pct: trader.stop_loss_threshold_pct,
                min_hold_seconds: trader.stop_loss_min_hold_seconds,
                allow_partial: trader.stop_loss_allow_partial,
                partial_exit_default_pct: trader.stop_loss_partial_pct,
            },
            trailing: TrailingPolicy {
                enabled: trader.trailing_stop_enabled,
                activation_pct: trader.trailing_stop_activation_pct,
                distance_pct: trader.trailing_stop_distance_pct,
                bands: trader.trailing_stop_bands(),
            },
            break_even: BreakEvenPolicy {
                enabled: trader.break_even_enabled,
                trigger_pct: trader.break_even_trigger_pct,
                offset_pct: trader.break_even_offset_pct,
            },
            take_profit: TakeProfitPolicy {
                enabled: trader.take_profit_enabled,
                levels: trader.take_profit_levels(),
            },
            time: TimePolicy {
                enabled: trader.time_override_enabled,
                loss_threshold_pct: trader.time_override_loss_threshold_percent,
                duration_seconds: trader
                    .time_override_unit
                    .to_seconds(trader.time_override_duration),
            },
            time_stagnation: TimeStagnationPolicy {
                enabled: trader.time_exit_stagnation_enabled,
                duration_seconds: trader.time_exit_stagnation_minutes * 60.0,
                band_pct: trader.time_exit_stagnation_band_pct,
            },
            dca: DcaConfigSnapshot {
                enabled: trader.dca_enabled,
                levels: trader.dca_levels(),
                measure_from: trader.dca_measure_from,
                cooldown_minutes: trader.dca_cooldown_minutes,
                min_hold_minutes: trader.dca_min_hold_minutes,
            },
        }
    }

    /// Refuse a policy whose stop loss, trailing stop or time override cannot act as
    /// configured. The one owner of these rules: the trader config and a copy task's
    /// merged overrides are both checked here, and the rules check their own policy
    /// before deciding.
    pub fn validate(&self) -> Result<(), ExitPolicyFault> {
        self.stop_loss.validate()?;
        self.trailing.validate()?;
        self.time.validate()
    }

    /// Overlay a copy task's overrides for the positions it runs. A rule whose merged
    /// values break [`Self::validate`] keeps this policy's own rule: a global change can
    /// leave a stored override contradicting a value it inherits (a task distance above a
    /// lowered global activation), and the global rule, which config validation keeps
    /// valid, is the last valid policy for that rule. The task's other rules still apply.
    pub fn apply_overrides(&mut self, overrides: &ExitPolicyOverrides) {
        let base = self.clone();
        self.merge_overrides(overrides);
        if self.stop_loss.validate().is_err() {
            self.stop_loss = base.stop_loss;
        }
        if self.trailing.validate().is_err() {
            self.trailing = base.trailing;
        }
        if self.time.validate().is_err() {
            self.time = base.time;
        }
    }

    /// Overlay every set override as it stands, valid or not.
    fn merge_overrides(&mut self, overrides: &ExitPolicyOverrides) {
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
        // Bands are tuned to the global activation and distance; a task that sets its own
        // trails at its own distance at every profit.
        if overrides.trailing.activation_pct.is_some() || overrides.trailing.distance_pct.is_some()
        {
            self.trailing.bands.clear();
        }
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
    /// The policy these overrides make of `base`, refused when the merged policy breaks
    /// a rule [`ExitPolicy::validate`] enforces or a share or target is out of range.
    pub fn validate_over(&self, mut base: ExitPolicy) -> Result<ExitPolicy, ExitPolicyFault> {
        if let Some(value) = self.stop_loss.partial_exit_default_pct {
            if !(value.is_finite() && value > 0.0 && value < 100.0) {
                return Err(ExitPolicyFault::PartialShare { value });
            }
        }
        if let Some(value) = self.roi.target_profit_pct {
            if !(value.is_finite() && value > 0.0) {
                return Err(ExitPolicyFault::TakeProfitTarget { value });
            }
        }
        base.merge_overrides(self);
        // A value the task sets is checked even while its rule is switched off: the switch
        // may be inherited and turned on later.
        let mut checked = base.clone();
        checked.stop_loss.enabled |= self.stop_loss.threshold_pct.is_some();
        checked.trailing.enabled |=
            self.trailing.activation_pct.is_some() || self.trailing.distance_pct.is_some();
        checked.time.enabled |=
            self.time.loss_threshold_pct.is_some() || self.time.duration_seconds.is_some();
        checked.validate()?;
        Ok(base)
    }
}

impl StopLossPolicy {
    /// An enabled stop loss needs a finite loss threshold above 0 and at most 100.
    pub fn validate(&self) -> Result<(), ExitPolicyFault> {
        if self.enabled && !percent_in_range(self.threshold_pct) {
            return Err(ExitPolicyFault::StopLossThreshold {
                value: self.threshold_pct,
            });
        }
        Ok(())
    }
}

impl TrailingPolicy {
    /// The distance the trail follows below a basis peak `peak_profit_pct` above the
    /// entry: the band with the highest threshold the peak has reached, else the base
    /// distance.
    pub fn distance_in_force(&self, peak_profit_pct: f64) -> f64 {
        self.bands
            .iter()
            .filter(|band| band.profit_pct <= peak_profit_pct)
            .max_by(|a, b| a.profit_pct.total_cmp(&b.profit_pct))
            .map_or(self.distance_pct, |band| band.distance_pct)
    }

    /// An enabled trailing stop needs an activation and a distance above 0 and at most
    /// 100, a distance below the activation (a trail armed at the activation must sit in
    /// profit), and bands that pass [`Self::validate_bands`].
    pub fn validate(&self) -> Result<(), ExitPolicyFault> {
        if !self.enabled {
            return Ok(());
        }
        if !percent_in_range(self.activation_pct) {
            return Err(ExitPolicyFault::TrailingActivation {
                value: self.activation_pct,
            });
        }
        if !percent_in_range(self.distance_pct) {
            return Err(ExitPolicyFault::TrailingDistance {
                value: self.distance_pct,
            });
        }
        if self.distance_pct >= self.activation_pct {
            return Err(ExitPolicyFault::TrailingDistanceNotBelowActivation {
                distance: self.distance_pct,
                activation: self.activation_pct,
            });
        }
        self.validate_bands()
    }

    /// At most [`TRAILING_STOP_MAX_BANDS`] bands whose thresholds ascend above the
    /// activation, each with a distance above 0 and below its threshold. Checked whether
    /// or not the trail is on: a copy task that switches the trail on inherits the bands.
    pub fn validate_bands(&self) -> Result<(), ExitPolicyFault> {
        if self.bands.len() > TRAILING_STOP_MAX_BANDS {
            return Err(ExitPolicyFault::TrailingBandCount {
                count: self.bands.len(),
            });
        }
        let mut floor = self.activation_pct;
        for band in &self.bands {
            if !band.profit_pct.is_finite() || band.profit_pct <= floor {
                return Err(ExitPolicyFault::TrailingBandOrder {
                    profit: band.profit_pct,
                    floor,
                });
            }
            if !band.distance_pct.is_finite()
                || band.distance_pct <= 0.0
                || band.distance_pct >= band.profit_pct
            {
                return Err(ExitPolicyFault::TrailingBandDistance {
                    profit: band.profit_pct,
                    distance: band.distance_pct,
                });
            }
            floor = band.profit_pct;
        }
        Ok(())
    }
}

impl TimePolicy {
    /// An enabled time override needs a finite duration above 0 and a finite loss
    /// threshold from -100 to 0 (a positive threshold would close a winning position).
    pub fn validate(&self) -> Result<(), ExitPolicyFault> {
        if !self.enabled {
            return Ok(());
        }
        if !self.duration_seconds.is_finite() || self.duration_seconds <= 0.0 {
            return Err(ExitPolicyFault::TimeDuration {
                seconds: self.duration_seconds,
            });
        }
        if !self.loss_threshold_pct.is_finite()
            || !(-100.0..=0.0).contains(&self.loss_threshold_pct)
        {
            return Err(ExitPolicyFault::TimeLossThreshold {
                value: self.loss_threshold_pct,
            });
        }
        Ok(())
    }
}

fn percent_in_range(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= 100.0
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
