use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Serialize;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

use crate::logger::{self, LogTag};
use crate::paths;
// TODO: Re-enable when trader module is fully integrated
// use crate::trader::CRITICAL_OPERATIONS_IN_PROGRESS;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};
use crate::{
    global::{
        self, are_core_services_ready, get_pending_services, CONNECTIVITY_SYSTEM_READY,
        POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
    },
    services::get_service_manager,
    startup::{self, StartupServiceStatus},
    wallet,
};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct RebootResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BootStatusResponse {
    pub timestamp: String,
    pub initialization_required: bool,
    pub initialization_complete: bool,
    pub onboarding_complete: bool,
    pub core_services_ready: bool,
    pub ui_ready: bool,
    pub ready_for_requests: bool,
    pub pending_services: Vec<&'static str>,
    pub services_total: usize,
    pub services_running: usize,
    pub connectivity_ready: bool,
    pub tokens_ready: bool,
    pub positions_ready: bool,
    pub pools_ready: bool,
    pub transactions_ready: bool,
    pub boot_progress: Vec<StartupServiceStatus>,
    pub wallet_snapshot_ready: bool,
    pub wallet_last_updated: Option<String>,
    pub uptime_seconds: u64,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PathsResponse {
    pub base_directory: String,
    pub data_directory: String,
    pub logs_directory: String,
    pub cache_pool_directory: String,
    pub analysis_exports_directory: String,
    pub config_path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenPathResponse {
    pub opened: bool,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
    pub exists: bool,
}

#[derive(Debug, Serialize)]
pub struct DataStatsResponse {
    pub databases: Vec<DatabaseStats>,
    pub total_size_mb: f64,
    pub config_path: String,
    pub config_size_bytes: u64,
    pub data_directory: String,
    pub timestamp: String,
}

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// POST /api/system/reboot - Restart the entire screenerbot process
async fn reboot_system() -> Response {
    logger::debug(LogTag::Webserver, "System reboot requested via API");

    // TODO: Re-enable critical operations check when trader module is integrated
    // Wait for critical operations to complete (max 30 seconds)
    // let timeout = Instant::now() + Duration::from_secs(30);
    // while CRITICAL_OPERATIONS_IN_PROGRESS.load(Ordering::SeqCst) > 0 {
    //     if Instant::now() > timeout {
    //         logger::info(
    //             LogTag::Webserver,
    //             "WARN",
    //             "Timeout waiting for critical operations during reboot",
    //         );
    //         break;
    //     }
    //     tokio::time::sleep(Duration::from_millis(500)).await;
    // }

    // Get current executable path and arguments
    let current_exe = match env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "System Error",
                &format!("Failed to get current executable path: {}", e),
                None,
            );
        }
    };

    let args: Vec<String> = env::args().skip(1).collect();

    logger::info(
        LogTag::Webserver,
        &format!(
            "Restarting process: {} with args: {:?}",
            current_exe.display(),
            args
        ),
    );

    // Spawn async task to perform restart after response is sent
    tokio::spawn(async move {
        // Small delay to ensure response is sent
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Perform OS-specific restart
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            logger::info(
                LogTag::Webserver,
                "Executing Unix exec() for process replacement",
            );

            let error = Command::new(current_exe).args(&args).exec(); // This replaces the current process

            // If exec returns, it failed
            logger::error(
                LogTag::Webserver,
                &format!("Failed to exec new process: {}", error),
            );
            std::process::exit(1);
        }

        #[cfg(windows)]
        {
            logger::info(
                LogTag::Webserver,
                "Spawning new process on Windows and exiting current",
            );

            match Command::new(current_exe).args(&args).spawn() {
                Ok(_) => {
                    logger::info(
                        LogTag::Webserver,
                        "New process spawned successfully, exiting current process",
                    );
                    std::process::exit(0);
                }
                Err(e) => {
                    logger::error(
                        LogTag::Webserver,
                        &format!("Failed to spawn new process: {}", e),
                    );
                    std::process::exit(1);
                }
            }
        }
    });

    let response = RebootResponse {
        success: true,
        message: "System reboot initiated. Process will restart shortly.".to_string(),
    };

    success_response(response)
}

// =============================================================================
// ROUTER
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/reboot", post(reboot_system))
        .route("/bootstrap", get(boot_status))
        .route("/paths", get(get_paths))
        .route("/paths/open-data", post(open_data_directory))
        .route("/open-url", post(open_url))
        .route("/exit", post(exit_app))
        .route("/data-stats", get(get_data_stats))
}

/// GET /api/system/bootstrap - Report real-time boot status for GUI/frontend gating
async fn boot_status(State(state): State<Arc<AppState>>) -> Response {
    let timestamp = Utc::now();
    let initialization_complete = global::is_initialization_complete();
    let initialization_required = !initialization_complete;
    let core_services_ready = are_core_services_ready();
    let ready_for_requests = initialization_complete && core_services_ready;

    let pending_services = if core_services_ready {
        Vec::new()
    } else {
        get_pending_services()
    };

    let connectivity_ready = CONNECTIVITY_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
    let tokens_ready = TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
    let positions_ready = POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
    let pools_ready = POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst);
    let transactions_ready = TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);

    let ui_prereqs_ready = connectivity_ready && tokens_ready && pools_ready;
    let ui_ready = initialization_required || ui_prereqs_ready;
    let boot_progress = task::spawn_blocking(startup::snapshot)
        .await
        .unwrap_or_else(|_| Vec::new());

    let snapshot_status = wallet::get_cached_wallet_snapshot_status();
    let wallet_snapshot_ready = snapshot_status.is_ready;
    let wallet_last_updated = snapshot_status
        .last_updated
        .map(|timestamp| timestamp.to_rfc3339());

    let (services_total, services_running) = match get_service_manager().await {
        Some(manager_ref) => {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                (
                    manager.get_all_service_names().len(),
                    manager.get_running_service_count(),
                )
            } else {
                (0, 0)
            }
        }
        None => (0, 0),
    };

    let phase = if initialization_required {
        "initialization"
    } else if !ui_prereqs_ready {
        "ui_startup"
    } else if !core_services_ready {
        "service_startup"
    } else {
        "ready"
    };

    let message = match phase {
        "initialization" => "Waiting for initial wallet/RPC setup",
        "ui_startup" => {
            if pending_services.is_empty() {
                "Frontend prerequisites warming up"
            } else {
                "Frontend prerequisites still starting"
            }
        }
        "service_startup" => {
            if pending_services.is_empty() {
                "Core services warming up"
            } else {
                "Core services still starting"
            }
        }
        _ => "All systems ready",
    }
    .to_string();

    let retry_after_ms = if ui_ready { None } else { Some(750) };

    // Get onboarding status from config
    let onboarding_complete = if initialization_required {
        // Check if onboarding was previously completed
        crate::arguments::is_dashboard_onboarding_forced()
            .then(|| false)
            .unwrap_or_else(|| {
                let config_path = crate::paths::get_config_path();
                if config_path.exists() {
                    crate::config::with_config(|cfg| cfg.gui.dashboard.startup.onboarding_complete)
                } else {
                    false
                }
            })
    } else {
        true // Already initialized, onboarding must be done
    };

    let response = BootStatusResponse {
        timestamp: timestamp.to_rfc3339(),
        initialization_required,
        initialization_complete,
        onboarding_complete,
        core_services_ready,
        ui_ready,
        ready_for_requests,
        pending_services,
        services_total,
        services_running,
        connectivity_ready,
        tokens_ready,
        positions_ready,
        pools_ready,
        transactions_ready,
        boot_progress,
        wallet_snapshot_ready,
        wallet_last_updated,
        uptime_seconds: state.uptime_seconds(),
        phase: phase.to_string(),
        message,
        retry_after_ms,
    };

    success_response(response)
}

/// GET /api/system/paths - Return key filesystem locations
async fn get_paths() -> Response {
    if let Err(err) = paths::ensure_all_directories() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PATHS_INIT_FAILED",
            &err,
            None,
        );
    }

    let response = PathsResponse {
        base_directory: paths::get_base_directory_display(),
        data_directory: paths::get_data_directory().display().to_string(),
        logs_directory: paths::get_logs_directory().display().to_string(),
        cache_pool_directory: paths::get_cache_pool_directory().display().to_string(),
        analysis_exports_directory: paths::get_analysis_exports_directory()
            .display()
            .to_string(),
        config_path: paths::get_config_path().display().to_string(),
    };

    success_response(response)
}

/// POST /api/system/paths/open-data - Open the data directory in the OS file manager
async fn open_data_directory() -> Response {
    if let Err(err) = paths::ensure_all_directories() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PATHS_INIT_FAILED",
            &err,
            None,
        );
    }

    let data_dir = paths::get_data_directory();

    match paths::open_directory_in_file_manager(&data_dir) {
        Ok(_) => success_response(OpenPathResponse {
            opened: true,
            message: "Data folder opened in your file manager".to_string(),
            path: data_dir.display().to_string(),
        }),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPEN_DATA_FAILED",
            &err,
            None,
        ),
    }
}

// =============================================================================
// OPEN URL IN BROWSER
// =============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct OpenUrlResponse {
    pub opened: bool,
    pub message: String,
    pub url: String,
}

/// POST /api/system/open-url - Open a URL in the system's default browser
async fn open_url(axum::Json(request): axum::Json<OpenUrlRequest>) -> Response {
    let url = request.url.trim();

    if url.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_URL",
            "URL cannot be empty",
            None,
        );
    }

    match paths::open_url_in_browser(url) {
        Ok(_) => success_response(OpenUrlResponse {
            opened: true,
            message: "URL opened in your default browser".to_string(),
            url: url.to_string(),
        }),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPEN_URL_FAILED",
            &err,
            None,
        ),
    }
}

// =============================================================================
// EXIT APP
// =============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct ExitAppRequest {
    /// Delay in milliseconds before exiting (default: 0)
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ExitAppResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/system/exit - Exit the application
/// Used for "Install & Restart" to close the app after opening the installer
async fn exit_app(axum::Json(request): axum::Json<ExitAppRequest>) -> Response {
    logger::info(
        LogTag::System,
        &format!("Exit requested via API with delay: {}ms", request.delay_ms),
    );

    // Spawn exit task with optional delay
    let delay_ms = request.delay_ms;
    task::spawn(async move {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        logger::info(LogTag::System, "Exiting application...");
        std::process::exit(0);
    });

    success_response(ExitAppResponse {
        success: true,
        message: format!("Application will exit in {}ms", request.delay_ms),
    })
}

// =============================================================================
// DATA STATS
// =============================================================================

/// GET /api/system/data-stats - Get statistics for all databases and data files
async fn get_data_stats() -> Response {
    let mut databases = Vec::new();
    let mut total_size: u64 = 0;

    // Helper to get file size
    fn get_file_stats(name: &str, path: std::path::PathBuf) -> DatabaseStats {
        let (size_bytes, exists) = std::fs::metadata(&path)
            .map(|m| (m.len(), true))
            .unwrap_or((0, false));
        DatabaseStats {
            name: name.to_string(),
            path: path.display().to_string(),
            size_bytes,
            size_mb: size_bytes as f64 / 1_048_576.0,
            exists,
        }
    }

    // Collect all database stats
    let db_configs = [
        ("Tokens", paths::get_tokens_db_path()),
        ("Transactions", paths::get_transactions_db_path()),
        ("Positions", paths::get_positions_db_path()),
        ("Events", paths::get_events_db_path()),
        ("OHLCV", paths::get_ohlcvs_db_path()),
        ("Wallet", paths::get_wallet_db_path()),
        ("Pools", paths::get_pools_db_path()),
        ("Strategies", paths::get_strategies_db_path()),
        ("Actions", paths::get_actions_db_path()),
    ];

    for (name, path) in db_configs {
        let stats = get_file_stats(name, path);
        total_size += stats.size_bytes;
        databases.push(stats);
    }

    // Sort by size descending
    databases.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    // Get config file size
    let config_path = paths::get_config_path();
    let config_size_bytes = std::fs::metadata(&config_path)
        .map(|m| m.len())
        .unwrap_or(0);

    success_response(DataStatsResponse {
        databases,
        total_size_mb: total_size as f64 / 1_048_576.0,
        config_path: config_path.display().to_string(),
        config_size_bytes,
        data_directory: paths::get_data_directory().display().to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
}
