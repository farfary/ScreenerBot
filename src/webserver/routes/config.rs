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
    pub webserver: config::WebserverConfig,
    pub services: config::ServicesConfig,
    pub monitoring: config::MonitoringConfig,
    pub ohlcv: config::OhlcvConfig,
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
        .route("/config/webserver", get(get_webserver_config))
        .route("/config/services", get(get_services_config))
        .route("/config/monitoring", get(get_monitoring_config))
        .route("/config/ohlcv", get(get_ohlcv_config))
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
            "/config/webserver",
            patch(patch_any_config::<config::WebserverConfig>),
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
        webserver: cfg.webserver.clone(),
        services: cfg.services.clone(),
        monitoring: cfg.monitoring.clone(),
        ohlcv: cfg.ohlcv.clone(),
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
        data: cfg.summary.clone(),
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

/// GET /api/config/webserver - Get webserver configuration
async fn get_webserver_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.webserver.clone(),
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
            "WebserverConfig" => serde_json::to_value(&cfg.webserver).ok(),
            "ServicesConfig" => serde_json::to_value(&cfg.services).ok(),
            "MonitoringConfig" => serde_json::to_value(&cfg.monitoring).ok(),
            "OhlcvConfig" => serde_json::to_value(&cfg.ohlcv).ok(),
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
            "WebserverConfig" => {
                let new_config: config::WebserverConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid WebserverConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.webserver = new_config;
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
    let result = config::update_config_section(
        |cfg| {
            *cfg = config::Config::default();
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
    let disk_result = std::fs::read_to_string(config::CONFIG_FILE_PATH);

    match disk_result {
        Ok(contents) => {
            match toml::from_str::<config::Config>(&contents) {
                Ok(disk_config) => {
                    fn sanitize_config_json(value: &mut serde_json::Value) {
                        if let Some(obj) = value.as_object_mut() {
                            obj.remove("main_wallet_private");
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
                        disk_file: config::CONFIG_FILE_PATH.to_string(),
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
