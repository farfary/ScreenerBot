//! Trailing stop preview, templates, and trader statistics

use axum::{extract::Query, http::StatusCode, response::Response, Json};

use crate::config::with_config;
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// TRADER STATS HANDLER
// =============================================================================

/// GET /api/trader/stats - realized trading performance over a selectable window.
pub async fn get_trader_stats(Query(query): Query<TraderStatsQuery>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_trader_stats());
    }
    success_response(crate::trader::stats::trader_stats(query.days.unwrap_or(30)).await)
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
    success_response(TemplateListResponse {
        templates: crate::trader::templates::all_templates(),
    })
}

/// POST /api/trader/apply-template - Apply a preset exit template
pub async fn apply_template(Json(request): Json<ApplyTemplateRequest>) -> Response {
    match crate::trader::templates::apply_template(&request.template_id) {
        Ok(template) => success_response(serde_json::json!({
            "message": format!("Template '{}' applied successfully", template.name),
            "template": template,
        })),
        Err(error @ crate::trader::Error::TemplateNotFound { .. }) => error_response(
            StatusCode::BAD_REQUEST,
            "TemplateNotFound",
            &error.to_string(),
            None,
        ),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &error.to_string(),
            None,
        ),
    }
}
