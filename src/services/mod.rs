mod health;
pub mod implementations;
mod metrics;

pub use health::ServiceHealth;
pub use metrics::{ MetricsCollector, ServiceMetrics };

use crate::arguments::is_debug_system_enabled;
use crate::logger::{ log, LogTag };
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Instant;
use tokio::sync::{ Notify, RwLock };
use tokio::task::JoinHandle;

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

    /// Start the service with TaskMonitor for metrics instrumentation
    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String>;

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

#[derive(Copy, Clone)]
pub enum ServiceLogEvent {
    InitializeStart,
    InitializeSuccess,
    StartStart,
    StartSuccess,
    StopStart,
    StopSuccess,
}

impl ServiceLogEvent {
    fn label(&self) -> &'static str {
        match self {
            ServiceLogEvent::InitializeStart => "init.begin",
            ServiceLogEvent::InitializeSuccess => "init.complete",
            ServiceLogEvent::StartStart => "start.begin",
            ServiceLogEvent::StartSuccess => "start.complete",
            ServiceLogEvent::StopStart => "stop.begin",
            ServiceLogEvent::StopSuccess => "stop.complete",
        }
    }

    fn level(&self) -> &'static str {
        match self {
            ServiceLogEvent::InitializeSuccess | ServiceLogEvent::StartSuccess => "SUCCESS",
            ServiceLogEvent::StopSuccess => "SUCCESS",
            _ => "INFO",
        }
    }
}

fn should_log_service_details(always: bool) -> bool {
    always || is_debug_system_enabled()
}

fn append_details(message: &mut String, details: Option<&str>) {
    if let Some(extra) = details {
        let trimmed = extra.trim();
        if !trimmed.is_empty() {
            message.push(' ');
            message.push_str(trimmed);
        }
    }
}

pub fn log_service_event(
    service_name: &str,
    event: ServiceLogEvent,
    details: Option<&str>,
    always: bool
) {
    if !should_log_service_details(always) {
        return;
    }

    let mut message = format!("service={} event={}", service_name, event.label());
    append_details(&mut message, details);

    log(LogTag::System, event.level(), &message);
}

pub fn log_service_notice(service_name: &str, kind: &str, details: Option<&str>, always: bool) {
    if !should_log_service_details(always) {
        return;
    }

    let mut message = format!("service_notice service={} kind={}", service_name, kind);
    append_details(&mut message, details);

    log(LogTag::System, "INFO", &message);
}

pub fn log_service_startup_phase(phase: &str, details: Option<&str>) {
    let mut message = format!("service_startup phase={}", phase);
    append_details(&mut message, details);

    log(LogTag::System, "INFO", &message);
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
        let total_registered = self.services.len();

        // Filter enabled services
        let enabled_services: Vec<&'static str> = self.services
            .iter()
            .filter(|(_, service)| service.is_enabled())
            .map(|(name, _)| *name)
            .collect();

        let debug_flag = if is_debug_system_enabled() { "on" } else { "off" };
        let disabled = total_registered.saturating_sub(enabled_services.len());
        let begin_details = format!(
            "registered={} enabled={} disabled={} debug_system={}",
            total_registered,
            enabled_services.len(),
            disabled,
            debug_flag
        );
        log_service_startup_phase("begin", Some(&begin_details));

        // Resolve dependencies and order by priority
        let ordered = self.resolve_startup_order(&enabled_services)?;

        if is_debug_system_enabled() {
            if !enabled_services.is_empty() {
                let enabled_list = enabled_services.join(",");
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("service_startup phase=enabled_list services=[{}]", enabled_list)
                );
            }

            if !ordered.is_empty() {
                let ordered_list = ordered.join(",");
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("service_startup phase=start_order services=[{}]", ordered_list)
                );
            }
        }

        let startup_timer = Instant::now();

        // Initialize and start each service
        for &service_name in ordered.iter() {
            // Get TaskMonitor FIRST (before any mutable borrow)
            let monitor = self.get_task_monitor(service_name);

            if let Some(service) = self.services.get_mut(service_name) {
                log_service_event(service_name, ServiceLogEvent::InitializeStart, None, false);
                service.initialize().await?;

                log_service_event(service_name, ServiceLogEvent::InitializeSuccess, None, false);
                log_service_event(service_name, ServiceLogEvent::StartStart, None, false);
                let handles = service.start(self.shutdown.clone(), monitor.clone()).await?;
                let handle_count = handles.len();
                let handle_detail = if is_debug_system_enabled() {
                    Some(format!("handles={}", handle_count))
                } else {
                    None
                };
                log_service_event(
                    service_name,
                    ServiceLogEvent::StartSuccess,
                    handle_detail.as_deref(),
                    false
                );
                self.handles.insert(service_name, handles);
            }

            // Register monitor with metrics collector and start intervals() background task
            self.metrics_collector.start_monitoring(
                service_name,
                monitor,
                self.shutdown.clone()
            ).await;
        }

        let elapsed = startup_timer.elapsed().as_millis();
        let completion = format!("started={} duration_ms={}", ordered.len(), elapsed);
        log_service_startup_phase("complete", Some(&completion));
        log(LogTag::System, "SUCCESS", &format!("service_startup status=ready {}", completion));
        Ok(())
    }

    /// Stop all services in reverse priority order
    pub async fn stop_all(&mut self) -> Result<(), String> {
        let running_services: Vec<&'static str> = self.handles.keys().copied().collect();
        let shutdown_begin = format!("running={} debug_system={}", running_services.len(), if
            is_debug_system_enabled()
        {
            "on"
        } else {
            "off"
        });
        log_service_startup_phase("shutdown_begin", Some(&shutdown_begin));

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Get services in reverse startup order
        let mut ordered = self.resolve_startup_order(&running_services)?;
        ordered.reverse();

        // Stop each service
        for &service_name in ordered.iter() {
            if let Some(service) = self.services.get_mut(service_name) {
                log_service_event(service_name, ServiceLogEvent::StopStart, None, false);

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

                log_service_event(service_name, ServiceLogEvent::StopSuccess, None, false);
            }
        }

        log(
            LogTag::System,
            "SUCCESS",
            &format!("service_shutdown status=complete services_stopped={}", running_services.len())
        );
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
        ordered.sort_by_key(|name|
            self.services
                .get(name)
                .map(|s| s.priority())
                .unwrap_or(100)
        );

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

    /// Get metrics (optimized - single system refresh for all services, no &mut needed)
    pub async fn get_metrics(&self) -> HashMap<&'static str, ServiceMetrics> {
        // Get service names
        let service_names: Vec<&'static str> = self.services.keys().copied().collect();

        // Collect base metrics efficiently (single refresh)
        let mut metrics = self.metrics_collector.collect_all(&service_names).await;

        // Merge service-specific metrics from each service's metrics() method
        for (name, service) in &self.services {
            if let Some(base_metrics) = metrics.get_mut(name) {
                let service_specific = service.metrics().await;

                // Merge service-specific operational metrics
                base_metrics.operations_total = service_specific.operations_total;
                base_metrics.operations_per_second = service_specific.operations_per_second;
                base_metrics.errors_total = service_specific.errors_total;

                // Merge custom metrics
                for (key, value) in service_specific.custom_metrics {
                    base_metrics.custom_metrics.insert(key, value);
                }

                base_metrics.sanitize();
            }
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
