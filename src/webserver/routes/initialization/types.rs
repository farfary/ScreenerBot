// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

/// `Debug` is implemented by hand so a stray `{:?}` (log line, error context, panic
/// message) never prints the private key or an RPC URL, which may carry a provider key.
#[derive(Deserialize)]
pub struct ValidateCredentialsRequest {
    pub wallet_private_key: String,
    pub rpc_urls: Vec<String>,
}

impl std::fmt::Debug for ValidateCredentialsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateCredentialsRequest")
            .field("wallet_private_key", &"[redacted]")
            .field("rpc_urls", &self.rpc_urls.len())
            .finish()
    }
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

/// `Debug` is implemented by hand for the reason given on [`ValidateCredentialsRequest`].
#[derive(Deserialize)]
pub struct CompleteInitializationRequest {
    pub validation_id: String,
    pub wallet_private_key: String,
    pub rpc_urls: Vec<String>,
}

impl std::fmt::Debug for CompleteInitializationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompleteInitializationRequest")
            .field("validation_id", &self.validation_id)
            .field("wallet_private_key", &"[redacted]")
            .field("rpc_urls", &self.rpc_urls.len())
            .finish()
    }
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
    pub services_started: usize,
    pub services_total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "placeholder-wallet-key-for-redaction";
    const RPC_URL: &str = "https://rpc.example.com/?api-key=provider-secret";

    #[test]
    fn credential_requests_never_print_the_key_or_an_rpc_url() {
        let validate = format!(
            "{:?}",
            ValidateCredentialsRequest {
                wallet_private_key: KEY.to_owned(),
                rpc_urls: vec![RPC_URL.to_owned()],
            }
        );
        let complete = format!(
            "{:#?}",
            CompleteInitializationRequest {
                validation_id: "receipt-1".to_owned(),
                wallet_private_key: KEY.to_owned(),
                rpc_urls: vec![RPC_URL.to_owned()],
            }
        );
        for output in [&validate, &complete] {
            assert!(!output.contains(KEY), "{output}");
            assert!(!output.contains("provider-secret"), "{output}");
            assert!(output.contains("[redacted]"), "{output}");
        }
        assert!(complete.contains("receipt-1"));
    }
}
