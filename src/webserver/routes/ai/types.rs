//! AI API request and response type definitions

use serde::{Deserialize, Serialize};

use crate::ai::chat::database as chat_db;
use crate::ai::ChatContext;
use crate::ai::ChatSession;

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AiStatusResponse {
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
    pub metrics: AiMetrics,
    /// Newest evaluation decisions for the overview feed.
    pub recent_decisions: Vec<AiDecision>,
}

#[derive(Debug, Serialize)]
pub struct AiMetrics {
    pub total_evaluations: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub active_providers: u32,
    pub total_providers: u32,
}

#[derive(Debug, Serialize)]
pub struct AiDecision {
    pub decision: String,
    pub context: String,
    pub token: String,
    pub timestamp: String,
    pub latency_ms: f64,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub has_api_key: bool,
    pub model: String,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Serialize)]
pub struct AiStatsResponse {
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
pub struct AiConfigResponse {
    // Master Control
    pub enabled: bool,
    pub default_provider: String,
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

#[derive(Debug, Serialize)]
pub struct ProvidersListResponse {
    pub providers: Vec<ProviderStatus>,
    pub default_provider: String,
}

#[derive(Debug, Serialize)]
pub struct TestProviderResponse {
    pub provider: String,
    pub success: bool,
    pub model: String,
    pub latency_ms: f64,
    pub tokens_used: u32,
    pub response_preview: String,
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

// ============================================================================
// REQUEST TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateAiConfigRequest {
    // Master Control
    pub enabled: Option<bool>,
    pub default_provider: Option<String>,
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
pub struct UpdateProviderRequest {
    pub enabled: Option<bool>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub rate_limit_per_minute: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub mint: Option<String>,
}

// ============================================================================
// CHAT REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SendChatMessageRequest {
    pub session_id: i64,
    pub message: String,
    pub context: Option<ChatContext>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateChatSessionResponse {
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GetChatSessionResponse {
    pub session: ChatSession,
    pub messages: Vec<chat_db::ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmToolExecutionRequest {
    pub approved: bool,
    pub session_id: Option<i64>,
}

// ============================================================================
// Assistant AUTH TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AssistantAuthStatusResponse {
    pub authenticated: bool,
    pub has_github_token: bool,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthStartResponse {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct AssistantAuthPollRequest {
    pub device_code: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthPollResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthLogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthTestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// AUTOMATION TYPES
// ============================================================================

#[derive(Deserialize)]
pub struct CreateAutomationTaskRequest {
    pub name: String,
    pub instruction: String,
    pub schedule_type: String,
    pub schedule_value: String,
    #[serde(default = "default_read_only")]
    pub tool_permissions: String,
    #[serde(default = "default_low")]
    pub priority: String,
    #[serde(default = "default_true")]
    pub notify_telegram: bool,
    #[serde(default = "default_true")]
    pub notify_on_success: bool,
    #[serde(default = "default_true")]
    pub notify_on_failure: bool,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

fn default_read_only() -> String {
    "readonly".to_owned()
}
fn default_low() -> String {
    "low".to_owned()
}
fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct UpdateAutomationTaskRequest {
    pub name: Option<String>,
    pub instruction: Option<String>,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub tool_permissions: Option<String>,
    pub priority: Option<String>,
    pub notify_telegram: Option<bool>,
    pub notify_on_success: Option<bool>,
    pub notify_on_failure: Option<bool>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}
