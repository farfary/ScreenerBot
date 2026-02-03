//! AI Analysis Module
//!
//! AI-powered token analysis, filtering decisions, and trading assistance.
//! Uses LLM providers from src/apis/llm/ for intelligent decision making.
//! ALL FEATURES DISABLED BY DEFAULT.

pub mod cache;
pub mod chat_db;
pub mod chat_engine;
pub mod db;
pub mod engine;
pub mod permissions;
pub mod prompts;
pub mod schemas;
pub mod tools;
pub mod types;

// Re-exports
pub use cache::AiCache;
pub use chat_db::{
    add_message, add_tool_execution, create_session, delete_message, delete_session, get_chat_pool,
    get_message, get_messages, get_session, get_sessions, get_tool_executions, init_chat_db,
    touch_session, update_session_summary, update_session_title, update_tool_execution,
    with_chat_db, ChatMessage, ChatSession, ToolExecution,
};
pub use chat_engine::{
    get_chat_engine, init_chat_engine, try_get_chat_engine, ChatContext, ChatEngine, ChatRequest,
    ChatResponse, PendingConfirmation, ToolCallInfo, ToolCallStatus,
};
pub use db::{
    clear_old_decisions, create_instruction, delete_instruction, get_ai_database,
    get_builtin_templates, get_decision, get_instruction, init_ai_database, list_decisions,
    list_decisions_for_mint, list_instructions, record_decision, reorder_instructions,
    update_instruction, with_ai_db, DecisionRecord, Instruction, InstructionTemplate,
};
pub use engine::{get_ai_engine, init_ai_engine, try_get_ai_engine, AiEngine};
pub use permissions::{PermissionLevel, ToolPermissions};
pub use schemas::{ExitSuggestion, FilterDecision, TradeDecision};
pub use tools::{
    create_tool_registry, Tool, ToolCategory, ToolDefinition, ToolRegistry, ToolResult,
};
pub use types::{AiDecision, AiError, EvaluationContext, EvaluationResult, Priority};
