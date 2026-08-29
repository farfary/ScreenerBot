//! The in-app assistant: dashboard conversation and scheduled conversation
//! automation.
//!
//! `chat` is the interactive dashboard assistant (engine, session/message
//! persistence, tool-confirmation flow). `scheduled` runs saved assistant
//! instructions on a timer. Both reach domain capabilities only through the
//! shared tool registry and permission policy in `crate::agent_control`, and
//! both call the LLM provider clients in `src/apis/llm/` directly for model
//! calls — model traffic does not pass through `crate::llm_analysis`, which is
//! a separate consumer of the same provider clients. All features are disabled
//! by default.

pub mod chat;
pub mod error;
pub mod scheduled;

pub use error::{Error, Result};

pub use chat::{
    add_message, add_tool_execution, cleanup_hidden_sessions, create_hidden_session,
    create_session, delete_message, delete_session, get_chat_pool, get_message, get_messages,
    get_session, get_sessions, get_tool_executions, init_chat_db, touch_session,
    update_session_summary, update_session_title, update_tool_execution, with_chat_db, ChatMessage,
    ChatSession, ToolExecution,
};
pub use chat::{
    get_chat_engine, init_chat_engine, try_get_chat_engine, ChatContext, ChatEngine,
    ChatProgressEvent, ChatRequest, ChatResponse, PendingConfirmation, ToolCallInfo,
    ToolCallStatus, ToolMode,
};
pub use scheduled::{
    calculate_next_run, cleanup_old_runs, create_task, delete_task, execute_task_public,
    get_automation_stats, get_due_tasks, get_run, get_task, initialize_scheduled_tables,
    list_recent_runs, list_runs_for_task, list_tasks, record_run_complete, record_run_start,
    scheduler_worker, toggle_task, update_task, update_task_after_run, AutomationStats, RunStatus,
    ScheduleType, ScheduledTask, TaskRun, TaskToolPermissions,
};
