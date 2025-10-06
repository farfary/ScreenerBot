use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};
use tokio::sync::{Mutex, Notify};
use tokio_metrics::TaskMonitor;

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

impl ServiceMetrics {
    /// Calculate service activity as percentage of total time spent polling (working)
    /// This is a much better indicator than CPU for async services in a single process.
    pub fn activity_percent(&self) -> f32 {
        let total_time = self.total_poll_duration_ns + self.total_idle_duration_ns;
        if total_time == 0 {
            return 0.0;
        }
        (((self.total_poll_duration_ns as f64) / (total_time as f64)) * 100.0) as f32
    }

    /// Get human-readable activity status based on poll time ratio
    pub fn activity_status(&self) -> &'static str {
        match self.activity_percent() {
            x if x > 80.0 => "Very Active",
            x if x > 50.0 => "Active",
            x if x > 20.0 => "Moderate",
            x if x > 5.0 => "Light",
            _ => "Idle",
        }
    }

    /// Calculate average polls per second based on uptime
    pub fn polls_per_second(&self) -> f32 {
        if self.uptime_seconds == 0 {
            return 0.0;
        }
        (self.total_polls as f32) / (self.uptime_seconds as f32)
    }

    /// Format poll duration in human-readable format (µs, ms, s)
    pub fn format_poll_duration(&self) -> String {
        Self::format_nanos(self.mean_poll_duration_ns)
    }

    /// Format idle duration in human-readable format (µs, ms, s)
    pub fn format_idle_duration(&self) -> String {
        Self::format_nanos(self.mean_idle_duration_ns)
    }

    /// Helper to format nanoseconds into human-readable duration
    fn format_nanos(nanos: u64) -> String {
        if nanos < 1_000 {
            format!("{}ns", nanos)
        } else if nanos < 1_000_000 {
            format!("{:.2}µs", (nanos as f64) / 1_000.0)
        } else if nanos < 1_000_000_000 {
            format!("{:.2}ms", (nanos as f64) / 1_000_000.0)
        } else {
            format!("{:.2}s", (nanos as f64) / 1_000_000_000.0)
        }
    }
}

/// Accumulated task metrics from intervals stream
#[derive(Debug, Clone)]
struct AccumulatedTaskMetrics {
    instrumented_count: usize,
    total_polls: u64,
    total_poll_duration_ns: u64,
    total_idle_duration_ns: u64,
    last_update: Instant,
}

impl Default for AccumulatedTaskMetrics {
    fn default() -> Self {
        Self {
            instrumented_count: 0,
            total_polls: 0,
            total_poll_duration_ns: 0,
            total_idle_duration_ns: 0,
            last_update: Instant::now(),
        }
    }
}

pub struct MetricsCollector {
    system: Arc<Mutex<System>>,
    accumulated_metrics: Arc<Mutex<HashMap<&'static str, AccumulatedTaskMetrics>>>,
    service_start_times: Arc<Mutex<HashMap<&'static str, Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            accumulated_metrics: Arc::new(Mutex::new(HashMap::new())),
            service_start_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start monitoring a service with provided TaskMonitor
    /// Spawns a background task that polls intervals() stream and accumulates metrics
    pub async fn start_monitoring(
        &self,
        service_name: &'static str,
        monitor: TaskMonitor,
        shutdown: Arc<Notify>,
    ) {
        // Initialize storage
        let mut start_times = self.service_start_times.lock().await;
        start_times.insert(service_name, Instant::now());
        drop(start_times);

        let mut metrics_storage = self.accumulated_metrics.lock().await;
        metrics_storage.insert(service_name, AccumulatedTaskMetrics::default());
        drop(metrics_storage);

        // Spawn background collector task that periodically samples cumulative() metrics
        // Note: tokio_metrics intervals() API is complex - using simple polling instead
        let accumulated_metrics = self.accumulated_metrics.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(1));

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        break;
                    }
                    _ = interval_timer.tick() => {
                        // Sample cumulative metrics
                        let metrics_snapshot = monitor.cumulative();

                        // Update storage with current cumulative values
                        let mut storage = accumulated_metrics.lock().await;
                        if let Some(metrics) = storage.get_mut(&service_name) {
                            metrics.instrumented_count = metrics_snapshot.instrumented_count as usize;
                            metrics.total_polls = metrics_snapshot.total_poll_count;
                            metrics.total_poll_duration_ns = metrics_snapshot.total_poll_duration.as_nanos() as u64;
                            metrics.total_idle_duration_ns = metrics_snapshot.total_idle_duration.as_nanos() as u64;
                            metrics.last_update = Instant::now();
                        }
                    }
                }
            }
        });
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

        // Get accumulated task metrics from intervals() collector
        let storage = self.accumulated_metrics.lock().await;
        let (
            task_count,
            total_polls,
            total_poll_duration,
            mean_poll_duration,
            total_idle_duration,
            mean_idle_duration,
        ) = if let Some(accumulated) = storage.get(name) {
            let mean_poll = if accumulated.total_polls > 0 {
                accumulated.total_poll_duration_ns / accumulated.total_polls
            } else {
                0
            };
            let mean_idle = if accumulated.total_polls > 0 {
                accumulated.total_idle_duration_ns / accumulated.total_polls
            } else {
                0
            };

            (
                accumulated.instrumented_count,
                accumulated.total_polls,
                accumulated.total_poll_duration_ns,
                mean_poll,
                accumulated.total_idle_duration_ns,
                mean_idle,
            )
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        drop(storage);

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
        service_names: &[&'static str],
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
        let storage = self.accumulated_metrics.lock().await;
        let start_times = self.service_start_times.lock().await;

        for &name in service_names {
            // Get accumulated task metrics from intervals() collector
            let (
                task_count,
                total_polls,
                total_poll_duration,
                mean_poll_duration,
                total_idle_duration,
                mean_idle_duration,
            ) = if let Some(accumulated) = storage.get(name) {
                let mean_poll = if accumulated.total_polls > 0 {
                    accumulated.total_poll_duration_ns / accumulated.total_polls
                } else {
                    0
                };
                let mean_idle = if accumulated.total_polls > 0 {
                    accumulated.total_idle_duration_ns / accumulated.total_polls
                } else {
                    0
                };

                (
                    accumulated.instrumented_count,
                    accumulated.total_polls,
                    accumulated.total_poll_duration_ns,
                    mean_poll,
                    accumulated.total_idle_duration_ns,
                    mean_idle,
                )
            } else {
                (0, 0, 0, 0, 0, 0)
            };

            // Calculate uptime
            let uptime = start_times
                .get(name)
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);

            metrics.insert(
                name,
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
                },
            );
        }

        metrics
    }
}
