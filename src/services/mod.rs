mod health;
mod metrics;
pub mod implementations;

pub use health::ServiceHealth;
pub use metrics::{ ServiceMetrics, MetricsCollector };

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::configs::Configs;
use crate::logger::{ log, LogTag };
use std::collections::HashMap;

/// Core service trait that all services must implement
#[async_trait]
pub trait Service: Send + Sync {
    /// Unique service identifier
    fn name(&self) -> &'static str;

    /// Service priority (lower = starts earlier, stops later)
    fn priority(&self) -> i32 {
        100
    }

    /// Services this service depends on
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Check if service is enabled in configuration
    fn is_enabled(&self, config: &Configs) -> bool {
        // Default: check if service is in config and enabled
        config.services.services
            .get(self.name())
            .map(|s| s.enabled)
            .unwrap_or(true)
    }

    /// Initialize the service
    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Start the service
    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String>;

    /// Stop the service
    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Check service health
    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    /// Get service metrics
    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}

pub struct ServiceManager {
    services: HashMap<&'static str, Box<dyn Service>>,
    handles: HashMap<&'static str, Vec<JoinHandle<()>>>,
    shutdown: Arc<Notify>,
    config: Configs,
    metrics_collector: MetricsCollector,
}

impl ServiceManager {
    pub async fn new(config: Configs) -> Result<Self, String> {
        Ok(Self {
            services: HashMap::new(),
            handles: HashMap::new(),
            shutdown: Arc::new(Notify::new()),
            config,
            metrics_collector: MetricsCollector::new(),
        })
    }

    /// Register a service
    pub fn register(&mut self, service: Box<dyn Service>) {
        let name = service.name();
        self.services.insert(name, service);
    }

    /// Start all enabled services in dependency and priority order
    pub async fn start_all(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Starting all services...");

        // Filter enabled services
        let enabled_services: Vec<&'static str> = self.services
            .iter()
            .filter(|(_, service)| service.is_enabled(&self.config))
            .map(|(name, _)| *name)
            .collect();

        log(LogTag::System, "INFO", &format!("Enabled services: {:?}", enabled_services));

        // Resolve dependencies and order by priority
        let ordered = self.resolve_startup_order(&enabled_services)?;

        log(LogTag::System, "INFO", &format!("Service startup order: {:?}", ordered));

        // Initialize and start each service
        for service_name in ordered {
            if let Some(service) = self.services.get_mut(service_name) {
                log(LogTag::System, "INFO", &format!("Initializing service: {}", service_name));
                service.initialize().await?;

                log(LogTag::System, "INFO", &format!("Starting service: {}", service_name));
                let handles = service.start(self.shutdown.clone()).await?;
                self.handles.insert(service_name, handles);

                // Start monitoring
                self.metrics_collector.start_monitoring(service_name);

                log(LogTag::System, "SUCCESS", &format!("✅ Service started: {}", service_name));
            }
        }

        log(LogTag::System, "SUCCESS", "✅ All services started successfully");
        Ok(())
    }

    /// Stop all services in reverse priority order
    pub async fn stop_all(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Stopping all services...");

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Get services in reverse startup order
        let running_services: Vec<&'static str> = self.handles.keys().copied().collect();
        let mut ordered = self.resolve_startup_order(&running_services)?;
        ordered.reverse();

        // Stop each service
        for service_name in ordered {
            if let Some(service) = self.services.get_mut(service_name) {
                log(LogTag::System, "INFO", &format!("Stopping service: {}", service_name));

                if let Err(e) = service.stop().await {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Service stop error for {}: {}", service_name, e)
                    );
                }

                // Wait for handles
                if let Some(handles) = self.handles.remove(service_name) {
                    for handle in handles {
                        let _ = tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            handle
                        ).await;
                    }
                }

                log(LogTag::System, "SUCCESS", &format!("✅ Service stopped: {}", service_name));
            }
        }

        log(LogTag::System, "SUCCESS", "✅ All services stopped successfully");
        Ok(())
    }

    /// Resolve service startup order
    fn resolve_startup_order(
        &self,
        services: &[&'static str]
    ) -> Result<Vec<&'static str>, String> {
        use std::collections::HashSet;

        let mut ordered = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit<'a>(
            name: &'static str,
            services: &'a HashMap<&'static str, Box<dyn Service>>,
            ordered: &mut Vec<&'static str>,
            visited: &mut HashSet<&'static str>,
            visiting: &mut HashSet<&'static str>
        ) -> Result<(), String> {
            if visited.contains(name) {
                return Ok(());
            }

            if visiting.contains(name) {
                return Err(format!("Circular dependency detected for service: {}", name));
            }

            visiting.insert(name);

            if let Some(service) = services.get(name) {
                for dep in service.dependencies() {
                    visit(dep, services, ordered, visited, visiting)?;
                }
            }

            visiting.remove(name);
            visited.insert(name);
            ordered.push(name);

            Ok(())
        }

        for &service_name in services {
            visit(service_name, &self.services, &mut ordered, &mut visited, &mut visiting)?;
        }

        // Sort by priority
        ordered.sort_by_key(|name| {
            self.services
                .get(name)
                .map(|s| s.priority())
                .unwrap_or(100)
        });

        Ok(ordered)
    }

    /// Get health status
    pub async fn get_health(&self) -> HashMap<&'static str, ServiceHealth> {
        let mut health = HashMap::new();
        for (name, service) in &self.services {
            health.insert(*name, service.health().await);
        }
        health
    }

    /// Get metrics
    pub async fn get_metrics(&mut self) -> HashMap<&'static str, ServiceMetrics> {
        let mut metrics = HashMap::new();
        for (name, _service) in &self.services {
            let service_metrics = self.metrics_collector.collect_for_service(name).await;
            metrics.insert(*name, service_metrics);
        }
        metrics
    }
}
