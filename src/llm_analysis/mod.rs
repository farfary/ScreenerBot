//! LLM-assisted token and trading analysis.
//!
//! Model-scored filtering, entry and exit decisions built on the outbound LLM
//! provider clients in `src/apis/llm/`. This module owns the analysis engine,
//! its response cache, the decision/instruction database, the prompt builders
//! and the typed decision schemas. Dashboard conversation lives in
//! `crate::assistant`; the shared tool registry and permission policy live in
//! `crate::agent_control`. All features are disabled by default.

pub mod background_worker;
pub mod cache;
pub mod database;
pub use database as db;
pub mod engine;
pub mod error;
pub mod prompts;
pub mod schemas;
pub mod types;

// Re-exports
pub use cache::AnalysisCache;
pub use db::{
    clear_old_decisions, create_instruction, delete_instruction, get_analysis_database,
    get_builtin_templates, get_decision, get_instruction, init_analysis_database, list_decisions,
    list_decisions_for_mint, list_instructions, record_decision, reorder_instructions,
    update_instruction, with_analysis_db,
};
pub use engine::{
    get_analysis_engine, init_analysis_engine, try_get_analysis_engine, AnalysisEngine,
};
pub use error::{Error, Result};
pub use schemas::{ExitSuggestion, FilterDecision, TradeDecision};
pub use types::{
    AnalysisDecision, DecisionRecord, EvaluationContext, EvaluationResult, Instruction,
    InstructionTemplate, Priority,
};
