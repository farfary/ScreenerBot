//! Initialization route types — request/response structs for onboarding endpoints.

use crate::rpc::RpcEndpointTestResult;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;

#[derive(Debug, Serialize)]
pub struct InitializationStatusResponse {
    pub required: bool,
    pub reason: String,
    pub config_exists: bool,
    pub initialization_complete: bool,
    pub onboarding_complete: bool,
    pub force_onboarding: bool,
    /// Whether the bot is running in discovery-only mode (wallet + RPC skipped).
    pub discovery_only_mode: bool,
    /// Whether the user previously chose to skip wallet + RPC setup (persisted).
    pub setup_skipped: bool,
}

#[derive(Debug, Serialize)]
pub struct SkipSetupResponse {
    pub success: bool,
    pub discovery_only_mode: bool,
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

/// Parse wallet private key from string (supports base58 and JSON array formats)
pub(super) fn parse_wallet_private_key(private_key: &str) -> Result<Keypair, String> {
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

    Err("Invalid private key format. Must be base58 string or JSON array of 64 bytes".to_owned())
}
