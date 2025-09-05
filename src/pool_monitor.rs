use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_monitor_enabled;
use crate::pool_constants::*;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of consecutive errors before task restart
const MAX_CONSECUTIVE_ERRORS: u64 = 5;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Task state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Stopped,
    Starting,
    Running,
    Error(String),
    Stopping,
}

/// Individual task status
#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub name: String,
    pub state: TaskState,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

impl TaskStatus {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: TaskState::Stopped,
            last_run: None,
            run_count: 0,
            error_count: 0,
            last_error: None,
        }
    }
}

/// Service state for monitoring
#[derive(Debug)]
pub struct MonitorServiceState {
    /// All tokens being tracked
    pub tracked_tokens: HashMap<String, DateTime<Utc>>, // token_mint -> last_seen
    /// Best pool for each token (highest liquidity)
    pub best_pools: HashMap<String, crate::pool_discovery::PoolData>, // token_mint -> pool_data
    /// Account addresses to fetch data for
    pub account_queue: Vec<crate::pool_discovery::AccountInfo>,
    /// Raw account data cache
    pub account_data_cache: HashMap<String, (Vec<u8>, DateTime<Utc>)>, // address -> (data, timestamp)
    /// Task statuses
    pub task_statuses: HashMap<String, TaskStatus>,
}

/// Pool monitor statistics
#[derive(Debug, Clone)]
pub struct PoolMonitorStats {
    pub total_monitoring_cycles: u64,
    pub successful_cycles: u64,
    pub failed_cycles: u64,
    pub tasks_restarted: u64,
    pub health_checks_performed: u64,
    pub average_health_percentage: f64,
    pub last_health_check: Option<DateTime<Utc>>,
    pub last_monitoring_cycle: Option<DateTime<Utc>>,
}

impl Default for PoolMonitorStats {
    fn default() -> Self {
        Self {
            total_monitoring_cycles: 0,
            successful_cycles: 0,
            failed_cycles: 0,
            tasks_restarted: 0,
            health_checks_performed: 0,
            average_health_percentage: 0.0,
            last_health_check: None,
            last_monitoring_cycle: None,
        }
    }
}

impl PoolMonitorStats {
    pub fn get_success_rate(&self) -> f64 {
        if self.total_monitoring_cycles == 0 {
            0.0
        } else {
            ((self.successful_cycles as f64) / (self.total_monitoring_cycles as f64)) * 100.0
        }
    }

    pub fn record_monitoring_cycle(&mut self, success: bool, health_percentage: f64) {
        self.total_monitoring_cycles += 1;
        if success {
            self.successful_cycles += 1;
        } else {
            self.failed_cycles += 1;
        }

        // Update average health percentage
        let total_health =
            self.average_health_percentage * ((self.total_monitoring_cycles - 1) as f64) +
            health_percentage;
        self.average_health_percentage = total_health / (self.total_monitoring_cycles as f64);

        self.last_monitoring_cycle = Some(Utc::now());
        self.last_health_check = Some(Utc::now());
    }
}

/// Pool monitor service
pub struct PoolMonitorService {
    stats: Arc<RwLock<PoolMonitorStats>>,
    debug_enabled: bool,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolMonitorService {
    /// Create new pool monitor service
    pub fn new() -> Self {
        let debug_enabled = is_debug_pool_monitor_enabled();

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool monitor service debug mode enabled");
        }

        Self {
            stats: Arc::new(RwLock::new(PoolMonitorStats::default())),
            debug_enabled,
        }
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool monitor service debug mode enabled (overridden)");
    }

    /// Monitor all task states and service health
    pub async fn monitor_service_health(
        &self,
        shared_state: &Arc<RwLock<MonitorServiceState>>
    ) -> Result<f64, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(LogTag::Pool, "MONITOR_START", "🔍 Starting service health monitoring");
        }

        let state = shared_state.read().await;
        let mut healthy_tasks = 0;
        let mut total_tasks = 0;
        let mut error_tasks = Vec::new();
        let mut stale_tasks = Vec::new();

        // Check task health
        for (name, status) in &state.task_statuses {
            total_tasks += 1;
            match &status.state {
                TaskState::Running => {
                    // Check if task is stale (hasn't run recently)
                    if let Some(last_run) = status.last_run {
                        let age = Utc::now().signed_duration_since(last_run);
                        if age.num_seconds() > TASK_HEALTH_TIMEOUT_SECS {
                            stale_tasks.push((
                                name.clone(),
                                format!("Task hasn't run for {} seconds", age.num_seconds()),
                            ));
                        } else {
                            healthy_tasks += 1;
                        }
                    } else {
                        // Task has never run
                        stale_tasks.push((name.clone(), "Task has never run".to_string()));
                    }
                }
                TaskState::Error(e) => {
                    error_tasks.push((name.clone(), e.clone()));
                }
                _ => {
                    // Task is stopped, starting, or stopping
                    if let Some(last_run) = status.last_run {
                        let age = Utc::now().signed_duration_since(last_run);
                        if age.num_seconds() > TASK_HEALTH_TIMEOUT_SECS {
                            stale_tasks.push((
                                name.clone(),
                                format!("Task hasn't run for {} seconds", age.num_seconds()),
                            ));
                        }
                    }
                }
            }
        }

        // Calculate health percentage
        let health_percentage = if total_tasks > 0 {
            ((healthy_tasks as f64) / (total_tasks as f64)) * 100.0
        } else {
            0.0
        };

        // Log health status
        if total_tasks > 0 {
            log(
                LogTag::Pool,
                "HEALTH_CHECK",
                &format!(
                    "Service health: {:.1}% ({}/{} tasks healthy)",
                    health_percentage,
                    healthy_tasks,
                    total_tasks
                )
            );

            // Log error tasks
            for (task_name, error) in &error_tasks {
                log(LogTag::Pool, "TASK_ERROR", &format!("Task {} error: {}", task_name, error));
            }

            // Log stale tasks
            for (task_name, reason) in &stale_tasks {
                log(
                    LogTag::Pool,
                    "TASK_STALE",
                    &format!("Task {} is stale: {}", task_name, reason)
                );
            }
        }

        // Log cache statistics
        log(
            LogTag::Pool,
            "CACHE_STATS",
            &format!(
                "Cache stats - Tracked tokens: {}, Best pools: {}, Account data: {}, Account queue: {}",
                state.tracked_tokens.len(),
                state.best_pools.len(),
                state.account_data_cache.len(),
                state.account_queue.len()
            )
        );

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.health_checks_performed += 1;
            stats.record_monitoring_cycle(true, health_percentage);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "MONITOR_COMPLETE",
                &format!(
                    "🔍 Health monitoring completed in {}ms - Health: {:.1}%, Errors: {}, Stale: {}",
                    start_time.elapsed().as_millis(),
                    health_percentage,
                    error_tasks.len(),
                    stale_tasks.len()
                )
            );
        }

        Ok(health_percentage)
    }

    /// Check if a task needs to be restarted
    pub async fn should_restart_task(&self, task_name: &str, task_status: &TaskStatus) -> bool {
        // Check if task has too many consecutive errors
        if task_status.error_count >= MAX_CONSECUTIVE_ERRORS {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "TASK_RESTART_NEEDED",
                    &format!(
                        "Task {} needs restart due to {} consecutive errors",
                        task_name,
                        task_status.error_count
                    )
                );
            }
            return true;
        }

        // Check if task is stale
        if let Some(last_run) = task_status.last_run {
            let age = Utc::now().signed_duration_since(last_run);
            if age.num_seconds() > TASK_HEALTH_TIMEOUT_SECS {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "TASK_RESTART_NEEDED",
                        &format!(
                            "Task {} needs restart due to staleness ({} seconds)",
                            task_name,
                            age.num_seconds()
                        )
                    );
                }
                return true;
            }
        }

        false
    }

    /// Restart a task (placeholder for future implementation)
    pub async fn restart_task(&self, task_name: &str) -> Result<(), String> {
        // TODO: Implement task restart logic
        // This would involve:
        // 1. Stopping the current task
        // 2. Waiting for graceful shutdown
        // 3. Starting a new instance of the task
        // 4. Updating task status

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TASK_RESTART",
                &format!("Task {} restart requested (not implemented yet)", task_name)
            );
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.tasks_restarted += 1;
        }

        Ok(())
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolMonitorStats {
        self.stats.read().await.clone()
    }

    /// Get service health summary
    pub async fn get_health_summary(
        &self,
        shared_state: &Arc<RwLock<MonitorServiceState>>
    ) -> (f64, usize, usize, usize) {
        let state = shared_state.read().await;
        let mut healthy_tasks = 0;
        let mut error_tasks = 0;
        let mut stale_tasks = 0;

        for (_, status) in &state.task_statuses {
            match &status.state {
                TaskState::Running => {
                    if let Some(last_run) = status.last_run {
                        let age = Utc::now().signed_duration_since(last_run);
                        if age.num_seconds() > TASK_HEALTH_TIMEOUT_SECS {
                            stale_tasks += 1;
                        } else {
                            healthy_tasks += 1;
                        }
                    } else {
                        stale_tasks += 1;
                    }
                }
                TaskState::Error(_) => {
                    error_tasks += 1;
                }
                _ => {
                    if let Some(last_run) = status.last_run {
                        let age = Utc::now().signed_duration_since(last_run);
                        if age.num_seconds() > TASK_HEALTH_TIMEOUT_SECS {
                            stale_tasks += 1;
                        }
                    }
                }
            }
        }

        let total_tasks = state.task_statuses.len();
        let health_percentage = if total_tasks > 0 {
            ((healthy_tasks as f64) / (total_tasks as f64)) * 100.0
        } else {
            0.0
        };

        (health_percentage, healthy_tasks, error_tasks, stale_tasks)
    }
}

// =============================================================================
// GLOBAL INSTANCE MANAGEMENT
// =============================================================================

static GLOBAL_POOL_MONITOR: std::sync::OnceLock<PoolMonitorService> = std::sync::OnceLock::new();

/// Initialize the global pool monitor service
pub fn init_pool_monitor() -> &'static PoolMonitorService {
    GLOBAL_POOL_MONITOR.get_or_init(|| {
        log(LogTag::Pool, "INIT", "Initializing global pool monitor service");
        PoolMonitorService::new()
    })
}

/// Get the global pool monitor service
pub fn get_pool_monitor() -> &'static PoolMonitorService {
    GLOBAL_POOL_MONITOR.get().expect("Pool monitor service not initialized")
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Monitor service health (convenience function)
pub async fn monitor_service_health(
    shared_state: &Arc<RwLock<MonitorServiceState>>
) -> Result<f64, String> {
    get_pool_monitor().monitor_service_health(shared_state).await
}

/// Check if task should be restarted (convenience function)
pub async fn should_restart_task(task_name: &str, task_status: &TaskStatus) -> bool {
    get_pool_monitor().should_restart_task(task_name, task_status).await
}

/// Restart task (convenience function)
pub async fn restart_task(task_name: &str) -> Result<(), String> {
    get_pool_monitor().restart_task(task_name).await
}

/// Get monitor statistics (convenience function)
pub async fn get_pool_monitor_stats() -> PoolMonitorStats {
    get_pool_monitor().get_stats().await
}

/// Get health summary (convenience function)
pub async fn get_health_summary(
    shared_state: &Arc<RwLock<MonitorServiceState>>
) -> (f64, usize, usize, usize) {
    get_pool_monitor().get_health_summary(shared_state).await
}
