//! Scheduled AI Tasks Worker
//!
//! Core business logic for executing scheduled AI tasks. The scheduler worker
//! polls for due tasks at configured intervals and executes them using the ChatEngine
//! in headless mode with configurable tool permissions. Supports interval, daily,
//! and weekly schedules with timeout handling, retry logic, and Telegram notifications.

use crate::ai::{chat_db, scheduled_db, ChatRequest, ToolMode};
use crate::config::with_config;
use crate::errors::ServiceError;
use crate::events::{record_scheduled_task_event, Severity};
use crate::logger::{self, LogTag};
use chrono;
use futures::FutureExt;
use r2d2;
use r2d2_sqlite;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// Main scheduler worker loop that polls for due tasks and executes them
pub async fn scheduler_worker(
    shutdown: Arc<Notify>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
) {
    logger::info(LogTag::System, "Scheduled AI tasks worker started");

    // Wait a bit for other services to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Clean up old hidden sessions on startup (older than 7 days)
    if let Some(pool) = chat_db::get_chat_pool() {
        match chat_db::cleanup_hidden_sessions(&pool, 7) {
            Ok(count) if count > 0 => {
                logger::info(
                    LogTag::System,
                    &format!("Cleaned up {} old hidden AI sessions", count),
                );
            }
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to clean up hidden sessions: {e}"),
                );
            }
            _ => {}
        }
    }

    loop {
        let (enabled, interval_secs, default_timeout) = with_config(|cfg| {
            (
                cfg.ai.enabled && cfg.ai.scheduled_tasks_enabled,
                cfg.ai.scheduled_tasks_check_interval_seconds,
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
                    logger::warning(LogTag::System, &format!("Failed to check due tasks: {e}"));
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

/// Safely truncates a string at a character boundary
fn safe_truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Execute a scheduled task with timeout handling
async fn execute_scheduled_task(
    pool: &Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    task: &scheduled_db::ScheduledTask,
    timeout_secs: u64,
) -> crate::Result<()> {
    // Create hidden chat session for this run
    let session_title = format!(
        "[Auto] {} - {}",
        task.name,
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    );
    let session_id = chat_db::create_hidden_session(pool, &session_title).map_err(|e| {
        crate::Error::Service(ServiceError::Generic {
            message: format!("Failed to create session: {e}"),
        })
    })?;

    // Record run start
    let run_id = scheduled_db::record_run_start(pool, task.id, Some(session_id)).map_err(|e| {
        crate::Error::Service(ServiceError::Generic {
            message: format!("Failed to record run start: {e}"),
        })
    })?;

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

    // Execute with timeout — select! drops (cancels) the losing branch
    let result: Result<Result<crate::ai::ChatResponse, String>, ()> = tokio::select! {
        res = execute_chat_request(request) => Ok(res),
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(timeout_secs)) => Err(()),
    };

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
            .map_err(|e| {
                crate::Error::Service(ServiceError::Generic {
                    message: format!("Failed to record run completion: {e}"),
                })
            })?;

            // Update task counters
            scheduled_db::update_task_after_run(pool, task.id, true).map_err(|e| {
                crate::Error::Service(ServiceError::Generic {
                    message: format!("Failed to update task: {e}"),
                })
            })?;

            // Send Telegram notification if configured
            if task.notify_telegram && task.notify_on_success {
                send_task_notification(task, true, &response.content, None).await;
            }

            // Record successful task completion event
            let preview = safe_truncate(&response.content, 200);
            record_scheduled_task_event(
                &format!("Task '{}' completed", task.name),
                preview,
                Severity::Info,
            );

            Ok(())
        }
        Ok(Err(e)) => {
            let error_msg = e.to_string();

            // Record failed run
            if let Err(e) = scheduled_db::record_run_complete(
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
            ) {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to record run completion: {e}"),
                );
            }

            // Update task counters
            if let Err(e) = scheduled_db::update_task_after_run(pool, task.id, false) {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to update task after run: {e}"),
                );
            }

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

            Err(crate::Error::Service(ServiceError::Generic {
                message: error_msg,
            }))
        }
        Err(_) => {
            // Timeout
            let error_msg = format!("Task timed out after {}s", timeout_secs);

            if let Err(e) = scheduled_db::record_run_complete(
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
            ) {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to record run completion: {e}"),
                );
            }

            if let Err(e) = scheduled_db::update_task_after_run(pool, task.id, false) {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to update task after run: {e}"),
                );
            }

            if task.notify_telegram && task.notify_on_failure {
                send_task_notification(task, false, "", Some(&error_msg)).await;
            }

            // Record timeout event
            record_scheduled_task_event(
                &format!("Task '{}' timed out", task.name),
                &error_msg,
                Severity::Warn,
            );

            Err(crate::Error::Service(ServiceError::Generic {
                message: error_msg,
            }))
        }
    }
}

/// Execute a chat request via the ChatEngine with panic recovery
async fn execute_chat_request(request: ChatRequest) -> Result<crate::ai::ChatResponse, String> {
    let engine = crate::ai::try_get_chat_engine()
        .ok_or_else(|| "Chat engine not initialized".to_string())?;

    // NOTE: catch_unwind has limitations with async code. It can catch panics in synchronous
    // code within the async block, but may not catch all async panics depending on executor state.
    // This provides best-effort panic recovery. For more robust handling, ensure the chat engine
    // itself handles errors gracefully rather than panicking.
    match std::panic::AssertUnwindSafe(engine.process_message(request))
        .catch_unwind()
        .await
    {
        Ok(result) => result.map_err(|e| format!("Chat engine error: {e}")),
        Err(_) => Err("Chat engine panicked during execution".to_string()),
    }
}

/// Send Telegram notification for task completion/failure
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
        format!("{}...", safe_truncate(response, 500))
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
    crate::telegram::notifier::queue_notification(notification);
    logger::debug(
        LogTag::System,
        &format!("Queued Telegram notification for task '{}'", task.name),
    );
}

/// Public function for triggering task execution from API
pub async fn execute_task_public(
    pool: &Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    task: &crate::ai::scheduled_db::ScheduledTask,
    timeout_secs: u64,
) -> crate::Result<()> {
    execute_scheduled_task(pool, task, timeout_secs).await
}
