//! Config API Types
//!
//! All type definitions, structs, and constants for the config API.

use serde::{Deserialize, Serialize};

use crate::config;
use crate::config::schemas::TabConfig;

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ConfigResponse<T> {
    pub data: T,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct FullConfigResponse {
    pub rpc: config::RpcConfig,
    pub trader: config::TraderConfig,
    pub positions: config::PositionsConfig,
    pub filtering: config::FilteringConfig,
    pub swaps: config::SwapsConfig,
    pub tokens: config::TokensConfig,
    pub sol_price: config::SolPriceConfig,
    pub events: config::EventsConfig,
    pub services: config::ServicesConfig,
    pub monitoring: config::MonitoringConfig,
    pub ohlcv: config::OhlcvConfig,
    pub gui: config::GuiConfig,
    pub telegram: config::TelegramConfig,
    pub ai: config::AiConfig,
    pub network: config::NetworkConfig,
    pub referral: config::ReferralConfig,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigMetadataResponse {
    pub data: config::metadata::ConfigMetadata,
    pub timestamp: String,
}

/// Response type for GUI defaults endpoint
#[derive(Debug, Serialize)]
pub struct GuiDefaultsData {
    pub tabs: Vec<TabConfig>,
}

#[derive(Debug, Serialize)]
pub struct GuiDefaultsResponse {
    pub success: bool,
    pub data: GuiDefaultsData,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateResponse {
    pub message: String,
    pub saved_to_disk: bool,
    pub timestamp: String,
}

// ============================================================================
// IMPORT/EXPORT TYPES
// ============================================================================

/// List of all config sections that can be imported/exported
pub const CONFIG_SECTIONS: &[&str] = &[
    "rpc",
    "trader",
    "positions",
    "filtering",
    "swaps",
    "tokens",
    "sol_price",
    "events",
    "services",
    "monitoring",
    "ohlcv",
    "gui",
    "telegram",
    "ai",
    "network",
];

/// Sensitive fields that should be sanitized on export (path format: "section.nested.field")
pub const SENSITIVE_FIELDS: &[(&str, &[&str])] = &[
    ("telegram", &["bot_token"]),
    (
        "gui",
        &[
            "dashboard.lockscreen.password_hash",
            "dashboard.lockscreen.password_salt",
        ],
    ),
    (
        "webserver",
        &[
            "auth_password_hash",
            "auth_password_salt",
            "auth_totp_secret",
        ],
    ),
    (
        "ai",
        &[
            "openai.api_key",
            "anthropic.api_key",
            "groq.api_key",
            "deepseek.api_key",
            "gemini.api_key",
            "together.api_key",
            "openrouter.api_key",
            "mistral.api_key",
        ],
    ),
    ("ohlcv", &["sources.solana_tracker.api_key"]),
];

#[derive(Debug, Deserialize)]
pub struct ExportConfigRequest {
    /// Which sections to export. If empty or None, exports all sections.
    pub sections: Option<Vec<String>>,
    /// Whether to include GUI settings (default: true)
    #[serde(default = "default_true")]
    pub include_gui: bool,
    /// Whether to include metadata like export timestamp
    #[serde(default = "default_true")]
    pub include_metadata: bool,
    /// Whether to sanitize sensitive fields (bot tokens, password hashes, etc.)
    /// Default: true for security. Set to false only for full backup purposes.
    #[serde(default = "default_true")]
    pub sanitize_secrets: bool,
}

pub fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ExportConfigResponse {
    pub config: serde_json::Value,
    pub sections: Vec<String>,
    pub exported_at: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportConfigPreviewRequest {
    /// The JSON config data to preview
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SectionPreview {
    pub name: String,
    pub label: String,
    pub present: bool,
    pub valid: bool,
    pub field_count: usize,
    pub error: Option<String>,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, Serialize)]
pub struct FieldChange {
    pub field: String,
    pub current: serde_json::Value,
    pub imported: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewResponse {
    pub valid: bool,
    pub sections: Vec<SectionPreview>,
    pub warnings: Vec<String>,
    pub total_changes: usize,
}

#[derive(Debug, Deserialize)]
pub struct ImportConfigRequest {
    /// The JSON config data to import
    pub config: serde_json::Value,
    /// Which sections to import. If empty or None, imports all present sections.
    pub sections: Option<Vec<String>>,
    /// Whether to merge with existing config (true) or replace sections entirely (false)
    #[serde(default)]
    pub merge: bool,
    /// Whether to save to disk after import
    #[serde(default = "default_true")]
    pub save_to_disk: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportConfigResponse {
    pub success: bool,
    pub message: String,
    pub imported_sections: Vec<String>,
    pub saved_to_disk: bool,
    pub timestamp: String,
}
