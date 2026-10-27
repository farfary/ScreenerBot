// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Preset exit templates for the auto trader: named bundles of trailing-stop,
//! take-profit and time-override settings applied to config in one step. A preset
//! writes only those rules; every other trader setting, the break-even stop and the
//! stagnation exit among them, keeps the value the user set.

use serde::{Deserialize, Serialize};

use crate::config::{update_config_section, TakeProfitLevel, TimeUnit, TraderConfig};
use crate::logger::{self, LogTag};
use crate::trader::{Error, Result};

#[derive(Debug, Serialize, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trading_style: String,
    pub config: TemplateConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateConfig {
    pub trailing_stop_enabled: bool,
    pub trailing_stop_activation_pct: f64,
    pub trailing_stop_distance_pct: f64,
    pub take_profit_enabled: bool,
    pub take_profit_levels: Vec<TakeProfitLevel>,
    pub time_override_enabled: bool,
    pub time_override_duration: f64,
    pub time_override_unit: TimeUnit,
    pub time_override_loss_threshold_pct: f64,
}

impl TemplateConfig {
    /// Write the rules this preset sets into `trader`, leaving every other field as it is.
    fn apply_to(&self, trader: &mut TraderConfig) {
        trader.trailing_stop_enabled = self.trailing_stop_enabled;
        trader.trailing_stop_activation_pct = self.trailing_stop_activation_pct;
        trader.trailing_stop_distance_pct = self.trailing_stop_distance_pct;
        // Bands are tuned to the activation and distance they replace; a preset's trail
        // follows its own distance at every profit.
        trader.trailing_stop_band_profit_pct.clear();
        trader.trailing_stop_band_distance_pct.clear();
        trader.take_profit_enabled = self.take_profit_enabled;
        trader.take_profit_levels_pct = self
            .take_profit_levels
            .iter()
            .map(|level| level.profit_pct)
            .collect();
        trader.take_profit_sell_pct = self
            .take_profit_levels
            .iter()
            .map(|level| level.sell_pct)
            .collect();
        trader.time_override_enabled = self.time_override_enabled;
        trader.time_override_duration = self.time_override_duration;
        trader.time_override_unit = self.time_override_unit;
        trader.time_override_loss_threshold_percent = self.time_override_loss_threshold_pct;
    }
}

/// Apply a template to the trader config and persist it.
pub fn apply_template(template_id: &str) -> Result<Template> {
    let template = all_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| Error::TemplateNotFound {
            template_id: template_id.to_owned(),
        })?;
    update_config_section(|config| template.config.apply_to(&mut config.trader), true).map_err(
        |e| Error::ConfigUpdate {
            detail: e.to_string(),
        },
    )?;
    logger::info(
        LogTag::Trader,
        &format!("Applied template '{}' ({})", template.name, template.id),
    );
    Ok(template)
}

pub fn all_templates() -> Vec<Template> {
    vec![
        Template {
            id: "conservative".to_owned(),
            name: "Conservative".to_owned(),
            description: "Low risk, secure profits early".to_owned(),
            trading_style: "conservative".to_owned(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 3.0,
                take_profit_enabled: true,
                take_profit_levels: vec![TakeProfitLevel::sell_all_at(10.0)],
                time_override_enabled: true,
                time_override_duration: 3.0,
                time_override_unit: TimeUnit::Days,
                time_override_loss_threshold_pct: -20.0,
            },
        },
        Template {
            id: "balanced".to_owned(),
            name: "Balanced".to_owned(),
            description: "Balanced risk/reward".to_owned(),
            trading_style: "balanced".to_owned(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 10.0,
                trailing_stop_distance_pct: 5.0,
                take_profit_enabled: true,
                take_profit_levels: vec![TakeProfitLevel::sell_all_at(20.0)],
                time_override_enabled: true,
                time_override_duration: 7.0,
                time_override_unit: TimeUnit::Days,
                time_override_loss_threshold_pct: -40.0,
            },
        },
        Template {
            id: "aggressive".to_owned(),
            name: "Aggressive".to_owned(),
            description: "High risk, chase large gains".to_owned(),
            trading_style: "aggressive".to_owned(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 15.0,
                trailing_stop_distance_pct: 7.0,
                take_profit_enabled: true,
                take_profit_levels: vec![TakeProfitLevel::sell_all_at(50.0)],
                time_override_enabled: true,
                time_override_duration: 14.0,
                time_override_unit: TimeUnit::Days,
                time_override_loss_threshold_pct: -60.0,
            },
        },
        Template {
            id: "day_trade".to_owned(),
            name: "Day Trade".to_owned(),
            description: "Quick exits, tight stops".to_owned(),
            trading_style: "day_trade".to_owned(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 2.0,
                take_profit_enabled: true,
                take_profit_levels: vec![TakeProfitLevel::sell_all_at(5.0)],
                time_override_enabled: true,
                time_override_duration: 4.0,
                time_override_unit: TimeUnit::Hours,
                time_override_loss_threshold_pct: -15.0,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_leaves_the_break_even_stop_and_the_stagnation_exit_as_set() {
        let mut configured = TraderConfig::default();
        configured.break_even_enabled = true;
        configured.break_even_trigger_pct = 22.0;
        configured.break_even_offset_pct = 3.5;
        configured.time_exit_stagnation_enabled = true;
        configured.time_exit_stagnation_minutes = 95.0;
        configured.time_exit_stagnation_band_pct = 2.5;
        let rules = |trader: &TraderConfig| {
            (
                trader.break_even_enabled,
                trader.break_even_trigger_pct,
                trader.break_even_offset_pct,
                trader.time_exit_stagnation_enabled,
                trader.time_exit_stagnation_minutes,
                trader.time_exit_stagnation_band_pct,
            )
        };
        for template in all_templates() {
            let mut trader = configured.clone();
            template.config.apply_to(&mut trader);
            assert_eq!(rules(&trader), rules(&configured), "{}", template.id);
            assert_eq!(
                trader.time_override_duration, template.config.time_override_duration,
                "{} writes its own rules",
                template.id
            );
        }
    }

    #[test]
    fn a_preset_trails_at_its_own_distance_and_passes_validation() {
        let mut configured = TraderConfig::default();
        configured.trailing_stop_band_profit_pct = vec![50.0];
        configured.trailing_stop_band_distance_pct = vec![4.0];
        for template in all_templates() {
            let mut trader = configured.clone();
            template.config.apply_to(&mut trader);
            assert!(trader.trailing_stop_bands().is_empty(), "{}", template.id);
            assert!(trader.validate().is_ok(), "{}", template.id);
        }
    }
}
