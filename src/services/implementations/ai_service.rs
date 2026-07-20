//! AI Service - Thin wrapper for AI background check worker
//!
//! Manages the lifecycle of the AI background check worker that periodically
//! evaluates held tokens and auto-blacklists those with high-confidence reject decisions.

use crate::ai::engine::AiEngine;
use crate::config::with_config;
use crate::errors::ServiceError;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
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
        crate::global::is_initialization_complete()
            && with_config(|cfg| cfg.ai.enabled && cfg.ai.background_check_enabled)
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        // Get global AI engine instance (should be initialized already)
        let engine = crate::ai::try_get_ai_engine().ok_or_else(|| {
            crate::Error::Service(ServiceError::Initialize {
                service: self.name().to_string(),
                message: "AI engine not initialized - call init_ai_engine() first".to_owned(),
            })
        })?;
        self.ai_engine = Some(engine);
        logger::info(LogTag::System, "AI service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let engine = self.ai_engine.clone().ok_or_else(|| {
            crate::Error::Service(ServiceError::Start {
                service: self.name().to_string(),
                message: "AI engine not initialized".to_owned(),
            })
        })?;

        // Spawn background check worker
        let handle = tokio::spawn(monitor.instrument(
            crate::ai::background_worker::background_check_loop(engine, shutdown),
        ));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
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
