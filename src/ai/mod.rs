//! AI Analysis Module
//!
//! AI-powered token analysis, filtering decisions, and trading assistance.
//! Uses LLM providers from src/apis/llm/ for intelligent decision making.
//! ALL FEATURES DISABLED BY DEFAULT.

pub mod cache;
pub mod db;
pub mod engine;
pub mod prompts;
pub mod schemas;
pub mod types;

// Re-exports
pub use cache::AiCache;
pub use db::{
    clear_old_decisions, create_instruction, delete_instruction, get_ai_database,
    get_builtin_templates, get_decision, get_instruction, init_ai_database, list_decisions,
    list_decisions_for_mint, list_instructions, record_decision, reorder_instructions,
    update_instruction, with_ai_db, DecisionRecord, Instruction, InstructionTemplate,
};
pub use engine::{get_ai_engine, init_ai_engine, try_get_ai_engine, AiEngine};
pub use schemas::{ExitSuggestion, FilterDecision, TradeDecision};
pub use types::{AiDecision, AiError, EvaluationContext, EvaluationResult, Priority};
