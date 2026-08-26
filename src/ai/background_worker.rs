//! AI Background Check Worker
//!
//! Periodically evaluates tokens in open positions and auto-blacklists
//! those that receive high-confidence reject decisions from the AI engine.

use crate::ai::engine::AiEngine;
use crate::ai::types::{EvaluationContext, Priority};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::state::POSITIONS;
use crate::tokens::cleanup::blacklist_token;
use crate::tokens::database::get_global_database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// Background loop that periodically evaluates tokens in open positions
///
/// This worker:
/// - Polls open positions at configured intervals
/// - Evaluates each token using the AI engine with LOW priority (uses cache)
/// - Auto-blacklists tokens with high-confidence reject decisions
/// - Respects rate limits with delays between evaluations
pub async fn background_check_loop(engine: Arc<AiEngine>, shutdown: Arc<Notify>) {
    logger::info(LogTag::System, "AI background check worker started");

    loop {
        // Get config values
        let (enabled, interval_secs, batch_size, auto_blacklist, min_confidence) =
            with_config(|cfg| {
                (
                    cfg.ai.enabled && cfg.ai.background_check_enabled,
                    cfg.ai.background_check_interval_seconds,
                    cfg.ai.background_batch_size as usize,
                    cfg.ai.auto_blacklist_enabled,
                    cfg.ai.auto_blacklist_min_confidence,
                )
            });

        if !enabled {
            // Wait and check again
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::System, "AI background check worker shutting down");
                    return;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
            continue;
        }

        // Get mints from open positions
        let mints: Vec<String> = {
            let positions = POSITIONS.read().await;
            positions
                .iter()
                .take(batch_size)
                .map(|p| p.mint.clone())
                .collect()
        };

        if !mints.is_empty() {
            logger::debug(
                LogTag::Filtering,
                &format!("AI background check: evaluating {} tokens", mints.len()),
            );

            for mint in mints {
                // Create evaluation context
                let context = EvaluationContext {
                    mint: mint.clone(),
                    ..Default::default()
                };

                // Evaluate with LOW priority (uses cache)
                match engine.evaluate_filter(context, Priority::Low).await {
                    Ok(result) => {
                        // Check if we should auto-blacklist
                        if auto_blacklist
                            && result.decision.decision == "reject"
                            && result.decision.confidence >= min_confidence
                        {
                            logger::warning(
                                LogTag::Filtering,
                                &format!(
                                    "AI auto-blacklisting token {} - confidence: {}%, reason: {}",
                                    mint, result.decision.confidence, result.decision.reasoning
                                ),
                            );

                            // Get database and blacklist the token
                            if let Some(db) = get_global_database() {
                                let blacklist_reason = format!(
                                    "AI auto-blacklist: {} ({}% confidence)",
                                    result
                                        .decision
                                        .reasoning
                                        .chars()
                                        .take(100)
                                        .collect::<String>(),
                                    result.decision.confidence
                                );

                                if let Err(e) =
                                    blacklist_token(&mint, &blacklist_reason, "auto_ai", &db)
                                {
                                    logger::error(
                                        LogTag::Filtering,
                                        &format!("Failed to blacklist token {mint}: {e}"),
                                    );
                                }
                            } else {
                                logger::error(
                                    LogTag::Filtering,
                                    "Cannot blacklist token: database not available",
                                );
                            }
                        }
                    }
                    Err(e) => {
                        logger::debug(
                            LogTag::Filtering,
                            &format!("AI background check failed for {mint}: {e}"),
                        );
                    }
                }

                // Small delay between evaluations to respect rate limits
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        // Wait for next interval
        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::System, "AI background check worker shutting down");
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(interval_secs)) => {}
        }
    }
}
