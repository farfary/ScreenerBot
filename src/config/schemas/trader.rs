// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading system configuration.

use crate::config::metadata::{
    ConfigCategory, ConfigImpact, FieldType, FieldTypeInfo, NestedMetadata,
};
use crate::config::Result;
use crate::errors::ConfigurationError;
use crate::field_metadata;
use crate::trader::paper::fill::{
    DEFAULT_LANDING_DELAY_MS, MAX_LANDING_DELAY_MS, READINESS_ROUNDS_RANGE,
};
use crate::trader::MIN_TRADE_SIZE_NATIVE;
use serde::{Deserialize, Serialize};

/// Unit of the time-override duration, stored as its lowercase name. Every write is
/// strict: a name that is neither a unit nor one of its short aliases is refused. A
/// config file that holds such a name still loads, because the load-time migration
/// (`config/migrate.rs`) rewrites it to hours before the typed parse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    #[default]
    Hours,
    Days,
}

/// Every canonical unit name, in the order the units grow.
pub(crate) const TIME_UNIT_NAMES: &[&str] = &["seconds", "minutes", "hours", "days"];

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

    /// The canonical lowercase name the unit is stored and sent as.
    pub const fn as_str(self) -> &'static str {
        match self {
            TimeUnit::Seconds => "seconds",
            TimeUnit::Minutes => "minutes",
            TimeUnit::Hours => "hours",
            TimeUnit::Days => "days",
        }
    }

    /// The unit `name` names, case-insensitively and with its short aliases; `None` for
    /// a name that matches no unit.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "seconds" | "s" | "sec" => Some(TimeUnit::Seconds),
            "minutes" | "m" | "min" => Some(TimeUnit::Minutes),
            "hours" | "h" | "hr" => Some(TimeUnit::Hours),
            "days" | "d" | "day" => Some(TimeUnit::Days),
            _ => None,
        }
    }
}

impl Serialize for TimeUnit {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TimeUnit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name)
            .ok_or_else(|| serde::de::Error::unknown_variant(&name, TIME_UNIT_NAMES))
    }
}

impl FieldTypeInfo for TimeUnit {
    fn field_type() -> FieldType {
        FieldType::String
    }
}

impl NestedMetadata for TimeUnit {}

/// The price a DCA step's drop is measured from, stored as its lowercase name. Every
/// write is strict: a name that matches no basis, in any letter case, is refused. A
/// config file that holds such a name still loads, because the load-time migration
/// (`config/migrate.rs`) rewrites it to the average entry before the typed parse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DcaMeasureFrom {
    /// The average entry price of every buy booked so far; an add moves it, so each step
    /// measures from the average the previous add left.
    #[default]
    Average,
    /// The fill price of the position's first buy; it never moves, so the steps must
    /// reach strictly deeper drops.
    FirstEntry,
}

/// Every canonical basis name.
pub(crate) const DCA_MEASURE_FROM_NAMES: &[&str] = &["average", "first_entry"];

impl DcaMeasureFrom {
    /// The canonical lowercase name the basis is stored and sent as.
    pub const fn as_str(self) -> &'static str {
        match self {
            DcaMeasureFrom::Average => "average",
            DcaMeasureFrom::FirstEntry => "first_entry",
        }
    }

    /// The basis `name` names, case-insensitively; `None` for a name that matches no
    /// basis.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "average" => Some(DcaMeasureFrom::Average),
            "first_entry" => Some(DcaMeasureFrom::FirstEntry),
            _ => None,
        }
    }
}

impl Serialize for DcaMeasureFrom {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DcaMeasureFrom {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name)
            .ok_or_else(|| serde::de::Error::unknown_variant(&name, DCA_MEASURE_FROM_NAMES))
    }
}

impl FieldTypeInfo for DcaMeasureFrom {
    fn field_type() -> FieldType {
        FieldType::String
    }
}

impl NestedMetadata for DcaMeasureFrom {}

config_struct! {
    /// Slippage configuration
    pub struct SlippageConfig {
        #[metadata(field_metadata! {
            min: 0.1,
            max: 25,
            step: 0.1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Quote,
        })]
        quote_default_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 50,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Exit,
        })]
        exit_profit_shortfall_pct: f64 = 3.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 50,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Exit,
        })]
        exit_loss_shortfall_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Exit,
        })]
        exit_retry_steps_pct: Vec<f64> = vec![3.0, 10.0, 25.0],
    }
}

/// Bounds of the paper account's starting balance, in SOL.
const PAPER_STARTING_BALANCE_SOL_MIN: f64 = 0.01;
const PAPER_STARTING_BALANCE_SOL_MAX: f64 = 1000.0;

config_struct! {
    /// Paper trading of the Auto Trader: the simulated account Paper strategies trade
    /// against, the simulated landing delay of a paper fill, and the closed paper rounds
    /// a strategy needs before it reads as ready for live.
    pub struct PaperTradingConfig {
        /// SOL the paper account starts each run with.
        #[metadata(field_metadata! {
            min: PAPER_STARTING_BALANCE_SOL_MIN,
            max: PAPER_STARTING_BALANCE_SOL_MAX,
            step: 0.01,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::PaperTrading,
        })]
        starting_balance_sol: f64 = 1.0,
        /// Wait between a paper decision and the quote it fills at, standing in for
        /// the time a live transaction takes to land.
        #[metadata(field_metadata! {
            min: 0,
            max: MAX_LANDING_DELAY_MS,
            step: 100,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::PaperTrading,
        })]
        landing_delay_ms: u64 = DEFAULT_LANDING_DELAY_MS,
        /// Closed paper rounds a strategy needs before its live readiness passes.
        #[metadata(field_metadata! {
            min: *READINESS_ROUNDS_RANGE.start(),
            max: *READINESS_ROUNDS_RANGE.end(),
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PaperTrading,
        })]
        readiness_min_closed_rounds: usize = 10,
    }
}

impl PaperTradingConfig {
    /// Refuse a starting balance, landing delay or readiness threshold outside its
    /// bounds.
    fn validate(&self) -> Result<()> {
        if !self.starting_balance_sol.is_finite()
            || !(PAPER_STARTING_BALANCE_SOL_MIN..=PAPER_STARTING_BALANCE_SOL_MAX)
                .contains(&self.starting_balance_sol)
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.paper.starting_balance_sol must be between {PAPER_STARTING_BALANCE_SOL_MIN} and {PAPER_STARTING_BALANCE_SOL_MAX}"
                ),
            }
            .into());
        }
        if self.landing_delay_ms > MAX_LANDING_DELAY_MS {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.paper.landing_delay_ms must be between 0 and {MAX_LANDING_DELAY_MS}"
                ),
            }
            .into());
        }
        if !READINESS_ROUNDS_RANGE.contains(&self.readiness_min_closed_rounds) {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.paper.readiness_min_closed_rounds must be between {} and {}",
                    READINESS_ROUNDS_RANGE.start(),
                    READINESS_ROUNDS_RANGE.end()
                ),
            }
            .into());
        }
        Ok(())
    }
}

/// The most steps a take-profit ladder holds; the Take Profit tab reads it from the
/// field metadata.
pub const TAKE_PROFIT_MAX_STEPS: usize = 4;
/// Bounds of one take-profit step's profit target, in percent from the average entry.
pub const TAKE_PROFIT_PROFIT_PCT_MIN: f64 = 1.0;
pub const TAKE_PROFIT_PROFIT_PCT_MAX: f64 = 1000.0;
/// Bounds of one take-profit step's sell share, in percent of the position as opened.
/// The maximum is the whole position, which a single-step ladder sells.
const TAKE_PROFIT_SELL_PCT_MIN: f64 = 5.0;
pub const TAKE_PROFIT_SELL_PCT_MAX: f64 = 100.0;

/// The most steps a DCA ladder holds; the DCA tab reads it from the field metadata.
pub const DCA_MAX_STEPS: usize = 5;
/// Bounds of one DCA step's drop, in percent from the chosen basis; a step is always a
/// loss.
pub const DCA_DROP_PCT_MIN: f64 = -95.0;
pub const DCA_DROP_PCT_MAX: f64 = -1.0;
/// Bounds of one DCA step's size, as a multiple of the position's first buy.
pub const DCA_SIZE_MULTIPLIER_MIN: f64 = 0.1;
pub const DCA_SIZE_MULTIPLIER_MAX: f64 = 5.0;
/// Bounds of the wait after a position's entry before its first add, in minutes.
const DCA_MIN_HOLD_MINUTES_MIN: i64 = 0;
const DCA_MIN_HOLD_MINUTES_MAX: i64 = 1440;

/// One step of the DCA ladder: once the price is `drop_pct` (negative) or further below
/// the basis, buy `size_multiplier` times the position's first buy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DcaLevel {
    pub drop_pct: f64,
    pub size_multiplier: f64,
}

impl DcaLevel {
    /// The amount an add at this step buys, in native units, for a position whose first
    /// buy spent `first_buy_native`.
    pub fn add_size_native(&self, first_buy_native: f64) -> f64 {
        first_buy_native * self.size_multiplier
    }
}

/// The most distance bands a trailing stop holds; the Trailing Stop tab reads it from
/// the field metadata.
pub const TRAILING_STOP_MAX_BANDS: usize = 3;
/// Bounds of one trailing band's threshold, in percent of basis-peak profit from the
/// average entry.
pub const TRAILING_STOP_BAND_PROFIT_PCT_MIN: f64 = 1.0;
pub const TRAILING_STOP_BAND_PROFIT_PCT_MAX: f64 = 1000.0;
/// Bounds of one trailing band's distance, in percent below the peak.
pub const TRAILING_STOP_BAND_DISTANCE_PCT_MIN: f64 = 1.0;
pub const TRAILING_STOP_BAND_DISTANCE_PCT_MAX: f64 = 90.0;

/// One distance band of the trailing stop: once the basis peak stands `profit_pct` or
/// more above the average entry, the trail follows `distance_pct` below the peak in
/// place of the base distance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrailingStopBand {
    pub profit_pct: f64,
    pub distance_pct: f64,
}

/// Bounds of the break-even stop's arming profit and its kept margin, in percent from
/// the average entry.
const BREAK_EVEN_TRIGGER_PCT_MIN: f64 = 0.5;
const BREAK_EVEN_TRIGGER_PCT_MAX: f64 = 1000.0;
const BREAK_EVEN_OFFSET_PCT_MIN: f64 = 0.0;
const BREAK_EVEN_OFFSET_PCT_MAX: f64 = 50.0;
/// Bounds of the stagnation exit's holding time, in minutes, and of its flat band, in
/// percent of profit or loss either side of the average entry.
const TIME_EXIT_STAGNATION_MINUTES_MIN: f64 = 5.0;
const TIME_EXIT_STAGNATION_MINUTES_MAX: f64 = 43200.0;
const TIME_EXIT_STAGNATION_BAND_PCT_MIN: f64 = 0.5;
const TIME_EXIT_STAGNATION_BAND_PCT_MAX: f64 = 50.0;

/// One step of the take-profit ladder: once profit from the average entry reaches
/// `profit_pct`, sell `sell_pct` of the position as it was opened.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TakeProfitLevel {
    pub profit_pct: f64,
    pub sell_pct: f64,
}

impl TakeProfitLevel {
    /// The one step of a single-target ladder: sell the whole position at `profit_pct`.
    pub fn sell_all_at(profit_pct: f64) -> Self {
        Self {
            profit_pct,
            sell_pct: TAKE_PROFIT_SELL_PCT_MAX,
        }
    }
}

config_struct! {
    /// Trading system configuration
    pub struct TraderConfig {
        // Trader control
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        enabled: bool = false,

        // Core trading parameters
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        max_open_positions: usize = 2,
        /// SOL each automatic entry buys; its minimum is the smallest trade the trader
        /// sends, which the DCA tab also reads to flag a step too small to buy.
        #[metadata(field_metadata! {
            min: MIN_TRADE_SIZE_NATIVE,
            max: 10,
            step: 0.001,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        trade_size_sol: f64 = 0.005,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CoreTrading,
        })]
        entry_sizes: Vec<f64> = vec![0.005, 0.01, 0.02, 0.05],

        /// Slippage tolerances for quotes and exit retries. Percent rules,
        /// identical on every chain.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Slippage,
        })]
        slippage: SlippageConfig = SlippageConfig::default(),

        /// The simulated account and thresholds of Paper strategies.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::PaperTrading,
        })]
        paper: PaperTradingConfig = PaperTradingConfig::default(),

        // ==================== TAKE PROFIT CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TakeProfit,
        })]
        take_profit_enabled: bool = true,
        /// Profit from the average entry at which each step sells, ascending; read
        /// together with `take_profit_sell_pct` through `take_profit_levels`.
        #[metadata(field_metadata! {
            min: TAKE_PROFIT_PROFIT_PCT_MIN,
            max: TAKE_PROFIT_PROFIT_PCT_MAX,
            step: 1,
            max_items: TAKE_PROFIT_MAX_STEPS,
            impact: ConfigImpact::High,
            category: ConfigCategory::TakeProfit,
        })]
        take_profit_levels_pct: Vec<f64> = vec![20.0],
        /// Share of the position as opened that each step sells; the shares add up to
        /// at most 100, and what they leave is the runner.
        #[metadata(field_metadata! {
            min: TAKE_PROFIT_SELL_PCT_MIN,
            max: TAKE_PROFIT_SELL_PCT_MAX,
            step: 1,
            max_items: TAKE_PROFIT_MAX_STEPS,
            impact: ConfigImpact::High,
            category: ConfigCategory::TakeProfit,
        })]
        take_profit_sell_pct: Vec<f64> = vec![100.0],

        // ==================== TIME OVERRIDE CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 1,
            max: 43200,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_duration: f64 = 168.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_unit: TimeUnit = TimeUnit::Hours,
        #[metadata(field_metadata! {
            min: -100,
            max: 0,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_loss_threshold_percent: f64 = -40.0,
        /// Close a position whose profit or loss has stayed inside a narrow band for a
        /// set time, whatever its direction.
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TimeOverride,
        })]
        time_exit_stagnation_enabled: bool = false,
        #[metadata(field_metadata! {
            min: TIME_EXIT_STAGNATION_MINUTES_MIN,
            max: TIME_EXIT_STAGNATION_MINUTES_MAX,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::TimeOverride,
        })]
        time_exit_stagnation_minutes: f64 = 240.0,
        #[metadata(field_metadata! {
            min: TIME_EXIT_STAGNATION_BAND_PCT_MIN,
            max: TIME_EXIT_STAGNATION_BAND_PCT_MAX,
            step: 0.5,
            impact: ConfigImpact::High,
            category: ConfigCategory::TimeOverride,
        })]
        time_exit_stagnation_band_pct: f64 = 5.0,

        // ==================== STOP LOSS CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_threshold_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_allow_partial: bool = false,
        /// Share of the position the first stop-loss sale sells while
        /// `stop_loss_allow_partial` is on; a later trigger sells the rest.
        #[metadata(field_metadata! {
            min: 10,
            max: 90,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_partial_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 86400,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_min_hold_seconds: u64 = 0,

        // ==================== TRAILING STOP CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0.1,
            max: 100,
            step: 0.1,
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_activation_pct: f64 = 10.0,
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_distance_pct: f64 = 5.0,
        /// Basis-peak profit from the average entry at which each band's distance takes
        /// over, ascending and above the activation; read together with
        /// `trailing_stop_band_distance_pct` through `trailing_stop_bands`. Empty trails
        /// at the base distance alone.
        #[metadata(field_metadata! {
            min: TRAILING_STOP_BAND_PROFIT_PCT_MIN,
            max: TRAILING_STOP_BAND_PROFIT_PCT_MAX,
            step: 1,
            max_items: TRAILING_STOP_MAX_BANDS,
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_band_profit_pct: Vec<f64> = vec![],
        /// Distance below the peak each band trails at, below its own threshold.
        #[metadata(field_metadata! {
            min: TRAILING_STOP_BAND_DISTANCE_PCT_MIN,
            max: TRAILING_STOP_BAND_DISTANCE_PCT_MAX,
            step: 1,
            max_items: TRAILING_STOP_MAX_BANDS,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_band_distance_pct: Vec<f64> = vec![],
        /// Once the best price since the last buy reaches `break_even_trigger_pct` above
        /// the average entry, sell everything when the price falls back to
        /// `break_even_offset_pct` above it.
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        break_even_enabled: bool = false,
        #[metadata(field_metadata! {
            min: BREAK_EVEN_TRIGGER_PCT_MIN,
            max: BREAK_EVEN_TRIGGER_PCT_MAX,
            step: 0.5,
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        break_even_trigger_pct: f64 = 15.0,
        #[metadata(field_metadata! {
            min: BREAK_EVEN_OFFSET_PCT_MIN,
            max: BREAK_EVEN_OFFSET_PCT_MAX,
            step: 0.5,
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        break_even_offset_pct: f64 = 1.0,

        // Position timing
        #[metadata(field_metadata! {
            min: 0,
            max: 1440,
            step: 5,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Timing,
        })]
        position_close_cooldown_minutes: i64 = 15,

        // Performance settings
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        entry_check_concurrency: usize = 10,

        // Sell concurrency
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        sell_concurrency: usize = 5,

        // ==================== DCA CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_enabled: bool = false,
        /// Drop from the basis, in percent (negative), at which each step adds; read
        /// together with `dca_size_multipliers` through `dca_levels`.
        #[metadata(field_metadata! {
            min: DCA_DROP_PCT_MIN,
            max: DCA_DROP_PCT_MAX,
            step: 1,
            max_items: DCA_MAX_STEPS,
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_levels_pct: Vec<f64> = vec![-10.0, -10.0],
        /// Size of each step's add, as a multiple of the position's first buy.
        #[metadata(field_metadata! {
            min: DCA_SIZE_MULTIPLIER_MIN,
            max: DCA_SIZE_MULTIPLIER_MAX,
            step: 0.1,
            max_items: DCA_MAX_STEPS,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Dca,
        })]
        dca_size_multipliers: Vec<f64> = vec![0.5, 0.5],
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_measure_from: DcaMeasureFrom = DcaMeasureFrom::Average,
        /// Minutes after the position's entry before its first add may run.
        #[metadata(field_metadata! {
            min: DCA_MIN_HOLD_MINUTES_MIN,
            max: DCA_MIN_HOLD_MINUTES_MAX,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Dca,
        })]
        dca_min_hold_minutes: i64 = 0,
        #[metadata(field_metadata! {
            min: 1,
            max: 1440,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Dca,
        })]
        dca_cooldown_minutes: i64 = 30,

        // ==================== ENTRY/EXIT MONITOR CONTROL ====================
        /// Enable entry monitor (scans for new entry opportunities)
        /// When false, entry monitor pauses but exit monitor continues
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::MonitorControl,
        })]
        entry_monitor_enabled: bool = true,

        /// Enable exit monitor (manages open positions, stop losses, exits)
        /// When false, exit monitor pauses - USE WITH CAUTION
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::MonitorControl,
        })]
        exit_monitor_enabled: bool = true,

        // ==================== PERIOD LOSS LIMIT PROTECTION ====================
        /// Enable period-based loss limit protection
        /// When cumulative realized losses exceed the limit, entry monitor pauses
        /// Default: false
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_enabled: bool = false,

        /// Maximum realized loss allowed in the period (in SOL)
        /// When cumulative losses reach this amount, new entries are blocked
        /// Default: 0.1 SOL
        #[metadata(field_metadata! {
            min: 0.001,
            max: 100.0,
            step: 0.01,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_sol: f64 = 0.1,

        /// Loss limit period in hours (1, 6, 12, 24, etc.)
        /// After this period, cumulative loss tracking resets
        /// Default: 24 (daily limit)
        #[metadata(field_metadata! {
            min: 1,
            max: 168,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_period_hours: u64 = 24,

        /// Auto-resume when period resets
        /// If true, entry monitor automatically resumes after period reset
        /// If false, manual resume required via dashboard
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_auto_resume: bool = true,
    }
}

impl TraderConfig {
    /// The take-profit ladder, one step per configured level, in ascending order. The
    /// only reader of `take_profit_levels_pct` and `take_profit_sell_pct`.
    pub fn take_profit_levels(&self) -> Vec<TakeProfitLevel> {
        self.take_profit_levels_pct
            .iter()
            .zip(&self.take_profit_sell_pct)
            .map(|(&profit_pct, &sell_pct)| TakeProfitLevel {
                profit_pct,
                sell_pct,
            })
            .collect()
    }

    /// The trailing stop's distance bands, one per configured threshold, in ascending
    /// order. The only reader of `trailing_stop_band_profit_pct` and
    /// `trailing_stop_band_distance_pct`.
    pub fn trailing_stop_bands(&self) -> Vec<TrailingStopBand> {
        self.trailing_stop_band_profit_pct
            .iter()
            .zip(&self.trailing_stop_band_distance_pct)
            .map(|(&profit_pct, &distance_pct)| TrailingStopBand {
                profit_pct,
                distance_pct,
            })
            .collect()
    }

    /// The DCA ladder, one step per configured drop, in the order the adds run. The only
    /// reader of `dca_levels_pct` and `dca_size_multipliers`.
    pub fn dca_levels(&self) -> Vec<DcaLevel> {
        self.dca_levels_pct
            .iter()
            .zip(&self.dca_size_multipliers)
            .map(|(&drop_pct, &size_multiplier)| DcaLevel {
                drop_pct,
                size_multiplier,
            })
            .collect()
    }

    /// Refuse a take-profit ladder the exit rule cannot follow: one to
    /// [`TAKE_PROFIT_MAX_STEPS`] steps with a sell share for every level, each value
    /// inside its bounds, levels strictly ascending, and shares adding up to at most 100.
    /// Refuse a DCA ladder the evaluator cannot follow (see [`Self::validate_dca`]).
    /// Refuse a break-even stop or stagnation exit outside its bounds, and a break-even
    /// margin that is not below its arming profit.
    /// Refuse trailing bands the rule cannot follow (see [`Self::validate_trailing_bands`]),
    /// and an exit policy whose rules contradict each other
    /// (`trader::policy::ExitPolicy::validate`, the one owner of those rules).
    /// Refuse paper trading settings outside their bounds.
    pub fn validate(&self) -> Result<()> {
        self.paper.validate()?;
        self.validate_dca()?;
        self.validate_trailing_bands()?;
        crate::trader::policy::ExitPolicy::from_trader_config(self)
            .validate()
            .map_err(|error| ConfigurationError::Generic {
                message: format!("trader: {error}"),
            })?;
        let steps = self.take_profit_levels_pct.len();
        if !(1..=TAKE_PROFIT_MAX_STEPS).contains(&steps) {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.take_profit_levels_pct must hold between 1 and {TAKE_PROFIT_MAX_STEPS} steps"
                ),
            }
            .into());
        }
        if self.take_profit_sell_pct.len() != steps {
            return Err(ConfigurationError::Generic {
                message: "trader.take_profit_sell_pct must hold one share per take-profit level"
                    .to_owned(),
            }
            .into());
        }
        let in_range =
            |value: f64, min: f64, max: f64| value.is_finite() && (min..=max).contains(&value);
        if !self.take_profit_levels_pct.iter().all(|&level| {
            in_range(
                level,
                TAKE_PROFIT_PROFIT_PCT_MIN,
                TAKE_PROFIT_PROFIT_PCT_MAX,
            )
        }) {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.take_profit_levels_pct values must be between {TAKE_PROFIT_PROFIT_PCT_MIN} and {TAKE_PROFIT_PROFIT_PCT_MAX}"
                ),
            }
            .into());
        }
        if !self
            .take_profit_sell_pct
            .iter()
            .all(|&share| in_range(share, TAKE_PROFIT_SELL_PCT_MIN, TAKE_PROFIT_SELL_PCT_MAX))
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.take_profit_sell_pct values must be between {TAKE_PROFIT_SELL_PCT_MIN} and {TAKE_PROFIT_SELL_PCT_MAX}"
                ),
            }
            .into());
        }
        if self
            .take_profit_levels_pct
            .windows(2)
            .any(|pair| pair[1] <= pair[0])
        {
            return Err(ConfigurationError::Generic {
                message: "trader.take_profit_levels_pct must be strictly ascending".to_owned(),
            }
            .into());
        }
        if self.take_profit_sell_pct.iter().sum::<f64>()
            > TAKE_PROFIT_SELL_PCT_MAX
                + crate::trader::evaluators::exit_take_profit::TAKE_PROFIT_SHARE_SUM_TOLERANCE_PCT
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.take_profit_sell_pct shares must add up to at most {TAKE_PROFIT_SELL_PCT_MAX}"
                ),
            }
            .into());
        }
        for (field, value, min, max) in [
            (
                "break_even_trigger_pct",
                self.break_even_trigger_pct,
                BREAK_EVEN_TRIGGER_PCT_MIN,
                BREAK_EVEN_TRIGGER_PCT_MAX,
            ),
            (
                "break_even_offset_pct",
                self.break_even_offset_pct,
                BREAK_EVEN_OFFSET_PCT_MIN,
                BREAK_EVEN_OFFSET_PCT_MAX,
            ),
            (
                "time_exit_stagnation_minutes",
                self.time_exit_stagnation_minutes,
                TIME_EXIT_STAGNATION_MINUTES_MIN,
                TIME_EXIT_STAGNATION_MINUTES_MAX,
            ),
            (
                "time_exit_stagnation_band_pct",
                self.time_exit_stagnation_band_pct,
                TIME_EXIT_STAGNATION_BAND_PCT_MIN,
                TIME_EXIT_STAGNATION_BAND_PCT_MAX,
            ),
        ] {
            if !in_range(value, min, max) {
                return Err(ConfigurationError::Generic {
                    message: format!("trader.{field} must be between {min} and {max}"),
                }
                .into());
            }
        }
        if self.break_even_offset_pct >= self.break_even_trigger_pct {
            return Err(ConfigurationError::Generic {
                message: "trader.break_even_offset_pct must be below trader.break_even_trigger_pct"
                    .to_owned(),
            }
            .into());
        }
        Ok(())
    }

    /// Refuse trailing bands the configuration cannot describe: at most
    /// [`TRAILING_STOP_MAX_BANDS`] bands with a distance for every threshold, each value
    /// inside its bounds, in the order the exit policy follows
    /// (`trader::policy::TrailingPolicy::validate_bands`, the one owner of that rule).
    /// The order is checked while the trailing stop is off too: a copy task that switches
    /// the trail on inherits these bands.
    fn validate_trailing_bands(&self) -> Result<()> {
        let bands = self.trailing_stop_band_profit_pct.len();
        if bands > TRAILING_STOP_MAX_BANDS {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.trailing_stop_band_profit_pct must hold at most {TRAILING_STOP_MAX_BANDS} bands"
                ),
            }
            .into());
        }
        if self.trailing_stop_band_distance_pct.len() != bands {
            return Err(ConfigurationError::Generic {
                message:
                    "trader.trailing_stop_band_distance_pct must hold one distance per trailing band"
                        .to_owned(),
            }
            .into());
        }
        for (field, values, min, max) in [
            (
                "trailing_stop_band_profit_pct",
                &self.trailing_stop_band_profit_pct,
                TRAILING_STOP_BAND_PROFIT_PCT_MIN,
                TRAILING_STOP_BAND_PROFIT_PCT_MAX,
            ),
            (
                "trailing_stop_band_distance_pct",
                &self.trailing_stop_band_distance_pct,
                TRAILING_STOP_BAND_DISTANCE_PCT_MIN,
                TRAILING_STOP_BAND_DISTANCE_PCT_MAX,
            ),
        ] {
            if !values
                .iter()
                .all(|&value| value.is_finite() && (min..=max).contains(&value))
            {
                return Err(ConfigurationError::Generic {
                    message: format!("trader.{field} values must be between {min} and {max}"),
                }
                .into());
            }
        }
        crate::trader::policy::ExitPolicy::from_trader_config(self)
            .trailing
            .validate_bands()
            .map_err(|fault| ConfigurationError::Generic {
                message: format!("trader: {fault}"),
            })?;
        Ok(())
    }

    /// Refuse a DCA ladder the evaluator cannot follow: one to [`DCA_MAX_STEPS`] steps
    /// with a size for every drop, each drop and size inside its bounds, and, measured
    /// from the first entry (a basis no add moves), every step deeper than the one before
    /// it. Measured from the average entry, equal drops are a valid uniform ladder. The
    /// wait after entry must lie inside its bounds.
    fn validate_dca(&self) -> Result<()> {
        let steps = self.dca_levels_pct.len();
        if !(1..=DCA_MAX_STEPS).contains(&steps) {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.dca_levels_pct must hold between 1 and {DCA_MAX_STEPS} steps"
                ),
            }
            .into());
        }
        if self.dca_size_multipliers.len() != steps {
            return Err(ConfigurationError::Generic {
                message: "trader.dca_size_multipliers must hold one size per DCA level".to_owned(),
            }
            .into());
        }
        let in_range =
            |value: f64, min: f64, max: f64| value.is_finite() && (min..=max).contains(&value);
        if !self
            .dca_levels_pct
            .iter()
            .all(|&drop| in_range(drop, DCA_DROP_PCT_MIN, DCA_DROP_PCT_MAX))
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.dca_levels_pct values must be between {DCA_DROP_PCT_MIN} and {DCA_DROP_PCT_MAX}"
                ),
            }
            .into());
        }
        if !self
            .dca_size_multipliers
            .iter()
            .all(|&size| in_range(size, DCA_SIZE_MULTIPLIER_MIN, DCA_SIZE_MULTIPLIER_MAX))
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.dca_size_multipliers values must be between {DCA_SIZE_MULTIPLIER_MIN} and {DCA_SIZE_MULTIPLIER_MAX}"
                ),
            }
            .into());
        }
        if self.dca_measure_from == DcaMeasureFrom::FirstEntry
            && self
                .dca_levels_pct
                .windows(2)
                .any(|pair| pair[1] >= pair[0])
        {
            return Err(ConfigurationError::Generic {
                message: "trader.dca_levels_pct must reach a deeper drop at every step when measured from the first entry"
                    .to_owned(),
            }
            .into());
        }
        if !(DCA_MIN_HOLD_MINUTES_MIN..=DCA_MIN_HOLD_MINUTES_MAX)
            .contains(&self.dca_min_hold_minutes)
        {
            return Err(ConfigurationError::Generic {
                message: format!(
                    "trader.dca_min_hold_minutes must be between {DCA_MIN_HOLD_MINUTES_MIN} and {DCA_MIN_HOLD_MINUTES_MAX}"
                ),
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DcaLevel, DcaMeasureFrom, PaperTradingConfig, TakeProfitLevel, TimeUnit, TraderConfig,
        DCA_MAX_STEPS, TAKE_PROFIT_MAX_STEPS,
    };
    use crate::trader::paper::fill::{MAX_LANDING_DELAY_MS, READINESS_ROUNDS_RANGE};

    #[test]
    fn auto_trading_is_opt_in_by_default() {
        assert!(!TraderConfig::default().enabled);
    }

    #[test]
    fn paper_trading_starts_with_one_sol_and_refuses_settings_out_of_bounds() {
        let defaults = PaperTradingConfig::default();
        assert_eq!(defaults.starting_balance_sol, 1.0);
        assert_eq!(defaults.landing_delay_ms, 1000);
        assert_eq!(defaults.readiness_min_closed_rounds, 10);
        TraderConfig::default()
            .validate()
            .expect("the default paper settings are valid");

        let with = |paper: PaperTradingConfig| TraderConfig {
            paper,
            ..TraderConfig::default()
        };
        for starting_balance_sol in [0.0, 0.009, 1000.5, f64::NAN, f64::INFINITY] {
            assert!(
                with(PaperTradingConfig {
                    starting_balance_sol,
                    ..defaults.clone()
                })
                .validate()
                .is_err(),
                "starting balance {starting_balance_sol} is refused"
            );
        }
        assert!(with(PaperTradingConfig {
            landing_delay_ms: MAX_LANDING_DELAY_MS + 1,
            ..defaults.clone()
        })
        .validate()
        .is_err());
        for readiness_min_closed_rounds in [
            READINESS_ROUNDS_RANGE.start() - 1,
            READINESS_ROUNDS_RANGE.end() + 1,
        ] {
            assert!(with(PaperTradingConfig {
                readiness_min_closed_rounds,
                ..defaults.clone()
            })
            .validate()
            .is_err());
        }
        with(PaperTradingConfig {
            starting_balance_sol: 0.01,
            landing_delay_ms: MAX_LANDING_DELAY_MS,
            readiness_min_closed_rounds: *READINESS_ROUNDS_RANGE.end(),
        })
        .validate()
        .expect("every bound is inclusive");
    }

    fn ladder(levels: &[f64], sells: &[f64]) -> TraderConfig {
        TraderConfig {
            take_profit_levels_pct: levels.to_vec(),
            take_profit_sell_pct: sells.to_vec(),
            ..TraderConfig::default()
        }
    }

    #[test]
    fn the_default_ladder_is_one_full_step_at_twenty_percent() {
        let config = TraderConfig::default();
        assert!(config.take_profit_enabled);
        assert_eq!(
            config.take_profit_levels(),
            vec![TakeProfitLevel {
                profit_pct: 20.0,
                sell_pct: 100.0,
            }]
        );
        config.validate().expect("the default ladder is valid");
    }

    #[test]
    fn the_ladder_pairs_each_level_with_its_share_in_order() {
        let config = ladder(&[20.0, 50.0, 100.0], &[30.0, 30.0, 40.0]);
        config.validate().expect("a three-step ladder is valid");
        let steps = config.take_profit_levels();
        assert_eq!(steps.len(), 3);
        assert_eq!(
            steps[1],
            TakeProfitLevel {
                profit_pct: 50.0,
                sell_pct: 30.0,
            }
        );
    }

    #[test]
    fn a_ladder_that_leaves_a_runner_is_valid() {
        ladder(&[25.0, 60.0], &[25.0, 25.0])
            .validate()
            .expect("shares under 100 leave a runner");
    }

    #[test]
    fn decimal_shares_that_sell_exactly_the_whole_position_are_valid() {
        let shares = [5.2, 78.9, 15.9];
        assert!(
            shares.iter().sum::<f64>() > 100.0,
            "the float sum carries past 100"
        );
        ladder(&[20.0, 50.0, 100.0], &shares)
            .validate()
            .expect("shares that add up to 100 are valid");
    }

    #[test]
    fn an_impossible_ladder_is_refused() {
        let too_many: Vec<f64> = (1..=TAKE_PROFIT_MAX_STEPS + 1)
            .map(|step| step as f64 * 10.0)
            .collect();
        let too_many_shares = vec![5.0; TAKE_PROFIT_MAX_STEPS + 1];
        for (case, config) in [
            ("no steps", ladder(&[], &[])),
            ("too many steps", ladder(&too_many, &too_many_shares)),
            ("unpaired share", ladder(&[20.0, 40.0], &[50.0])),
            ("not ascending", ladder(&[40.0, 40.0], &[50.0, 50.0])),
            ("descending", ladder(&[60.0, 40.0], &[50.0, 50.0])),
            ("over 100 sold", ladder(&[20.0, 40.0], &[60.0, 50.0])),
            ("over 100 by a tenth", ladder(&[20.0, 40.0], &[50.0, 50.1])),
            ("level under its bound", ladder(&[0.5], &[100.0])),
            ("level over its bound", ladder(&[1001.0], &[100.0])),
            ("share under its bound", ladder(&[20.0], &[4.0])),
            ("level not finite", ladder(&[f64::NAN], &[100.0])),
            ("share not finite", ladder(&[20.0], &[f64::INFINITY])),
        ] {
            assert!(config.validate().is_err(), "{case} must be refused");
        }
    }

    #[test]
    fn break_even_and_stagnation_are_off_by_default_and_valid() {
        let config = TraderConfig::default();
        assert!(!config.break_even_enabled);
        assert_eq!(config.break_even_trigger_pct, 15.0);
        assert_eq!(config.break_even_offset_pct, 1.0);
        assert!(!config.time_exit_stagnation_enabled);
        assert_eq!(config.time_exit_stagnation_minutes, 240.0);
        assert_eq!(config.time_exit_stagnation_band_pct, 5.0);
        config.validate().expect("the defaults are valid");
    }

    #[test]
    fn break_even_and_stagnation_bounds_are_enforced() {
        let with = |edit: fn(&mut TraderConfig)| {
            let mut config = TraderConfig::default();
            edit(&mut config);
            config
        };
        for (case, config) in [
            (
                "trigger under its bound",
                with(|c| {
                    c.break_even_trigger_pct = 0.4;
                    c.break_even_offset_pct = 0.0;
                }),
            ),
            (
                "trigger over its bound",
                with(|c| c.break_even_trigger_pct = 1000.5),
            ),
            ("offset negative", with(|c| c.break_even_offset_pct = -1.0)),
            (
                "offset over its bound",
                with(|c| {
                    c.break_even_trigger_pct = 100.0;
                    c.break_even_offset_pct = 51.0;
                }),
            ),
            (
                "offset equal to trigger",
                with(|c| c.break_even_offset_pct = 15.0),
            ),
            (
                "offset above trigger",
                with(|c| c.break_even_offset_pct = 20.0),
            ),
            (
                "trigger not finite",
                with(|c| c.break_even_trigger_pct = f64::NAN),
            ),
            (
                "minutes under its bound",
                with(|c| c.time_exit_stagnation_minutes = 4.0),
            ),
            (
                "minutes over its bound",
                with(|c| c.time_exit_stagnation_minutes = 43201.0),
            ),
            (
                "band under its bound",
                with(|c| c.time_exit_stagnation_band_pct = 0.4),
            ),
            (
                "band over its bound",
                with(|c| c.time_exit_stagnation_band_pct = 50.5),
            ),
            (
                "band not finite",
                with(|c| c.time_exit_stagnation_band_pct = f64::INFINITY),
            ),
        ] {
            assert!(config.validate().is_err(), "{case} must be refused");
        }
        with(|c| {
            c.break_even_trigger_pct = 0.5;
            c.break_even_offset_pct = 0.0;
            c.time_exit_stagnation_minutes = 5.0;
            c.time_exit_stagnation_band_pct = 50.0;
        })
        .validate()
        .expect("values on their bounds are valid");
    }

    #[test]
    fn the_time_unit_round_trips_under_its_stored_names() {
        for (unit, name) in [
            (TimeUnit::Seconds, "seconds"),
            (TimeUnit::Minutes, "minutes"),
            (TimeUnit::Hours, "hours"),
            (TimeUnit::Days, "days"),
        ] {
            assert_eq!(unit.as_str(), name);
            let stored = serde_json::to_value(unit).expect("a unit serializes");
            assert_eq!(stored, serde_json::json!(name));
            let loaded: TimeUnit = serde_json::from_value(stored).expect("a unit loads");
            assert_eq!(loaded, unit);
        }
        for (alias, unit) in [
            ("MIN", TimeUnit::Minutes),
            ("s", TimeUnit::Seconds),
            ("Day", TimeUnit::Days),
            ("hr", TimeUnit::Hours),
        ] {
            let loaded: TimeUnit =
                serde_json::from_value(serde_json::json!(alias)).expect("an alias loads");
            assert_eq!(loaded, unit, "{alias}");
        }
    }

    #[test]
    fn a_name_that_matches_no_unit_is_refused() {
        for name in ["weeks", "fortnights", "", "hourss"] {
            let refused = serde_json::from_value::<TimeUnit>(serde_json::json!(name))
                .expect_err("an unknown unit is refused");
            assert!(
                refused.to_string().contains("unknown variant"),
                "{name}: {refused}"
            );
        }
        let section = toml::from_str::<TraderConfig>("time_override_unit = \"weeks\"\n");
        assert!(
            section.is_err(),
            "a typed trader section refuses an unknown unit"
        );
    }

    fn dca_ladder(drops: &[f64], sizes: &[f64], measure_from: DcaMeasureFrom) -> TraderConfig {
        TraderConfig {
            dca_levels_pct: drops.to_vec(),
            dca_size_multipliers: sizes.to_vec(),
            dca_measure_from: measure_from,
            ..TraderConfig::default()
        }
    }

    #[test]
    fn the_default_dca_ladder_is_two_half_size_adds_ten_percent_below_the_average() {
        let config = TraderConfig::default();
        assert!(!config.dca_enabled);
        assert_eq!(config.dca_measure_from, DcaMeasureFrom::Average);
        assert_eq!(config.dca_min_hold_minutes, 0);
        assert_eq!(config.dca_cooldown_minutes, 30);
        assert_eq!(
            config.dca_levels(),
            vec![
                DcaLevel {
                    drop_pct: -10.0,
                    size_multiplier: 0.5,
                };
                2
            ]
        );
        config.validate().expect("the default DCA ladder is valid");
    }

    #[test]
    fn a_dca_step_buys_its_multiple_of_the_first_buy() {
        let level = DcaLevel {
            drop_pct: -20.0,
            size_multiplier: 1.5,
        };
        assert_eq!(level.add_size_native(0.02), 0.02 * 1.5);
    }

    #[test]
    fn a_dca_ladder_the_evaluator_can_follow_is_valid() {
        dca_ladder(
            &[-10.0, -10.0, -10.0],
            &[0.5, 1.0, 2.0],
            DcaMeasureFrom::Average,
        )
        .validate()
        .expect("equal drops from the average are a uniform ladder");
        dca_ladder(
            &[-10.0, -25.0, -40.0],
            &[0.5, 1.0, 2.0],
            DcaMeasureFrom::FirstEntry,
        )
        .validate()
        .expect("deeper drops from the first entry are valid");
        dca_ladder(&[-95.0], &[5.0], DcaMeasureFrom::Average)
            .validate()
            .expect("values on their bounds are valid");
    }

    #[test]
    fn an_impossible_dca_ladder_is_refused() {
        let too_many = vec![-10.0; DCA_MAX_STEPS + 1];
        let too_many_sizes = vec![0.5; DCA_MAX_STEPS + 1];
        let average = DcaMeasureFrom::Average;
        let first = DcaMeasureFrom::FirstEntry;
        for (case, config) in [
            ("no steps", dca_ladder(&[], &[], average)),
            (
                "too many steps",
                dca_ladder(&too_many, &too_many_sizes, average),
            ),
            (
                "unpaired size",
                dca_ladder(&[-10.0, -20.0], &[0.5], average),
            ),
            ("a gain", dca_ladder(&[5.0], &[0.5], average)),
            ("no drop", dca_ladder(&[0.0], &[0.5], average)),
            (
                "drop under its bound",
                dca_ladder(&[-96.0], &[0.5], average),
            ),
            ("drop over its bound", dca_ladder(&[-0.5], &[0.5], average)),
            (
                "size under its bound",
                dca_ladder(&[-10.0], &[0.05], average),
            ),
            ("size over its bound", dca_ladder(&[-10.0], &[5.5], average)),
            ("drop not finite", dca_ladder(&[f64::NAN], &[0.5], average)),
            (
                "size not finite",
                dca_ladder(&[-10.0], &[f64::INFINITY], average),
            ),
            (
                "equal from first entry",
                dca_ladder(&[-10.0, -10.0], &[0.5, 0.5], first),
            ),
            (
                "shallower from first entry",
                dca_ladder(&[-20.0, -10.0], &[0.5, 0.5], first),
            ),
        ] {
            assert!(config.validate().is_err(), "{case} must be refused");
        }
        for minutes in [-1, 1441] {
            let config = TraderConfig {
                dca_min_hold_minutes: minutes,
                ..TraderConfig::default()
            };
            assert!(
                config.validate().is_err(),
                "min hold {minutes} must be refused"
            );
        }
    }

    #[test]
    fn the_dca_basis_round_trips_under_its_stored_names_and_refuses_others() {
        for (basis, name) in [
            (DcaMeasureFrom::Average, "average"),
            (DcaMeasureFrom::FirstEntry, "first_entry"),
        ] {
            assert_eq!(basis.as_str(), name);
            let stored = serde_json::to_value(basis).expect("a basis serializes");
            assert_eq!(stored, serde_json::json!(name));
            let loaded: DcaMeasureFrom = serde_json::from_value(stored).expect("a basis loads");
            assert_eq!(loaded, basis);
        }
        let loaded: DcaMeasureFrom =
            serde_json::from_value(serde_json::json!("First_Entry")).expect("any case loads");
        assert_eq!(loaded, DcaMeasureFrom::FirstEntry);
        for name in ["peak", "first entry", "", "averages"] {
            assert!(
                serde_json::from_value::<DcaMeasureFrom>(serde_json::json!(name)).is_err(),
                "{name} is refused"
            );
        }
        assert!(
            toml::from_str::<TraderConfig>("dca_measure_from = \"peak\"\n").is_err(),
            "a typed trader section refuses an unknown basis"
        );
    }
}
