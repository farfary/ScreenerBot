//! Scheduled AI Tasks Service
//!
//! Background service that polls for due scheduled tasks and executes them
//! using the ChatEngine in headless mode. Supports interval, daily, and
//! weekly schedules with configurable tool permissions and retries.

use crate::ai::{chat_db, scheduled_db, ChatRequest, ToolMode};
use crate::config::with_config;
use crate::events::{record_scheduled_task_event, Severity};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio_metrics::TaskMonitor;

pub struct ScheduledAiTasksService {
    tasks_completed: Arc<AtomicU64>,
    tasks_failed: Arc<AtomicU64>,
}

impl Default for ScheduledAiTasksService {
    fn default() -> Self {
        Self {
            tasks_completed: Arc::new(AtomicU64::new(0)),
            tasks_failed: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Service for ScheduledAiTasksService {
    fn name(&self) -> &'static str {
        "scheduled_ai_tasks"
    }

    fn priority(&self) -> i32 {
        92
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["webserver"]
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled)
    }

    async fn initialize(&mut self) -> Result<(), String> {
        logger::info(LogTag::System, "Scheduled AI tasks service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let completed = Arc::clone(&self.tasks_completed);
        let failed = Arc::clone(&self.tasks_failed);

        let handle =
            tokio::spawn(monitor.instrument(scheduler_worker(shutdown, completed, failed)));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(LogTag::System, "Scheduled AI tasks service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if !with_config(|cfg| cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled) {
            return ServiceHealth::Degraded("Disabled in config".to_string());
        }
        ServiceHealth::Healthy
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics {
            operations_total: self.tasks_completed.load(Ordering::Relaxed),
            errors_total: self.tasks_failed.load(Ordering::Relaxed),
            ..Default::default()
        }
    }
}

async fn scheduler_worker(
    shutdown: Arc<Notify>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
) {
    logger::info(LogTag::System, "Scheduled AI tasks worker started");

    // Wait a bit for other services to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    loop {
        let (enabled, interval_secs, _max_concurrent, default_timeout) = with_config(|cfg| {
            (
                cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled,
                cfg.ai.scheduled_tasks_check_interval_seconds,
                cfg.ai.scheduled_tasks_max_concurrent,
                cfg.ai.scheduled_tasks_default_timeout_seconds,
            )
        });

        if !enabled {
            logger::debug(LogTag::System, "Scheduled tasks disabled, stopping worker");
            break;
        }

        // Check for due tasks
        if let Some(pool) = chat_db::get_chat_pool() {
            match scheduled_db::get_due_tasks(&pool) {
                Ok(tasks) if !tasks.is_empty() => {
                    logger::debug(
                        LogTag::System,
                        &format!("Found {} due scheduled tasks", tasks.len()),
                    );

                    for task in tasks {
                        let task_timeout = if task.timeout_seconds > 0 {
                            task.timeout_seconds as u64
                        } else {
                            default_timeout
                        };

                        match execute_scheduled_task(&pool, &task, task_timeout).await {
                            Ok(_) => {
                                completed.fetch_add(1, Ordering::Relaxed);
                                logger::info(
                                    LogTag::System,
                                    &format!(
                                        "Scheduled task '{}' completed successfully",
                                        task.name
                                    ),
                                );
                            }
                            Err(e) => {
                                failed.fetch_add(1, Ordering::Relaxed);
                                logger::warning(
                                    LogTag::System,
                                    &format!("Scheduled task '{}' failed: {}", task.name, e),
                                );
                            }
                        }
                    }
                }
                Ok(_) => {
                    // No due tasks
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to check due tasks: {}", e));
                }
            }
        }

        // Wait for next check or shutdown
        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::System, "Scheduled AI tasks worker shutting down");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)) => {
                // Continue loop
            }
        }
    }
}

async fn execute_scheduled_task(
    pool: &Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    task: &scheduled_db::ScheduledTask,
    timeout_secs: u64,
) -> Result<(), String> {
    // Create hidden chat session for this run
    let session_title = format!(
        "[Auto] {} - {}",
        task.name,
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    );
    let session_id = chat_db::create_hidden_session(pool, &session_title)
        .map_err(|e| format!("Failed to create session: {}", e))?;

    // Record run start
    let run_id = scheduled_db::record_run_start(pool, task.id, Some(session_id))
        .map_err(|e| format!("Failed to record run start: {}", e))?;

    let start_time = std::time::Instant::now();

    // Build the tool mode
    let tool_mode = match task.tool_permissions.as_str() {
        "full" => ToolMode::Full,
        _ => ToolMode::ReadOnly,
    };

    // Build the chat request
    let request = ChatRequest {
        session_id,
        message: task.instruction.clone(),
        context: None,
        headless: true,
        tool_mode,
    };

    // Execute with timeout
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        execute_chat_request(request),
    )
    .await;

    let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(Ok(response)) => {
            // Serialize tool calls
            let tool_calls_json = if response.tool_calls.is_empty() {
                None
            } else {
                serde_json::to_string(&response.tool_calls).ok()
            };

            // Record successful run
            scheduled_db::record_run_complete(
                pool,
                run_id,
                "success",
                Some(&response.content),
                tool_calls_json.as_deref(),
                None, // tokens_used not easily available from ChatResponse
                None, // provider
                None, // model
                None, // no error
                duration_ms,
            )
            .map_err(|e| format!("Failed to record run completion: {}", e))?;

            // Update task counters
            scheduled_db::update_task_after_run(pool, task.id, true)
                .map_err(|e| format!("Failed to update task: {}", e))?;

            // Send Telegram notification if configured
            if task.notify_telegram && task.notify_on_success {
                send_task_notification(task, true, &response.content, None).await;
            }

            // Record successful task completion event
            let preview = if response.content.len() > 200 {
                &response.content[..200]
            } else {
                &response.content
            };
            record_scheduled_task_event(
                &format!("Task '{}' completed", task.name),
                preview,
                Severity::Info,
            );

            Ok(())
        }
        Ok(Err(e)) => {
            let error_msg = format!("{}", e);

            // Record failed run
            let _ = scheduled_db::record_run_complete(
                pool,
                run_id,
                "failed",
                None,
                None,
                None,
                None,
                None,
                Some(&error_msg),
                duration_ms,
            );

            // Update task counters
            let _ = scheduled_db::update_task_after_run(pool, task.id, false);

            // Send Telegram notification if configured
            if task.notify_telegram && task.notify_on_failure {
                send_task_notification(task, false, "", Some(&error_msg)).await;
            }

            // Record failed task event
            record_scheduled_task_event(
                &format!("Task '{}' failed", task.name),
                &error_msg,
                Severity::Warn,
            );

            Err(error_msg)
        }
        Err(_) => {
            // Timeout
            let error_msg = format!("Task timed out after {}s", timeout_secs);

            let _ = scheduled_db::record_run_complete(
                pool,
                run_id,
                "timeout",
                None,
                None,
                None,
                None,
                None,
                Some(&error_msg),
                duration_ms,
            );

            let _ = scheduled_db::update_task_after_run(pool, task.id, false);

            if task.notify_telegram && task.notify_on_failure {
                send_task_notification(task, false, "", Some(&error_msg)).await;
            }

            // Record timeout event
            record_scheduled_task_event(
                &format!("Task '{}' timed out", task.name),
                &error_msg,
                Severity::Warn,
            );

            Err(error_msg)
        }
    }
}

async fn execute_chat_request(request: ChatRequest) -> Result<crate::ai::ChatResponse, String> {
    let engine = crate::ai::try_get_chat_engine()
        .ok_or_else(|| "Chat engine not initialized".to_string())?;

    engine
        .process_message(request)
        .await
        .map_err(|e| format!("Chat engine error: {}", e))
}

async fn send_task_notification(
    task: &scheduled_db::ScheduledTask,
    success: bool,
    response: &str,
    error: Option<&str>,
) {
    // Check if telegram is enabled first
    let telegram_enabled = with_config(|cfg| cfg.telegram.enabled);
    if !telegram_enabled {
        return;
    }

    let emoji = if success { "✅" } else { "❌" };
    let status = if success { "completed" } else { "failed" };

    // Truncate response for Telegram (max ~4000 chars)
    let summary = if response.len() > 500 {
        format!("{}...", &response[..500])
    } else {
        response.to_string()
    };

    let mut message = format!(
        "{} <b>Scheduled Task {}</b>\n\n<b>{}</b>\n",
        emoji, status, task.name
    );

    if !summary.is_empty() {
        message.push_str(&format!(
            "\n{}\n",
            crate::telegram::formatters::html_escape(&summary)
        ));
    }

    if let Some(err) = error {
        message.push_str(&format!(
            "\n⚠️ Error: {}\n",
            crate::telegram::formatters::html_escape(err)
        ));
    }

    // Create a notification using the proper notification system
    use crate::telegram::types::{Notification, NotificationType};

    let notification = Notification {
        notification_type: NotificationType::BotCommand {
            command: "scheduled_task".to_string(),
            response: message,
        },
        timestamp: chrono::Utc::now(),
    };

    // Send via the proper async notification channel
    crate::telegram::notifier::send_notification(notification).await;
}

/// Public function for triggering task execution from API
pub async fn execute_scheduled_task_public(
    pool: &Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    task: &crate::ai::scheduled_db::ScheduledTask,
    timeout_secs: u64,
) -> Result<(), String> {
    execute_scheduled_task(pool, task, timeout_secs).await
}
