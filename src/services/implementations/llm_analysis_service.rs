//! LLM Analysis Service - thin wrapper for the model-scored background check worker
//!
//! Manages the lifecycle of the background check worker that periodically
//! evaluates held tokens and auto-blacklists those with high-confidence reject decisions.

use crate::config::with_config;
use crate::errors::ServiceError;
use crate::llm_analysis::engine::AnalysisEngine;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct LlmAnalysisService {
    analysis_engine: Option<Arc<AnalysisEngine>>,
}

impl Default for LlmAnalysisService {
    fn default() -> Self {
        Self {
            analysis_engine: None,
        }
    }
}

#[async_trait]
impl Service for LlmAnalysisService {
    fn name(&self) -> &'static str {
        "llm_analysis"
    }

    fn priority(&self) -> i32 {
        90 // Run after most services are ready
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["tokens", "positions", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
            && with_config(|cfg| cfg.llm.enabled && cfg.llm_analysis.background_check_enabled)
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        // Get the global analysis engine instance (should be initialized already)
        let engine = crate::llm_analysis::try_get_analysis_engine().ok_or_else(|| {
            crate::Error::Service(ServiceError::Initialize {
                service: self.name().to_string(),
                message: "analysis engine not initialized - call init_analysis_engine() first"
                    .to_owned(),
            })
        })?;
        self.analysis_engine = Some(engine);
        logger::info(LogTag::System, "LLM analysis service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let engine = self.analysis_engine.clone().ok_or_else(|| {
            crate::Error::Service(ServiceError::Start {
                service: self.name().to_string(),
                message: "analysis engine not initialized".to_owned(),
            })
        })?;

        // Spawn background check worker
        let handle = tokio::spawn(monitor.instrument(
            crate::llm_analysis::background_worker::background_check_loop(engine, shutdown),
        ));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::System, "LLM analysis service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if self.analysis_engine.is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
