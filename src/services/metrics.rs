use serde::{ Deserialize, Serialize };
use sysinfo::{ System, Pid };
use tokio_metrics::TaskMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

/// Service resource metrics with accurate per-service tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceMetrics {
    /// Process-wide CPU usage (all services share this)
    pub process_cpu_percent: f32,
    /// Process-wide memory usage (all services share this)
    pub process_memory_bytes: u64,

    /// Per-service task metrics
    pub task_count: usize,
    pub total_polls: u64,
    pub total_poll_duration_ns: u64,
    pub mean_poll_duration_ns: u64,
    pub total_idle_duration_ns: u64,
    pub mean_idle_duration_ns: u64,

    /// Service uptime
    pub uptime_seconds: u64,

    /// Service-specific operational metrics (populated by individual services)
    pub operations_total: u64,
    pub operations_per_second: f32,
    pub errors_total: u64,
    pub custom_metrics: HashMap<String, f64>,
}

pub struct MetricsCollector {
    system: Arc<Mutex<System>>,
    task_monitors: Arc<Mutex<HashMap<&'static str, TaskMonitor>>>,
    service_start_times: Arc<Mutex<HashMap<&'static str, Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            task_monitors: Arc::new(Mutex::new(HashMap::new())),
            service_start_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start monitoring a service with provided TaskMonitor
    pub async fn start_monitoring(&self, service_name: &'static str, monitor: TaskMonitor) {
        let mut monitors = self.task_monitors.lock().await;
        monitors.insert(service_name, monitor);

        let mut start_times = self.service_start_times.lock().await;
        start_times.insert(service_name, Instant::now());
    }

    /// Collect metrics for a specific service (async-safe, no &mut needed)
    pub async fn collect_for_service(&self, name: &str) -> ServiceMetrics {
        // Refresh system info once (process-wide metrics) - async-safe
        {
            let mut sys = self.system.lock().await;
            sys.refresh_all();
        }

        // Get current process (shared across all services) - async-safe
        let pid = Pid::from_u32(std::process::id());
        let sys = self.system.lock().await;

        let (cpu, memory) = if let Some(process) = sys.process(pid) {
            (process.cpu_usage(), process.memory())
        } else {
            (0.0, 0)
        };

        // Drop the lock before doing more work
        drop(sys);

        // Get task metrics if available (per-service)
        let monitors = self.task_monitors.lock().await;
        let (
            task_count,
            total_polls,
            total_poll_duration,
            mean_poll_duration,
            total_idle_duration,
            mean_idle_duration,
        ) = if let Some(monitor) = monitors.get(name) {
            let metrics = monitor.cumulative();
            (
                metrics.instrumented_count as usize,
                metrics.instrumented_count, // Use instrumented_count as proxy for total polls
                metrics.total_poll_duration.as_nanos() as u64,
                metrics.mean_poll_duration().as_nanos() as u64,
                metrics.total_idle_duration.as_nanos() as u64,
                metrics.mean_idle_duration().as_nanos() as u64,
            )
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        drop(monitors);

        // Calculate uptime
        let start_times = self.service_start_times.lock().await;
        let uptime = start_times
            .get(name)
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);
        drop(start_times);

        ServiceMetrics {
            process_cpu_percent: cpu,
            process_memory_bytes: memory,
            task_count,
            total_polls,
            total_poll_duration_ns: total_poll_duration,
            mean_poll_duration_ns: mean_poll_duration,
            total_idle_duration_ns: total_idle_duration,
            mean_idle_duration_ns: mean_idle_duration,
            uptime_seconds: uptime,
            operations_total: 0,
            operations_per_second: 0.0,
            errors_total: 0,
            custom_metrics: HashMap::new(),
        }
    }

    /// Collect metrics for all services efficiently (single refresh, no &mut needed)
    pub async fn collect_all(
        &self,
        service_names: &[&'static str]
    ) -> HashMap<&'static str, ServiceMetrics> {
        // Refresh system info ONCE for all services - async-safe
        {
            let mut sys = self.system.lock().await;
            sys.refresh_all();
        }

        // Get process info once
        let pid = Pid::from_u32(std::process::id());
        let sys = self.system.lock().await;
        let (cpu, memory) = if let Some(process) = sys.process(pid) {
            (process.cpu_usage(), process.memory())
        } else {
            (0.0, 0)
        };
        drop(sys);

        let mut metrics = HashMap::new();

        // Lock once for all services
        let monitors = self.task_monitors.lock().await;
        let start_times = self.service_start_times.lock().await;

        for &name in service_names {
            // Get task metrics if available (per-service)
            let (
                task_count,
                total_polls,
                total_poll_duration,
                mean_poll_duration,
                total_idle_duration,
                mean_idle_duration,
            ) = if let Some(monitor) = monitors.get(name) {
                let m = monitor.cumulative();
                (
                    m.instrumented_count as usize,
                    m.instrumented_count,
                    m.total_poll_duration.as_nanos() as u64,
                    m.mean_poll_duration().as_nanos() as u64,
                    m.total_idle_duration.as_nanos() as u64,
                    m.mean_idle_duration().as_nanos() as u64,
                )
            } else {
                (0, 0, 0, 0, 0, 0)
            };

            // Calculate uptime
            let uptime = start_times
                .get(name)
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);

            metrics.insert(name, ServiceMetrics {
                process_cpu_percent: cpu,
                process_memory_bytes: memory,
                task_count,
                total_polls,
                total_poll_duration_ns: total_poll_duration,
                mean_poll_duration_ns: mean_poll_duration,
                total_idle_duration_ns: total_idle_duration,
                mean_idle_duration_ns: mean_idle_duration,
                uptime_seconds: uptime,
                operations_total: 0,
                operations_per_second: 0.0,
                errors_total: 0,
                custom_metrics: HashMap::new(),
            });
        }

        metrics
    }
}
