// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain access API handlers - the access status and the mode switch, validated like setup and applied at a restart.

use axum::extract::Json;
use axum::response::{IntoResponse as _, Response};

use super::types::*;
use crate::chains::access::{self as chain_access, ModeSwitch};
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::routes::initialization::validation::validate_rpc_url_list;
use crate::webserver::utils::success_response;

/// GET /api/chain/access
/// The active chain's access: mode, service state, capabilities and account standing.
pub(super) async fn get_chain_access() -> Response {
    success_response(chain_access::active_status())
}

/// POST /api/chain/access
/// Switch the access mode. Own RPC tests the given endpoints and saves the working ones;
/// the service is probed first. The mode is saved and applies at the restart scheduled
/// here; a switch to the current mode is refused.
pub(super) async fn switch_chain_access(Json(request): Json<SwitchChainAccessRequest>) -> Response {
    let rpc_urls = match chain_access::plan_active_switch(request.mode) {
        ModeSwitch::Unchanged => {
            return ApiError::new(ApiErrorCode::Conflict, ids::ERRORS_CHAIN_ACCESS_UNCHANGED)
                .into_response();
        }
        ModeSwitch::TestEndpoints => match working_endpoints(&request.rpc_urls).await {
            Ok(urls) => Some(urls),
            Err(error) => return error.into_response(),
        },
        ModeSwitch::ProbeService => {
            if let Err(refusal) = chain_access::probe_active_service().await {
                return ApiError::chain_access(&refusal).into_response();
            }
            None
        }
    };

    let mode = request.mode;
    let saved =
        tokio::task::spawn_blocking(move || chain_access::save_active_mode(mode, rpc_urls)).await;
    let chain = match saved {
        Ok(Ok(chain)) => chain,
        Ok(Err(error)) => {
            return ApiError::new(
                ApiErrorCode::ConfigError,
                ids::ERRORS_CHAIN_ACCESS_SWITCH_FAILED,
            )
            .details(error.to_string())
            .into_response()
        }
        Err(error) => {
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_CHAIN_ACCESS_SWITCH_FAILED,
            )
            .details(error.to_string())
            .into_response()
        }
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "Chain access of {chain} switched to {}; restarting",
            mode.as_config_value()
        ),
    );
    crate::webserver::routes::system::schedule_graceful_restart("chain access changed");
    success_response(SwitchChainAccessResponse {
        mode,
        restart_required: true,
        instance_id: crate::global::instance_id().to_owned(),
    })
}

/// The endpoints of `urls` that pass the setup checks and answer the endpoint test, in
/// their order. Refused when none does.
async fn working_endpoints(urls: &[String]) -> Result<Vec<String>, ApiError> {
    let errors = validate_rpc_url_list(urls);
    if !errors.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CHAIN_ACCESS_SWITCH_FAILED,
        )
        .details(errors.join("; ")));
    }
    let results = chain_access::test_active_endpoints(urls).await;
    let working: Vec<String> = results
        .iter()
        .zip(urls)
        .filter(|(success, _)| **success)
        .map(|(_, url)| url.clone())
        .collect();
    if working.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::UpstreamError,
            ids::ERRORS_CHAIN_ACCESS_SWITCH_FAILED,
        )
        .details("All RPC endpoints failed connection tests"));
    }
    Ok(working)
}
