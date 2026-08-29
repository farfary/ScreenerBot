//! Request/response types for the outbound LLM provider API (`/api/llm`).

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub enabled: Option<bool>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub rate_limit_per_minute: Option<u32>,
}

/// Master LLM configuration owned by `/api/llm/config`: the whole-subsystem
/// enable switch and the fallback provider. Provider credentials are never part
/// of this shape — they move only through `PATCH /api/llm/providers/:provider`.
#[derive(Debug, Serialize)]
pub struct LlmConfigResponse {
    pub enabled: bool,
    pub default_provider: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLlmConfigRequest {
    pub enabled: Option<bool>,
    pub default_provider: Option<String>,
}
