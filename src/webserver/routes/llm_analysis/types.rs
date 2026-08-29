//! Request/response types for the model-scored analysis API (`/api/llm-analysis`).

use serde::{Deserialize, Serialize};

pub use crate::webserver::routes::llm::types::ProviderStatus;

// ============================================================================
// STATUS / STATS / CONFIG
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AnalysisStatusResponse {
    pub enabled: bool,
    pub filtering_enabled: bool,
    pub entry_analysis_enabled: bool,
    pub exit_analysis_enabled: bool,
    pub default_provider: String,
    pub configured_providers: Vec<ProviderStatus>,
    pub total_evaluations: u64,
    pub cache_entries: usize,
    pub cache_fresh_entries: usize,
    /// Headline metrics for the overview cards.
    pub metrics: AnalysisMetrics,
    /// Newest evaluation decisions for the overview feed.
    pub recent_decisions: Vec<AnalysisDecision>,
}

#[derive(Debug, Serialize)]
pub struct AnalysisMetrics {
    pub total_evaluations: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub active_providers: u32,
    pub total_providers: u32,
}

#[derive(Debug, Serialize)]
pub struct AnalysisDecision {
    pub decision: String,
    pub context: String,
    pub token: String,
    pub timestamp: String,
    pub latency_ms: f64,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalysisStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub total_entries: usize,
    pub fresh_entries: usize,
    pub ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct AnalysisConfigResponse {
    // Master Control (`enabled` / `default_provider`) is owned by `/api/llm/config`
    // and is deliberately absent here.
    // Filtering
    pub filtering_enabled: bool,
    pub filtering_min_confidence: u8,
    pub filtering_fallback_pass: bool,
    pub filtering_use_cache: bool,
    // Trading
    pub entry_analysis_enabled: bool,
    pub exit_analysis_enabled: bool,
    pub ai_trailing_stop_enabled: bool,
    pub trading_bypass_cache: bool,
    // Auto Blacklist
    pub auto_blacklist_enabled: bool,
    pub auto_blacklist_min_confidence: u8,
    // Background Check
    pub background_check_enabled: bool,
    pub background_check_interval_seconds: u64,
    pub background_batch_size: u32,
    // Rate Limits
    pub max_evaluations_per_minute: u32,
    // Performance
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAnalysisConfigRequest {
    // Master Control (`enabled` / `default_provider`) is owned by
    // `PATCH /api/llm/config`; this endpoint cannot mutate it.
    // Filtering
    pub filtering_enabled: Option<bool>,
    pub filtering_min_confidence: Option<u8>,
    pub filtering_fallback_pass: Option<bool>,
    pub filtering_use_cache: Option<bool>,
    // Trading
    pub entry_analysis_enabled: Option<bool>,
    pub exit_analysis_enabled: Option<bool>,
    pub ai_trailing_stop_enabled: Option<bool>,
    pub trading_bypass_cache: Option<bool>,
    // Auto Blacklist
    pub auto_blacklist_enabled: Option<bool>,
    pub auto_blacklist_min_confidence: Option<u8>,
    // Background Check
    pub background_check_enabled: Option<bool>,
    pub background_check_interval_seconds: Option<u64>,
    pub background_batch_size: Option<u32>,
    // Rate Limits
    pub max_evaluations_per_minute: Option<u32>,
    // Performance
    pub cache_ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TestEvaluateRequest {
    pub mint: String,
    pub priority: Option<String>, // "high", "medium", "low"
}

#[derive(Debug, Serialize)]
pub struct TestEvaluateResponse {
    pub decision: String,
    pub confidence: u8,
    pub reasoning: String,
    pub risk_level: String,
    pub factors: Vec<FactorResponse>,
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub latency_ms: f64,
    pub cached: bool,
}

#[derive(Debug, Serialize)]
pub struct FactorResponse {
    pub name: String,
    pub impact: String,
    pub weight: u8,
}

// ============================================================================
// INSTRUCTIONS / TEMPLATES / HISTORY
// ============================================================================

#[derive(Debug, Serialize)]
pub struct InstructionResponse {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub category: String,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct InstructionsListResponse {
    pub instructions: Vec<InstructionResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplatesListResponse {
    pub templates: Vec<TemplateResponse>,
}

#[derive(Debug, Serialize)]
pub struct DecisionHistoryResponse {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub decision: String,
    pub confidence: u8,
    pub reasoning: Option<String>,
    pub risk_level: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub tokens_used: u32,
    pub latency_ms: f64,
    pub cached: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HistoryListResponse {
    pub decisions: Vec<DecisionHistoryResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateInstructionRequest {
    pub name: String,
    pub content: String,
    pub category: Option<String>, // defaults to "general"
}

#[derive(Debug, Deserialize)]
pub struct UpdateInstructionRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderInstructionsRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub mint: Option<String>,
}
