//! Notification service implementation for ServiceManager integration

use crate::notifications;
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Notification service for Telegram integration
pub struct NotificationService;

#[async_trait]
impl Service for NotificationService {
    fn name(&self) -> &'static str {
        "notifications"
    }

    fn priority(&self) -> i32 {
        150 // Start after core services but before trader
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No hard dependencies, can start early
    }

    fn is_enabled(&self) -> bool {
        // Only enable if initialization is complete
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Initialize the notification service singleton
        notifications::init_notification_service().await;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Start the notification workers (sender and command handler)
        let handles = notifications::start_notification_service(shutdown, monitor).await?;

        // Send bot started notification if enabled
        let mode = if crate::global::is_gui_mode() {
            "GUI"
        } else {
            "CLI"
        };
        let version = crate::version::VERSION.to_string();

        // Queue the startup notification
        notifications::queue_notification(notifications::Notification::bot_started(
            version,
            mode.to_string(),
        ));

        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        // Send bot stopped notification
        notifications::queue_notification(notifications::Notification::bot_stopped(
            "Graceful shutdown".to_string(),
        ));

        // Give a moment for the notification to be sent
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if notifications::is_notification_service_enabled().await {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded("Telegram notifications disabled in config".to_string())
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
