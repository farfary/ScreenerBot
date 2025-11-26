use axum::{
    extract::Json,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{Keypair, Signer};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::{
    config::{self, schemas::Config},
    global,
    logger::{self, LogTag},
    rpc::{self, RpcEndpointTestResult},
    services,
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

// ============================================================================
// REQUEST/RESPONSE TYPES (INLINE)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct InitializationStatusResponse {
    pub required: bool,
    pub reason: String,
    pub config_exists: bool,
    pub initialization_complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCredentialsRequest {
    pub wallet_private_key: String,
    pub rpc_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub wallet_address: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub rpc_test_results: Vec<RpcEndpointTestResult>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteInitializationRequest {
    pub wallet_private_key: String,
    pub rpc_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InitializationCompleteResponse {
    pub success: bool,
    pub wallet_address: String,
    pub services_started: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InitializationProgressResponse {
    pub step: String,
    pub status: String,
    pub message: String,
    pub services_started: usize,
    pub services_total: usize,
}

// ============================================================================
// ROUTES
// ============================================================================

/// Create initialization routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(initialization_status))
        .route("/validate", post(validate_credentials))
        .route("/complete", post(complete_initialization))
        .route("/progress", get(initialization_progress))
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/initialization/status
/// Check if initialization is required
async fn initialization_status() -> Response {
    logger::debug(LogTag::Webserver, "Checking initialization status");

    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();
    let initialization_complete = global::is_initialization_complete();

    let (required, reason) = if !config_exists {
        (
            true,
            "Configuration file does not exist. Initial setup required.".to_string(),
        )
    } else if !initialization_complete {
        (
            true,
            "Initialization in progress or incomplete.".to_string(),
        )
    } else {
        (false, "System fully initialized.".to_string())
    };

    let response = InitializationStatusResponse {
        required,
        reason,
        config_exists,
        initialization_complete,
    };

    success_response(response)
}

/// POST /api/initialization/validate
/// Validate credentials without persisting
async fn validate_credentials(Json(request): Json<ValidateCredentialsRequest>) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!(
            "Validating credentials: {} RPC endpoint(s)",
            request.rpc_urls.len()
        ),
    );

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut wallet_address: Option<String> = None;

    // Validate RPC URLs count
    if request.rpc_urls.is_empty() {
        errors.push("At least one RPC URL is required".to_string());
    } else if request.rpc_urls.len() > 10 {
        errors.push("Maximum 10 RPC URLs allowed".to_string());
    }

    // Validate wallet private key
    let keypair_result = parse_wallet_private_key(&request.wallet_private_key);
    match keypair_result {
        Ok(keypair) => {
            wallet_address = Some(keypair.pubkey().to_string());
            logger::info(
                LogTag::Webserver,
                &format!("Wallet validated: {}", keypair.pubkey()),
            );
        }
        Err(e) => {
            errors.push(format!("Invalid wallet private key: {}", e));
        }
    }

    // Test RPC endpoints concurrently
    let rpc_test_results = if !request.rpc_urls.is_empty() && errors.is_empty() {
        logger::info(LogTag::Webserver, "Testing RPC endpoints...");
        logger::info(
            LogTag::Webserver,
            &format!(
                "BEFORE rpc::test_rpc_endpoints() call with {} URLs",
                request.rpc_urls.len()
            ),
        );
        let results = rpc::test_rpc_endpoints(&request.rpc_urls).await;
        logger::info(
            LogTag::Webserver,
            &format!(
                "AFTER rpc::test_rpc_endpoints() call, got {} results",
                results.len()
            ),
        );
        results
    } else {
        vec![]
    };

    // Analyze RPC test results
    let successful_rpcs: Vec<_> = rpc_test_results.iter().filter(|r| r.success).collect();
    let failed_rpcs: Vec<_> = rpc_test_results.iter().filter(|r| !r.success).collect();

    logger::info(
        LogTag::Webserver,
        &format!(
            "RPC test results: {} successful, {} failed",
            successful_rpcs.len(),
            failed_rpcs.len()
        ),
    );
    for result in &rpc_test_results {
        logger::info(
            LogTag::Webserver,
            &format!(
                "  - {}: success={}, error={:?}",
                result.url, result.success, result.error
            ),
        );
    }

    if successful_rpcs.is_empty() && !rpc_test_results.is_empty() {
        errors.push("All RPC endpoints failed connection tests".to_string());
    } else if !failed_rpcs.is_empty() {
        warnings.push(format!(
            "{} of {} RPC endpoint(s) failed - will only use working endpoints",
            failed_rpcs.len(),
            rpc_test_results.len()
        ));
    }

    // Check for non-mainnet endpoints
    for result in &rpc_test_results {
        if result.success {
            if let Some(false) = result.is_mainnet {
                warnings.push(format!(
                    "RPC endpoint {} is not Solana mainnet-beta",
                    result.url
                ));
            }
            if !result.is_premium {
                warnings.push(format!(
                    "RPC endpoint {} does not appear to be a premium provider - may experience rate limiting",
                    result.url
                ));
            }
        }
    }

    let valid = errors.is_empty();

    let response = ValidationResult {
        valid,
        wallet_address,
        errors,
        warnings,
        rpc_test_results,
    };

    success_response(response)
}

/// POST /api/initialization/complete
/// Complete initialization (validate + persist + start services)
async fn complete_initialization(Json(request): Json<CompleteInitializationRequest>) -> Response {
    logger::info(
        LogTag::Webserver,
        "Starting initialization completion process",
    );

    let mut errors = Vec::new();

    // Step 1: Validate wallet private key
    let keypair = match parse_wallet_private_key(&request.wallet_private_key) {
        Ok(kp) => kp,
        Err(e) => {
            errors.push(format!("Invalid wallet private key: {}", e));
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_CREDENTIALS",
                &errors.join("; "),
                None,
            );
        }
    };

    let wallet_address = keypair.pubkey();
    logger::info(
        LogTag::Webserver,
        &format!("Wallet validated: {}", wallet_address),
    );

    // Step 2: Test RPC endpoints
    if request.rpc_urls.is_empty() {
        errors.push("At least one RPC URL is required".to_string());
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_CREDENTIALS",
            &errors.join("; "),
            None,
        );
    }

    logger::info(LogTag::Webserver, "Testing RPC endpoints...");
    let rpc_test_results = rpc::test_rpc_endpoints(&request.rpc_urls).await;

    // Filter to only working endpoints
    let working_rpc_urls: Vec<String> = rpc_test_results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.url.clone())
        .collect();

    if working_rpc_urls.is_empty() {
        errors.push("No working RPC endpoints found".to_string());
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_CREDENTIALS",
            &errors.join("; "),
            None,
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "RPC validation complete: {} of {} endpoints working",
            working_rpc_urls.len(),
            request.rpc_urls.len()
        ),
    );

    // Step 3: Create and save config
    logger::info(LogTag::Webserver, "Creating configuration...");

    let config = Config {
        main_wallet_private: request.wallet_private_key.clone(),
        rpc: crate::config::schemas::RpcConfig {
            urls: working_rpc_urls,
        },
        ..Default::default()
    };

    let config_path = crate::paths::get_config_path();
    if let Err(e) =
        config::utils::save_config_to_file(&config, &config_path.to_string_lossy(), true)
    {
        errors.push(format!("Failed to save configuration: {}", e));
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_SAVE_FAILED",
            &errors.join("; "),
            None,
        );
    }

    logger::info(LogTag::Webserver, "Configuration saved successfully");

    // Step 4: Set credential flags (but NOT initialization complete yet)
    global::CREDENTIALS_VALID.store(true, Ordering::SeqCst);
    global::RPC_VALID.store(true, Ordering::SeqCst);

    logger::info(LogTag::Webserver, "Credential validation flags set");

    // Step 5: Set initialization complete flag BEFORE starting services
    // (services check this flag in their is_enabled() method)
    global::INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);
    logger::info(
        LogTag::Webserver,
        "Initialization complete flag set - services can now start",
    );

    // Step 6: Start remaining services
    logger::info(LogTag::Webserver, "Starting services...");

    let mut services_started = 0usize;

    match start_remaining_services().await {
        Ok(report) => {
            services_started = report.started.len();

            logger::info(
                LogTag::Webserver,
                &format!(
                    "Service startup summary: started={} already_running={} total_enabled={} duration_ms={}",
                    report.started.len(),
                    report.already_running,
                    report.total_enabled,
                    report.duration_ms
                ),
            );

            if report.started.is_empty() {
                logger::warning(
                    LogTag::Webserver,
                    "No new services were started during initialization completion",
                );
            }

            if !report.failures.is_empty() {
                let failure_names = report
                    .failures
                    .iter()
                    .map(|failure| failure.name)
                    .collect::<Vec<_>>()
                    .join(", ");

                errors.push(format!(
                    "Failed to start {} service(s): {}",
                    report.failures.len(),
                    failure_names
                ));

                for failure in report.failures {
                    logger::error(
                        LogTag::Webserver,
                        &format!(
                            "Service startup failure: {} -> {}",
                            failure.name, failure.error
                        ),
                    );
                }
            }
        }
        Err(e) => {
            logger::error(
                LogTag::Webserver,
                &format!("Failed to start some services: {}", e),
            );
            errors.push(format!("Service startup incomplete: {}", e));
        }
    }

    // Step 7: Build and return response
    let response = InitializationCompleteResponse {
        success: errors.is_empty(),
        wallet_address: wallet_address.to_string(),
        services_started,
        errors,
    };

    success_response(response)
}

/// GET /api/initialization/progress
/// Get initialization progress (services startup status)
async fn initialization_progress() -> Response {
    let initialization_complete = global::is_initialization_complete();

    // Get service progress metrics
    let (services_started, services_total) =
        if let Some(manager_ref) = services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                let all_services = manager.get_all_service_names();
                let enabled_services: Vec<&'static str> = all_services
                    .iter()
                    .copied()
                    .filter(|name| manager.is_service_enabled(name))
                    .collect();

                let running_services = manager.get_running_service_names();
                let running_enabled = running_services
                    .iter()
                    .filter(|name| manager.is_service_enabled(*name))
                    .count();

                (running_enabled, enabled_services.len())
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

    let (step, status, message) = if !initialization_complete {
        (
            "pre-initialization".to_string(),
            "waiting".to_string(),
            "Awaiting user credentials".to_string(),
        )
    } else if services_total == 0 {
        (
            "services-startup".to_string(),
            "idle".to_string(),
            "No enabled services registered".to_string(),
        )
    } else if services_started < services_total {
        (
            "services-startup".to_string(),
            "starting".to_string(),
            format!(
                "Starting services ({} / {})...",
                services_started, services_total
            ),
        )
    } else {
        (
            "services-startup".to_string(),
            "complete".to_string(),
            "All services initialized".to_string(),
        )
    };

    let response = InitializationProgressResponse {
        step,
        status,
        message,
        services_started,
        services_total,
    };

    success_response(response)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Parse wallet private key from string (supports base58 and JSON array formats)
fn parse_wallet_private_key(private_key: &str) -> Result<Keypair, String> {
    let trimmed = private_key.trim();

    // Try base58 format first
    if let Ok(bytes) = bs58::decode(trimmed).into_vec() {
        if bytes.len() == 64 {
            if let Ok(keypair) = Keypair::from_bytes(&bytes) {
                return Ok(keypair);
            }
        }
    }

    // Try JSON array format [1,2,3,...]
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(trimmed) {
            if bytes.len() == 64 {
                if let Ok(keypair) = Keypair::from_bytes(&bytes) {
                    return Ok(keypair);
                }
            }
        }
    }

    Err("Invalid private key format. Must be base58 string or JSON array of 64 bytes".to_string())
}

/// Start remaining services after initialization
async fn start_remaining_services() -> Result<services::ServiceStartupReport, String> {
    logger::info(
        LogTag::Webserver,
        "Requesting service startup from ServiceManager",
    );

    let manager_ref = services::get_service_manager()
        .await
        .ok_or("ServiceManager not available".to_string())?;

    let mut manager_guard = manager_ref.write().await;
    let manager = manager_guard
        .as_mut()
        .ok_or("ServiceManager not initialized".to_string())?;

    // Start newly enabled services
    let report = manager.start_newly_enabled().await?;

    Ok(report)
}
