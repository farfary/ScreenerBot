//! Config API Getters
//!
//! GET endpoints for viewing config sections, plus generic PATCH handler.

use axum::http::StatusCode;
use axum::{response::Response, Json};

use crate::config;
use crate::config::metadata::collect_config_metadata;
use crate::config::schemas::{default_tabs, TabConfig};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// ============================================================================
// HANDLERS - GET ENDPOINTS
// ============================================================================

/// GET /api/config - Get full configuration (all sections)
pub async fn get_full_config() -> Response {
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
        ai: cfg.ai.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/rpc - Get RPC configuration
pub async fn get_rpc_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.rpc.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/trader - Get trader configuration
pub async fn get_trader_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.trader.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/positions - Get positions configuration
pub async fn get_positions_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.positions.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/filtering - Get filtering configuration
pub async fn get_filtering_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.filtering.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/swaps - Get swaps configuration
pub async fn get_swaps_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.swaps.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/tokens - Get tokens configuration
pub async fn get_tokens_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.tokens.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/pools - Get pools configuration
pub async fn get_pools_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.pools.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/maintenance - Get maintenance configuration
pub async fn get_maintenance_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.maintenance.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/sol_price - Get SOL price service configuration
pub async fn get_sol_price_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.sol_price.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/summary - Get summary display configuration
pub async fn get_summary_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: serde_json::json!({}),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/events - Get events system configuration
pub async fn get_events_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.events.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/services - Get services configuration
pub async fn get_services_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.services.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/monitoring - Get monitoring configuration
pub async fn get_monitoring_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.monitoring.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/ohlcv - Get OHLCV configuration
pub async fn get_ohlcv_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.ohlcv.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/gui - Get GUI/Dashboard configuration
pub async fn get_gui_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.gui.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/gui/defaults - Get default GUI configuration (for reset operations)
pub async fn get_gui_defaults() -> Response {
    let response = GuiDefaultsResponse {
        success: true,
        data: GuiDefaultsData {
            tabs: default_tabs(),
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    success_response(response)
}

/// GET /api/config/telegram - Get Telegram configuration
pub async fn get_telegram_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.telegram.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/ai - Get AI configuration
pub async fn get_ai_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.ai.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(data)
}

/// GET /api/config/strategies - Get strategies configuration
pub async fn get_strategies_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.strategies.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    success_response(data)
}

/// GET /api/config/holder_watch - Get holder watch configuration
pub async fn get_holder_watch_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.holder_watch.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    success_response(data)
}

/// GET /api/config/wallet - Get wallet configuration
pub async fn get_wallet_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.wallet.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    success_response(data)
}

/// GET /api/config/performance - Get performance configuration
pub async fn get_performance_config() -> Response {
    let data = config::with_config(|cfg| ConfigResponse {
        data: cfg.performance.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    success_response(data)
}

/// GET /api/config/metadata - Get configuration metadata for UI rendering
pub async fn get_config_metadata() -> Response {
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
pub async fn patch_any_config<T>(Json(updates): Json<serde_json::Value>) -> Response
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
            "PoolsConfig" => serde_json::to_value(&cfg.pools).ok(),
            "MaintenanceConfig" => serde_json::to_value(&cfg.maintenance).ok(),
            "RpcConfig" => serde_json::to_value(&cfg.rpc).ok(),
            "SolPriceConfig" => serde_json::to_value(&cfg.sol_price).ok(),
            "EventsConfig" => serde_json::to_value(&cfg.events).ok(),
            "ServicesConfig" => serde_json::to_value(&cfg.services).ok(),
            "MonitoringConfig" => serde_json::to_value(&cfg.monitoring).ok(),
            "OhlcvConfig" => serde_json::to_value(&cfg.ohlcv).ok(),
            "GuiConfig" => serde_json::to_value(&cfg.gui).ok(),
            "TelegramConfig" => serde_json::to_value(&cfg.telegram).ok(),
            "AiConfig" => serde_json::to_value(&cfg.ai).ok(),
            "StrategiesConfig" => serde_json::to_value(&cfg.strategies).ok(),
            "HolderWatchConfig" => serde_json::to_value(&cfg.holder_watch).ok(),
            "WalletConfig" => serde_json::to_value(&cfg.wallet).ok(),
            "PerformanceConfig" => serde_json::to_value(&cfg.performance).ok(),
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
            "PoolsConfig" => {
                let new_config: config::PoolsConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid PoolsConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.pools = new_config;
                    },
                    true,
                )?;
            }
            "MaintenanceConfig" => {
                let new_config: config::MaintenanceConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid MaintenanceConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.maintenance = new_config;
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
            "AiConfig" => {
                let new_config: config::AiConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid AiConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.ai = new_config;
                    },
                    true,
                )?;
            }
            "StrategiesConfig" => {
                let new_config: config::StrategiesConfig =
                    serde_json::from_value(section_json)
                        .map_err(|e| format!("Invalid StrategiesConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.strategies = new_config;
                    },
                    true,
                )?;
            }
            "HolderWatchConfig" => {
                let new_config: config::HolderWatchConfig =
                    serde_json::from_value(section_json)
                        .map_err(|e| format!("Invalid HolderWatchConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.holder_watch = new_config;
                    },
                    true,
                )?;
            }
            "WalletConfig" => {
                let new_config: config::WalletConfig = serde_json::from_value(section_json)
                    .map_err(|e| format!("Invalid WalletConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.wallet = new_config;
                    },
                    true,
                )?;
            }
            "PerformanceConfig" => {
                let new_config: config::PerformanceConfig =
                    serde_json::from_value(section_json)
                        .map_err(|e| format!("Invalid PerformanceConfig: {}", e))?;
                config::update_config_section(
                    |cfg| {
                        cfg.performance = new_config;
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
