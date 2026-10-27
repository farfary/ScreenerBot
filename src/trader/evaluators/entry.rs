// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Entry evaluation logic with integrated LLM analysis
//!
//! Evaluates whether an entry should be made for a token by checking:
//! 1. Admission of each mode with enabled entry strategies, on its own book (see
//!    `trader::admission::admit_modes`): force stop, loss limit (live only), entry storage,
//!    connectivity, position limits, existing position, re-entry cooldown, paper cash
//!    (paper only), blacklist
//! 2. LLM entry analysis (if enabled)
//! 3. Strategy signals of the admitted modes

use crate::chains::ChainId;
use crate::pools::PriceResult;
use crate::strategies::db::has_enabled_strategies;
use crate::strategies::types::StrategyType;
use crate::trader::admission::{admit_modes, EntryBlock, EntryCycle};
use crate::trader::paper::TradingMode;
use crate::trader::types::TradeDecision;
use crate::trader::{evaluators, llm_analysis};

/// The modes an entry is evaluated for: those with an enabled entry strategy, live
/// first. With none, live, so the live admission gauntlet reports as it always has.
fn wanted_modes() -> Vec<TradingMode> {
    let wanted: Vec<TradingMode> = [TradingMode::Live, TradingMode::Paper]
        .into_iter()
        .filter(|mode| matches!(has_enabled_strategies(StrategyType::Entry, *mode), Ok(true)))
        .collect();
    if wanted.is_empty() {
        vec![TradingMode::Live]
    } else {
        wanted
    }
}

/// The inputs every token of one entry cycle is admitted against, read once for the
/// cycle: the wanted modes, a SQLite read run on the blocking pool, and the paper cash
/// when paper is wanted.
pub async fn entry_cycle() -> EntryCycle {
    let wanted = match tokio::task::spawn_blocking(wanted_modes).await {
        Ok(wanted) => wanted,
        Err(error) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("Wanted entry modes unavailable: {error}"),
            );
            vec![TradingMode::Live]
        }
    };
    EntryCycle::read(wanted).await
}

/// Evaluate entry opportunity for a token
///
/// Runs the admission gauntlet of each mode `cycle` wants first (see
/// [`crate::trader::admission::admit_modes`]), then LLM entry analysis (if enabled), then
/// the strategies of the admitted modes.
///
/// Returns:
/// - Ok(Some(TradeDecision)) if entry should be made
/// - Ok(None) if no entry signal or admission check failed
/// - Err(String) if evaluation failed due to connectivity or other errors
pub async fn evaluate_entry_for_token(
    chain: ChainId,
    token_mint: &str,
    price_info: &PriceResult,
    cycle: &EntryCycle,
) -> crate::trader::Result<Option<TradeDecision>> {
    let admitted = match admit_modes(
        token_mint,
        &["rpc", "dexscreener", "rugcheck"],
        cycle,
        evaluators::strategies::strategy_entry_size_native(),
    )
    .await
    {
        Ok(admitted) => admitted,
        Err(block) => {
            return match block {
                EntryBlock::Connectivity(unhealthy) => {
                    Err(crate::trader::Error::UnhealthyEndpoints { detail: unhealthy })
                }
                EntryBlock::CheckFailed(e) => Err(crate::trader::Error::StrategyEvaluation {
                    mint: token_mint.to_owned(),
                    detail: e,
                }),
                EntryBlock::ForceStopped
                | EntryBlock::LossLimit
                | EntryBlock::StorageUnavailable(_)
                | EntryBlock::PositionLimit
                | EntryBlock::AlreadyOpen
                | EntryBlock::ReentryCooldown
                | EntryBlock::OpenCooldown { .. }
                | EntryBlock::EntryReserved
                | EntryBlock::PaperBalanceExhausted
                | EntryBlock::Blacklisted => Ok(None),
            }
        }
    };

    // 6. LLM entry analysis - check the model-scored entry decision when enabled.
    if llm_analysis::should_analyze_entry() {
        // Get token data for LLM analysis
        match crate::tokens::get_full_token_async(chain, token_mint).await {
            Ok(Some(token)) => {
                match llm_analysis::analyze_entry(&token).await {
                    Some(result) => {
                        if !result.should_enter {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "LLM analysis rejected entry for {} (confidence: {}%, reason: {})",
                                    token.symbol, result.confidence, result.reasoning
                                ),
                            );
                            return Ok(None);
                        } else {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "LLM analysis approved entry for {} (confidence: {}%, reason: {})",
                                    token.symbol, result.confidence, result.reasoning
                                ),
                            );
                        }
                    }
                    None => {
                        // LLM analysis failed or is disabled, continue with strategy checks
                        crate::logger::debug(
                            crate::logger::LogTag::Trader,
                            &format!("LLM entry analysis unavailable for {token_mint}"),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!("Token data not found for LLM analysis: {token_mint}"),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for LLM analysis: {e}"),
                );
            }
        }
    }

    // 7. Strategy evaluation - check configured entry strategies
    evaluators::StrategyEvaluator::check_entry_strategies(chain, token_mint, price_info, &admitted)
        .await
}
