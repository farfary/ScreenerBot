//! Initialization route types — request/response structs for onboarding endpoints.

use crate::chains::solana::rpc::RpcEndpointTestResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct InitializationStatusResponse {
    pub required: bool,
    pub reason: String,
    pub config_exists: bool,
    pub initialization_complete: bool,
    pub onboarding_complete: bool,
    pub force_onboarding: bool,
    /// Whether the bot is running in Explore Mode without wallet + RPC.
    pub explore_mode: bool,
    /// Whether the user persisted Explore Mode as the current startup choice.
    pub explore_mode_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ExploreModeResponse {
    pub success: bool,
    pub explore_mode: bool,
    pub services_started: usize,
    pub errors: Vec<String>,
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
    /// One-time receipt proving this exact wallet/RPC snapshot passed validation.
    /// The receipt contains no credentials and expires after five minutes.
    pub validation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteInitializationRequest {
    pub validation_id: String,
    pub wallet_private_key: String,
    pub rpc_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InitializationCompleteResponse {
    pub success: bool,
    pub wallet_address: String,
    pub services_started: usize,
    pub errors: Vec<String>,
    /// Full setup is activated by a clean process boot, not an in-process tier mutation.
    pub restart_required: bool,
    /// Identity of the process that accepted setup. Browser clients wait until
    /// health reports a different instance before reloading.
    pub instance_id: String,
}

#[derive(Debug, Serialize)]
pub struct InitializationProgressResponse {
    pub step: String,
    pub status: String,
    pub message: String,
    pub services_started: usize,
    pub services_total: usize,
}
