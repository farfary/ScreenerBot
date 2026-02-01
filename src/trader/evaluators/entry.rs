//! Entry evaluation logic with integrated safety checks and AI analysis
//!
//! Evaluates whether an entry should be made for a token by checking:
//! 1. Connectivity health
//! 2. Position limits
//! 3. Existing position check
//! 4. Re-entry cooldown
//! 5. Blacklist status
//! 6. AI entry analysis (if enabled)
//! 7. Strategy signals

use crate::pools::PriceResult;
use crate::trader::types::TradeDecision;
use crate::trader::{ai_analysis, evaluators, safety};

/// Evaluate entry opportunity for a token
///
/// Performs all safety checks before strategy evaluation:
/// - Connectivity check (RPC, DexScreener, RugCheck must be healthy)
/// - Position limits (can't exceed max open positions)
/// - Existing position check (no duplicate entries)
/// - Re-entry cooldown (prevents immediate re-entry after exit)
/// - Blacklist check (token not blacklisted)
/// - AI entry analysis (if enabled, checks AI recommendation)
/// - Strategy evaluation (signals from configured strategies)
///
/// Returns:
/// - Ok(Some(TradeDecision)) if entry should be made
/// - Ok(None) if no entry signal or safety check failed
/// - Err(String) if evaluation failed due to connectivity or other errors
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    // Early exit: Force stop is active
    if crate::global::is_force_stopped() {
        return Ok(None);
    }

    // Early exit: Loss limit reached
    if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
        return Ok(None);
    }

    // 1. Connectivity check - critical endpoints must be healthy
    if let Some(unhealthy) =
        crate::connectivity::check_endpoints_healthy(&["rpc", "dexscreener", "rugcheck"]).await
    {
        return Err(format!("Unhealthy endpoints: {}", unhealthy));
    }

    // 2. Position limits - check if we can open more positions
    if !safety::check_position_limits().await? {
        return Ok(None); // Hit position limit
    }

    // 3. Existing position check - prevent duplicate entries
    if safety::has_open_position(token_mint).await? {
        return Ok(None); // Already have position
    }

    // 4. Re-entry cooldown - prevent immediate re-entry after exit
    if safety::is_in_reentry_cooldown(token_mint).await? {
        return Ok(None); // Still in cooldown
    }

    // 5. Blacklist check - sync check, no caching needed
    if safety::is_blacklisted(token_mint) {
        return Ok(None); // Token is blacklisted
    }

    // 6. AI entry analysis - check if AI recommends entry (if enabled)
    if ai_analysis::should_analyze_entry() {
        // Get token data for AI analysis
        match crate::tokens::get_full_token_async(token_mint).await {
            Ok(Some(token)) => {
                match ai_analysis::analyze_entry(&token).await {
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
                            &format!("AI entry analysis unavailable for {}", token_mint),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!("Token data not found for AI analysis: {}", token_mint),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for AI analysis: {}", e),
                );
            }
        }
    }

    // 7. Strategy evaluation - check configured entry strategies
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
