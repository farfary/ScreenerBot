mod health;
pub mod implementations;
mod metrics;

pub use health::ServiceHealth;
pub use metrics::{MetricsCollector, ServiceMetrics};

use crate::logger::{self, LogTag};
use crate::startup;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Instant;
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

/// Global ServiceManager instance for webserver and other components access
static GLOBAL_SERVICE_MANAGER: LazyLock<Arc<RwLock<Option<ServiceManager>>>> =
  LazyLock::new(|| Arc::new(RwLock::new(None)));

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
    monitor: tokio_metrics::TaskMonitor,
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
  always
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
  always: bool,
) {
  if !should_log_service_details(always) {
    return;
  }

  let mut message = format!("service={} event={}", service_name, event.label());
  append_details(&mut message, details);

  // Map dynamic level string to logger methods
  match event.level() {
 "DEBUG"=> logger::debug(LogTag::System, &message),
 "SUCCESS"| "INFO"=> logger::info(LogTag::System, &message),
 "WARN"| "WARNING"=> logger::warning(LogTag::System, &message),
 "ERROR"=> logger::error(LogTag::System, &message),
    _ => logger::info(LogTag::System, &message),
  }
}

pub fn log_service_notice(service_name: &str, kind: &str, details: Option<&str>, always: bool) {
  if !should_log_service_details(always) {
    return;
  }

  let mut message = format!("service_notice service={} kind={}", service_name, kind);
  append_details(&mut message, details);

  logger::info(LogTag::System, &message);
}

pub fn log_service_startup_phase(phase: &str, details: Option<&str>) {
  let mut message = format!("service_startup phase={}", phase);
  append_details(&mut message, details);
  logger::info(LogTag::System, &message);
}

pub struct ServiceStartupFailure {
  pub name: &'static str,
  pub error: String,
}

pub struct ServiceStartupReport {
  pub attempted: Vec<&'static str>,
  pub started: Vec<&'static str>,
  pub failures: Vec<ServiceStartupFailure>,
  pub already_running: usize,
  pub total_enabled: usize,
  pub duration_ms: u128,
}

pub struct ServiceManager {
  services: HashMap<&'static str, Box<dyn Service>>,
  handles: HashMap<&'static str, Vec<JoinHandle<()>>>,
  shutdown: Arc<Notify>,
  metrics_collector: MetricsCollector,
  task_monitors: HashMap<&'static str, tokio_metrics::TaskMonitor>,
  // Cached health/metrics to avoid blocking during snapshot collection
  cached_health: Arc<RwLock<HashMap<&'static str, ServiceHealth>>>,
  cached_metrics: Arc<RwLock<HashMap<&'static str, ServiceMetrics>>>,
}

impl ServiceManager {
  pub async fn new() -> Result<Self, String> {
    Ok(Self {
      services: HashMap::new(),
      handles: HashMap::new(),
      shutdown: Arc::new(Notify::new()),
      metrics_collector: MetricsCollector::new(),
      task_monitors: HashMap::new(),
      cached_health: Arc::new(RwLock::new(HashMap::new())),
      cached_metrics: Arc::new(RwLock::new(HashMap::new())),
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

    // Filter enabled services with debug logging
    let mut enabled_services: Vec<&'static str> = Vec::new();
    for (name, service) in self.services.iter() {
      logger::debug(
        LogTag::System,
        &format!("Checking is_enabled() for service: {}", name),
      );
      if service.is_enabled() {
        enabled_services.push(*name);
      }
    }

    let debug_flag = "on"; // Debug system always logs when logger::debug is called
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

    if !enabled_services.is_empty() {
      let enabled_list = enabled_services.join(",");
      logger::debug(
        LogTag::System,
        &format!(
          "service_startup phase=enabled_list services=[{}]",
          enabled_list
        ),
      );
    }

    if !ordered.is_empty() {
      let ordered_list = ordered.join(",");
      logger::debug(
        LogTag::System,
        &format!(
          "service_startup phase=start_order services=[{}]",
          ordered_list
        ),
      );
    }

    let startup_timer = Instant::now();
    let mut last_service_time = startup_timer;

    // Initialize and start each service
    for &service_name in ordered.iter() {
      // Log gap between services if > 100ms
      let gap = last_service_time.elapsed().as_millis();
      if gap > 100 {
        logger::info(
          LogTag::System,
          &format!("Gap before '{}': {}ms", service_name, gap),
        );
      }

      let service_timer = Instant::now();

      // Get TaskMonitor FIRST (before any mutable borrow)
      let monitor = self.get_task_monitor(service_name);

      if let Some(service) = self.services.get_mut(service_name) {
        startup::mark_service_start(service_name);
        log_service_event(service_name, ServiceLogEvent::InitializeStart, None, false);
        let init_start = Instant::now();
        service.initialize().await?;
        let init_elapsed = init_start.elapsed().as_millis();

        log_service_event(
          service_name,
          ServiceLogEvent::InitializeSuccess,
          None,
          false,
        );

        log_service_event(service_name, ServiceLogEvent::StartStart, None, false);
        let start_timer = Instant::now();
        let handles = service
          .start(self.shutdown.clone(), monitor.clone())
          .await?;
        let start_elapsed = start_timer.elapsed().as_millis();
        let handle_count = handles.len();

        // Always log timing for services that take > 100ms
        let total_service_time = service_timer.elapsed().as_millis();
        if total_service_time > 100 {
          logger::info(
            LogTag::System,
            &format!(
 "Service '{}'took {}ms (init: {}ms, start: {}ms, handles: {})",
              service_name,
              total_service_time,
              init_elapsed,
              start_elapsed,
              handle_count
            ),
          );
        }

        let handle_detail = Some(format!("handles={}", handle_count));
        log_service_event(
          service_name,
          ServiceLogEvent::StartSuccess,
          handle_detail.as_deref(),
          false,
        );
        self.handles.insert(service_name, handles);
        startup::mark_service_ready(service_name);
      }

      // Update last service time for gap detection
      last_service_time = Instant::now();

      // Register monitor with metrics collector and start intervals() background task
      self.metrics_collector
        .start_monitoring(service_name, monitor, self.shutdown.clone())
        .await;
    }

    let elapsed = startup_timer.elapsed().as_millis();
    let completion = format!("started={} duration_ms={}", ordered.len(), elapsed);
    log_service_startup_phase("complete", Some(&completion));
    logger::info(
      LogTag::System,
      &format!("service_startup status=ready {}", completion),
    );
    Ok(())
  }

  /// Start services that are now enabled (after initialization)
  /// This method is idempotent - skips services that are already running
  /// Handles topological sorting and collects partial failures
  pub async fn start_newly_enabled(&mut self) -> Result<ServiceStartupReport, String> {
    logger::info(
      LogTag::System,
      "Starting newly enabled services after initialization",
    );

    // Get all enabled services
    let all_enabled: Vec<&'static str> = self
      .services
      .iter()
      .filter(|(_, service)| service.is_enabled())
      .map(|(name, _)| *name)
      .collect();

    // Filter out services that are already running
    let to_start: Vec<&'static str> = all_enabled
      .iter()
      .filter(|name| !self.handles.contains_key(**name))
      .copied()
      .collect();

    let already_running = all_enabled.len() - to_start.len();

    if to_start.is_empty() {
      logger::info(
        LogTag::System,
        "No new services to start - all enabled services are already running",
      );
      return Ok(ServiceStartupReport {
        attempted: Vec::new(),
        started: Vec::new(),
        failures: Vec::new(),
        already_running,
        total_enabled: all_enabled.len(),
        duration_ms: 0,
      });
    }
    logger::info(
      LogTag::System,
      &format!(
        "Starting {} new services (skipping {} already running)",
        to_start.len(),
        already_running
      ),
    );

    // Resolve dependencies and order by priority (includes dependency checking)
    let ordered = match self.resolve_startup_order(&to_start) {
      Ok(order) => order,
      Err(e) => {
        logger::error(
          LogTag::System,
          &format!("Failed to resolve service startup order: {}", e),
        );
        return Err(e);
      }
    };

    logger::info(
      LogTag::System,
      &format!("Service startup order: {}", ordered.join(", ")),
    );

    let startup_timer = Instant::now();
    let mut successful_starts = Vec::new();
    let mut failed_starts: Vec<ServiceStartupFailure> = Vec::new();

    // Initialize and start each service
    for &service_name in ordered.iter() {
      let service_timer = Instant::now();

      // Get TaskMonitor FIRST (before any mutable borrow)
      let monitor = self.get_task_monitor(service_name);

      if let Some(service) = self.services.get_mut(service_name) {
        startup::mark_service_start(service_name);
        // Initialize
        log_service_event(service_name, ServiceLogEvent::InitializeStart, None, true);
        match service.initialize().await {
          Ok(_) => {
            log_service_event(
              service_name,
              ServiceLogEvent::InitializeSuccess,
              None,
              true,
            );
          }
          Err(e) => {
            logger::error(
              LogTag::System,
              &format!("Failed to initialize service '{}': {}", service_name, e),
            );
            failed_starts.push(ServiceStartupFailure {
              name: service_name,
              error: format!("Initialize failed: {}", e),
            });
            continue; // Skip starting this service
          }
        }

        // Start
        log_service_event(service_name, ServiceLogEvent::StartStart, None, true);
        match service.start(self.shutdown.clone(), monitor.clone()).await {
          Ok(handles) => {
            let handle_count = handles.len();
            let elapsed = service_timer.elapsed().as_millis();

            logger::info(
              LogTag::System,
              &format!(
 "Service '{}'started successfully ({}ms, {} handles)",
                service_name, elapsed, handle_count
              ),
            );

            log_service_event(
              service_name,
              ServiceLogEvent::StartSuccess,
              Some(&format!("handles={}", handle_count)),
              true,
            );

            self.handles.insert(service_name, handles);
            successful_starts.push(service_name);
            startup::mark_service_ready(service_name);

            // Register monitor with metrics collector
            self.metrics_collector
              .start_monitoring(service_name, monitor, self.shutdown.clone())
              .await;
          }
          Err(e) => {
            logger::error(
              LogTag::System,
              &format!("Failed to start service '{}': {}", service_name, e),
            );
            failed_starts.push(ServiceStartupFailure {
              name: service_name,
              error: format!("Start failed: {}", e),
            });
          }
        }
      }
    }

    let total_elapsed = startup_timer.elapsed().as_millis();

    if failed_starts.is_empty() {
      logger::info(
        LogTag::System,
        &format!(
          "All {} newly enabled services started successfully (duration_ms={})",
          successful_starts.len(),
          total_elapsed
        ),
      );
    } else {
      logger::warning(
        LogTag::System,
        &format!(
          "Service startup completed with partial failures: successful={} failed={} duration_ms={}",
          successful_starts.len(),
          failed_starts.len(),
          total_elapsed
        ),
      );

      for failure in &failed_starts {
        logger::error(
          LogTag::System,
 &format!("{}: {}", failure.name, failure.error),
        );
      }
    }

    Ok(ServiceStartupReport {
      attempted: ordered,
      started: successful_starts,
      failures: failed_starts,
      already_running,
      total_enabled: all_enabled.len(),
      duration_ms: total_elapsed,
    })
  }

  /// Check if services can be started (initialization complete)
  pub fn can_start_services(&self) -> bool {
    crate::global::is_initialization_complete()
  }

  /// Stop all services in reverse priority order
  pub async fn stop_all(&mut self) -> Result<(), String> {
    let running_services: Vec<&'static str> = self.handles.keys().copied().collect();
    let shutdown_begin = format!("running={} debug_system=on", running_services.len());
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
          logger::warning(
            LogTag::System,
            &format!("Service stop error for {}: {}", service_name, e),
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
                  logger::debug(
                    LogTag::System,
                    &format!(
                      "Service {} task {}/{} stopped cleanly",
                      service_name,
                      idx + 1,
                      handle_count
                    ),
                  );
                }
              }
              Ok(Err(e)) => {
                logger::warning(
                  LogTag::System,
                  &format!(
                    "Service {} task {}/{} panicked: {:?}",
                    service_name,
                    idx + 1,
                    handle_count,
                    e
                  ),
                );
              }
              Err(_) => {
                logger::warning(
                  LogTag::System,
                  &format!(
                    "Service {} task {}/{} shutdown timed out after {}s",
                    service_name,
                    idx + 1,
                    handle_count,
                    timeout_duration.as_secs()
                  ),
                );
              }
            }
          }
        }

        log_service_event(service_name, ServiceLogEvent::StopSuccess, None, false);
      }
    }

    logger::info(
      LogTag::System,
      &format!(
        "service_shutdown status=complete services_stopped={}",
        running_services.len()
      ),
    );
    Ok(())
  }

  /// Resolve service startup order
  fn resolve_startup_order(
    &self,
    services: &[&'static str],
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
      visiting: &mut HashSet<&'static str>,
    ) -> Result<(), String> {
      if visited.contains(name) {
        return Ok(());
      }

      if visiting.contains(name) {
        return Err(format!(
          "Circular dependency detected for service: {}",
          name
        ));
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
      visit(
        service_name,
        &self.services,
        &mut ordered,
        &mut visited,
        &mut visiting,
      )?;
    }

    // Sort by priority
    ordered.sort_by_key(|name| self.services.get(name).map(|s| s.priority()).unwrap_or(100));

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

  /// Get cached health status (non-blocking, instant read)
  pub async fn get_health_cached(&self) -> HashMap<&'static str, ServiceHealth> {
    self.cached_health.read().await.clone()
  }

  /// Get cached metrics (non-blocking, instant read)
  pub async fn get_metrics_cached(&self) -> HashMap<&'static str, ServiceMetrics> {
    self.cached_metrics.read().await.clone()
  }

  /// Update cached health and metrics (called periodically by background task)
  pub async fn update_cache(&self) {
    logger::debug(LogTag::System, "ServiceManager: Starting cache update");

    // Collect fresh health
    let health = self.get_health().await;
    *self.cached_health.write().await = health;

    // Collect fresh metrics
    let metrics = self.get_metrics().await;

    let sample_service = metrics.iter().next();
    if let Some((name, m)) = sample_service {
      logger::debug(
        LogTag::System,
        &format!(
 "ServiceManager: Cache updated - sample service '{}'uptime={}s",
          name, m.uptime_seconds
        ),
      );
    }

    *self.cached_metrics.write().await = metrics;
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

  /// Get the names of services that currently have running handles
  pub fn get_running_service_names(&self) -> Vec<&'static str> {
    self.handles.keys().copied().collect()
  }

  /// Get the number of services that currently have running handles
  pub fn get_running_service_count(&self) -> usize {
    self.handles.len()
  }
}

// =============================================================================
// Global ServiceManager Access Functions
// =============================================================================

/// Initialize global ServiceManager instance
pub async fn init_global_service_manager(manager: ServiceManager) {
  // Do initial cache update
  manager.update_cache().await;

  let mut global = GLOBAL_SERVICE_MANAGER.write().await;
  *global = Some(manager);
 logger::info(LogTag::System, "Global ServiceManager initialized");

  // Spawn background task to update cache every 5 seconds
  // (most services are idle, so less frequent updates reduce CPU overhead)
  tokio::spawn(async {
    loop {
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

      if let Some(manager_ref) = get_service_manager().await {
        if let Some(manager) = manager_ref.read().await.as_ref() {
          // Add timeout to prevent hanging forever
          let update_future = manager.update_cache();
          match tokio::time::timeout(tokio::time::Duration::from_secs(3), update_future)
            .await
          {
            Ok(_) => {
              // Update completed successfully
              logger::debug(LogTag::System, "ServiceManager: Cache update completed");
            }
            Err(_) => {
              logger::warning(LogTag::System, "ServiceManager: Cache update timed out after 3s - continuing with stale cache");
            }
          }
        }
      }
    }
  });
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
