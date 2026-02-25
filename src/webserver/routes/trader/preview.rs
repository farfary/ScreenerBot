//! Trailing stop preview, templates, and trader statistics

use axum::{extract::Query, http::StatusCode, response::Response, Json};

use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// TRADER STATS HANDLER
// =============================================================================

/// GET /api/trader/stats - Get trader performance statistics
pub async fn get_trader_stats() -> Response {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return crate::webserver::utils::success_response(
            crate::webserver::demo::get_demo_trader_stats(),
        );
    }

    // Get open positions
    let open_positions = positions::get_open_positions().await;
    let open_positions_count = open_positions.len();

    // Calculate locked SOL (use total_size_sol for DCA support)
    let locked_sol: f64 = open_positions.iter().map(|p| p.total_size_sol).sum();

    // Query closed positions from database for statistics (optimized - 30 days only)
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    let recent_closed = {
        let db_ref = positions::db::get_positions_database().await.ok();
        if let Some(db_arc) = db_ref {
            let db_guard = db_arc.lock().await;
            if let Some(db) = db_guard.as_ref() {
                db.get_closed_positions_since(thirty_days_ago)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    let total_trades = recent_closed.len();

    // Calculate win rate (using pnl_percent for closed positions)
    let winners = recent_closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or_default() > 0.0)
        .count();
    let win_rate_pct = if total_trades > 0 {
        (winners as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    // Calculate average hold time
    let total_hold_seconds: i64 = recent_closed
        .iter()
        .filter_map(|p| {
            p.exit_time
                .map(|exit_time| (exit_time - p.entry_time).num_seconds())
        })
        .sum();

    let avg_hold_time_hours = if total_trades > 0 {
        (total_hold_seconds as f64 / total_trades as f64) / 3600.0
    } else {
        0.0
    };

    // Find best trade (using pnl_percent)
    let best_trade_pct = recent_closed
        .iter()
        .filter_map(|p| p.pnl_percent)
        .fold(f64::NEG_INFINITY, f64::max);

    let best_trade_pct = if best_trade_pct == f64::NEG_INFINITY {
        0.0
    } else {
        best_trade_pct
    };

    // Build exit breakdown from closed_reason in closed positions
    use std::collections::HashMap;
    let mut exit_stats: HashMap<String, (usize, Vec<f64>)> = HashMap::new();

    for pos in &recent_closed {
        let exit_type = pos
            .closed_reason
            .clone()
            .unwrap_or_else(|| "unknown".to_owned());
        let entry = exit_stats.entry(exit_type).or_insert((0, Vec::new()));
        entry.0 += 1;
        if let Some(pnl_pct) = pos.pnl_percent {
            entry.1.push(pnl_pct);
        }
    }

    let mut exit_breakdown = Vec::new();
    for (exit_type, (count, profits)) in exit_stats {
        let avg_profit_pct = if !profits.is_empty() {
            profits.iter().sum::<f64>() / profits.len() as f64
        } else {
            0.0
        };

        exit_breakdown.push(ExitBreakdown {
            exit_type,
            count,
            avg_profit_pct,
        });
    }

    // Sort by count descending
    exit_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

    let stats = TraderStatsResponse {
        open_positions_count,
        locked_sol,
        win_rate_pct,
        total_trades,
        avg_hold_time_hours,
        best_trade_pct,
        exit_breakdown,
    };

    success_response(stats)
}

// =============================================================================
// TRAILING STOP PREVIEW
// =============================================================================

/// GET /api/trader/preview-trailing-stop - Preview trailing stop for a position
pub async fn get_trailing_stop_preview(Query(query): Query<TrailingStopPreviewQuery>) -> Response {
    use crate::pools::get_pool_price;

    // Get config values (or use query overrides)
    let (activation_pct, distance_pct) = with_config(|cfg| {
        let act = query
            .activation_pct
            .unwrap_or(cfg.positions.trailing_stop_activation_pct);
        let dist = query
            .distance_pct
            .unwrap_or(cfg.positions.trailing_stop_distance_pct);
        (act, dist)
    });

    // Get position data (or create simulation)
    let (position_id, symbol, entry_price, current_price, peak_price) =
        if let Some(pos_id) = query.position_id {
            // Try to get real position
            let positions = positions::get_open_positions().await;
            if let Some(pos) = positions.iter().find(|p| p.id == Some(pos_id)) {
                let current = get_pool_price(&pos.mint)
                    .map(|pr| pr.price_sol)
                    .unwrap_or(pos.entry_price);
                let peak = if pos.price_highest > 0.0 {
                    pos.price_highest
                } else {
                    current.max(pos.entry_price)
                };
                (
                    Some(pos_id),
                    pos.symbol.clone(),
                    pos.entry_price,
                    current,
                    peak,
                )
            } else {
                // Position not found, use simulation
                (None, "SIMULATED".to_owned(), 0.001, 0.00119, 0.00123)
            }
        } else {
            // No position_id, use simulation
            (None, "SIMULATED".to_owned(), 0.001, 0.00119, 0.00123)
        };

    // Calculate current profit
    let current_profit_pct = ((current_price - entry_price) / entry_price) * 100.0;
    let peak_profit_pct = ((peak_price - entry_price) / entry_price) * 100.0;

    // Calculate trail state
    let trail_active = peak_profit_pct >= activation_pct;
    let trail_activated_at_pct = if trail_active {
        Some(activation_pct)
    } else {
        None
    };
    let trail_stop_price = if trail_active {
        Some(peak_price * (1.0 - distance_pct / 100.0))
    } else {
        None
    };
    let distance_to_exit_pct = if let Some(stop_price) = trail_stop_price {
        Some(((current_price - stop_price) / current_price) * 100.0)
    } else {
        None
    };

    // Estimated exit
    let estimated_exit_price = trail_stop_price.unwrap_or(entry_price);
    let estimated_exit_profit_pct = ((estimated_exit_price - entry_price) / entry_price) * 100.0;

    // Calculate unrealized P&L (assuming 0.01 SOL position for simulation)
    let position_size = 0.01;
    let unrealized_pnl = (current_price - entry_price) * (position_size / entry_price);

    // Generate what-if scenarios
    let what_if_scenarios = generate_what_if_scenarios(
        entry_price,
        current_price,
        peak_price,
        activation_pct,
        distance_pct,
    );

    let preview = TrailingStopPreviewResponse {
        position_id,
        symbol,
        entry_price,
        current_price,
        peak_price,
        current_profit_pct,
        unrealized_pnl,
        trail_active,
        trail_activated_at_pct,
        trail_stop_price,
        distance_to_exit_pct,
        estimated_exit_price,
        estimated_exit_profit_pct,
        what_if_scenarios,
    };

    success_response(preview)
}

pub fn generate_what_if_scenarios(
    entry_price: f64,
    current_price: f64,
    peak_price: f64,
    base_activation: f64,
    base_distance: f64,
) -> Vec<WhatIfScenario> {
    let mut scenarios = Vec::new();

    // Helper to calculate scenario
    let calc_scenario = |act: f64, dist: f64| -> WhatIfScenario {
        let peak_profit = ((peak_price - entry_price) / entry_price) * 100.0;
        let trail_active = peak_profit >= act;
        let exit_price = if trail_active {
            peak_price * (1.0 - dist / 100.0)
        } else {
            entry_price
        };
        let exit_profit = ((exit_price - entry_price) / entry_price) * 100.0;

        WhatIfScenario {
            description: format!("Activation {act}%, Distance {dist}%"),
            activation_pct: act,
            distance_pct: dist,
            trail_active,
            exit_price,
            exit_profit_pct: exit_profit,
        }
    };

    // Scenario 1: Current settings
    scenarios.push(calc_scenario(base_activation, base_distance));

    // Scenario 2: Tighter activation (current - 5%)
    if base_activation > 5.0 {
        scenarios.push(calc_scenario(base_activation - 5.0, base_distance));
    }

    // Scenario 3: Looser activation (current + 5%)
    scenarios.push(calc_scenario(base_activation + 5.0, base_distance));

    // Scenario 4: Tighter distance (current - 2%)
    if base_distance > 2.0 {
        scenarios.push(calc_scenario(base_activation, base_distance - 2.0));
    }

    scenarios
}

// =============================================================================
// TEMPLATE ENDPOINTS
// =============================================================================

pub async fn get_templates() -> Response {
    let templates = get_all_templates();
    success_response(TemplateListResponse { templates })
}

/// POST /api/trader/apply-template - Apply a preset template
pub async fn apply_template(Json(request): Json<ApplyTemplateRequest>) -> Response {
    use crate::config::update_config_section;

    // Get all templates (TODO: DRY this up with get_templates)
    let templates = get_all_templates();

    // Find the requested template
    let template = templates.iter().find(|t| t.id == request.template_id);

    if template.is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TemplateNotFound",
            &format!("Template '{}' not found", request.template_id),
            None,
        );
    }

    let template = template.unwrap();
    let cfg = template.config.clone();

    // Update positions config
    let result = update_config_section(
        |config| {
            config.positions.trailing_stop_enabled = cfg.trailing_stop_enabled;
            config.positions.trailing_stop_activation_pct = cfg.trailing_stop_activation_pct;
            config.positions.trailing_stop_distance_pct = cfg.trailing_stop_distance_pct;
        },
        false, // Don't save yet
    );

    if let Err(e) = result {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to update positions config: {e}"),
            None,
        );
    }

    // Update trader config
    let result = update_config_section(
        |config| {
            config.trader.roi_exit_enabled = cfg.roi_exit_enabled;
            config.trader.roi_target_percent = cfg.roi_target_pct;

            config.trader.time_override_enabled = cfg.time_override_enabled;
            config.trader.time_override_duration = cfg.time_override_duration;
            config.trader.time_override_unit = cfg.time_override_unit.clone();
            config.trader.time_override_loss_threshold_percent =
                cfg.time_override_loss_threshold_pct;
        },
        true, // Save to disk
    );

    if let Err(e) = result {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to update trader config: {e}"),
            None,
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!("Applied template '{}' ({})", template.name, template.id),
    );

    success_response(serde_json::json!({
        "message": format!("Template '{}' applied successfully", template.name),
        "template": template,
    }))
}

pub fn get_all_templates() -> Vec<Template> {
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
