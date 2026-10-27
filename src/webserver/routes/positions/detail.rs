// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position detail route — serves detailed data for a single position view.

use axum::{
    extract::Path,
    response::{IntoResponse as _, Response},
};
use chrono::Utc;

use super::list::map_position_to_response_async;
use super::pending_swaps::load_pending_swaps;
use super::types::*;
use crate::chains::adapter;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::native_price;
use crate::pools;
use crate::positions;
use crate::tokens;
use crate::tokens::security::SecurityRiskBand;
use crate::trader::policy::ExitPolicy;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

pub async fn get_position_details(Path(key): Path<String>) -> Response {
    match resolve_position_by_key(&key).await {
        Ok(Some(position)) => {
            let mint = &position.mint;

            // Fetch all data concurrently for better performance
            let (detail, (entries, exits), pending_swaps) = tokio::join!(
                map_position_to_detail(&position),
                load_entry_exit_history(&position),
                load_pending_swaps(&position)
            );

            // Fetch token data from database
            let token_data =
                tokens::database::get_full_token_async(crate::chains::active_chain(), mint)
                    .await
                    .ok()
                    .flatten();

            // Build token info from token database
            let token_info = token_data.as_ref().map(|token| {
                let website = token.websites.first().map(|w| w.url.clone());
                let twitter = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("twitter"))
                    .map(|s| s.url.clone());
                let telegram = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("telegram"))
                    .map(|s| s.url.clone());

                PositionTokenInfo {
                    decimals: token.decimals,
                    description: token.description.clone(),
                    image_url: token.image_url.clone(),
                    website,
                    twitter,
                    telegram,
                }
            });

            // Build market data from token database
            let market_data = token_data.as_ref().map(|token| PositionMarketData {
                market_cap: token.market_cap,
                fdv: token.fdv,
                liquidity_usd: token.liquidity_usd,
                volume_24h: token.volume_h24,
                price_change_h1: token.price_change_h1,
                price_change_h24: token.price_change_h24,
                holder_count: token.holder_count(),
            });

            // Build security summary from token database
            let security = token_data.as_ref().map(|token| {
                let top_risks: Vec<String> = token
                    .security_risks
                    .iter()
                    .take(3)
                    .map(|r| r.name.clone())
                    .collect();

                PositionSecuritySummary {
                    score_normalized: token.security_score_normalised,
                    risk_level: SecurityRiskBand::of_optional(token.security_score_normalised),
                    has_mint_authority: token.mint_authority.is_some(),
                    has_freeze_authority: token.freeze_authority.is_some(),
                    top_risks,
                }
            });

            // Get pool info from pool service
            let pool_info =
                pools::get_pool_price(crate::chains::active_chain(), mint).map(|price_result| {
                    PositionPoolInfo {
                        pool_address: Some(price_result.pool_address.clone()),
                        pool_program: pools::pool_protocol(
                            crate::chains::active_chain(),
                            mint,
                            &price_result.pool_address,
                        )
                        .map(str::to_owned),
                        liquidity_native: Some(price_result.native_reserves),
                    }
                });

            // Build external links
            let external_links = ExternalLinks::for_mint(mint);

            // Calculate position age in seconds
            let position_age_seconds = Some(
                Utc::now()
                    .signed_duration_since(position.entry_time)
                    .num_seconds(),
            );

            // Get SOL price in USD
            let native_price_usd = {
                let price = native_price::get_native_price();
                if price > 0.0 {
                    Some(price)
                } else {
                    None
                }
            };

            success_response(PositionDetailResponse {
                position: Some(detail),
                entries,
                exits,
                token_info,
                market_data,
                security,
                pool_info,
                external_links,
                position_age_seconds,
                native_price_usd,
                pending_swaps,
                fetched_at: Utc::now().to_rfc3339(),
            })
        }
        Ok(None) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
            .details(key.clone())
            .into_response(),
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to resolve position for key {key}: {err}"),
            );

            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_POSITIONS_DETAIL_FAILED)
                .details(err.to_string())
                .into_response()
        }
    }
}

/// `id:<n>` resolves ANY position (open, closed or archived) — that is how the Closed and
/// Archived tabs open their rows. A MINT key resolves only the OPEN position: every caller
/// that looks a position up by mint (the trade dialog, the token-details Positions tab, the
/// context menu) is asking "is this token held right now", and answering with a closed
/// position made the dashboard turn a Buy into an Add that the trade route then rejected
/// with "no open position for this token".
pub(super) async fn resolve_position_by_key(
    key: &str,
) -> crate::webserver::Result<Option<positions::Position>> {
    if let Some(id_part) = key.strip_prefix("id:") {
        let id: i64 = id_part
            .parse()
            .map_err(|_| crate::webserver::Error::InvalidPositionKey {
                detail: format!("Invalid position id: {id_part}"),
            })?;
        return Ok(positions::get_position_by_id(id).await);
    }

    if let Some(mint_part) = key.strip_prefix("mint:") {
        return Ok(positions::get_position_by_mint(mint_part).await);
    }

    Ok(positions::get_position_by_mint(key).await)
}

async fn map_position_to_detail(position: &positions::Position) -> PositionDetail {
    let policy = if runs_policy_exits(position) {
        Some(crate::trader::policy::resolve_exit_policy(position).await)
    } else {
        None
    };
    let (trailing_stop_active, break_even_active) = peak_stops_active(policy.as_ref());
    let exit_plan = policy.as_ref().map(|policy| exit_plan(position, policy));
    PositionDetail {
        summary: map_position_to_response_async(position).await,
        phantom_remove: position.phantom_remove,
        phantom_first_seen: position.phantom_first_seen.map(|dt| dt.timestamp()),
        peak_since_last_buy: peak_since_last_buy(position),
        trailing_stop_active,
        break_even_active,
        default_add_size_native: default_add_size_native(position),
        quote_asset: crate::chains::adapter().is_quote_asset(&position.mint),
        exit_plan,
    }
}

/// Every exit rule `policy` switches on for `position`; a take-profit step below the
/// round's reserved level is taken.
fn exit_plan(position: &positions::Position, policy: &ExitPolicy) -> ExitPlanResponse {
    let take_profit = &policy.take_profit;
    let steps = if take_profit.enabled {
        take_profit.levels.as_slice()
    } else {
        &[]
    };
    let trailing = &policy.trailing;
    let break_even = &policy.break_even;
    let time = &policy.time;
    let stagnation = &policy.time_stagnation;
    ExitPlanResponse {
        stop_loss_pct: policy
            .stop_loss
            .enabled
            .then_some(policy.stop_loss.threshold_pct),
        take_profit_steps: steps
            .iter()
            .enumerate()
            .map(|(index, step)| ExitPlanStepResponse {
                profit_pct: step.profit_pct,
                sell_pct: step.sell_pct,
                taken: (index as u64) < u64::from(position.take_profit_level),
            })
            .collect(),
        trailing_stop: trailing.enabled.then(|| ExitPlanTrailingResponse {
            activation_pct: trailing.activation_pct,
            distance_pct: trailing.distance_pct,
            bands: trailing.bands.clone(),
        }),
        break_even: break_even.enabled.then_some(ExitPlanBreakEvenResponse {
            trigger_pct: break_even.trigger_pct,
            offset_pct: break_even.offset_pct,
        }),
        time_loss: time.enabled.then_some(ExitPlanTimeLossResponse {
            duration_seconds: time.duration_seconds,
            loss_threshold_pct: time.loss_threshold_pct,
        }),
        time_stagnation: stagnation.enabled.then_some(ExitPlanStagnationResponse {
            duration_seconds: stagnation.duration_seconds,
            band_pct: stagnation.band_pct,
        }),
    }
}

/// The size a manual add of `position` presets, from the one owner the add route and
/// the agent add tool share; `None` once the position is closed.
fn default_add_size_native(position: &positions::Position) -> Option<f64> {
    if !positions::state::is_position_open(position) {
        return None;
    }
    let levels = crate::trader::config::get_dca_levels();
    let trade_size = crate::trader::config::get_trade_size_native();
    crate::trader::evaluators::default_add_size_native(position, &levels, trade_size)
}

/// Which stops measured from the basis peak the exit monitor evaluates, as
/// `(trailing stop, break-even stop)`, under the policy it resolves for the position;
/// `None` when it evaluates no policy exit for it.
fn peak_stops_active(policy: Option<&ExitPolicy>) -> (bool, bool) {
    policy.map_or((false, false), |policy| {
        (policy.trailing.enabled, policy.break_even.enabled)
    })
}

/// The basis peak of `position` as the peak since its last buy, when a buy restarted it
/// and it differs from the round's peak. A wallet-derived round restarts it at the round's
/// average, which no buy paid, so it has none.
fn peak_since_last_buy(position: &positions::Position) -> Option<f64> {
    (!position.is_wallet_derived()
        && position.basis_peak_price > 0.0
        && position.basis_peak_price != position.price_highest)
        .then_some(position.basis_peak_price)
}

/// Whether the exit monitor evaluates the exit policy of `position`: it is open and not
/// archived, and its management runs policy exits.
fn runs_policy_exits(position: &positions::Position) -> bool {
    positions::state::is_position_open(position) && position.management.allows_policy_exits()
}

/// Load entry and exit history for a position
async fn load_entry_exit_history(
    position: &positions::Position,
) -> (Vec<EntryRecordResponse>, Vec<ExitRecordResponse>) {
    let Some(id) = position.id else {
        return (Vec::new(), Vec::new());
    };

    // Load entries
    let entries = match positions::get_entry_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| EntryRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                native_spent: r.native_spent,
                transaction_signature: r.transaction_signature,
                is_dca: r.is_dca,
                fees_native: r.fees_raw.map(|l| adapter().raw_to_native(l)),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load entry history for position {id}: {err}"),
            );
            Vec::new()
        }
    };

    // Load exits
    let exits = match positions::get_exit_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| ExitRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                native_received: r.native_received,
                transaction_signature: r.transaction_signature,
                is_partial: r.is_partial,
                percentage: r.percentage,
                fees_native: r.fees_raw.map(|l| adapter().raw_to_native(l)),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load exit history for position {id}: {err}"),
            );
            Vec::new()
        }
    };

    (entries, exits)
}

#[cfg(test)]
mod tests {
    use super::{exit_plan, peak_since_last_buy, peak_stops_active, runs_policy_exits};
    use crate::config::{TakeProfitLevel, TraderConfig, TrailingStopBand};
    use crate::positions::{Position, PositionManagement, PositionOrigin};
    use crate::trader::policy::ExitPolicy;
    use crate::webserver::routes::positions::types::{
        ExitPlanBreakEvenResponse, ExitPlanResponse, ExitPlanStagnationResponse,
        ExitPlanStepResponse, ExitPlanTimeLossResponse, ExitPlanTrailingResponse,
    };

    /// An open bot position that took an add: its basis peak restarted below the round's
    /// peak.
    fn averaged_position() -> Position {
        serde_json::from_str(
            r#"{
            "id": 7,
            "mint": "So11111111111111111111111111111111111111112",
            "symbol": "TEST",
            "name": "Test",
            "entry_price": 1.0,
            "entry_time": "2026-10-01T00:00:00Z",
            "exit_price": null,
            "exit_time": null,
            "position_type": "buy",
            "entry_size_native": 1.0,
            "total_size_native": 2.0,
            "price_highest": 1.4,
            "price_lowest": 0.8,
            "basis_peak_price": 0.95,
            "entry_transaction_signature": "entry",
            "exit_transaction_signature": null,
            "token_amount": "2000000",
            "effective_entry_price": 1.0,
            "effective_exit_price": null,
            "native_received": null,
            "profit_target_min": null,
            "profit_target_max": null,
            "liquidity_tier": null,
            "transaction_entry_verified": true,
            "transaction_exit_verified": false,
            "entry_fee_raw": null,
            "exit_fee_raw": null,
            "current_price": 0.9,
            "current_price_updated": null,
            "phantom_remove": false,
            "phantom_confirmations": 0,
            "phantom_first_seen": null,
            "synthetic_exit": false,
            "closed_reason": null,
            "pnl": null,
            "pnl_percent": null,
            "unrealized_pnl": null,
            "unrealized_pnl_percent": null,
            "remaining_token_amount": "2000000",
            "total_exited_amount": "0",
            "average_exit_price": null,
            "partial_exit_count": 0,
            "dca_count": 1,
            "average_entry_price": 0.9,
            "last_dca_time": null,
            "origin": { "kind": "auto", "strategy_id": null },
            "management": "auto_trader"
        }"#,
        )
        .expect("position fixture deserializes")
    }

    #[test]
    fn the_peak_since_the_last_buy_is_reported_only_where_a_buy_restarted_it() {
        let averaged = averaged_position();
        assert_eq!(peak_since_last_buy(&averaged), Some(0.95));

        let mut never_averaged = averaged_position();
        never_averaged.basis_peak_price = never_averaged.price_highest;
        assert_eq!(peak_since_last_buy(&never_averaged), None);

        let mut unseeded = averaged_position();
        unseeded.basis_peak_price = 0.0;
        assert_eq!(peak_since_last_buy(&unseeded), None);

        let mut derived = averaged_position();
        derived.origin = PositionOrigin::External;
        assert_eq!(
            peak_since_last_buy(&derived),
            None,
            "a wallet-derived round restarts its peak at the round's average, not at a buy"
        );
    }

    #[test]
    fn policy_exits_run_only_on_open_positions_whose_management_runs_them() {
        for (management, runs) in [
            (PositionManagement::AutoTrader, true),
            (PositionManagement::Hybrid, true),
            (PositionManagement::UserOnly, false),
            (PositionManagement::CopyTask, false),
        ] {
            let mut position = averaged_position();
            position.management = management;
            assert_eq!(runs_policy_exits(&position), runs, "{management:?}");
        }

        let mut closed = averaged_position();
        closed.exit_time = Some(closed.entry_time);
        closed.transaction_exit_verified = true;
        assert!(!runs_policy_exits(&closed), "a closed position");

        let mut archived = averaged_position();
        archived.archived = true;
        assert!(!runs_policy_exits(&archived), "an archived position");
    }

    #[test]
    fn each_peak_stop_is_active_only_where_its_own_rule_is_on() {
        crate::config::utils::install_default_config();
        let mut break_even_only = ExitPolicy::from_config();
        break_even_only.trailing.enabled = false;
        break_even_only.break_even.enabled = true;
        assert_eq!(peak_stops_active(Some(&break_even_only)), (false, true));

        let mut trailing_only = break_even_only.clone();
        trailing_only.trailing.enabled = true;
        trailing_only.break_even.enabled = false;
        assert_eq!(peak_stops_active(Some(&trailing_only)), (true, false));

        let mut both = trailing_only.clone();
        both.break_even.enabled = true;
        assert_eq!(peak_stops_active(Some(&both)), (true, true));

        assert_eq!(
            peak_stops_active(None),
            (false, false),
            "no stop is active where the exit monitor runs no policy exit"
        );
    }

    #[test]
    fn the_exit_plan_states_every_exit_rule_the_policy_switches_on() {
        let mut policy = ExitPolicy::from_trader_config(&TraderConfig::default());
        policy.stop_loss.enabled = true;
        policy.stop_loss.threshold_pct = 30.0;
        policy.take_profit.enabled = true;
        policy.take_profit.levels = vec![
            TakeProfitLevel {
                profit_pct: 20.0,
                sell_pct: 30.0,
            },
            TakeProfitLevel {
                profit_pct: 50.0,
                sell_pct: 70.0,
            },
        ];
        policy.trailing.enabled = true;
        policy.trailing.activation_pct = 20.0;
        policy.trailing.distance_pct = 10.0;
        policy.trailing.bands = vec![TrailingStopBand {
            profit_pct: 50.0,
            distance_pct: 6.0,
        }];
        policy.break_even.enabled = true;
        policy.break_even.trigger_pct = 15.0;
        policy.break_even.offset_pct = 2.0;
        policy.time.enabled = true;
        policy.time.duration_seconds = 7200.0;
        policy.time.loss_threshold_pct = -10.0;
        policy.time_stagnation.enabled = true;
        policy.time_stagnation.duration_seconds = 14_400.0;
        policy.time_stagnation.band_pct = 3.0;
        let mut position = averaged_position();
        position.take_profit_level = 1;

        assert_eq!(
            exit_plan(&position, &policy),
            ExitPlanResponse {
                stop_loss_pct: Some(30.0),
                take_profit_steps: vec![
                    ExitPlanStepResponse {
                        profit_pct: 20.0,
                        sell_pct: 30.0,
                        taken: true,
                    },
                    ExitPlanStepResponse {
                        profit_pct: 50.0,
                        sell_pct: 70.0,
                        taken: false,
                    },
                ],
                trailing_stop: Some(ExitPlanTrailingResponse {
                    activation_pct: 20.0,
                    distance_pct: 10.0,
                    bands: vec![TrailingStopBand {
                        profit_pct: 50.0,
                        distance_pct: 6.0,
                    }],
                }),
                break_even: Some(ExitPlanBreakEvenResponse {
                    trigger_pct: 15.0,
                    offset_pct: 2.0,
                }),
                time_loss: Some(ExitPlanTimeLossResponse {
                    duration_seconds: 7200.0,
                    loss_threshold_pct: -10.0,
                }),
                time_stagnation: Some(ExitPlanStagnationResponse {
                    duration_seconds: 14_400.0,
                    band_pct: 3.0,
                }),
            }
        );

        policy.stop_loss.enabled = false;
        policy.take_profit.enabled = false;
        policy.trailing.enabled = false;
        policy.break_even.enabled = false;
        policy.time.enabled = false;
        policy.time_stagnation.enabled = false;
        assert_eq!(
            exit_plan(&position, &policy),
            ExitPlanResponse {
                stop_loss_pct: None,
                take_profit_steps: Vec::new(),
                trailing_stop: None,
                break_even: None,
                time_loss: None,
                time_stagnation: None,
            },
            "a rule that is off states nothing"
        );
    }
}
