//! System route handlers — endpoint implementations for system info and control.

use super::types::*;
use crate::logger::{self, LogTag};
use crate::paths;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};
use crate::{
    global::{
        self, are_core_services_ready, get_pending_services, CONNECTIVITY_SYSTEM_READY,
        POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
    },
    services::get_service_manager,
    startup, wallet,
};
use axum::{extract::State, http::StatusCode, response::Response, Json};
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

/// Set the first time the dashboard frontend reports itself fully loaded, so the
/// "frontend ready" log line fires exactly once per process.
static CLIENT_READY_REPORTED: AtomicBool = AtomicBool::new(false);

/// Whether the dashboard frontend has reported that it finished loading.
pub fn is_client_ready() -> bool {
    CLIENT_READY_REPORTED.load(Ordering::SeqCst)
}

/// POST /api/system/reboot — Restart the entire screenerbot process
pub(super) async fn reboot_system() -> Response {
    logger::debug(LogTag::Webserver, "System reboot requested via API");

    schedule_graceful_restart("manual dashboard request");

    let response = RebootResponse {
        success: true,
        message: "Graceful restart scheduled. The dashboard will reconnect automatically."
            .to_owned(),
        instance_id: global::instance_id().to_owned(),
    };

    success_response(response)
}

/// Delay the signal long enough for Axum to flush the response, then let the
/// run loop stop services and release the process lock in normal order.
pub(crate) fn schedule_graceful_restart(reason: &'static str) {
    task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(350)).await;
        logger::info(
            LogTag::System,
            &format!("Scheduling graceful process restart: {reason}"),
        );
        global::request_restart();
    });
}

/// GET /api/system/bootstrap — Report real-time boot status for GUI/frontend gating
pub(super) async fn boot_status(State(state): State<Arc<AppState>>) -> Response {
    let timestamp = Utc::now();
    let initialization_complete = global::is_initialization_complete();
    let explore_mode = global::is_explore_mode();
    // In Explore Mode the user deliberately skipped setup, so the dashboard must
    // load (not the setup screen) even though full initialization is not complete.
    let initialization_required = !initialization_complete && !explore_mode;
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

    // Explore Mode never starts the pools service, so it is not a UI prerequisite
    // there — connectivity + tokens are enough for the discovery dashboard to be usable.
    let ui_prereqs_ready = if explore_mode {
        connectivity_ready && tokens_ready
    } else {
        connectivity_ready && tokens_ready && pools_ready
    };
    let ui_ready = initialization_required || ui_prereqs_ready;
    let boot_progress = task::spawn_blocking(startup::snapshot)
        .await
        .unwrap_or_default();

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
    } else if explore_mode {
        // Explore Mode: once UI prerequisites are up the dashboard is fully usable; the
        // remaining (wallet/RPC) services are intentionally not running.
        "ready"
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

    let retry_after_ms = (!ui_ready).then_some(750);

    // Get onboarding status from config
    let onboarding_complete = if initialization_required {
        // Check if onboarding was previously completed
        crate::arguments::is_dashboard_onboarding_forced()
            .then_some(false)
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
        explore_mode,
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

/// GET /api/system/paths — Return key filesystem locations
pub(super) async fn get_paths() -> Response {
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

/// POST /api/system/paths/open-data — Open the data directory in the OS file manager
pub(super) async fn open_data_directory() -> Response {
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
            message: "Data folder opened in your file manager".to_owned(),
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

/// POST /api/system/open-url — Open a URL in the system's default browser
pub(super) async fn open_url(Json(request): Json<OpenUrlRequest>) -> Response {
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
            message: "URL opened in your default browser".to_owned(),
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

/// GET /api/system/data-stats — Get statistics for all databases and data files
pub(super) async fn get_data_stats() -> Response {
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
        .unwrap_or_default();

    success_response(DataStatsResponse {
        databases,
        total_size_mb: total_size as f64 / 1_048_576.0,
        config_path: config_path.display().to_string(),
        config_size_bytes,
        data_directory: paths::get_data_directory().display().to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

/// POST /api/system/client-ready — the dashboard frontend signals that it has
/// finished loading everything and is fully started (initial page rendered and
/// its first live data shown). Logged once per process so boot-to-interactive
/// time is observable, and exposed via `is_client_ready()` for anything that
/// wants to know the UI is up. Idempotent: repeat reports are acknowledged
/// without re-logging.
pub(super) async fn client_ready(
    State(state): State<Arc<AppState>>,
    body: Option<Json<ClientReadyRequest>>,
) -> Response {
    let req = body.map(|Json(inner)| inner).unwrap_or_default();
    let first_report = !CLIENT_READY_REPORTED.swap(true, Ordering::SeqCst);

    if first_report {
        let page = req.page.as_deref().unwrap_or("home");
        let load_info = req
            .load_ms
            .map(|ms| format!(", client load {ms}ms"))
            .unwrap_or_default();
        logger::info(
            LogTag::Webserver,
            &format!(
                "Dashboard frontend fully loaded and started (page={page}, {}s after boot{load_info})",
                state.uptime_seconds()
            ),
        );
    }

    success_response(ClientReadyResponse {
        acknowledged: true,
        first_report,
        uptime_seconds: state.uptime_seconds(),
    })
}
