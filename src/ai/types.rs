use serde::{Deserialize, Serialize};

/// AI evaluation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High,   // Trading decisions - bypass cache
    Medium, // Trailing stop - use recent cache
    Low,    // Filtering/background - always use cache
}

/// AI decision result (after processing LLM response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
    pub decision: String, // "pass", "reject", "buy", "sell", etc.
    pub confidence: u8,   // 0-100
    pub reasoning: String,
    pub risk_level: RiskLevel,
    pub factors: Vec<Factor>,
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factor {
    pub name: String,
    pub impact: Impact,
    pub weight: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Impact {
    Positive,
    Negative,
    Neutral,
}

/// Evaluation context with multi-source data
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    pub mint: String,
    pub dexscreener_data: Option<serde_json::Value>,
    pub geckoterminal_data: Option<serde_json::Value>,
    pub rugcheck_data: Option<serde_json::Value>,
    pub pool_data: Option<serde_json::Value>,
    pub opening_snapshot: Option<serde_json::Value>,
    pub price_history: Option<Vec<f64>>,
}

/// Evaluation result (generic container for any decision type)
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub decision: AiDecision,
    pub cached: bool,
}

/// AI error types
#[derive(Debug, Clone)]
pub enum AiError {
    Disabled,
    ProviderNotConfigured(String),
    RateLimited { retry_after: Option<u64> },
    Timeout,
    LlmError(String),
    ParseError(String),
    ValidationError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::Disabled => write!(f, "AI module is disabled"),
            AiError::ProviderNotConfigured(p) => write!(f, "Provider not configured: {}", p),
            AiError::RateLimited { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "Rate limited, retry after {} seconds", secs)
                } else {
                    write!(f, "Rate limited")
                }
            }
            AiError::Timeout => write!(f, "AI request timed out"),
            AiError::LlmError(e) => write!(f, "LLM error: {}", e),
            AiError::ParseError(e) => write!(f, "Failed to parse AI response: {}", e),
            AiError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for AiError {}

// Convert String to AiError for ? operator compatibility
impl From<String> for AiError {
    fn from(s: String) -> Self {
        AiError::ParseError(s)
    }
}
