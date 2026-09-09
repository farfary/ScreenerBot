//! Trailing stop preview, templates, and trader statistics

use axum::{extract::Query, http::StatusCode, response::Response, Json};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// TRADER STATS HANDLER
// =============================================================================

/// GET /api/trader/stats - realized trading performance over a selectable window.
///
/// One aggregation over one row set. Every figure the Stats tab shows is derived
/// here, so the tab cannot show a number the backend never computed (the "Total
/// P&L" card used to average `avg_profit_pct` in JavaScript while `total_pnl_sol`
/// went unread).
pub async fn get_trader_stats(Query(query): Query<TraderStatsQuery>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return crate::webserver::utils::success_response(
            crate::webserver::promo::get_promo_trader_stats(),
        );
    }

    let period_days = query.days.unwrap_or(30).clamp(1, 365);

    // Live exposure. Not window-bound: what is at risk right now is independent of
    // how far back the realized window reaches.
    let open_positions = positions::get_open_positions().await;
    let open_positions_count = open_positions.len();
    let locked_sol: f64 = open_positions.iter().map(|p| p.total_size_sol).sum();
    let max_open_positions = with_config(|cfg| cfg.trader.max_open_positions);

    let window_start = chrono::Utc::now() - chrono::Duration::days(i64::from(period_days));
    let recent_closed = {
        let db_ref = positions::db::get_positions_database().await.ok();
        if let Some(db_arc) = db_ref {
            let db_guard = db_arc.lock().await;
            if let Some(db) = db_guard.as_ref() {
                db.get_closed_positions_since(window_start)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    // A round whose basis or history is incomplete has no honest P&L. Overview
    // already refuses to price those; counting them here is what let this tab
    // disagree with Home on the same wallet.
    let fetched = recent_closed.len();
    let closed: Vec<_> = recent_closed
        .into_iter()
        .filter(|p| p.has_trustworthy_pnl())
        .collect();
    let excluded_untrusted = fetched - closed.len();
    let total_trades = closed.len();

    let winners = closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or_default() > 0.0)
        .count();
    let losers = closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or_default() < 0.0)
        .count();
    let win_rate_pct = (total_trades > 0).then(|| (winners as f64 / total_trades as f64) * 100.0);

    // Hold time. The median is reported alongside the mean because one forgotten
    // bag drags the mean far away from what a typical round actually looked like.
    let mut hold_hours: Vec<f64> = closed
        .iter()
        .filter_map(|p| {
            p.exit_time
                .map(|exit| (exit - p.entry_time).num_seconds() as f64 / 3600.0)
        })
        .collect();
    let avg_hold_time_hours =
        (!hold_hours.is_empty()).then(|| hold_hours.iter().sum::<f64>() / hold_hours.len() as f64);
    hold_hours.sort_by(f64::total_cmp);
    let median_hold_time_hours = (!hold_hours.is_empty()).then(|| {
        let mid = hold_hours.len() / 2;
        if hold_hours.len() % 2 == 0 {
            (hold_hours[mid - 1] + hold_hours[mid]) / 2.0
        } else {
            hold_hours[mid]
        }
    });

    // Best/worst by percent, keeping the token so the card can name it.
    let best_trade = closed
        .iter()
        .filter(|p| p.pnl_percent.is_some())
        .max_by(|a, b| {
            a.pnl_percent
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&b.pnl_percent.unwrap_or(f64::NEG_INFINITY))
        });
    let worst_trade = closed
        .iter()
        .filter(|p| p.pnl_percent.is_some())
        .min_by(|a, b| {
            a.pnl_percent
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b.pnl_percent.unwrap_or(f64::INFINITY))
        });

    let best_trade_pct = best_trade.and_then(|p| p.pnl_percent);
    let best_trade_token = best_trade.map(|p| p.symbol.clone());
    let worst_trade_pct = worst_trade.and_then(|p| p.pnl_percent);
    let worst_trade_token = worst_trade.map(|p| p.symbol.clone());

    // Realized P&L in SOL across the window.
    //
    // Uses the `pnl` the position booked at close — fee-aware and DCA-aware. Deriving it
    // as `sol_received - entry_size_sol` counted every DCA add as pure profit, because
    // `entry_size_sol` is only the FIRST buy and never grows.
    let total_pnl_sol: f64 = closed.iter().filter_map(|p| p.pnl).sum();
    let gross_profit_sol: f64 = closed
        .iter()
        .filter_map(|p| p.pnl)
        .filter(|v| *v > 0.0)
        .sum();
    let gross_loss_sol: f64 = closed
        .iter()
        .filter_map(|p| p.pnl)
        .filter(|v| *v < 0.0)
        .map(f64::abs)
        .sum();
    // Profit factor is undefined without a loss to divide by — a losing streak with
    // no wins is 0.0, but a clean run with no losses is "no answer yet", not infinity.
    let profit_factor = (gross_loss_sol > 0.0).then(|| gross_profit_sol / gross_loss_sol);
    let expectancy_sol = (total_trades > 0).then(|| total_pnl_sol / total_trades as f64);

    let avg_win_pct = {
        let wins: Vec<f64> = closed
            .iter()
            .filter_map(|p| p.pnl_percent)
            .filter(|v| *v > 0.0)
            .collect();
        (!wins.is_empty()).then(|| wins.iter().sum::<f64>() / wins.len() as f64)
    };
    let avg_loss_pct = {
        let losses: Vec<f64> = closed
            .iter()
            .filter_map(|p| p.pnl_percent)
            .filter(|v| *v < 0.0)
            .collect();
        (!losses.is_empty()).then(|| losses.iter().sum::<f64>() / losses.len() as f64)
    };

    // Daily buckets and the drawdown share one pass over the window in exit order.
    // `get_closed_positions_since` returns newest first, so walk it in reverse.
    use std::collections::HashMap;
    let mut per_day: HashMap<String, (f64, usize)> = HashMap::new();
    let mut equity = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut max_drawdown_sol = 0.0_f64;

    for pos in closed.iter().rev() {
        let pnl = pos.pnl.unwrap_or_default();
        equity += pnl;
        peak = peak.max(equity);
        max_drawdown_sol = max_drawdown_sol.max(peak - equity);

        if let Some(exit) = pos.exit_time {
            let entry = per_day
                .entry(exit.format("%Y-%m-%d").to_string())
                .or_insert((0.0, 0));
            entry.0 += pnl;
            entry.1 += 1;
        }
    }

    // Emit every calendar day in the window, including the empty ones, so the curve
    // keeps a true time axis instead of compressing quiet stretches away.
    let today = chrono::Utc::now().date_naive();
    let first_day = window_start.date_naive();
    let mut daily_pnl = Vec::new();
    let mut day = first_day;
    while day <= today {
        let key = day.format("%Y-%m-%d").to_string();
        let (net_pnl_sol, trades) = per_day.get(&key).copied().unwrap_or((0.0, 0));
        daily_pnl.push(DailyPnlPoint {
            date: key,
            net_pnl_sol,
            trades,
        });
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
    }

    // Exit breakdown from closed_reason, carrying the SOL each reason actually
    // returned — a reason can be the most frequent exit and still be the one losing
    // the money.
    let mut exit_stats: HashMap<String, (usize, Vec<f64>, f64)> = HashMap::new();
    for pos in &closed {
        let exit_type = pos
            .closed_reason
            .clone()
            .unwrap_or_else(|| "unknown".to_owned());
        let entry = exit_stats.entry(exit_type).or_insert((0, Vec::new(), 0.0));
        entry.0 += 1;
        if let Some(pnl_pct) = pos.pnl_percent {
            entry.1.push(pnl_pct);
        }
        entry.2 += pos.pnl.unwrap_or_default();
    }

    let mut exit_breakdown: Vec<ExitBreakdown> = exit_stats
        .into_iter()
        .map(|(exit_type, (count, profits, net_pnl_sol))| ExitBreakdown {
            exit_type,
            count,
            avg_profit_pct: if profits.is_empty() {
                0.0
            } else {
                profits.iter().sum::<f64>() / profits.len() as f64
            },
            net_pnl_sol,
        })
        .collect();
    exit_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

    success_response(TraderStatsResponse {
        period_days,
        open_positions_count,
        max_open_positions,
        locked_sol,
        total_trades,
        winners,
        losers,
        excluded_untrusted,
        total_pnl_sol,
        gross_profit_sol,
        gross_loss_sol,
        profit_factor,
        expectancy_sol,
        max_drawdown_sol,
        win_rate_pct,
        avg_win_pct,
        avg_loss_pct,
        avg_hold_time_hours,
        median_hold_time_hours,
        best_trade_pct,
        best_trade_token,
        worst_trade_pct,
        worst_trade_token,
        daily_pnl,
        exit_breakdown,
    })
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
    _current_price: f64,
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
