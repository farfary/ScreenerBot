//! AI Analysis Module
//!
//! AI-powered token analysis, filtering decisions, and trading assistance.
//! Uses LLM providers from src/apis/llm/ for intelligent decision making.
//! ALL FEATURES DISABLED BY DEFAULT.

pub mod cache;
pub mod engine;
pub mod prompts;
pub mod schemas;
pub mod types;

// Re-exports
pub use cache::AiCache;
pub use engine::AiEngine;
pub use schemas::{ExitSuggestion, FilterDecision, TradeDecision};
pub use types::{AiDecision, AiError, EvaluationContext, EvaluationResult, Priority};
