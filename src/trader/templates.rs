//! Preset exit templates for the auto trader: named bundles of trailing-stop,
//! ROI and time-override settings applied to config in one step.

use serde::{Deserialize, Serialize};

use crate::config::update_config_section;
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
    pub roi_exit_enabled: bool,
    pub roi_target_pct: f64,
    pub time_override_enabled: bool,
    pub time_override_duration: f64,
    pub time_override_unit: String,
    pub time_override_loss_threshold_pct: f64,
}

/// Apply a template to the positions and trader config and persist it.
pub fn apply_template(template_id: &str) -> Result<Template> {
    let template = all_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| Error::TemplateNotFound {
            template_id: template_id.to_owned(),
        })?;
    let cfg = template.config.clone();
    update_config_section(
        |config| {
            config.positions.trailing_stop_enabled = cfg.trailing_stop_enabled;
            config.positions.trailing_stop_activation_pct = cfg.trailing_stop_activation_pct;
            config.positions.trailing_stop_distance_pct = cfg.trailing_stop_distance_pct;
            config.trader.roi_exit_enabled = cfg.roi_exit_enabled;
            config.trader.roi_target_percent = cfg.roi_target_pct;
            config.trader.time_override_enabled = cfg.time_override_enabled;
            config.trader.time_override_duration = cfg.time_override_duration;
            config.trader.time_override_unit = cfg.time_override_unit.clone();
            config.trader.time_override_loss_threshold_percent =
                cfg.time_override_loss_threshold_pct;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;
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
                roi_exit_enabled: true,
                roi_target_pct: 10.0,
                time_override_enabled: true,
                time_override_duration: 3.0,
                time_override_unit: "days".to_owned(),
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
                roi_exit_enabled: true,
                roi_target_pct: 20.0,
                time_override_enabled: true,
                time_override_duration: 7.0,
                time_override_unit: "days".to_owned(),
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
                roi_exit_enabled: true,
                roi_target_pct: 50.0,
                time_override_enabled: true,
                time_override_duration: 14.0,
                time_override_unit: "days".to_owned(),
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
                roi_exit_enabled: true,
                roi_target_pct: 5.0,
                time_override_enabled: true,
                time_override_duration: 4.0,
                time_override_unit: "hours".to_owned(),
                time_override_loss_threshold_pct: -15.0,
            },
        },
    ]
}
