//! Entry evaluation logic with integrated AI analysis
//!
//! Evaluates whether an entry should be made for a token by checking:
//! 1. Source-independent admission (see `trader::admission::check_entry_admission`):
//!    force stop, loss limit, connectivity, position limits, existing position,
//!    re-entry cooldown, blacklist
//! 2. AI entry analysis (if enabled)
//! 3. Strategy signals

use crate::pools::PriceResult;
use crate::trader::admission::{check_entry_admission, EntryBlock};
use crate::trader::types::TradeDecision;
use crate::trader::{evaluators, llm_analysis};

/// Evaluate entry opportunity for a token
///
/// Runs the source-independent admission gauntlet first (see
/// [`crate::trader::admission::check_entry_admission`]), then AI entry analysis (if
/// enabled), then strategy evaluation.
///
/// Returns:
/// - Ok(Some(TradeDecision)) if entry should be made
/// - Ok(None) if no entry signal or admission check failed
/// - Err(String) if evaluation failed due to connectivity or other errors
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> crate::trader::Result<Option<TradeDecision>> {
    if let Err(block) = check_entry_admission(token_mint, &["rpc", "dexscreener", "rugcheck"]).await
    {
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
            | EntryBlock::PositionLimit
            | EntryBlock::AlreadyOpen
            | EntryBlock::ReentryCooldown
            | EntryBlock::OpenCooldown { .. }
            | EntryBlock::EntryReserved
            | EntryBlock::Blacklisted => Ok(None),
        };
    }

    // 6. AI entry analysis - check if AI recommends entry (if enabled)
    if llm_analysis::should_analyze_entry() {
        // Get token data for AI analysis
        match crate::tokens::get_full_token_async(token_mint).await {
            Ok(Some(token)) => {
                match llm_analysis::analyze_entry(&token).await {
                    Some(result) => {
                        if !result.should_enter {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "AI rejected entry for {} (confidence: {}%, reason: {})",
                                    token.symbol, result.confidence, result.reasoning
                                ),
                            );
                            return Ok(None); // AI rejected entry
                        } else {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "AI approved entry for {} (confidence: {}%, reason: {})",
                                    token.symbol, result.confidence, result.reasoning
                                ),
                            );
                        }
                    }
                    None => {
                        // AI analysis failed or is disabled, continue with strategy checks
                        crate::logger::debug(
                            crate::logger::LogTag::Trader,
                            &format!("AI entry analysis unavailable for {token_mint}"),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!("Token data not found for AI analysis: {token_mint}"),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for AI analysis: {e}"),
                );
            }
        }
    }

    // 7. Strategy evaluation - check configured entry strategies
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
