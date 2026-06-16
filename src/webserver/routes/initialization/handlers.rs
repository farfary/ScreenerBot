//! Initialization route handlers — onboarding and setup endpoint implementations.

use super::types::*;
use crate::{
    arguments,
    config::{self, schemas::Config},
    global,
    logger::{self, LogTag},
    rpc, services,
    webserver::utils::{error_response, success_response},
};
use axum::{extract::Json, http::StatusCode, response::Response};
use solana_sdk::signature::Signer;
use std::sync::atomic::Ordering;

/// GET /api/initialization/status
/// Check if initialization is required
pub(super) async fn initialization_status() -> Response {
    logger::debug(LogTag::Webserver, "Checking initialization status");

    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();
    let initialization_complete = global::is_initialization_complete();
    let discovery_only_mode = global::is_discovery_only_mode();
    let force_onboarding = arguments::is_dashboard_onboarding_forced();

    let setup_skipped = if config_exists {
        config::with_config(|cfg| cfg.gui.dashboard.startup.setup_skipped)
    } else {
        false
    };

    let onboarding_complete = if force_onboarding {
        false
    } else if !config_exists {
        false
    } else {
        config::with_config(|cfg| cfg.gui.dashboard.startup.onboarding_complete)
    };

    let (required, reason) = if discovery_only_mode {
        // Discovery-only mode: setup was skipped. The app is usable (token discovery);
        // setup is not "required" so the dashboard is not blocked by the setup screen.
        (
            false,
            "Running in discovery-only mode - wallet + RPC setup skipped.".to_owned(),
        )
    } else if !config_exists {
        (
            true,
            "Configuration file does not exist. Initial setup required.".to_owned(),
        )
    } else if !initialization_complete {
        (true, "Initialization in progress or incomplete.".to_owned())
    } else {
        (false, "System fully initialized.".to_owned())
    };

    let response = InitializationStatusResponse {
        required,
        reason,
        config_exists,
        initialization_complete,
        onboarding_complete,
        force_onboarding,
        discovery_only_mode,
        setup_skipped,
    };

    success_response(response)
}

/// POST /api/initialization/onboarding/complete
/// Mark onboarding as complete in memory only (NOT persisted until setup completes)
pub(super) async fn complete_onboarding() -> Response {
    logger::info(
        LogTag::Webserver,
        "Marking onboarding as complete (in-memory only)",
    );

    // IMPORTANT: Do NOT save to disk here!
    // The config.toml should only be created when the user completes the full setup flow
    // (via /api/initialization/complete). Setting save_to_disk=false ensures we only
    // update the in-memory config state for the current session.
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.gui.dashboard.startup.onboarding_complete = true;
        },
        false, // Do NOT save to disk - config.toml should not exist until setup is done
    ) {
        logger::error(
            LogTag::Webserver,
            &format!("Failed to update onboarding state: {e}"),
        );
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to update onboarding state",
            Some(&e),
        );
    }

    success_response(serde_json::json!({ "success": true }))
}

/// POST /api/initialization/skip
/// Skip wallet + RPC setup and enter discovery-only mode.
///
/// Persists a config with an empty wallet, the public default RPC, and
/// `setup_skipped = true`, then starts only the discovery tier (connectivity,
/// events, tokens, filtering, webserver). Trading and all wallet/RPC-dependent
/// services stay stopped until the user completes setup later.
pub(super) async fn skip_setup() -> Response {
    logger::info(
        LogTag::Webserver,
        "Skipping wallet + RPC setup - entering discovery-only mode",
    );

    let mut errors = Vec::new();

    // Preserve existing settings if a config is already loaded; otherwise start from
    // defaults. Wallet stays empty; RPC falls back to the public default endpoint.
    let mut config = if config::is_config_initialized() {
        config::get_config_clone()
    } else {
        Config::default()
    };

    config.wallet_encrypted = String::new();
    config.wallet_nonce = String::new();
    if config.rpc.urls.is_empty() {
        config.rpc = crate::config::schemas::RpcConfig::default();
    }
    config.gui.dashboard.startup.setup_skipped = true;
    config.gui.dashboard.startup.onboarding_complete = true;

    let config_path = crate::paths::get_config_path();
    if let Err(e) =
        config::utils::save_config_to_file(&config, &config_path.to_string_lossy(), true)
    {
        errors.push(format!("Failed to save configuration: {e}"));
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_SAVE_FAILED",
            &errors.join("; "),
            None,
        );
    }

    logger::info(LogTag::Webserver, "Discovery-only configuration saved");

    // Enter discovery-only mode (INITIALIZATION_COMPLETE intentionally stays false).
    global::set_discovery_only_mode(true);

    // Start discovery-tier services. start_newly_enabled is idempotent and the
    // ServiceManager filters out disabled (wallet/RPC-bound) services.
    let mut services_started = 0usize;
    match start_remaining_services().await {
        Ok(report) => {
            services_started = report.started.len();
            logger::info(
                LogTag::Webserver,
                &format!(
                    "Discovery-only startup summary: started={} already_running={} total_enabled={} duration_ms={}",
                    report.started.len(),
                    report.already_running,
                    report.total_enabled,
                    report.duration_ms
                ),
            );

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
                &format!("Failed to start discovery-tier services: {e}"),
            );
            errors.push(format!("Service startup incomplete: {e}"));
        }
    }

    let response = SkipSetupResponse {
        success: errors.is_empty(),
        discovery_only_mode: true,
        services_started,
        errors,
    };

    success_response(response)
}

/// POST /api/initialization/validate
/// Validate credentials without persisting
pub(super) async fn validate_credentials(
    Json(request): Json<ValidateCredentialsRequest>,
) -> Response {
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
        errors.push("At least one RPC URL is required".to_owned());
    } else if request.rpc_urls.len() > 10 {
        errors.push("Maximum 10 RPC URLs allowed".to_owned());
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
            errors.push(format!("Invalid wallet private key: {e}"));
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
        errors.push("All RPC endpoints failed connection tests".to_owned());
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
pub(super) async fn complete_initialization(
    Json(request): Json<CompleteInitializationRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
        "Starting initialization completion process",
    );

    let mut errors = Vec::new();

    // Step 1: Validate wallet private key
    let keypair = match parse_wallet_private_key(&request.wallet_private_key) {
        Ok(kp) => kp,
        Err(e) => {
            errors.push(format!("Invalid wallet private key: {e}"));
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
        &format!("Wallet validated: {wallet_address}"),
    );

    // Step 2: Test RPC endpoints
    if request.rpc_urls.is_empty() {
        errors.push("At least one RPC URL is required".to_owned());
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
        errors.push("No working RPC endpoints found".to_owned());
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

    // Step 3: Encrypt the private key and create config
    logger::info(LogTag::Webserver, "Encrypting wallet private key...");

    let encrypted = match crate::secure_storage::encrypt_private_key(&request.wallet_private_key) {
        Ok(enc) => enc,
        Err(e) => {
            errors.push(format!("Failed to encrypt private key: {e}"));
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ENCRYPTION_FAILED",
                &errors.join("; "),
                None,
            );
        }
    };

    logger::info(
        LogTag::Webserver,
        "Creating configuration with encrypted wallet...",
    );

    // Merge into the existing config when one is already loaded (e.g. completing setup
    // from discovery-only mode) so user settings are preserved. Only fall back to
    // defaults on a true first run with no config in memory.
    let mut config = if config::is_config_initialized() {
        config::get_config_clone()
    } else {
        Config::default()
    };

    config.wallet_encrypted = encrypted.ciphertext;
    config.wallet_nonce = encrypted.nonce;
    config.rpc.urls = working_rpc_urls;

    // Setup is now complete: clear the discovery-only skip marker and mark onboarding done.
    config.gui.dashboard.startup.setup_skipped = false;
    config.gui.dashboard.startup.onboarding_complete = true;

    let config_path = crate::paths::get_config_path();
    if let Err(e) =
        config::utils::save_config_to_file(&config, &config_path.to_string_lossy(), true)
    {
        errors.push(format!("Failed to save configuration: {e}"));
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
    // (services check this flag in their is_enabled() method). Clear discovery-only
    // mode: full mode and discovery-only mode are mutually exclusive.
    global::set_discovery_only_mode(false);
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
                &format!("Failed to start some services: {e}"),
            );
            errors.push(format!("Service startup incomplete: {e}"));
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
pub(super) async fn initialization_progress() -> Response {
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
            "pre-initialization".to_owned(),
            "waiting".to_owned(),
            "Awaiting user credentials".to_owned(),
        )
    } else if services_total == 0 {
        (
            "services-startup".to_owned(),
            "idle".to_owned(),
            "No enabled services registered".to_owned(),
        )
    } else if services_started < services_total {
        (
            "services-startup".to_owned(),
            "starting".to_owned(),
            format!(
                "Starting services ({} / {})...",
                services_started, services_total
            ),
        )
    } else {
        (
            "services-startup".to_owned(),
            "complete".to_owned(),
            "All services initialized".to_owned(),
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

/// Start remaining services after initialization
async fn start_remaining_services() -> Result<services::ServiceStartupReport, String> {
    logger::info(
        LogTag::Webserver,
        "Requesting service startup from ServiceManager",
    );

    let manager_ref = services::get_service_manager()
        .await
        .ok_or_else(|| "ServiceManager not available".to_owned())?;

    let mut manager_guard = manager_ref.write().await;
    let manager = manager_guard
        .as_mut()
        .ok_or_else(|| "ServiceManager not initialized".to_owned())?;

    // Start newly enabled services
    let report = manager
        .start_newly_enabled()
        .await
        .map_err(|e| e.to_string())?;

    Ok(report)
}
