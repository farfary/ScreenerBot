// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Service lifecycle management — orchestrates all bot subsystems.
mod global;
mod health;
pub mod implementations;
#[cfg(test)]
mod label_tests;
mod logging;
mod metrics;
pub mod startup;
#[cfg(test)]
mod startup_order_tests;
#[cfg(test)]
mod stop_service_tests;

pub use global::*;
pub use health::ServiceHealth;
pub use logging::*;
pub use metrics::{MetricsCollector, ServiceMetrics};

use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

/// Core service trait that all services must implement
#[async_trait]
pub trait Service: Send + Sync {
    /// Unique service identifier
    fn name(&self) -> &'static str;

    /// Whether a failure to initialize or start this service stops startup. Any other
    /// service that fails is reported, left down with its dependents, and startup
    /// continues without it.
    fn essential(&self) -> bool {
        false
    }

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
    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Start the service with TaskMonitor for metrics instrumentation
    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>>;

    /// Stop the service
    async fn stop(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Whether [`ServiceManager::stop_service`] may stop this service while the rest keep
    /// running. Only a service whose `stop` ends its own tasks qualifies: the shared
    /// shutdown signal ends every service, so it is never sent to stop one, and any task
    /// still running after the stop grace period is aborted.
    fn stops_alone(&self) -> bool {
        false
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
    pub(crate) fn label(&self) -> &'static str {
        match self {
            ServiceLogEvent::InitializeStart => "init.begin",
            ServiceLogEvent::InitializeSuccess => "init.complete",
            ServiceLogEvent::StartStart => "start.begin",
            ServiceLogEvent::StartSuccess => "start.complete",
            ServiceLogEvent::StopStart => "stop.begin",
            ServiceLogEvent::StopSuccess => "stop.complete",
        }
    }

    pub(crate) fn level(&self) -> &'static str {
        match self {
            ServiceLogEvent::InitializeSuccess | ServiceLogEvent::StartSuccess => "SUCCESS",
            ServiceLogEvent::StopSuccess => "SUCCESS",
            _ => "INFO",
        }
    }
}

/// Log a service that failed to `phase` with its whole cause chain, so the log names
/// the service and the failure that started it.
fn log_start_failure(service: &str, phase: &str, error: &crate::Error) {
    logger::error(
        LogTag::System,
        &format!(
            "Service '{service}' failed to {phase}: {}",
            crate::errors::source_chain(error).join(": ")
        ),
    );
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

/// How long [`ServiceManager::stop_service`] waits for each task of the stopped service
/// before aborting it.
const STOP_ALONE_GRACE: Duration = Duration::from_secs(5);

pub struct ServiceManager {
    services: HashMap<&'static str, Box<dyn Service>>,
    handles: HashMap<&'static str, Vec<JoinHandle<()>>>,
    shutdown: Arc<Notify>,
    metrics_collector: MetricsCollector,
    task_monitors: HashMap<&'static str, tokio_metrics::TaskMonitor>,
    /// Why each service that failed to start, or was not started because a dependency
    /// failed, is down; reported as its health until it starts.
    start_failures: HashMap<&'static str, UiText>,
    // Cached health/metrics to avoid blocking during snapshot collection
    cached_health: Arc<RwLock<HashMap<&'static str, ServiceHealth>>>,
    cached_metrics: Arc<RwLock<HashMap<&'static str, ServiceMetrics>>>,
}

impl ServiceManager {
    pub async fn new() -> crate::Result<Self> {
        Ok(Self {
            services: HashMap::new(),
            handles: HashMap::new(),
            shutdown: Arc::new(Notify::new()),
            metrics_collector: MetricsCollector::new(),
            task_monitors: HashMap::new(),
            start_failures: HashMap::new(),
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

    /// Number of services registered with this manager.
    pub fn registered_count(&self) -> usize {
        self.services.len()
    }

    /// Start all enabled services in dependency and priority order.
    ///
    /// A service that fails to initialize or start is reported, its dependents are not
    /// attempted, and startup continues with the rest. Only the failure of an
    /// [`essential`](Service::essential) service stops startup and is returned.
    pub async fn start_all(&mut self) -> crate::Result<ServiceStartupReport> {
        let total_registered = self.services.len();

        // Filter enabled services with debug logging
        let mut enabled_services: Vec<&'static str> = Vec::new();
        for (name, service) in self.services.iter() {
            logger::debug(
                LogTag::System,
                &format!("Checking is_enabled() for service: {name}"),
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

        // Resolve the dependency-first startup order
        let mut ordered = self.resolve_startup_order(&enabled_services)?;

        // resolve_startup_order pulls in declared dependencies transitively, even
        // ones that are disabled. Dependency declarations only define ordering, not
        // a hard requirement to run (see validate_dependencies: warn, don't fail).
        // Retain only enabled services so a disabled dependency (e.g. RPC-bound
        // pools/transactions in Explore Mode) is never started. No-op when
        // every service is enabled (normal mode).
        ordered.retain(|name| {
            self.services
                .get(name)
                .map(|s| s.is_enabled())
                .unwrap_or(false)
        });

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
        let (started, failures) = self.start_ordered(&ordered, false).await?;

        let elapsed = startup_timer.elapsed().as_millis();
        let completion = format!(
            "started={} failed={} duration_ms={}",
            started.len(),
            failures.len(),
            elapsed
        );
        log_service_startup_phase("complete", Some(&completion));
        if failures.is_empty() {
            logger::info(
                LogTag::System,
                &format!("service_startup status=ready {completion}"),
            );
        } else {
            logger::warning(
                LogTag::System,
                &format!("service_startup status=partial {completion}"),
            );
        }

        Ok(ServiceStartupReport {
            attempted: ordered,
            started,
            failures,
            already_running: 0,
            total_enabled: enabled_services.len(),
            duration_ms: elapsed,
        })
    }

    /// Start services that are now enabled (after initialization)
    /// This method is idempotent - skips services that are already running
    /// Handles topological sorting and collects partial failures
    pub async fn start_newly_enabled(&mut self) -> crate::Result<ServiceStartupReport> {
        logger::info(
            LogTag::System,
            "Starting newly enabled services after initialization",
        );
        let candidates: Vec<&'static str> = self.services.keys().copied().collect();
        self.start_enabled_among(&candidates).await
    }

    /// Start one service whose config has just enabled it, with any enabled dependency
    /// that is not running yet. A disabled or already running service is left as it is.
    pub async fn start_enabled_service(
        &mut self,
        name: &'static str,
    ) -> crate::Result<ServiceStartupReport> {
        logger::info(
            LogTag::System,
            &format!("Starting service '{name}' after its config enabled it"),
        );
        self.start_enabled_among(&[name]).await
    }

    /// Start every enabled, not yet running service of `candidates` in dependency order.
    async fn start_enabled_among(
        &mut self,
        candidates: &[&'static str],
    ) -> crate::Result<ServiceStartupReport> {
        let all_enabled: Vec<&'static str> = candidates
            .iter()
            .copied()
            .filter(|name| self.is_service_enabled(name))
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

        // Resolve the dependency-first startup order (includes dependency checking)
        let mut ordered = match self.resolve_startup_order(&to_start) {
            Ok(order) => order,
            Err(e) => {
                logger::error(
                    LogTag::System,
                    &format!("Failed to resolve service startup order: {e}"),
                );
                return Err(e);
            }
        };

        // Drop transitively-pulled-in dependencies that are disabled (e.g.
        // RPC-bound services in Explore Mode). Also drop anything already
        // running so we never re-init it. See start_all for the rationale.
        ordered.retain(|name| {
            self.services
                .get(name)
                .map(|s| s.is_enabled())
                .unwrap_or(false)
                && !self.handles.contains_key(name)
        });

        logger::info(
            LogTag::System,
            &format!("Service startup order: {}", ordered.join(", ")),
        );

        let startup_timer = Instant::now();
        let (successful_starts, failed_starts) = self.start_ordered(&ordered, true).await?;
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

    /// Initialize and start every service of `ordered`, which is in startup order.
    ///
    /// A service one of whose dependencies failed to start is not attempted. A failed
    /// essential service stops the pass and its error is returned; every other failure
    /// is logged with its cause, recorded as the service's health and returned in the
    /// list of failures. `post_init` marks a pass that runs after initialization in the
    /// service log.
    async fn start_ordered(
        &mut self,
        ordered: &[&'static str],
        post_init: bool,
    ) -> crate::Result<(Vec<&'static str>, Vec<ServiceStartupFailure>)> {
        let mut started = Vec::new();
        let mut failures: Vec<ServiceStartupFailure> = Vec::new();
        let mut last_service_time = Instant::now();

        for &service_name in ordered {
            // Log gap between services if > 100ms
            let gap = last_service_time.elapsed().as_millis();
            if gap > 100 {
                logger::info(
                    LogTag::System,
                    &format!("Gap before '{service_name}': {gap}ms"),
                );
            }
            let service_timer = Instant::now();

            // Get TaskMonitor FIRST (before any mutable borrow)
            let monitor = self.get_task_monitor(service_name);

            let Some(service) = self.services.get_mut(service_name) else {
                continue;
            };

            if let Some(dependency) = service
                .dependencies()
                .into_iter()
                .find(|dependency| self.start_failures.contains_key(dependency))
            {
                logger::warning(
                    LogTag::System,
                    &format!(
                        "Service '{service_name}' not started: its dependency '{dependency}' failed to start"
                    ),
                );
                self.start_failures.insert(
                    service_name,
                    UiText::new(ids::SERVICES_HEALTH_DEPENDENCY_FAILED)
                        .arg("dependency", UiArg::Text(dependency.to_owned())),
                );
                failures.push(ServiceStartupFailure {
                    name: service_name,
                    error: format!("Dependency '{dependency}' failed to start"),
                });
                last_service_time = Instant::now();
                continue;
            }

            startup::mark_service_start(service_name);
            log_service_event(
                service_name,
                ServiceLogEvent::InitializeStart,
                None,
                post_init,
            );
            let init_start = Instant::now();
            let outcome = match service.initialize().await {
                Ok(()) => {
                    log_service_event(
                        service_name,
                        ServiceLogEvent::InitializeSuccess,
                        None,
                        post_init,
                    );
                    log_service_event(service_name, ServiceLogEvent::StartStart, None, post_init);
                    let init_elapsed = init_start.elapsed().as_millis();
                    let start_timer = Instant::now();
                    service
                        .start(self.shutdown.clone(), monitor.clone())
                        .await
                        .map(|handles| (handles, init_elapsed, start_timer.elapsed().as_millis()))
                        .map_err(|error| ("start", error))
                }
                Err(error) => Err(("initialize", error)),
            };

            match outcome {
                Ok((handles, init_elapsed, start_elapsed)) => {
                    let handle_count = handles.len();
                    let total_service_time = service_timer.elapsed().as_millis();
                    if total_service_time > 100 {
                        logger::info(
                            LogTag::System,
                            &format!(
                                "Service '{service_name}' took {total_service_time}ms (init: {init_elapsed}ms, start: {start_elapsed}ms, handles: {handle_count})"
                            ),
                        );
                    }
                    log_service_event(
                        service_name,
                        ServiceLogEvent::StartSuccess,
                        Some(&format!("handles={handle_count}")),
                        post_init,
                    );
                    self.handles.insert(service_name, handles);
                    self.start_failures.remove(service_name);
                    started.push(service_name);
                    startup::mark_service_ready(service_name);
                    // Register monitor with metrics collector and start intervals() background task
                    self.metrics_collector
                        .start_monitoring(service_name, monitor, self.shutdown.clone())
                        .await;
                }
                Err((phase, error)) => {
                    log_start_failure(service_name, phase, &error);
                    if service.essential() {
                        return Err(error);
                    }
                    let cause = crate::errors::source_chain(&error).join(": ");
                    self.start_failures.insert(
                        service_name,
                        UiText::new(ids::SERVICES_HEALTH_START_FAILED)
                            .arg("reason", UiArg::Text(cause.clone())),
                    );
                    failures.push(ServiceStartupFailure {
                        name: service_name,
                        error: format!("Failed to {phase}: {cause}"),
                    });
                }
            }

            last_service_time = Instant::now();
        }

        Ok((started, failures))
    }

    /// Check if services can be started (initialization complete)
    pub fn can_start_services(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    /// Stop all services in reverse startup order
    pub async fn stop_all(&mut self) -> crate::Result<()> {
        let running_services: Vec<&'static str> = self.handles.keys().copied().collect();
        let shutdown_begin = format!("running={} debug_system=on", running_services.len());
        log_service_startup_phase("shutdown_begin", Some(&shutdown_begin));

        // Signal shutdown.
        //
        // notify_waiters() is EDGE-triggered: it only wakes tasks that are parked on
        // `.notified()` at this exact instant. Any background task that is mid-work
        // right now (running an update batch, an RPC wallet snapshot, a pool-priority
        // sync, a filter scan) is NOT parked, so it misses this single broadcast — and
        // because the loop helpers create a FRESH `.notified()` future on the next
        // iteration, every subsequent wait is registered too late to ever catch a
        // broadcast that already fired, so the task hangs until its 10s per-task
        // timeout below. That is exactly the residual wallet + tokens pool-sync stall.
        //
        // Fix at the source: keep re-broadcasting on a short interval for the whole
        // duration of shutdown. Any task is then guaranteed to catch the signal within
        // one interval of becoming parked again, whatever it was doing when shutdown
        // began — with zero changes required at the ~50 `.notified()` consumer sites.
        self.shutdown.notify_waiters();
        let renotify = {
            let notify = self.shutdown.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    notify.notify_waiters();
                }
            })
        };

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
                        &format!("Service stop error for {service_name}: {e}"),
                    );
                }

                // Wait for handles with increased timeout for cleanup tasks
                if let Some(handles) = self.handles.remove(service_name) {
                    let timeout_duration = Duration::from_secs(10);
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

        // All handles have been awaited — stop re-broadcasting the shutdown signal.
        renotify.abort();

        logger::info(
            LogTag::System,
            &format!(
                "service_shutdown status=complete services_stopped={}",
                running_services.len()
            ),
        );
        Ok(())
    }

    /// Stop one running service while every other service keeps running. Returns `false`
    /// when the service is not running. Refused for a service that cannot stop alone
    /// ([`Service::stops_alone`]) and for one a running service depends on.
    pub async fn stop_service(&mut self, name: &'static str) -> crate::Result<bool> {
        if !self.handles.contains_key(name) {
            return Ok(false);
        }
        let Some(service) = self.services.get(name) else {
            return Ok(false);
        };
        if !service.stops_alone() {
            return Err(crate::Error::Service(crate::errors::ServiceError::Stop {
                service: name.to_owned(),
                message: "the service cannot stop while the others keep running".to_owned(),
            }));
        }
        if let Some(dependent) = self.handles.keys().copied().find(|running| {
            self.services
                .get(running)
                .is_some_and(|s| s.dependencies().contains(&name))
        }) {
            return Err(crate::Error::Service(
                crate::errors::ServiceError::Dependency {
                    service: dependent.to_owned(),
                    dependency: name.to_owned(),
                    message: "a running service depends on the one being stopped".to_owned(),
                },
            ));
        }

        log_service_event(name, ServiceLogEvent::StopStart, None, true);
        if let Some(service) = self.services.get_mut(name) {
            if let Err(e) = service.stop().await {
                logger::warning(
                    LogTag::System,
                    &format!("Service stop error for {name}: {e}"),
                );
            }
        }
        for handle in self.handles.remove(name).unwrap_or_default() {
            let abort = handle.abort_handle();
            if tokio::time::timeout(STOP_ALONE_GRACE, handle)
                .await
                .is_err()
            {
                abort.abort();
                logger::warning(
                    LogTag::System,
                    &format!(
                        "Service {name} task still running {}s after its stop; aborted",
                        STOP_ALONE_GRACE.as_secs()
                    ),
                );
            }
        }
        log_service_event(name, ServiceLogEvent::StopSuccess, None, true);
        Ok(true)
    }

    /// Resolve the startup order of `services` and every registered service they
    /// depend on, transitively.
    ///
    /// The order is topological: a service is placed only after all of its
    /// registered dependencies. Among the services whose dependencies are all
    /// placed, the lowest `priority()` goes first and the name breaks ties, so the
    /// result is deterministic regardless of registration order. A dependency on
    /// an unregistered name is ignored here (`validate_dependencies` warns about
    /// it). A cycle is a `ServiceError::Dependency`.
    fn resolve_startup_order(&self, services: &[&'static str]) -> crate::Result<Vec<&'static str>> {
        use std::collections::{BTreeSet, HashSet};

        self.validate_dependencies(services)?;

        // Requested services plus their registered dependencies, each with its
        // distinct registered dependencies.
        let mut dependencies: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        let mut pending: Vec<&'static str> = services
            .iter()
            .copied()
            .filter(|name| self.services.contains_key(name))
            .collect();
        while let Some(name) = pending.pop() {
            if dependencies.contains_key(name) {
                continue;
            }
            let declared: HashSet<&'static str> = self
                .services
                .get(name)
                .map(|service| service.dependencies())
                .unwrap_or_default()
                .into_iter()
                .filter(|dep| self.services.contains_key(dep))
                .collect();
            pending.extend(declared.iter().copied());
            dependencies.insert(name, declared);
        }

        let priority = |name: &str| self.services.get(name).map_or(100, |s| s.priority());
        let mut dependents: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        let mut unmet: HashMap<&'static str, usize> = HashMap::new();
        let mut ready: BTreeSet<(i32, &'static str)> = BTreeSet::new();
        for (&name, declared) in &dependencies {
            unmet.insert(name, declared.len());
            if declared.is_empty() {
                ready.insert((priority(name), name));
            }
            for &dep in declared {
                dependents.entry(dep).or_default().push(name);
            }
        }

        let mut ordered = Vec::with_capacity(dependencies.len());
        while let Some((_, name)) = ready.pop_first() {
            ordered.push(name);
            for &dependent in dependents.get(name).into_iter().flatten() {
                if let Some(count) = unmet.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert((priority(dependent), dependent));
                    }
                }
            }
        }

        if ordered.len() < dependencies.len() {
            let mut blocked: Vec<&'static str> = unmet
                .into_iter()
                .filter(|(_, count)| *count > 0)
                .map(|(name, _)| name)
                .collect();
            blocked.sort_unstable();
            return Err(crate::Error::Service(
                crate::errors::ServiceError::Dependency {
                    service: blocked.first().copied().unwrap_or_default().to_owned(),
                    dependency: "circular".to_owned(),
                    message: format!("Circular dependency among services: {}", blocked.join(", ")),
                },
            ));
        }

        Ok(ordered)
    }

    /// Validate that all declared dependencies exist as registered services
    fn validate_dependencies(&self, services: &[&'static str]) -> crate::Result<()> {
        let registered: std::collections::HashSet<&str> = self.services.keys().copied().collect();
        let mut missing_deps: Vec<String> = Vec::new();

        for &service_name in services {
            if let Some(service) = self.services.get(service_name) {
                for dep in service.dependencies() {
                    if !registered.contains(dep) {
                        missing_deps.push(format!(
                            "Service '{}' depends on '{}' which is not registered",
                            service_name, dep
                        ));
                    }
                }
            }
        }

        if !missing_deps.is_empty() {
            // Log warnings but don't fail - allows for optional dependencies
            for msg in &missing_deps {
                logger::warning(LogTag::System, msg);
            }
        }

        Ok(())
    }

    /// Get health status. A service that failed to start reports `Unhealthy` with the
    /// reason; intentionally-disabled services report `Disabled` (a normal state) rather
    /// than their internal health, so they are never surfaced as issues.
    pub async fn get_health(&self) -> HashMap<&'static str, ServiceHealth> {
        let mut health = HashMap::new();
        for (name, service) in &self.services {
            let status = if let Some(reason) = self.start_failures.get(name) {
                ServiceHealth::Unhealthy(reason.clone())
            } else if service.is_enabled() {
                service.health().await
            } else {
                ServiceHealth::Disabled
            };
            health.insert(*name, status);
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
            .unwrap_or_default()
    }

    /// Get the names of services that currently have running handles
    pub fn get_running_service_names(&self) -> Vec<&'static str> {
        self.handles.keys().copied().collect()
    }

    /// Whether `name` has been started and is running.
    pub fn is_service_running(&self, name: &str) -> bool {
        self.handles.contains_key(name)
    }

    /// Get the number of services that currently have running handles
    pub fn get_running_service_count(&self) -> usize {
        self.handles.len()
    }
}
