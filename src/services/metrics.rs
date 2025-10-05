use serde::{ Deserialize, Serialize };
use sysinfo::{ System, Pid };
use tokio_metrics::TaskMonitor;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
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
    task_monitors: HashMap<&'static str, TaskMonitor>,
    service_start_times: HashMap<&'static str, Instant>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            task_monitors: HashMap::new(),
            service_start_times: HashMap::new(),
        }
    }

    /// Start monitoring a service with provided TaskMonitor
    pub fn start_monitoring(&mut self, service_name: &'static str, monitor: TaskMonitor) {
        self.task_monitors.insert(service_name, monitor);
        self.service_start_times.insert(service_name, Instant::now());
    }

    /// Collect metrics for a specific service
    pub async fn collect_for_service(&mut self, name: &str) -> ServiceMetrics {
        // Refresh system info (process-wide metrics)
        {
            let mut sys = self.system.lock().unwrap();
            sys.refresh_all();
        }

        // Get current process (shared across all services)
        let pid = Pid::from_u32(std::process::id());
        let sys = self.system.lock().unwrap();

        let (cpu, memory) = if let Some(process) = sys.process(pid) {
            (process.cpu_usage(), process.memory())
        } else {
            (0.0, 0)
        };

        // Get task metrics if available (per-service)
        let (
            task_count,
            total_polls,
            total_poll_duration,
            mean_poll_duration,
            total_idle_duration,
            mean_idle_duration,
        ) = if let Some(monitor) = self.task_monitors.get(name) {
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

        // Calculate uptime
        let uptime = self.service_start_times
            .get(name)
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

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
}
