mod health;
mod metrics;
pub mod implementations;

pub use health::ServiceHealth;
pub use metrics::{ ServiceMetrics, MetricsCollector };

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{ Notify, RwLock };
use tokio::task::JoinHandle;
use crate::logger::{ log, LogTag };
use std::collections::HashMap;
use std::sync::LazyLock;

/// Global ServiceManager instance for webserver and other components access
static GLOBAL_SERVICE_MANAGER: LazyLock<Arc<RwLock<Option<ServiceManager>>>> = LazyLock::new(||
    Arc::new(RwLock::new(None))
);

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

    /// Check if service is enabled
    fn is_enabled(&self) -> bool {
        // All services are enabled by default
        true
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
    metrics_collector: MetricsCollector,
    task_monitors: HashMap<&'static str, tokio_metrics::TaskMonitor>,
}

impl ServiceManager {
    pub async fn new() -> Result<Self, String> {
        Ok(Self {
            services: HashMap::new(),
            handles: HashMap::new(),
            shutdown: Arc::new(Notify::new()),
            metrics_collector: MetricsCollector::new(),
            task_monitors: HashMap::new(),
        })
    }

    /// Get TaskMonitor for a service (creates if doesn't exist)
    pub fn get_task_monitor(&mut self, service_name: &'static str) -> tokio_metrics::TaskMonitor {
        self.task_monitors
            .entry(service_name)
            .or_insert_with(|| tokio_metrics::TaskMonitor::new())
            .clone()
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
            .filter(|(_, service)| service.is_enabled())
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

                // Start monitoring with TaskMonitor
                let monitor = self.get_task_monitor(service_name);
                self.metrics_collector.start_monitoring(service_name, monitor);

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

                // Wait for handles with increased timeout for cleanup tasks
                if let Some(handles) = self.handles.remove(service_name) {
                    let timeout_duration = tokio::time::Duration::from_secs(10);
                    let handle_count = handles.len();

                    for (idx, handle) in handles.into_iter().enumerate() {
                        match tokio::time::timeout(timeout_duration, handle).await {
                            Ok(Ok(_)) => {
                                if handle_count > 1 {
                                    log(
                                        LogTag::System,
                                        "DEBUG",
                                        &format!(
                                            "Service {} task {}/{} stopped cleanly",
                                            service_name,
                                            idx + 1,
                                            handle_count
                                        )
                                    );
                                }
                            }
                            Ok(Err(e)) => {
                                log(
                                    LogTag::System,
                                    "WARN",
                                    &format!(
                                        "Service {} task {}/{} panicked: {:?}",
                                        service_name,
                                        idx + 1,
                                        handle_count,
                                        e
                                    )
                                );
                            }
                            Err(_) => {
                                log(
                                    LogTag::System,
                                    "WARN",
                                    &format!(
                                        "Service {} task {}/{} shutdown timed out after {}s",
                                        service_name,
                                        idx + 1,
                                        handle_count,
                                        timeout_duration.as_secs()
                                    )
                                );
                            }
                        }
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

    /// Get all registered service names
    pub fn get_all_service_names(&self) -> Vec<&'static str> {
        self.services.keys().copied().collect()
    }

    /// Get service by name
    pub fn get_service(&self, name: &str) -> Option<&Box<dyn Service>> {
        self.services.get(name)
    }

    /// Check if service is enabled
    pub fn is_service_enabled(&self, name: &str) -> bool {
        self.services
            .get(name)
            .map(|s| s.is_enabled())
            .unwrap_or(false)
    }
}

// =============================================================================
// Global ServiceManager Access Functions
// =============================================================================

/// Initialize global ServiceManager instance
pub async fn init_global_service_manager(manager: ServiceManager) {
    let mut global = GLOBAL_SERVICE_MANAGER.write().await;
    *global = Some(manager);
    log(LogTag::System, "INFO", "✅ Global ServiceManager initialized");
}

/// Get reference to global ServiceManager
pub async fn get_service_manager() -> Option<Arc<RwLock<Option<ServiceManager>>>> {
    let global = GLOBAL_SERVICE_MANAGER.read().await;
    if global.is_some() {
        Some(GLOBAL_SERVICE_MANAGER.clone())
    } else {
        None
    }
}
