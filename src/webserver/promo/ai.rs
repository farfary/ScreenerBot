//! Promo generator for the AI assistant "Overview" tab (`/api/ai/status`).
//!
//! Produces a realistic, active-looking AI module snapshot — enabled module,
//! a mix of configured providers, headline metrics, and a recent-decisions feed —
//! so the overview tab makes a compelling marketing screenshot.

use chrono::{Duration, Utc};

use crate::webserver::routes::ai::types::{
    AiDecision, AiMetrics, AiStatusResponse, ProviderStatus,
};

/// (id, name, enabled, has_api_key, model, rate_limit_per_minute)
const PROMO_PROVIDERS: &[(&str, &str, bool, bool, &str, u32)] = &[
    ("anthropic", "Anthropic", true, true, "claude-sonnet-5", 60),
    ("openai", "OpenAI", true, true, "gpt-5", 60),
    ("groq", "Groq", true, true, "llama-3.3-70b-versatile", 30),
    ("deepseek", "DeepSeek", true, true, "deepseek-chat", 60),
    ("gemini", "Gemini", true, true, "gemini-2.5-pro", 60),
    (
        "together",
        "Together AI",
        false,
        false,
        "meta-llama/Llama-3.3-70B",
        60,
    ),
    ("openrouter", "OpenRouter", false, false, "auto", 60),
    (
        "mistral",
        "Mistral AI",
        false,
        false,
        "mistral-large-latest",
        60,
    ),
    ("ollama", "Ollama (Local)", false, true, "llama3.1", 120),
];

/// (decision, context, token, minutes_ago, latency_ms, confidence)
const PROMO_DECISIONS: &[(&str, &str, &str, i64, f64, f64)] = &[
    ("allow", "Filter", "MOODENG", 1, 684.0, 0.92),
    ("reject", "Filter", "SAFEMOON2", 3, 731.0, 0.88),
    ("allow", "Entry", "GIGA", 6, 902.0, 0.81),
    ("allow", "Filter", "FWOG", 11, 655.0, 0.9),
    ("reject", "Entry", "RUGME", 17, 812.0, 0.95),
    ("allow", "Exit", "TRUMP", 24, 588.0, 0.86),
    ("reject", "Filter", "PONZI", 33, 767.0, 0.93),
    ("allow", "Filter", "GOAT", 41, 623.0, 0.89),
];

/// Generate a rich promo AI status snapshot.
pub fn get_promo_ai_status() -> AiStatusResponse {
    let now = Utc::now();

    let configured_providers: Vec<ProviderStatus> = PROMO_PROVIDERS
        .iter()
        .map(
            |(id, name, enabled, has_api_key, model, rate)| ProviderStatus {
                id: (*id).to_owned(),
                name: (*name).to_owned(),
                enabled: *enabled,
                has_api_key: *has_api_key,
                model: (*model).to_owned(),
                rate_limit_per_minute: *rate,
            },
        )
        .collect();

    let active_providers = configured_providers
        .iter()
        .filter(|p| p.enabled && p.has_api_key)
        .count() as u32;
    let total_providers = configured_providers.len() as u32;

    let recent_decisions: Vec<AiDecision> = PROMO_DECISIONS
        .iter()
        .map(
            |(decision, context, token, mins_ago, latency, confidence)| AiDecision {
                decision: (*decision).to_owned(),
                context: (*context).to_owned(),
                token: (*token).to_owned(),
                timestamp: (now - Duration::minutes(*mins_ago)).to_rfc3339(),
                latency_ms: *latency,
                confidence: *confidence,
            },
        )
        .collect();

    AiStatusResponse {
        enabled: true,
        filtering_enabled: true,
        entry_analysis_enabled: true,
        exit_analysis_enabled: true,
        default_provider: "anthropic".to_owned(),
        configured_providers,
        total_evaluations: 18_432,
        cache_entries: 1_284,
        cache_fresh_entries: 947,
        metrics: AiMetrics {
            total_evaluations: 18_432,
            cache_hit_rate: 0.86,
            avg_response_time_ms: 742.0,
            active_providers,
            total_providers,
        },
        recent_decisions,
    }
}
