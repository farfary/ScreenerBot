/// Configuration API routes
///
/// Provides REST API endpoints for viewing and managing bot configuration.
/// All responses follow the standard ScreenerBot API format.

use axum::{ extract::State, response::Response, routing::{ get, patch, post }, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;
use axum::http::StatusCode;

use crate::webserver::state::AppState;
use crate::webserver::utils::{ success_response, error_response };
use crate::config;

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
    pub summary: config::SummaryConfig,
    pub events: config::EventsConfig,
    pub webserver: config::WebserverConfig,
    pub services: config::ServicesConfig,
    pub monitoring: config::MonitoringConfig,
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
        // PATCH endpoints - Partial updates (use JSON with only fields to update)
        .route("/config/trader", patch(patch_any_config::<config::TraderConfig>))
        .route("/config/positions", patch(patch_any_config::<config::PositionsConfig>))
        .route("/config/filtering", patch(patch_any_config::<config::FilteringConfig>))
        .route("/config/swaps", patch(patch_any_config::<config::SwapsConfig>))
        .route("/config/tokens", patch(patch_any_config::<config::TokensConfig>))
        .route("/config/rpc", patch(patch_any_config::<config::RpcConfig>))
        .route("/config/sol_price", patch(patch_any_config::<config::SolPriceConfig>))
        .route("/config/summary", patch(patch_any_config::<config::SummaryConfig>))
        .route("/config/events", patch(patch_any_config::<config::EventsConfig>))
        .route("/config/webserver", patch(patch_any_config::<config::WebserverConfig>))
        .route("/config/services", patch(patch_any_config::<config::ServicesConfig>))
        .route("/config/monitoring", patch(patch_any_config::<config::MonitoringConfig>))
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
        summary: cfg.summary.clone(),
        events: cfg.events.clone(),
        webserver: cfg.webserver.clone(),
        services: cfg.services.clone(),
        monitoring: cfg.monitoring.clone(),
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

// ============================================================================
// HANDLERS - PATCH ENDPOINTS (Config Updates)
// ============================================================================

/// Generic PATCH handler for any config section
/// Accepts partial JSON updates - only fields provided will be updated
async fn patch_any_config<T>(Json(updates): Json<serde_json::Value>) -> Response
    where T: serde::Serialize + serde::de::DeserializeOwned + Clone + std::fmt::Debug + 'static
{
    // Determine which section based on type T
    let section_name = std::any::type_name::<T>().split("::").last().unwrap_or("unknown");

    // Prepare the merged config outside closure
    let merge_result: Result<(), String> = (|| {
        // Get current config
        let current_section = config::with_config(|cfg| {
            match section_name {
                "TraderConfig" => serde_json::to_value(&cfg.trader).ok(),
                "PositionsConfig" => serde_json::to_value(&cfg.positions).ok(),
                "FilteringConfig" => serde_json::to_value(&cfg.filtering).ok(),
                "SwapsConfig" => serde_json::to_value(&cfg.swaps).ok(),
                "TokensConfig" => serde_json::to_value(&cfg.tokens).ok(),
                "RpcConfig" => serde_json::to_value(&cfg.rpc).ok(),
                "SolPriceConfig" => serde_json::to_value(&cfg.sol_price).ok(),
                "SummaryConfig" => serde_json::to_value(&cfg.summary).ok(),
                "EventsConfig" => serde_json::to_value(&cfg.events).ok(),
                "WebserverConfig" => serde_json::to_value(&cfg.webserver).ok(),
                "ServicesConfig" => serde_json::to_value(&cfg.services).ok(),
                "MonitoringConfig" => serde_json::to_value(&cfg.monitoring).ok(),
                _ => None,
            }
        });

        let mut section_json = current_section.ok_or("Failed to serialize current config")?;

        // Merge updates into existing config
        if
            let (Some(section_obj), Some(updates_obj)) = (
                section_json.as_object_mut(),
                updates.as_object(),
            )
        {
            for (key, value) in updates_obj {
                section_obj.insert(key.clone(), value.clone());
            }
        }

        // Now update the config with merged values
        let section_json = section_json; // Make immutable for the closure
        let section_name = section_name; // Capture for closure

        config::update_config_section(
            |cfg| {
                // Deserialize and update the appropriate section
                // We ignore errors here, they're returned from the outer closure
                let _ = match section_name {
                    "TraderConfig" => {
                        cfg.trader = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.trader.clone());
                    }
                    "PositionsConfig" => {
                        cfg.positions = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.positions.clone());
                    }
                    "FilteringConfig" => {
                        cfg.filtering = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.filtering.clone());
                    }
                    "SwapsConfig" => {
                        cfg.swaps = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.swaps.clone());
                    }
                    "TokensConfig" => {
                        cfg.tokens = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.tokens.clone());
                    }
                    "RpcConfig" => {
                        cfg.rpc = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.rpc.clone());
                    }
                    "SolPriceConfig" => {
                        cfg.sol_price = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.sol_price.clone());
                    }
                    "SummaryConfig" => {
                        cfg.summary = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.summary.clone());
                    }
                    "EventsConfig" => {
                        cfg.events = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.events.clone());
                    }
                    "WebserverConfig" => {
                        cfg.webserver = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.webserver.clone());
                    }
                    "ServicesConfig" => {
                        cfg.services = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.services.clone());
                    }
                    "MonitoringConfig" => {
                        cfg.monitoring = serde_json
                            ::from_value(section_json.clone())
                            .unwrap_or(cfg.monitoring.clone());
                    }
                    _ => {}
                };
            },
            true // save_to_disk
        )?;

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
        Err(e) =>
            error_response(
                StatusCode::BAD_REQUEST,
                "CONFIG_UPDATE_FAILED",
                &format!("Failed to update config: {}", e),
                None
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
        Ok(_) =>
            success_response(UpdateResponse {
                message: "Configuration reloaded from disk successfully".to_string(),
                saved_to_disk: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }),
        Err(e) =>
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "RELOAD_FAILED",
                &format!("Failed to reload config: {}", e),
                None
            ),
    }
}

/// POST /api/config/reset - Reset configuration to defaults
async fn reset_config_to_defaults() -> Response {
    let result = config::update_config_section(
        |cfg| {
            *cfg = config::Config::default();
        },
        true // Save to disk
    );

    match result {
        Ok(_) =>
            success_response(UpdateResponse {
                message: "Configuration reset to defaults successfully".to_string(),
                saved_to_disk: true,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }),
        Err(e) =>
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "RESET_FAILED",
                &format!("Failed to reset config: {}", e),
                None
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
                    let has_changes =
                        format!("{:?}", memory_config) != format!("{:?}", disk_config);

                    #[derive(Serialize)]
                    struct DiffResponse {
                        has_changes: bool,
                        memory_timestamp: String,
                        disk_file: String,
                        message: String,
                    }

                    success_response(DiffResponse {
                        has_changes,
                        memory_timestamp: chrono::Utc::now().to_rfc3339(),
                        disk_file: config::CONFIG_FILE_PATH.to_string(),
                        message: if has_changes {
                            "In-memory configuration differs from disk version".to_string()
                        } else {
                            "In-memory configuration matches disk version".to_string()
                        },
                    })
                }
                Err(e) =>
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PARSE_ERROR",
                        &format!("Failed to parse disk config: {}", e),
                        None
                    ),
            }
        }
        Err(e) =>
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "READ_ERROR",
                &format!("Failed to read disk config: {}", e),
                None
            ),
    }
}
