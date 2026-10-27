// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exit evaluation coordinator with priority-based checks and LLM analysis
//!
//! Evaluates whether an exit should be made for a position by checking in priority order,
//! split into two groups:
//!
//! Safety (1-2, always run, regardless of who owns this position's exits):
//! 1. Blacklist (emergency - sync)
//! 2. Risk limits (>90% loss - emergency)
//!
//! Policy (3-10, only when the auto-trader owns this position's exits):
//! 3. LLM exit analysis (emergency, high or normal priority by the analysis' urgency - if enabled)
//! 4. Stop loss (high priority - fixed threshold from entry)
//! 5. Trailing stop (high priority - from the basis peak)
//! 6. Break-even stop (high priority - armed by the basis peak, fires below entry too)
//! 7. Take profit (normal priority - one ladder step per evaluation)
//! 8. Time override (high priority)
//! 9. Stagnation exit (normal priority - held flat inside the band)
//! 10. Strategy exit (normal priority)
//!
//! Priorities 4-9 are the built-in rules; their order is owned by `exit_rules`, which the
//! paper copy book shares.

use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::Position;
use crate::trader::evaluators::exit_rules::{self, ExitInputs, ExitRule, RuleOutcome};
use crate::trader::policy::ExitPolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use crate::trader::{evaluators, llm_analysis, safety};

/// Force stop, the price cascade and the fresh position re-read — everything every exit
/// rule needs before any of them can run. `None` means nothing is evaluable this tick.
///
/// Force stop lives here on purpose: it is the global halt, so no caller can reach an
/// exit rule without passing it, whichever of the two evaluators below it chooses.
async fn resolve_evaluation_input(position: &Position) -> Option<(Position, f64)> {
    // Early exit: Force stop is active (even exits are halted during force stop)
    if crate::global::is_force_stopped() {
        return None;
    }

    // Get current price with fallback cascade:
    // 1. Pool price (real-time on-chain) — preferred
    // 2. API price from token database (DexScreener/GeckoTerminal) — if fresh
    // 3. Force-fetch fresh API price if stale
    // This matches the price resolution used by position open/close/DCA operations.
    let current_price = match get_price_with_api_fallback(&position.mint).await {
        Some((price_result, _source)) => {
            if price_result.price_native > 0.0 && price_result.price_native.is_finite() {
                price_result.price_native
            } else {
                return None; // Invalid price
            }
        }
        None => return None, // No price data from any source
    };

    // Get fresh position with updated basis peak for accurate trailing stop calculation
    let fresh_position = match crate::positions::get_position_by_mint(&position.mint).await {
        Some(pos) => pos,
        None => return None, // Position disappeared
    };

    Some((fresh_position, current_price))
}

/// Priorities 1-2: blacklist and the >90% loss risk limit. These are SAFETY, not policy:
/// the token turned out to be a rug, or the position is already destroyed. They apply to
/// every position the bot is permitted to act on, whatever owns its exits.
pub(crate) async fn evaluate_safety_exit(
    position: &Position,
    current_price: f64,
) -> crate::trader::Result<Option<TradeDecision>> {
    // Priority 1: Blacklist (emergency - token-level DB check only)
    if let Some(decision) = safety::check_blacklist_exit(position, current_price).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "Token {} blacklisted! Emergency exit signal",
                position.symbol
            ),
        );
        return Ok(Some(decision));
    }

    // Priority 2: Risk limits (>90% loss - emergency)
    if let Some(decision) = safety::check_risk_limits(position, current_price).await? {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "Risk limit triggered for {} (>90% loss)! Emergency exit signal",
                position.symbol
            ),
        );
        return Ok(Some(decision));
    }

    Ok(None)
}

/// Priorities 3-10: LLM analysis, stop loss, trailing stop, break-even stop, take profit, time
/// override, stagnation exit, strategy exit.
/// These are POLICY — opinions about when to take a trade off — and only run when the
/// auto-trader owns this position's exits.
pub(crate) async fn evaluate_policy_exit(
    position: &Position,
    current_price: f64,
) -> crate::trader::Result<Option<TradeDecision>> {
    let policy = crate::trader::policy::resolve_exit_policy(position).await;
    // Resolved once: the LLM and strategy exits read the position's token on its chain.
    let chain = crate::chains::chain_for_address(&position.mint);

    // Priority 3: LLM exit analysis (high priority - if enabled)
    if llm_analysis::should_analyze_exit() {
        // Get token data for LLM analysis
        let stored = match &chain {
            Ok(chain) => crate::tokens::get_full_token_async(*chain, &position.mint).await,
            Err(_) => Ok(None),
        };
        match stored {
            Ok(Some(token)) => {
                match llm_analysis::analyze_exit(position, &token).await {
                    Some(result) => {
                        if result.action == llm_analysis::ExitAction::Exit {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "LLM analysis suggests exit for {} (confidence: {}%, urgency: {:?}, reason: {})",
                                    position.symbol,
                                    result.confidence,
                                    result.urgency,
                                    result.reasoning
                                ),
                            );

                            // Create exit decision
                            let trade_decision = TradeDecision {
                                position_id: position.id.map(|id| id.to_string()),
                                mint: position.mint.clone(),
                                action: TradeAction::Sell,
                                reason: TradeReason::LlmAnalysisExit,
                                strategy_id: Some("llm_analysis_exit".to_owned()),
                                timestamp: chrono::Utc::now(),
                                priority: match result.urgency {
                                    llm_analysis::ExitUrgency::Immediate => {
                                        TradePriority::Emergency
                                    }
                                    llm_analysis::ExitUrgency::High => TradePriority::High,
                                    _ => TradePriority::Normal,
                                },
                                price_native: Some(current_price),
                                size_native: None,
                                exit_percentage: None, // Full exit
                                // Auto-trader slippage always follows config.
                                slippage_pct: None,
                                take_profit_level: None,
                            };

                            // Record the LLM-analysis exit signal event.
                            crate::events::record_trader_event(
                                "exit_signal_llm_analysis",
                                crate::events::Severity::Info,
                                Some(&position.mint),
                                None,
                                serde_json::json!({
                                    "exit_type": "llm_analysis_exit",
                                    "mint": position.mint,
                                    "symbol": position.symbol,
                                    "current_price": current_price,
                                    "confidence": result.confidence,
                                    "urgency": format!("{:?}", result.urgency),
                                    "reasoning": result.reasoning,
                                    "provider": result.provider,
                                }),
                            )
                            .await;

                            return Ok(Some(trade_decision));
                        } else if result.action == llm_analysis::ExitAction::Hold {
                            crate::logger::debug(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "LLM analysis suggests holding {} (confidence: {}%, reason: {})",
                                    position.symbol, result.confidence, result.reasoning
                                ),
                            );
                        }
                    }
                    None => {
                        // LLM analysis failed or is unavailable
                        crate::logger::debug(
                            crate::logger::LogTag::Trader,
                            &format!("LLM exit analysis unavailable for {}", position.symbol),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!(
                        "Token data not found for LLM exit analysis: {}",
                        position.mint
                    ),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for LLM exit analysis: {e}"),
                );
            }
        }
    }

    // Priorities 4-9: the built-in rules, in the one order `exit_rules` owns. A partial
    // outcome is passed over while the positions store awaits reconciliation, and a partial
    // take-profit step while another partial exit of the mint is confirming (the next step
    // sells only after the previous one settles), so the rules after it still run this tick.
    let inputs = live_exit_inputs(position, current_price, chrono::Utc::now());
    let partial_pending = crate::positions::is_partial_exit_pending(&position.mint).await;
    let admit = |outcome: &RuleOutcome| {
        if outcome.exit_percentage.is_none() {
            return true;
        }
        let refusal = if crate::positions::awaiting_reconciliation() {
            "the positions store is not reconciled"
        } else if partial_pending && matches!(outcome.rule, ExitRule::TakeProfit { .. }) {
            "a partial exit is still confirming"
        } else {
            return true;
        };
        crate::logger::debug(
            crate::logger::LogTag::Trader,
            &format!(
                "Partial {:?} exit skipped for {}: {refusal}",
                outcome.rule, position.symbol
            ),
        );
        false
    };
    let decision = exit_rules::decide_admitting(&inputs, &policy, admit);
    for (rule, e) in &decision.rule_errors {
        let check = match rule {
            ExitRule::StopLoss => "stop loss",
            ExitRule::Trailing => "trailing stop",
            ExitRule::BreakEven => "break-even stop",
            ExitRule::TakeProfit { .. } => "take profit",
            ExitRule::TimeLoss => "time override",
            ExitRule::TimeStagnation => "stagnation exit",
        };
        crate::logger::warning(
            crate::logger::LogTag::Trader,
            &format!("Error checking {check} for {}: {e}", position.symbol),
        );
    }
    if let Some(outcome) = decision.outcome {
        record_rule_signal(position, current_price, &outcome, &policy).await;
        return Ok(Some(rule_decision(position, current_price, &outcome)));
    }

    // Priority 10: Strategy exit (normal priority)
    let chain = match chain {
        Ok(chain) => chain,
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Strategy exit skipped for {} this tick: {e}",
                    position.symbol
                ),
            );
            return Ok(None);
        }
    };
    match evaluators::StrategyEvaluator::check_exit_strategies(chain, position, current_price).await
    {
        Ok(Some(decision)) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Strategy exit signal for {} (strategy: {:?})",
                    position.symbol, decision.strategy_id
                ),
            );

            // Record strategy exit signal event
            crate::events::record_trader_event(
                "exit_signal_strategy",
                crate::events::Severity::Info,
                Some(&position.mint),
                None,
                serde_json::json!({
                  "exit_type": "strategy",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "strategy_id": decision.strategy_id,
                  "current_price": current_price,
                }),
            )
            .await;

            return Ok(Some(decision));
        }
        Ok(None) => {}
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Error checking strategy exit for {}: {}",
                    position.symbol, e
                ),
            );
        }
    }

    // No exit signals
    Ok(None)
}

/// What the live book gives the built-in exit rules: the position's average entry and
/// basis peak, the resolved price, its age at `now`, whether it already booked a partial
/// exit, and the take-profit steps it reserved.
pub fn live_exit_inputs(
    position: &Position,
    current_price: f64,
    now: chrono::DateTime<chrono::Utc>,
) -> ExitInputs {
    ExitInputs {
        average_entry_price: position.average_entry_price,
        current_price,
        basis_peak_price: position.basis_peak_price,
        held_seconds: (now - position.entry_time).num_seconds(),
        partial_exits_taken: position.partial_exit_count > 0,
        take_profit_levels_taken: position.take_profit_level,
    }
}

/// The sell a fired built-in rule asks of the live book. Stop loss, trailing stop,
/// break-even stop and the time override sell at high priority; a take-profit step at
/// normal priority, carrying the step it takes, and the stagnation exit at normal
/// priority, since a flat holding is not at risk.
pub fn rule_decision(
    position: &Position,
    current_price: f64,
    outcome: &RuleOutcome,
) -> TradeDecision {
    let (reason, priority) = match outcome.rule {
        ExitRule::StopLoss => (TradeReason::StopLoss, TradePriority::High),
        ExitRule::Trailing => (TradeReason::TrailingStop, TradePriority::High),
        ExitRule::BreakEven => (TradeReason::BreakEvenStop, TradePriority::High),
        ExitRule::TakeProfit { .. } => (TradeReason::TakeProfit, TradePriority::Normal),
        ExitRule::TimeLoss => (TradeReason::TimeOverride, TradePriority::High),
        ExitRule::TimeStagnation => (TradeReason::TimeStagnation, TradePriority::Normal),
    };
    let take_profit_level = match outcome.rule {
        ExitRule::TakeProfit { level } => Some(level),
        ExitRule::StopLoss
        | ExitRule::Trailing
        | ExitRule::BreakEven
        | ExitRule::TimeLoss
        | ExitRule::TimeStagnation => None,
    };
    TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::Sell,
        reason,
        strategy_id: None,
        timestamp: chrono::Utc::now(),
        priority,
        price_native: Some(current_price),
        size_native: None,
        exit_percentage: outcome.exit_percentage,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
        take_profit_level,
    }
}

/// Log and record the exit signal of a fired built-in rule.
async fn record_rule_signal(
    position: &Position,
    current_price: f64,
    outcome: &RuleOutcome,
    policy: &ExitPolicy,
) {
    let (event, severity, payload) = match outcome.rule {
        ExitRule::StopLoss => {
            evaluators::exit_stop_loss::log_stop_loss_triggered(
                position,
                current_price,
                outcome.exit_percentage,
                &policy.stop_loss,
            );
            (
                "exit_signal_stop_loss",
                crate::events::Severity::Warn,
                serde_json::json!({
                  "exit_type": "stop_loss",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "entry_price": position.average_entry_price,
                  "current_price": current_price,
                  "loss_pct": ((position.average_entry_price - current_price) / position.average_entry_price) * 100.0,
                }),
            )
        }
        ExitRule::Trailing => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("Trailing stop triggered for {}", position.symbol),
            );
            (
                "exit_signal_trailing_stop",
                crate::events::Severity::Info,
                serde_json::json!({
                  "exit_type": "trailing_stop",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "current_price": current_price,
                }),
            )
        }
        ExitRule::BreakEven => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Break-even stop triggered for {} ({:.1}% armed, {:.1}% kept)",
                    position.symbol, policy.break_even.trigger_pct, policy.break_even.offset_pct
                ),
            );
            (
                "exit_signal_break_even",
                crate::events::Severity::Info,
                serde_json::json!({
                  "exit_type": "break_even",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "entry_price": position.average_entry_price,
                  "basis_peak_price": position.basis_peak_price,
                  "current_price": current_price,
                }),
            )
        }
        ExitRule::TakeProfit { level } => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Take-profit step {level} reached for {} ({})",
                    position.symbol,
                    outcome
                        .exit_percentage
                        .map_or("full exit".to_owned(), |pct| format!("sells {pct:.2}%"))
                ),
            );
            // The event id and exit type are stored with every recorded event, so they
            // keep the names the event log already holds.
            (
                "exit_signal_roi_target",
                crate::events::Severity::Info,
                serde_json::json!({
                  "exit_type": "roi_target",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "current_price": current_price,
                  "level": level,
                  "exit_percentage": outcome.exit_percentage,
                }),
            )
        }
        ExitRule::TimeLoss => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("Time override triggered for {}", position.symbol),
            );
            (
                "exit_signal_time_override",
                crate::events::Severity::Info,
                serde_json::json!({
                  "exit_type": "time_override",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "current_price": current_price,
                }),
            )
        }
        ExitRule::TimeStagnation => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Stagnation exit triggered for {} (flat within {:.1}% for {:.0} min)",
                    position.symbol,
                    policy.time_stagnation.band_pct,
                    policy.time_stagnation.duration_seconds / 60.0
                ),
            );
            (
                "exit_signal_time_stagnation",
                crate::events::Severity::Info,
                serde_json::json!({
                  "exit_type": "time_stagnation",
                  "mint": position.mint,
                  "symbol": position.symbol,
                  "entry_price": position.average_entry_price,
                  "current_price": current_price,
                }),
            )
        }
    };
    crate::events::record_trader_event(event, severity, Some(&position.mint), None, payload).await;
}

/// Evaluate exit opportunity for a position
///
/// Checks exit conditions in priority order. First matching condition returns immediately.
///
/// Resolves the current price and a fresh position read (see [`resolve_evaluation_input`]),
/// then runs safety checks (priorities 1-2, see [`evaluate_safety_exit`]) followed by policy
/// checks (priorities 3-10, see [`evaluate_policy_exit`]).
///
/// Returns:
/// - Ok(Some(TradeDecision)) if exit should be made
/// - Ok(None) if no exit signal
/// - Err(String) if evaluation failed
pub async fn evaluate_exit_for_position(
    position: Position,
) -> crate::trader::Result<Option<TradeDecision>> {
    let Some((fresh_position, current_price)) = resolve_evaluation_input(&position).await else {
        return Ok(None);
    };
    if fresh_position.management.allows_safety_exits() {
        if let Some(decision) = evaluate_safety_exit(&fresh_position, current_price).await? {
            return Ok(Some(decision));
        }
    }
    if fresh_position.management.allows_policy_exits() {
        evaluate_policy_exit(&fresh_position, current_price).await
    } else {
        Ok(None)
    }
}
