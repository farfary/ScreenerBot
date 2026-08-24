//! AI Analysis Module
//!
//! AI-powered token analysis, filtering decisions, and trading assistance.
//! Uses LLM providers from src/apis/llm/ for intelligent decision making.
//! ALL FEATURES DISABLED BY DEFAULT.

pub mod background_worker;
pub mod cache;
pub mod chat;
pub mod database;
pub use database as db;
pub mod engine;
mod error;
pub mod permissions;
pub mod prompts;
pub mod scheduled;
pub mod schemas;
pub mod tools;
pub mod types;

// Re-exports
pub use cache::AiCache;
pub use chat::{
    add_message, add_tool_execution, cleanup_hidden_sessions, create_hidden_session,
    create_session, delete_message, delete_session, get_chat_pool, get_message, get_messages,
    get_session, get_sessions, get_tool_executions, init_chat_db, touch_session,
    update_session_summary, update_session_title, update_tool_execution, with_chat_db, ChatMessage,
    ChatSession, ToolExecution,
};
pub use chat::{
    get_chat_engine, init_chat_engine, try_get_chat_engine, ChatContext, ChatEngine, ChatRequest,
    ChatResponse, PendingConfirmation, ToolCallInfo, ToolCallStatus, ToolMode,
};
pub use db::{
    clear_old_decisions, create_instruction, delete_instruction, get_ai_database,
    get_builtin_templates, get_decision, get_instruction, init_ai_database, list_decisions,
    list_decisions_for_mint, list_instructions, record_decision, reorder_instructions,
    update_instruction, with_ai_db,
};
pub use engine::{get_ai_engine, init_ai_engine, try_get_ai_engine, AiEngine};
pub use error::{Error, Result};
pub use permissions::{PermissionLevel, ToolPermissions};
pub use scheduled::{
    calculate_next_run, create_task, delete_task, get_due_tasks, get_task,
    initialize_scheduled_tables, list_tasks, toggle_task, update_task, update_task_after_run,
};
pub use scheduled::{
    cleanup_old_runs, get_automation_stats, get_run, list_recent_runs, list_runs_for_task,
    record_run_complete, record_run_start,
};
pub use scheduled::{
    AutomationStats, RunStatus, ScheduleType, ScheduledTask, TaskRun, TaskToolPermissions,
};
pub use schemas::{ExitSuggestion, FilterDecision, TradeDecision};
pub use tools::{
    create_tool_registry, Tool, ToolCategory, ToolDefinition, ToolRegistry, ToolResult,
};
pub use types::{
    AiDecision, DecisionRecord, EvaluationContext, EvaluationResult, Instruction,
    InstructionTemplate, Priority,
};
