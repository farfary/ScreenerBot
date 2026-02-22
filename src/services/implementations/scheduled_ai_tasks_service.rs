//! Scheduled AI Tasks Service
//!
//! Thin Service trait wrapper for scheduled AI tasks.
//! Business logic in ai::scheduled_worker.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio_metrics::TaskMonitor;

pub struct ScheduledAiTasksService {
    tasks_completed: Arc<AtomicU64>,
    tasks_failed: Arc<AtomicU64>,
}

impl Default for ScheduledAiTasksService {
    fn default() -> Self {
        Self {
            tasks_completed: Arc::new(AtomicU64::new(0)),
            tasks_failed: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Service for ScheduledAiTasksService {
    fn name(&self) -> &'static str {
        "scheduled_ai_tasks"
    }

    fn priority(&self) -> i32 {
        92
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["webserver"]
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled)
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(LogTag::System, "Scheduled AI tasks service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let completed = Arc::clone(&self.tasks_completed);
        let failed = Arc::clone(&self.tasks_failed);

        let handle = tokio::spawn(monitor.instrument(
            crate::ai::scheduled_worker::scheduler_worker(shutdown, completed, failed),
        ));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::System, "Scheduled AI tasks service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if !with_config(|cfg| cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled) {
            return ServiceHealth::Degraded("Disabled in config".to_string());
        }
        ServiceHealth::Healthy
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics {
            operations_total: self.tasks_completed.load(Ordering::Relaxed),
            errors_total: self.tasks_failed.load(Ordering::Relaxed),
            ..Default::default()
        }
    }
}
