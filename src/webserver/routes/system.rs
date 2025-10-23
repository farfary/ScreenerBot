use axum::{extract::State, http::StatusCode, response::Response, routing::post, Router};
use serde::Serialize;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

use crate::arguments::is_debug_webserver_enabled;
use crate::logger::{log, LogTag};
// TODO: Re-enable when trader module is fully integrated
// use crate::trader::CRITICAL_OPERATIONS_IN_PROGRESS;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct RebootResponse {
    pub success: bool,
    pub message: String,
}

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// POST /api/system/reboot - Restart the entire screenerbot process
async fn reboot_system() -> Response {
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "System reboot requested via API");
    }

    // TODO: Re-enable critical operations check when trader module is integrated
    // Wait for critical operations to complete (max 30 seconds)
    // let timeout = Instant::now() + Duration::from_secs(30);
    // while CRITICAL_OPERATIONS_IN_PROGRESS.load(Ordering::SeqCst) > 0 {
    //     if Instant::now() > timeout {
    //         log(
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

    log(
        LogTag::Webserver,
        "INFO",
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
            log(
                LogTag::Webserver,
                "INFO",
                "Executing Unix exec() for process replacement",
            );

            let error = Command::new(current_exe).args(&args).exec(); // This replaces the current process

            // If exec returns, it failed
            log(
                LogTag::Webserver,
                "ERROR",
                &format!("Failed to exec new process: {}", error),
            );
            std::process::exit(1);
        }

        #[cfg(windows)]
        {
            log(
                LogTag::Webserver,
                "INFO",
                "Spawning new process on Windows and exiting current",
            );

            match Command::new(current_exe).args(&args).spawn() {
                Ok(_) => {
                    log(
                        LogTag::Webserver,
                        "INFO",
                        "New process spawned successfully, exiting current process",
                    );
                    std::process::exit(0);
                }
                Err(e) => {
                    log(
                        LogTag::Webserver,
                        "ERROR",
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
    Router::new().route("/reboot", post(reboot_system))
}
