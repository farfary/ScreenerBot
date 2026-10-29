// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies overview route — every strategy with its run status and the figures of the
//! book it trades on, for the strategies roster.

use axum::response::Response;
use chrono::Utc;

use super::types::{StrategiesOverviewResponse, StrategyItem, StrategyOverviewItem};
use super::utils::err_cause;
use crate::i18n::ids;
use crate::strategies::overview::strategies_overview;
use crate::webserver::api_error::ApiErrorCode;
use crate::webserver::promo;
use crate::webserver::utils::success_response;

/// GET /api/strategies/overview - The trader's run state, its trader-level notice, the
/// rule exits and every strategy with its status and figures. Answers in Explore Mode and
/// with the trader stopped; reads only.
pub async fn get_strategies_overview() -> Response {
    let overview = if promo::are_promo_fixtures_enabled() {
        promo::get_promo_strategies_overview()
    } else {
        match strategies_overview().await {
            Ok(overview) => overview,
            Err(e) => {
                return err_cause(
                    ApiErrorCode::Internal,
                    ids::ERRORS_STRATEGIES_LIST_FAILED,
                    &e,
                );
            }
        }
    };
    success_response(StrategiesOverviewResponse {
        run_state: overview.run_state,
        notice: overview.notice,
        rule_exits: overview.rule_exits,
        counts: overview.counts,
        strategies: overview
            .strategies
            .into_iter()
            .map(|summary| StrategyOverviewItem {
                item: StrategyItem::from(summary.strategy.clone()),
                summary,
            })
            .collect(),
        timestamp: Utc::now().to_rfc3339(),
    })
}
