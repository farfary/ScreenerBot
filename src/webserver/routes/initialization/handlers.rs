// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Initialization route handlers — onboarding and setup endpoint implementations.

use super::types::*;
use super::validation::{
    clear_setup_validation, consume_validated_key, requested_access, store_setup_validation,
    validate_rpc_url_list, RpcSource,
};
use crate::chains::access::ChainAccessMode;
use crate::chains::ChainId;
use crate::i18n::ids;
use crate::process::restart_safety::RestartBlocker;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::routes::wallets::handlers::replacement_error;
use crate::{
    arguments,
    chains::solana::{
        accounts::{generate_key, keypair_to_address, parse_private_key},
        chain_metadata, rpc,
    },
    config::{self, schemas::Config},
    global,
    logger::{self, LogTag},
    services,
    webserver::{utils::success_response, Error, Result},
};
use axum::response::IntoResponse as _;
use axum::{extract::Json, response::Response};

/// GET /api/initialization/status
/// Check if initialization is required
pub(super) async fn initialization_status() -> Response {
    logger::debug(LogTag::Webserver, "Checking initialization status");

    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();
    let initialization_complete = global::is_initialization_complete();
    let explore_mode = global::is_explore_mode();
    let force_onboarding = arguments::is_dashboard_onboarding_forced();

    let explore_mode_enabled = if config_exists {
        config::with_config(|cfg| cfg.gui.dashboard.startup.explore_mode_enabled)
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

    let (required, reason) = if explore_mode {
        // Explore Mode is a deliberate limited session, so the dashboard is usable;
        // setup is not "required" so the dashboard is not blocked by the setup screen.
        (
            false,
            "Running in Explore Mode without wallet or RPC setup.".to_owned(),
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
        explore_mode,
        explore_mode_enabled,
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
        return ApiError::new(
            ApiErrorCode::ConfigError,
            ids::ERRORS_INITIALIZATION_ONBOARDING_UPDATE_FAILED,
        )
        .details(e.to_string())
        .into_response();
    }

    success_response(serde_json::json!({ "success": true }))
}

/// POST /api/initialization/explore
/// Enter Explore Mode without wallet + RPC setup.
///
/// Persists a config with an empty wallet, the public default RPC, and
/// `explore_mode_enabled = true`, then starts only the Explore tier (connectivity,
/// events, tokens, filtering, webserver). Trading and all wallet/RPC-dependent
/// services stay stopped until the user completes setup later.
pub(super) async fn enter_explore_mode() -> Response {
    logger::info(
        LogTag::Webserver,
        "Entering Explore Mode without wallet + RPC setup",
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
    let rpc = config.chains.rpc_mut(chain_metadata().id);
    if rpc.urls.is_empty() {
        *rpc = crate::config::schemas::RpcConfig::default();
    }
    config.gui.dashboard.startup.explore_mode_enabled = true;
    config.gui.dashboard.startup.onboarding_complete = true;

    let config_path = crate::paths::get_config_path();
    if let Err(e) =
        config::utils::save_config_to_file(&config, &config_path.to_string_lossy(), true)
    {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    logger::info(LogTag::Webserver, "Explore Mode configuration saved");

    // Enter Explore Mode (INITIALIZATION_COMPLETE intentionally stays false).
    global::set_explore_mode(true);

    // Start explore-tier services. start_newly_enabled is idempotent and the
    // ServiceManager filters out disabled (wallet/RPC-bound) services.
    let mut services_started = 0usize;
    match start_remaining_services().await {
        Ok(report) => {
            services_started = report.started.len();
            logger::info(
                LogTag::Webserver,
                &format!(
                    "Explore Mode startup summary: started={} already_running={} total_enabled={} duration_ms={}",
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
                &format!("Failed to start explore-tier services: {e}"),
            );
            errors.push(format!("Service startup incomplete: {e}"));
        }
    }

    let response = ExploreModeResponse {
        success: errors.is_empty(),
        explore_mode: true,
        services_started,
        errors,
    };

    success_response(response)
}

/// POST /api/initialization/validate
/// Validate credentials without persisting. After a full setup a request without RPC
/// endpoints tests the configured ones, which a replacement then keeps; their URLs never
/// reach the response, which names each endpoint by its host only.
pub(super) async fn validate_credentials(
    Json(request): Json<ValidateCredentialsRequest>,
) -> Response {
    // The key is parsed in Solana's format below.
    let chain = chain_metadata().id;
    if requested_access(request.access, chain) == ChainAccessMode::Service {
        return validate_service_access(&request, chain).await;
    }
    let rpc_source = if request.rpc_urls.is_empty() && global::is_initialization_complete() {
        RpcSource::Configured
    } else {
        RpcSource::Request
    };
    let rpc_urls = match rpc_source {
        RpcSource::Request => request.rpc_urls.clone(),
        RpcSource::Configured => config::with_config(|cfg| cfg.chains.rpc(chain).urls.clone()),
    };
    logger::info(
        LogTag::Webserver,
        &format!(
            "Validating credentials: {} RPC endpoint(s), source {rpc_source:?}",
            rpc_urls.len()
        ),
    );

    clear_setup_validation();

    let mut errors = match rpc_source {
        RpcSource::Request => validate_rpc_url_list(&rpc_urls),
        RpcSource::Configured if rpc_urls.is_empty() => {
            vec!["No RPC endpoints are configured".to_owned()]
        }
        RpcSource::Configured => Vec::new(),
    };
    let mut warnings = Vec::new();
    let mut wallet_address: Option<String> = None;

    // Validate wallet private key
    let keypair_result = parse_private_key(&request.wallet_private_key);
    match keypair_result {
        Ok(keypair) => {
            let address = keypair_to_address(&keypair);
            logger::info(LogTag::Webserver, &format!("Wallet validated: {address}"));
            wallet_address = Some(address);
        }
        Err(e) => {
            errors.push(format!("Invalid wallet private key: {e}"));
        }
    }

    // Test RPC endpoints concurrently only after the request itself is valid.
    let rpc_test_results = if !rpc_urls.is_empty() && errors.is_empty() {
        logger::info(LogTag::Webserver, "Testing RPC endpoints...");
        rpc::test_rpc_endpoints(&rpc_urls, wallet_address.as_deref()).await
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
                result.display_url, result.success, result.error
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

    // Healthy non-mainnet endpoints are rejected by the tester. Premium detection
    // remains guidance: a self-hosted mainnet RPC can still be perfectly valid.
    for result in &rpc_test_results {
        if result.success && !result.is_premium {
            warnings.push(format!(
                "RPC endpoint {} does not appear to be a managed provider - monitor it for rate limiting",
                result.display_url
            ));
        }
    }

    let valid = errors.is_empty();
    let validation_id = if valid {
        let working_rpc_indices = rpc_test_results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| result.success.then_some(index))
            .collect();

        Some(store_setup_validation(
            &request.wallet_private_key,
            &request.rpc_urls,
            ChainAccessMode::OwnRpc,
            chain,
            wallet_address.clone().unwrap_or_default(),
            rpc_source,
            working_rpc_indices,
        ))
    } else {
        None
    };

    let response = ValidationResult {
        valid,
        wallet_address,
        errors,
        warnings,
        rpc_test_results,
        validation_id,
        connection: None,
    };

    success_response(response)
}

/// The service-access half of [`validate_credentials`]: the key, the account's
/// standing and one network read of the service. No RPC endpoint is tested or saved.
async fn validate_service_access(request: &ValidateCredentialsRequest, chain: ChainId) -> Response {
    logger::info(
        LogTag::Webserver,
        "Validating credentials for service chain access",
    );
    clear_setup_validation();

    let mut errors = Vec::new();
    let wallet_address = match parse_private_key(&request.wallet_private_key) {
        Ok(keypair) => Some(keypair_to_address(&keypair)),
        Err(e) => {
            errors.push(format!("Invalid wallet private key: {e}"));
            None
        }
    };
    let state = crate::chains::solana::access::probe_service().await;
    if !state.ready() {
        errors.push(format!(
            "The ScreenerBot service cannot serve this install yet: {}",
            state.key()
        ));
    }

    let valid = errors.is_empty();
    let validation_id = valid.then(|| {
        store_setup_validation(
            &request.wallet_private_key,
            &request.rpc_urls,
            ChainAccessMode::Service,
            chain,
            wallet_address.clone().unwrap_or_default(),
            RpcSource::Request,
            Vec::new(),
        )
    });

    success_response(ValidationResult {
        valid,
        wallet_address,
        errors,
        warnings: Vec::new(),
        rpc_test_results: Vec::new(),
        validation_id,
        connection: Some(ValidationConnection {
            mode: ChainAccessMode::Service,
            state: state.key(),
        }),
    })
}

/// POST /api/initialization/wallet/generate
/// A fresh trading-wallet keypair, returned once and never stored: the secret reaches
/// storage only when setup or a replacement completes with it. Only the address is logged.
pub(super) async fn generate_wallet() -> Response {
    let (address, private_key) = generate_key();
    logger::info(
        LogTag::Webserver,
        &format!("Generated a new wallet keypair {address}"),
    );
    let mut response = success_response(GeneratedWalletResponse {
        address,
        private_key,
    });
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

/// POST /api/initialization/complete
/// Complete initialization (validate + persist + schedule a clean full boot)
pub(super) async fn complete_initialization(
    Json(request): Json<CompleteInitializationRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
        "Starting initialization completion process",
    );

    let errors: Vec<String> = Vec::new();

    // Validation already performed the expensive network checks. Consume the
    // one-time receipt only when it belongs to this exact immutable snapshot.
    let key = match consume_validated_key(&request) {
        Ok(key) => key,
        Err(error) => return error.into_response(),
    };
    // Own-RPC setup saves the endpoints it validated; only a fully set-up app keeps its
    // own. Service access saves none.
    if key.access == ChainAccessMode::OwnRpc && key.rpc_urls.is_empty() {
        return ApiError::new(
            ApiErrorCode::SetupValidationRequired,
            ids::ERRORS_INITIALIZATION_VALIDATION_REQUIRED,
        )
        .details("No validated RPC endpoints are available to save")
        .into_response();
    }
    let chain = key.chain;
    let wallet_address = key.address.clone();

    // Setup is reachable only before a full setup (`setup_only_gate`). A vault that
    // already holds another trading wallet (setup completed from Explore Mode, or a
    // config that lost its key) keeps it until a staged replacement moves that wallet's
    // stores at the next start; the same wallet, or an empty vault, needs nothing.
    let current = tokio::task::spawn_blocking(move || {
        crate::wallets::replacement::trading_wallet_address(chain)
    })
    .await;
    let replacement_staged = match current {
        Ok(Ok(Some(current))) if current != wallet_address => {
            if let Err(e) = crate::wallets::replacement::stage(key.clone()).await {
                logger::warning(
                    LogTag::Webserver,
                    &format!("Setup refused to replace the trading wallet: {e}"),
                );
                // Open positions cannot be closed from setup: the refusal says to finish
                // setup with the current trading wallet's key and replace the wallet from
                // the Wallets page afterwards.
                let refusal = match &e {
                    crate::wallets::Error::ReplacementBlocked {
                        reason: RestartBlocker::OpenPositions { count },
                    } => ApiError::new(
                        ApiErrorCode::OpenPositions,
                        ids::ERRORS_INITIALIZATION_REPLACE_OPEN_POSITIONS,
                    )
                    .count_arg("count", i64::try_from(*count).unwrap_or(i64::MAX))
                    .details(e.to_string()),
                    other => replacement_error(other),
                };
                return refusal.into_response();
            }
            true
        }
        Ok(Ok(_)) => false,
        Ok(Err(e)) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_REPLACE_FAILED)
                .details(e.to_string())
                .into_response();
        }
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_REPLACE_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    logger::info(
        LogTag::Webserver,
        "Creating configuration with encrypted wallet...",
    );

    // Merge into the existing config when one is already loaded (e.g. completing setup
    // from Explore Mode) so user settings are preserved. Only fall back to
    // defaults on a true first run with no config in memory.
    let mut config = if config::is_config_initialized() {
        config::get_config_clone()
    } else {
        Config::default()
    };

    config.wallet_encrypted = key.ciphertext;
    config.wallet_nonce = key.nonce;
    *config.chains.access_mut(chain) = key.access.as_config_value().to_owned();
    if key.access == ChainAccessMode::OwnRpc {
        config.chains.rpc_mut(chain).urls = key.rpc_urls;
    }

    // Setup is now complete: clear the Explore Mode marker and mark onboarding done.
    config.gui.dashboard.startup.explore_mode_enabled = false;
    config.gui.dashboard.startup.onboarding_complete = true;

    let config_path = crate::paths::get_config_path();
    if let Err(e) =
        config::utils::save_config_to_file(&config, &config_path.to_string_lossy(), true)
    {
        // A staged replacement holds every trade and tool until the restart applies it,
        // so the restart is scheduled even though these settings were not saved.
        if replacement_staged {
            logger::error(
                LogTag::Webserver,
                &format!(
                    "Setup settings were not saved ({e}); the staged wallet replacement applies at the restart"
                ),
            );
            crate::webserver::routes::system::schedule_graceful_restart(
                "wallet replacement staged",
            );
            return ApiError::new(
                ApiErrorCode::ConfigError,
                ids::ERRORS_INITIALIZATION_SAVE_FAILED_REPLACEMENT_STAGED,
            )
            .details(e.to_string())
            .into_response();
        }
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    logger::info(LogTag::Webserver, "Configuration saved successfully");

    // A full restart is intentional here. Explore Mode may already own process-wide
    // wallet/RPC/service singletons; mutating that graph live made setup behave
    // differently from every later boot. The normal boot path now activates the
    // saved wallet and RPC from a clean state.
    crate::webserver::routes::system::schedule_graceful_restart("setup completed");

    let response = InitializationCompleteResponse {
        success: true,
        wallet_address,
        services_started: 0,
        errors,
        restart_required: true,
        instance_id: global::instance_id().to_owned(),
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

    let (step, status) = if !initialization_complete {
        ("pre-initialization", "waiting")
    } else if services_total == 0 {
        ("services-startup", "idle")
    } else if services_started < services_total {
        ("services-startup", "starting")
    } else {
        ("services-startup", "complete")
    };

    let response = InitializationProgressResponse {
        step: step.to_owned(),
        status: status.to_owned(),
        services_started,
        services_total,
    };

    success_response(response)
}

/// Start remaining services after initialization
async fn start_remaining_services() -> Result<services::ServiceStartupReport> {
    logger::info(
        LogTag::Webserver,
        "Requesting service startup from ServiceManager",
    );

    let manager_ref =
        services::get_service_manager()
            .await
            .ok_or_else(|| Error::ServiceStartup {
                detail: "ServiceManager not available".to_owned(),
            })?;

    let mut manager_guard = manager_ref.write().await;
    let manager = manager_guard
        .as_mut()
        .ok_or_else(|| Error::ServiceStartup {
            detail: "ServiceManager not initialized".to_owned(),
        })?;

    // Start newly enabled services
    let report = manager
        .start_newly_enabled()
        .await
        .map_err(|e| Error::ServiceStartup {
            detail: e.to_string(),
        })?;

    Ok(report)
}
