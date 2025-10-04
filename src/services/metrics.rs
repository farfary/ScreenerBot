use sysinfo::{ System, Pid };
use tokio_metrics::TaskMonitor;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::Instant;

/// Service resource metrics
#[derive(Debug, Clone, Default)]
pub struct ServiceMetrics {
    pub cpu_usage_percent: f32,
    pub memory_usage_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub task_count: usize,
    pub task_avg_poll_time_ns: u64,
    pub uptime_seconds: u64,
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

    /// Start monitoring a service
    pub fn start_monitoring(&mut self, service_name: &'static str) -> TaskMonitor {
        let monitor = TaskMonitor::new();
        self.task_monitors.insert(service_name, monitor.clone());
        self.service_start_times.insert(service_name, Instant::now());
        monitor
    }

    /// Collect metrics for a specific service
    pub async fn collect_for_service(&mut self, name: &str) -> ServiceMetrics {
        // Refresh system info
        {
            let mut sys = self.system.lock().unwrap();
            sys.refresh_all();
        }

        // Get current process
        let pid = Pid::from_u32(std::process::id());
        let sys = self.system.lock().unwrap();

        let (cpu, memory) = if let Some(process) = sys.process(pid) {
            (process.cpu_usage(), process.memory())
        } else {
            (0.0, 0)
        };

        // Get task metrics if available
        let (task_count, avg_poll_time) = if let Some(monitor) = self.task_monitors.get(name) {
            let metrics = monitor.cumulative();
            (metrics.instrumented_count as usize, metrics.mean_poll_duration().as_nanos() as u64)
        } else {
            (0, 0)
        };

        // Calculate uptime
        let uptime = self.service_start_times
            .get(name)
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

        ServiceMetrics {
            cpu_usage_percent: cpu,
            memory_usage_bytes: memory,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            task_count,
            task_avg_poll_time_ns: avg_poll_time,
            uptime_seconds: uptime,
        }
    }
}
