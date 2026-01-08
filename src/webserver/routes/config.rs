use axum::http::StatusCode;
/// Configuration API routes
///
/// Provides REST API endpoints for viewing and managing bot configuration.
/// All responses follow the standard ScreenerBot API format.
use axum::{
    extract::State,
    response::Response,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config;
use crate::config::metadata::collect_config_metadata;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// ============================================================================
// RESPONSE TYPES (inline per ScreenerBot convention)
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
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigMetadataResponse {
    pub data: config::metadata::ConfigMetadata,
    pub timestamp: String,
}

// ============================================================================
// ROUTES
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // GET endpoints - View configuration
        .route("/config", get(get_full_config))
        .route("/config/rpc", get(get_rpc_config))
        .route("/config/trader", get(get_trader_config))
        .route("/config/positions", get(get_positions_config))
        .route("/config/filtering", get(get_filtering_config))
        .route("/config/swaps", get(get_swaps_config))
        .route("/config/tokens", get(get_tokens_config))
        .route("/config/sol_price", get(get_sol_price_config))
        .route("/config/summary", get(get_summary_config))
        .route("/config/events", get(get_events_config))
        .route("/config/services", get(get_services_config))
        .route("/config/monitoring", get(get_monitoring_config))
        .route("/config/ohlcv", get(get_ohlcv_config))
        .route("/config/gui", get(get_gui_config))
        .route("/config/telegram", get(get_telegram_config))
        .route("/config/metadata", get(get_config_metadata))
        // PATCH endpoints - Partial updates (use JSON with only fields to update)
        .route(
            "/config/trader",
            patch(patch_any_config::<config::TraderConfig>),
        )
        .route(
            "/config/positions",
            patch(patch_any_config::<config::PositionsConfig>),
        )
        .route(
            "/config/filtering",
            patch(patch_any_config::<config::FilteringConfig>),
        )
        .route(
            "/config/swaps",
            patch(patch_any_config::<config::SwapsConfig>),
        )
        .route(
            "/config/tokens",
            patch(patch_any_config::<config::TokensConfig>),
        )
        .route("/config/rpc", patch(patch_any_config::<config::RpcConfig>))
        .route(
            "/config/sol_price",
            patch(patch_any_config::<config::SolPriceConfig>),
        )
        .route(
            "/config/events",
            patch(patch_any_config::<config::EventsConfig>),
        )
        .route(
            "/config/services",
            patch(patch_any_config::<config::ServicesConfig>),
        )
        .route(
            "/config/monitoring",
            patch(patch_any_config::<config::MonitoringConfig>),
        )
        .route(
            "/config/ohlcv",
            patch(patch_any_config::<config::OhlcvConfig>),
        )
        .route("/config/gui", patch(patch_any_config::<config::GuiConfig>))
        .route(
            "/config/telegram",
            patch(patch_any_config::<config::TelegramConfig>),
        )
        // Import/Export endpoints
        .route("/config/export", post(export_config))
        .route("/config/import/preview", post(import_config_preview))
        .route("/config/import", post(import_config))
        // Utility endpoints
        .route("/config/reload", post(reload_config_from_disk))
        .route("/config/reset", post(reset_config_to_defaults))
        .route("/config/diff", get(get_config_diff))
}

// ============================================================================
// HANDLERS - GET ENDPOINTS
// ============================================================================

/// GET /api/config - Get full configuration (all sections)
async fn get_full_config() -> Response {
    let data = config::with_config(|cfg| FullConfigResponse {
        rpc: cfg.rpc.clone(),
        trader: cfg.trader.clone(),
        positions: cfg.positions.clone(),
        filtering: cfg.filtering.clone(),
        swaps: cfg.swaps.clone(),
        tokens: cfg.tokens.clone(),
        sol_price: cfg.sol_price.clone(),
        events: cfg.events.clone(),
        services: cfg.services.clone(),
        monitoring: cfg.monitoring.clone(),
        ohlcv: cfg.ohlcv.clone(),
        gui: cfg.gui.clone(),
        telegram: cfg.telegram.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/rpc - Get RPC configuration
async fn get_rpc_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.rpc.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/trader - Get trader configuration
async fn get_trader_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.trader.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/positions - Get positions configuration
async fn get_positions_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.positions.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/filtering - Get filtering configuration
async fn get_filtering_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.filtering.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/swaps - Get swaps configuration
async fn get_swaps_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.swaps.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/tokens - Get tokens configuration
async fn get_tokens_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.tokens.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/sol_price - Get SOL price service configuration
async fn get_sol_price_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.sol_price.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/summary - Get summary display configuration
async fn get_summary_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: serde_json::json!({}),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/events - Get events system configuration
async fn get_events_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.events.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/services - Get services configuration
async fn get_services_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.services.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/monitoring - Get monitoring configuration
async fn get_monitoring_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.monitoring.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/ohlcv - Get OHLCV configuration
async fn get_ohlcv_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.ohlcv.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/gui - Get GUI/Dashboard configuration
async fn get_gui_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.gui.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/telegram - Get Telegram configuration
async fn get_telegram_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.telegram.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/metadata - Get configuration metadata for UI rendering
async fn get_config_metadata() -> Response {
    let response = ConfigMetadataResponse {
        data: collect_config_metadata(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    success_response(response)
}

// ============================================================================
// HANDLERS - PATCH ENDPOINTS (Config Updates)
// ============================================================================

/// Generic PATCH handler for any config section
/// Accepts partial JSON updates - only fields provided will be updated
async fn patch_any_config<T>(Json(updates): Json<serde_json::Value>) -> Response
where
    T: serde::Serialize + serde::de::DeserializeOwned + Clone + std::fmt::Debug + 'static,
{
    // Determine which section based on type T
    let section_name = std::any::type_name::<T>()
        .split("::")
        .last()
        .unwrap_or("unknown");

    // Prepare the merged config outside closure
    let merge_result: Result<(), String> = (|| {
        // Get current config
        let current_section = config::with_config(|cfg| match section_name {
            "TraderConfig" => serde_json::to_value(&cfg.trader).ok(),
            "PositionsConfig" => serde_json::to_value(&cfg.positions).ok(),
            "FilteringConfig" => serde_json::to_value(&cfg.filtering).ok(),
            "SwapsConfig" => serde_json::to_value(&cfg.swaps).ok(),
            "TokensConfig" => serde_json::to_value(&cfg.tokens).ok(),
            "RpcConfig" => serde_json::to_value(&cfg.rpc).ok(),
            "SolPriceConfig" => serde_json::to_value(&cfg.sol_price).ok(),
            "EventsConfig" => serde_json::to_value(&cfg.events).ok(),
            "ServicesConfig" => serde_json::to_value(&cfg.services).ok(),
            "MonitoringConfig" => serde_json::to_value(&cfg.monitoring).ok(),
            "OhlcvConfig" => serde_json::to_value(&cfg.ohlcv).ok(),
            "GuiConfig" => serde_json::to_value(&cfg.gui).ok(),
            "TelegramConfig" => serde_json::to_value(&cfg.telegram).ok(),
            _ => None,
        });

        let mut section_json = current_section.ok_or("Failed to serialize current config")?;

        // Merge updates into existing config
        if let (Some(section_obj), Some(updates_obj)) =
            (section_json.as_object_mut(), updates.as_object())
        {
            for (key, value) in updates_obj {
                section_obj.insert(key.clone(), value.clone());
            }
        }

        // Now update the config with merged values
        let section_json = section_json; // Make immutable for the closure
        let section_name = section_name; // Capture for closure

        // Validate and deserialize before updating (fail fast on errors)
        match section_name {
            "TraderConfig" => {
                let new_config: config::TraderConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid TraderConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.trader = new_config;
                    },
                    true,
                )?;
            }
            "PositionsConfig" => {
                let new_config: config::PositionsConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid PositionsConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.positions = new_config;
                    },
                    true,
                )?;
            }
            "FilteringConfig" => {
                let new_config: config::FilteringConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid FilteringConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.filtering = new_config;
                    },
                    true,
                )?;
            }
            "SwapsConfig" => {
                let new_config: config::SwapsConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid SwapsConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.swaps = new_config;
                    },
                    true,
                )?;
            }
            "TokensConfig" => {
                let new_config: config::TokensConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid TokensConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.tokens = new_config;
                    },
                    true,
                )?;
            }
            "RpcConfig" => {
                let new_config: config::RpcConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid RpcConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.rpc = new_config;
                    },
                    true,
                )?;
            }
            "SolPriceConfig" => {
                let new_config: config::SolPriceConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid SolPriceConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.sol_price = new_config;
                    },
                    true,
                )?;
            }
            "EventsConfig" => {
                let new_config: config::EventsConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid EventsConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.events = new_config;
                    },
                    true,
                )?;
            }
            "ServicesConfig" => {
                let new_config: config::ServicesConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid ServicesConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.services = new_config;
                    },
                    true,
                )?;
            }
            "MonitoringConfig" => {
                let new_config: config::MonitoringConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid MonitoringConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.monitoring = new_config;
                    },
                    true,
                )?;
            }
            "OhlcvConfig" => {
                let new_config: config::OhlcvConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid OhlcvConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.ohlcv = new_config;
                    },
                    true,
                )?;
            }
            "GuiConfig" => {
                let new_config: config::GuiConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid GuiConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.gui = new_config;
                    },
                    true,
                )?;
            }
            "TelegramConfig" => {
                let new_config: config::TelegramConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid TelegramConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.telegram = new_config;
                    },
                    true,
                )?;
            }
            _ => {
                return Err(format!("Unknown config section: {}", section_name));
            }
        }

        Ok(())
    })();

    match merge_result {
        Ok(()) => {
            let response = UpdateResponse {
                message: format!("{} updated successfully", section_name),
                saved_to_disk: true,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::BAD_REQUEST,
            "CONFIG_UPDATE_FAILED",
            &format!("Failed to update config: {}", e),
            None,
        ),
    }
}

// ============================================================================
// UTILITY ENDPOINTS
// ============================================================================

// ============================================================================
// UPDATE RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UpdateResponse {
    pub message: String,
    pub saved_to_disk: bool,
    pub timestamp: String,
}

// ============================================================================
// UTILITY ENDPOINTS
// ============================================================================

/// POST /api/config/reload - Reload configuration from disk
async fn reload_config_from_disk() -> Response {
    match config::reload_config() {
        Ok(_) => success_response(UpdateResponse {
            message: "Configuration reloaded from disk successfully".to_string(),
            saved_to_disk: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "RELOAD_FAILED",
            &format!("Failed to reload config: {}", e),
            None,
        ),
    }
}

/// POST /api/config/reset - Reset configuration to defaults
async fn reset_config_to_defaults() -> Response {
    let (wallet_encrypted, wallet_nonce, coingecko_api_key) = config::with_config(|cfg| {
        (
            cfg.wallet_encrypted.clone(),
            cfg.wallet_nonce.clone(),
            cfg.tokens.discovery.coingecko.api_key.clone(),
        )
    });

    let result = config::update_config_section(
        |cfg| {
            // Keep secrets that are not recoverable via the UI while resetting everything else.
            let mut fresh = config::Config::default();
            if !wallet_encrypted.is_empty() && !wallet_nonce.is_empty() {
                fresh.wallet_encrypted = wallet_encrypted.clone();
                fresh.wallet_nonce = wallet_nonce.clone();
            }
            fresh.tokens.discovery.coingecko.api_key = coingecko_api_key.clone();
            *cfg = fresh;
        },
        true, // Save to disk
    );

    match result {
        Ok(_) => success_response(UpdateResponse {
            message: "Configuration reset to defaults successfully".to_string(),
            saved_to_disk: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "RESET_FAILED",
            &format!("Failed to reset config: {}", e),
            None,
        ),
    }
}

/// GET /api/config/diff - Compare in-memory config with disk version
async fn get_config_diff() -> Response {
    // Load current in-memory config
    let memory_config = config::get_config_clone();

    // Try to load disk config
    let config_path = crate::paths::get_config_path();
    let disk_result = std::fs::read_to_string(&config_path);

    match disk_result {
        Ok(contents) => {
            match toml::from_str::<config::Config>(&contents) {
                Ok(disk_config) => {
                    fn sanitize_config_json(value: &mut serde_json::Value) {
                        if let Some(obj) = value.as_object_mut() {
                            // Remove encrypted wallet fields from comparison output
                            obj.remove("wallet_encrypted");
                            obj.remove("wallet_nonce");
                        }
                    }

                    // Compare using JSON serialization for accurate comparison
                    let mut memory_json = serde_json::to_value(&memory_config)
                        .unwrap_or_else(|_| serde_json::Value::Null);
                    let mut disk_json = serde_json::to_value(&disk_config)
                        .unwrap_or_else(|_| serde_json::Value::Null);

                    sanitize_config_json(&mut memory_json);
                    sanitize_config_json(&mut disk_json);

                    let has_changes = memory_json != disk_json;

                    #[derive(Serialize)]
                    struct DiffResponse {
                        has_changes: bool,
                        memory: serde_json::Value,
                        disk: serde_json::Value,
                        memory_timestamp: String,
                        disk_file: String,
                        message: String,
                    }

                    success_response(DiffResponse {
                        has_changes,
                        memory: memory_json,
                        disk: disk_json,
                        memory_timestamp: chrono::Utc::now().to_rfc3339(),
                        disk_file: config_path.to_string_lossy().to_string(),
                        message: if has_changes {
                            "In-memory configuration differs from disk version".to_string()
                        } else {
                            "In-memory configuration matches disk version".to_string()
                        },
                    })
                }
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "PARSE_ERROR",
                    &format!("Failed to parse disk config: {}", e),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "READ_ERROR",
            &format!("Failed to read disk config: {}", e),
            None,
        ),
    }
}

// ============================================================================
// IMPORT/EXPORT ENDPOINTS
// ============================================================================

/// List of all config sections that can be imported/exported
const CONFIG_SECTIONS: &[&str] = &[
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
];

/// Sensitive fields that should be sanitized on export (path format: "section.nested.field")
const SENSITIVE_FIELDS: &[(&str, &[&str])] = &[
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
];

/// Sanitize a section by removing/masking sensitive fields
fn sanitize_section(section_name: &str, value: &mut serde_json::Value) {
    for (section, fields) in SENSITIVE_FIELDS {
        if *section != section_name {
            continue;
        }
        for field_path in *fields {
            remove_nested_field(value, field_path);
        }
    }
}

/// Remove a nested field by dot-separated path (e.g., "dashboard.lockscreen.password_hash")
fn remove_nested_field(value: &mut serde_json::Value, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - remove the field
            if let Some(obj) = current.as_object_mut() {
                obj.remove(*part);
            }
        } else {
            // Navigate to nested object
            if let Some(obj) = current.as_object_mut() {
                if let Some(next) = obj.get_mut(*part) {
                    current = next;
                } else {
                    return; // Path not found
                }
            } else {
                return; // Not an object
            }
        }
    }
}

/// Check if a nested field exists by dot-separated path
fn has_nested_field(value: &serde_json::Value, path: &str) -> bool {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return false;
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if let Some(obj) = current.as_object() {
            if i == parts.len() - 1 {
                // Last part - check if field exists and is non-empty
                if let Some(val) = obj.get(*part) {
                    return !val.is_null()
                        && !(val.is_string() && val.as_str().unwrap_or("").is_empty());
                }
                return false;
            } else if let Some(next) = obj.get(*part) {
                current = next;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    false
}

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

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ExportConfigResponse {
    pub config: serde_json::Value,
    pub sections: Vec<String>,
    pub exported_at: String,
    pub version: String,
}

/// POST /api/config/export - Export configuration with options
async fn export_config(Json(request): Json<ExportConfigRequest>) -> Response {
    // Determine which sections to export
    let sections_to_export: Vec<&str> = match &request.sections {
        Some(sections) if !sections.is_empty() => sections
            .iter()
            .filter(|s| CONFIG_SECTIONS.contains(&s.as_str()))
            .map(|s| s.as_str())
            .collect(),
        _ => CONFIG_SECTIONS.to_vec(),
    };

    // Filter out GUI if requested
    let sections_to_export: Vec<&str> = if !request.include_gui {
        sections_to_export
            .into_iter()
            .filter(|s| *s != "gui")
            .collect()
    } else {
        sections_to_export
    };

    // Build the export object
    let mut export_obj = serde_json::Map::new();
    let sanitize = request.sanitize_secrets;

    config::with_config(|cfg| {
        for section in &sections_to_export {
            let section_value = match *section {
                "rpc" => serde_json::to_value(&cfg.rpc).ok(),
                "trader" => serde_json::to_value(&cfg.trader).ok(),
                "positions" => serde_json::to_value(&cfg.positions).ok(),
                "filtering" => serde_json::to_value(&cfg.filtering).ok(),
                "swaps" => serde_json::to_value(&cfg.swaps).ok(),
                "tokens" => serde_json::to_value(&cfg.tokens).ok(),
                "sol_price" => serde_json::to_value(&cfg.sol_price).ok(),
                "events" => serde_json::to_value(&cfg.events).ok(),
                "services" => serde_json::to_value(&cfg.services).ok(),
                "monitoring" => serde_json::to_value(&cfg.monitoring).ok(),
                "ohlcv" => serde_json::to_value(&cfg.ohlcv).ok(),
                "gui" => serde_json::to_value(&cfg.gui).ok(),
                "telegram" => serde_json::to_value(&cfg.telegram).ok(),
                _ => None,
            };

            if let Some(mut value) = section_value {
                // Sanitize sensitive fields if requested
                if sanitize {
                    sanitize_section(section, &mut value);
                }
                export_obj.insert(section.to_string(), value);
            }
        }
    });

    // Add metadata if requested
    if request.include_metadata {
        export_obj.insert(
            "timestamp".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );
    }

    success_response(ExportConfigResponse {
        config: serde_json::Value::Object(export_obj),
        sections: sections_to_export.iter().map(|s| s.to_string()).collect(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
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

/// Helper to get section label for display
fn get_section_label(section: &str) -> String {
    match section {
        "rpc" => "RPC".to_string(),
        "trader" => "Auto Trader".to_string(),
        "positions" => "Positions".to_string(),
        "filtering" => "Filtering".to_string(),
        "swaps" => "Swaps".to_string(),
        "tokens" => "Tokens".to_string(),
        "sol_price" => "SOL Price".to_string(),
        "events" => "Events".to_string(),
        "services" => "Services".to_string(),
        "monitoring" => "Monitoring".to_string(),
        "ohlcv" => "OHLCV".to_string(),
        "gui" => "GUI".to_string(),
        "telegram" => "Telegram".to_string(),
        _ => section.to_string(),
    }
}

/// Count fields in a JSON value (recursive for objects)
fn count_fields(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => map.len(),
        _ => 0,
    }
}

/// Compare two JSON values and return field changes
fn compare_values(
    current: &serde_json::Value,
    imported: &serde_json::Value,
    prefix: &str,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if let (Some(curr_obj), Some(imp_obj)) = (current.as_object(), imported.as_object()) {
        for (key, imp_val) in imp_obj {
            let field_path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            match curr_obj.get(key) {
                Some(curr_val) => {
                    if curr_val != imp_val {
                        // Check if both are objects for recursive comparison
                        if curr_val.is_object() && imp_val.is_object() {
                            changes.extend(compare_values(curr_val, imp_val, &field_path));
                        } else {
                            changes.push(FieldChange {
                                field: field_path,
                                current: curr_val.clone(),
                                imported: imp_val.clone(),
                            });
                        }
                    }
                }
                None => {
                    // New field being added
                    changes.push(FieldChange {
                        field: field_path,
                        current: serde_json::Value::Null,
                        imported: imp_val.clone(),
                    });
                }
            }
        }
    }

    changes
}

/// POST /api/config/import/preview - Preview what would be imported
async fn import_config_preview(Json(request): Json<ImportConfigPreviewRequest>) -> Response {
    let imported = request.config;
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    let mut total_changes = 0;

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Config must be a JSON object",
                None,
            );
        }
    };

    // Check for unknown sections
    for key in imported_obj.keys() {
        if key != "timestamp" && !CONFIG_SECTIONS.contains(&key.as_str()) {
            warnings.push(format!("Unknown section '{}' will be ignored", key));
        }
    }

    // Check for security-sensitive fields being imported
    for (section, fields) in SENSITIVE_FIELDS {
        if let Some(section_val) = imported_obj.get(*section) {
            for field_path in *fields {
                if has_nested_field(section_val, field_path) {
                    warnings.push(format!(
                        "⚠️ Security warning: Importing '{}.{}' may overwrite authentication settings",
                        section, field_path
                    ));
                }
            }
        }
    }

    // Analyze each known section
    for section in CONFIG_SECTIONS {
        let section_value = imported_obj.get(*section);
        let present = section_value.is_some();

        if !present {
            sections.push(SectionPreview {
                name: section.to_string(),
                label: get_section_label(section),
                present: false,
                valid: true,
                field_count: 0,
                error: None,
                changes: Vec::new(),
            });
            continue;
        }

        let value = section_value.unwrap();
        let field_count = count_fields(value);

        // Validate by attempting to deserialize
        let validation_result: Result<(), String> = match *section {
            "rpc" => serde_json::from_value::<config::RpcConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "trader" => serde_json::from_value::<config::TraderConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "positions" => serde_json::from_value::<config::PositionsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "filtering" => serde_json::from_value::<config::FilteringConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "swaps" => serde_json::from_value::<config::SwapsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "tokens" => serde_json::from_value::<config::TokensConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "sol_price" => serde_json::from_value::<config::SolPriceConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "events" => serde_json::from_value::<config::EventsConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "services" => serde_json::from_value::<config::ServicesConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "monitoring" => serde_json::from_value::<config::MonitoringConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "ohlcv" => serde_json::from_value::<config::OhlcvConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "gui" => serde_json::from_value::<config::GuiConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "telegram" => serde_json::from_value::<config::TelegramConfig>(value.clone())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            _ => Ok(()),
        };

        // Get current config for comparison
        let current_value = config::with_config(|cfg| match *section {
            "rpc" => serde_json::to_value(&cfg.rpc).ok(),
            "trader" => serde_json::to_value(&cfg.trader).ok(),
            "positions" => serde_json::to_value(&cfg.positions).ok(),
            "filtering" => serde_json::to_value(&cfg.filtering).ok(),
            "swaps" => serde_json::to_value(&cfg.swaps).ok(),
            "tokens" => serde_json::to_value(&cfg.tokens).ok(),
            "sol_price" => serde_json::to_value(&cfg.sol_price).ok(),
            "events" => serde_json::to_value(&cfg.events).ok(),
            "services" => serde_json::to_value(&cfg.services).ok(),
            "monitoring" => serde_json::to_value(&cfg.monitoring).ok(),
            "ohlcv" => serde_json::to_value(&cfg.ohlcv).ok(),
            "gui" => serde_json::to_value(&cfg.gui).ok(),
            "telegram" => serde_json::to_value(&cfg.telegram).ok(),
            _ => None,
        });

        let changes = if let Some(curr) = current_value {
            compare_values(&curr, value, "")
        } else {
            Vec::new()
        };

        total_changes += changes.len();

        sections.push(SectionPreview {
            name: section.to_string(),
            label: get_section_label(section),
            present: true,
            valid: validation_result.is_ok(),
            field_count,
            error: validation_result.err(),
            changes,
        });
    }

    let all_valid = sections.iter().filter(|s| s.present).all(|s| s.valid);

    success_response(ImportPreviewResponse {
        valid: all_valid,
        sections,
        warnings,
        total_changes,
    })
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

/// Helper to apply a section value to a config struct (used for validation before commit)
fn apply_section_to_config(
    cfg: &mut config::Config,
    section: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    match section {
        "rpc" => {
            cfg.rpc =
                serde_json::from_value(value).map_err(|e| format!("Invalid RpcConfig: {}", e))?;
        }
        "trader" => {
            cfg.trader = serde_json::from_value(value)
                .map_err(|e| format!("Invalid TraderConfig: {}", e))?;
        }
        "positions" => {
            cfg.positions = serde_json::from_value(value)
                .map_err(|e| format!("Invalid PositionsConfig: {}", e))?;
        }
        "filtering" => {
            cfg.filtering = serde_json::from_value(value)
                .map_err(|e| format!("Invalid FilteringConfig: {}", e))?;
        }
        "swaps" => {
            cfg.swaps =
                serde_json::from_value(value).map_err(|e| format!("Invalid SwapsConfig: {}", e))?;
        }
        "tokens" => {
            cfg.tokens = serde_json::from_value(value)
                .map_err(|e| format!("Invalid TokensConfig: {}", e))?;
        }
        "sol_price" => {
            cfg.sol_price = serde_json::from_value(value)
                .map_err(|e| format!("Invalid SolPriceConfig: {}", e))?;
        }
        "events" => {
            cfg.events = serde_json::from_value(value)
                .map_err(|e| format!("Invalid EventsConfig: {}", e))?;
        }
        "services" => {
            cfg.services = serde_json::from_value(value)
                .map_err(|e| format!("Invalid ServicesConfig: {}", e))?;
        }
        "monitoring" => {
            cfg.monitoring = serde_json::from_value(value)
                .map_err(|e| format!("Invalid MonitoringConfig: {}", e))?;
        }
        "ohlcv" => {
            cfg.ohlcv =
                serde_json::from_value(value).map_err(|e| format!("Invalid OhlcvConfig: {}", e))?;
        }
        "gui" => {
            cfg.gui =
                serde_json::from_value(value).map_err(|e| format!("Invalid GuiConfig: {}", e))?;
        }
        "telegram" => {
            cfg.telegram = serde_json::from_value(value)
                .map_err(|e| format!("Invalid TelegramConfig: {}", e))?;
        }
        _ => return Err(format!("Unknown section: {}", section)),
    }
    Ok(())
}

/// POST /api/config/import - Import configuration
async fn import_config(Json(request): Json<ImportConfigRequest>) -> Response {
    let imported = request.config;

    let imported_obj = match imported.as_object() {
        Some(obj) => obj,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "Config must be a JSON object",
                None,
            );
        }
    };

    // Determine which sections to import
    let sections_to_import: Vec<String> = match &request.sections {
        Some(sections) if !sections.is_empty() => sections
            .iter()
            .filter(|s| {
                CONFIG_SECTIONS.contains(&s.as_str()) && imported_obj.contains_key(s.as_str())
            })
            .cloned()
            .collect(),
        _ => imported_obj
            .keys()
            .filter(|k| CONFIG_SECTIONS.contains(&k.as_str()))
            .cloned()
            .collect(),
    };

    if sections_to_import.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_SECTIONS",
            "No valid sections found to import",
            None,
        );
    }

    // PHASE 1: Build a candidate config by cloning current and applying all changes
    // This allows us to validate BEFORE modifying the live config
    let mut candidate_config = config::get_config_clone();
    let mut imported_sections = Vec::new();
    let mut errors = Vec::new();

    for section in &sections_to_import {
        let value = match imported_obj.get(section) {
            Some(v) => v.clone(),
            None => continue,
        };

        let result: Result<serde_json::Value, String> = (|| {
            // Get current config section for merging if needed
            let final_value = if request.merge {
                let current = match section.as_str() {
                    "rpc" => serde_json::to_value(&candidate_config.rpc).ok(),
                    "trader" => serde_json::to_value(&candidate_config.trader).ok(),
                    "positions" => serde_json::to_value(&candidate_config.positions).ok(),
                    "filtering" => serde_json::to_value(&candidate_config.filtering).ok(),
                    "swaps" => serde_json::to_value(&candidate_config.swaps).ok(),
                    "tokens" => serde_json::to_value(&candidate_config.tokens).ok(),
                    "sol_price" => serde_json::to_value(&candidate_config.sol_price).ok(),
                    "events" => serde_json::to_value(&candidate_config.events).ok(),
                    "services" => serde_json::to_value(&candidate_config.services).ok(),
                    "monitoring" => serde_json::to_value(&candidate_config.monitoring).ok(),
                    "ohlcv" => serde_json::to_value(&candidate_config.ohlcv).ok(),
                    "gui" => serde_json::to_value(&candidate_config.gui).ok(),
                    "telegram" => serde_json::to_value(&candidate_config.telegram).ok(),
                    _ => None,
                };

                if let Some(mut curr) = current {
                    // Merge: imported values override current
                    if let (Some(curr_obj), Some(imp_obj)) =
                        (curr.as_object_mut(), value.as_object())
                    {
                        for (key, val) in imp_obj {
                            curr_obj.insert(key.clone(), val.clone());
                        }
                    }
                    curr
                } else {
                    value
                }
            } else {
                value
            };

            Ok(final_value)
        })();

        match result {
            Ok(final_value) => {
                // Apply to candidate config
                if let Err(e) = apply_section_to_config(&mut candidate_config, section, final_value)
                {
                    errors.push(format!("{}: {}", section, e));
                } else {
                    imported_sections.push(section.clone());
                }
            }
            Err(e) => errors.push(format!("{}: {}", section, e)),
        }
    }

    // PHASE 2: Validate the full candidate config BEFORE committing
    // This catches cross-field validation errors (e.g., DCA settings require valid thresholds)
    if !imported_sections.is_empty() {
        if let Err(validation_error) = config::validate_config(&candidate_config) {
            // Validation failed - don't commit anything
            return error_response(
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                &format!(
                    "Config validation failed: {}. No changes were applied.",
                    validation_error
                ),
                None,
            );
        }
    }

    // PHASE 3: Validation passed - commit the changes atomically
    if !imported_sections.is_empty() {
        if let Err(e) = config::update_config_section(
            |cfg| {
                // Apply all validated sections at once
                for section in &imported_sections {
                    match section.as_str() {
                        "rpc" => cfg.rpc = candidate_config.rpc.clone(),
                        "trader" => cfg.trader = candidate_config.trader.clone(),
                        "positions" => cfg.positions = candidate_config.positions.clone(),
                        "filtering" => cfg.filtering = candidate_config.filtering.clone(),
                        "swaps" => cfg.swaps = candidate_config.swaps.clone(),
                        "tokens" => cfg.tokens = candidate_config.tokens.clone(),
                        "sol_price" => cfg.sol_price = candidate_config.sol_price.clone(),
                        "events" => cfg.events = candidate_config.events.clone(),
                        "services" => cfg.services = candidate_config.services.clone(),
                        "monitoring" => cfg.monitoring = candidate_config.monitoring.clone(),
                        "ohlcv" => cfg.ohlcv = candidate_config.ohlcv.clone(),
                        "gui" => cfg.gui = candidate_config.gui.clone(),
                        "telegram" => cfg.telegram = candidate_config.telegram.clone(),
                        _ => {}
                    }
                }
            },
            false, // Don't save to disk yet
        ) {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "COMMIT_FAILED",
                &format!("Failed to commit config changes: {}", e),
                None,
            );
        }
    }

    // PHASE 4: Save to disk if requested and no errors
    let saved_to_disk =
        if request.save_to_disk && !imported_sections.is_empty() && errors.is_empty() {
            match config::save_config(None) {
                Ok(()) => true,
                Err(e) => {
                    errors.push(format!("Failed to save to disk: {}", e));
                    false
                }
            }
        } else {
            false
        };

    if imported_sections.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "IMPORT_FAILED",
            &format!("Failed to import config: {}", errors.join(", ")),
            None,
        );
    }

    let message = if errors.is_empty() {
        format!(
            "Successfully imported {} section(s)",
            imported_sections.len()
        )
    } else {
        format!(
            "Imported {} section(s) with {} warning(s): {}",
            imported_sections.len(),
            errors.len(),
            errors.join(", ")
        )
    };

    success_response(ImportConfigResponse {
        success: true,
        message,
        imported_sections,
        saved_to_disk,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
