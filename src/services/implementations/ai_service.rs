//! AI Service - Background worker for AI-powered auto-blacklisting
//!
//! When enabled, periodically evaluates held tokens and auto-blacklists
//! those that receive high-confidence reject decisions.

use crate::ai::engine::AiEngine;
use crate::ai::types::{EvaluationContext, Priority};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::state::POSITIONS;
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::cleanup::blacklist_token;
use crate::tokens::database::get_global_database;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct AiService {
    ai_engine: Option<Arc<AiEngine>>,
}

impl Default for AiService {
    fn default() -> Self {
        Self { ai_engine: None }
    }
}

#[async_trait]
impl Service for AiService {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn priority(&self) -> i32 {
        90 // Run after most services are ready
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["tokens", "positions", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.ai.enabled && cfg.ai.background_check_enabled)
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Initialize AI engine if not already done
        let engine = AiEngine::new();
        self.ai_engine = Some(Arc::new(engine));
        logger::info(LogTag::System, "AI service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let engine = self.ai_engine.clone().ok_or("AI engine not initialized")?;

        // Spawn background check worker
        let handle = tokio::spawn(monitor.instrument(background_check_loop(engine, shutdown)));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(LogTag::System, "AI service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if self.ai_engine.is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}

/// Background loop that periodically evaluates tokens in open positions
async fn background_check_loop(engine: Arc<AiEngine>, shutdown: Arc<Notify>) {
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
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
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

                                if let Err(e) = blacklist_token(&mint, &blacklist_reason, &db) {
                                    logger::error(
                                        LogTag::Filtering,
                                        &format!("Failed to blacklist token {}: {}", mint, e),
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
                            &format!("AI background check failed for {}: {}", mint, e),
                        );
                    }
                }

                // Small delay between evaluations to respect rate limits
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        // Wait for next interval
        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::System, "AI background check worker shutting down");
                return;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {}
        }
    }
}
