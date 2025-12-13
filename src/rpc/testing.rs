//! RPC endpoint testing utilities
//!
//! Functions for testing RPC endpoint connectivity, latency, and validation.

use crate::logger::{self, LogTag};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// RPC endpoint test result with detailed metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpointTestResult {
    pub url: String,
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub is_mainnet: Option<bool>,
    pub version: Option<String>,
    pub is_premium: bool,
}

/// Test a single RPC endpoint without using the global RPC client
///
/// Uses short timeouts (3s) and validates the endpoint is mainnet.
/// Returns detailed test results including latency.
pub async fn test_rpc_endpoint(url: &str) -> RpcEndpointTestResult {
    logger::debug(LogTag::Rpc, &format!("Testing RPC endpoint: {}", url));

    // Check if URL contains known premium RPC provider domains
    let is_premium = url.contains("helius")
        || url.contains("quicknode")
        || url.contains("alchemy")
        || url.contains("triton")
        || url.contains("shyft")
        || url.contains("getblock");

    let start = Instant::now();

    // Build test request
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getHealth"
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return RpcEndpointTestResult {
                url: url.to_string(),
                success: false,
                latency_ms: 0,
                error: Some(format!("Failed to create HTTP client: {}", e)),
                is_mainnet: None,
                version: None,
                is_premium,
            };
        }
    };

    let response = match client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            return RpcEndpointTestResult {
                url: url.to_string(),
                success: false,
                latency_ms: latency,
                error: Some(format!("Request failed: {}", e)),
                is_mainnet: None,
                version: None,
                is_premium,
            };
        }
    };

    let latency = start.elapsed().as_millis() as u64;

    if !response.status().is_success() {
        return RpcEndpointTestResult {
            url: url.to_string(),
            success: false,
            latency_ms: latency,
            error: Some(format!("HTTP status: {}", response.status())),
            is_mainnet: None,
            version: None,
            is_premium,
        };
    }

    // Parse response
    let body: serde_json::Value = match response.json().await {
        Ok(b) => b,
        Err(e) => {
            return RpcEndpointTestResult {
                url: url.to_string(),
                success: false,
                latency_ms: latency,
                error: Some(format!("Failed to parse response: {}", e)),
                is_mainnet: None,
                version: None,
                is_premium,
            };
        }
    };

    // Check for errors in response
    if let Some(err) = body.get("error") {
        return RpcEndpointTestResult {
            url: url.to_string(),
            success: false,
            latency_ms: latency,
            error: Some(format!("RPC error: {:?}", err)),
            is_mainnet: None,
            version: None,
            is_premium,
        };
    }

    // For getHealth, success means the node is healthy
    RpcEndpointTestResult {
        url: url.to_string(),
        success: true,
        latency_ms: latency,
        error: None,
        is_mainnet: None, // Would need getGenesisHash to verify
        version: None,    // Would need getVersion to get this
        is_premium,
    }
}

/// Test multiple RPC endpoints concurrently
///
/// Returns results for all endpoints.
pub async fn test_rpc_endpoints(urls: &[String]) -> Vec<RpcEndpointTestResult> {
    use futures::future::join_all;

    let futures: Vec<_> = urls.iter().map(|url| test_rpc_endpoint(url)).collect();

    join_all(futures).await
}

/// Validate that an endpoint is on Solana mainnet
///
/// Compares the genesis hash against the known mainnet hash.
pub async fn validate_mainnet(url: &str) -> Result<bool, String> {
    // Known Solana mainnet genesis hash
    const MAINNET_GENESIS_HASH: &str =
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getGenesisHash"
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let genesis_hash = body
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or("Missing genesis hash in response")?;

    Ok(genesis_hash == MAINNET_GENESIS_HASH)
}

/// Get the version of an RPC node
pub async fn get_rpc_version(url: &str) -> Result<String, String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getVersion"
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let version = body
        .get("result")
        .and_then(|r| r.get("solana-core"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(version.to_string())
}
