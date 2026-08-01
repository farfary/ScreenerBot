//! RPC endpoint testing utilities
//!
//! Functions for testing RPC endpoint connectivity, latency, and validation.

use crate::logger::{self, LogTag};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const MAINNET_GENESIS_HASH: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";

/// RPC endpoint test result with detailed metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpointTestResult {
    /// Retained for internal selection after validation, never serialized to the dashboard.
    #[serde(skip_serializing)]
    pub url: String,
    /// Hostname-only label safe for logs, screenshots, and dashboard output.
    pub display_url: String,
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub is_mainnet: Option<bool>,
    pub version: Option<String>,
    pub is_premium: bool,
}

/// Test a single HTTPS endpoint without using the global RPC client.
///
/// Health, genesis hash, and node version are independent and run concurrently.
/// An endpoint is usable only when it is healthy and proves it is mainnet-beta.
pub async fn test_rpc_endpoint(url: &str) -> RpcEndpointTestResult {
    let parsed = match url::Url::parse(url) {
        Ok(parsed) if parsed.scheme() == "https" && parsed.host_str().is_some() => parsed,
        _ => {
            return failed_result(
                url,
                "Invalid endpoint",
                0,
                "RPC endpoints must be valid HTTPS URLs",
                None,
                false,
            );
        }
    };

    let display_url = parsed.host_str().unwrap_or("RPC endpoint").to_owned();
    logger::debug(LogTag::Rpc, &format!("Testing RPC endpoint: {display_url}"));

    let lower = display_url.to_lowercase();
    let is_premium = lower.contains("helius")
        || lower.contains("quicknode")
        || lower.contains("quiknode")
        || lower.contains("alchemy")
        || lower.contains("triton")
        || lower.contains("shyft")
        || lower.contains("getblock");

    let client = match crate::net::client_builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return failed_result(
                url,
                &display_url,
                0,
                "Failed to create the RPC test client",
                None,
                is_premium,
            );
        }
    };

    let started_at = Instant::now();
    let (health, genesis, version) = tokio::join!(
        rpc_request(&client, url, "getHealth"),
        rpc_request(&client, url, "getGenesisHash"),
        rpc_request(&client, url, "getVersion")
    );
    let latency_ms = started_at.elapsed().as_millis() as u64;

    match health {
        Ok(body) if body.get("result").and_then(|result| result.as_str()) == Some("ok") => {}
        Ok(_) => {
            return failed_result(
                url,
                &display_url,
                latency_ms,
                "RPC health check did not return ok",
                None,
                is_premium,
            );
        }
        Err(error) => {
            return failed_result(url, &display_url, latency_ms, &error, None, is_premium);
        }
    }

    let genesis_hash = match genesis {
        Ok(body) => body
            .get("result")
            .and_then(|result| result.as_str())
            .unwrap_or_default()
            .to_owned(),
        Err(error) => {
            return failed_result(
                url,
                &display_url,
                latency_ms,
                &format!("Could not verify Solana network: {error}"),
                None,
                is_premium,
            );
        }
    };

    if genesis_hash != MAINNET_GENESIS_HASH {
        return failed_result(
            url,
            &display_url,
            latency_ms,
            "Endpoint is not Solana mainnet-beta",
            Some(false),
            is_premium,
        );
    }

    let version = version.ok().and_then(|body| {
        body.get("result")
            .and_then(|result| result.get("solana-core"))
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    });

    RpcEndpointTestResult {
        url: url.to_owned(),
        display_url,
        success: true,
        latency_ms,
        error: None,
        is_mainnet: Some(true),
        version,
        is_premium,
    }
}

async fn rpc_request(
    client: &reqwest::Client,
    url: &str,
    method: &str,
) -> Result<serde_json::Value, String> {
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
        }))
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                "RPC request timed out".to_owned()
            } else if error.is_connect() {
                "Could not connect to the RPC endpoint".to_owned()
            } else {
                "RPC request failed".to_owned()
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "RPC endpoint returned an invalid JSON response".to_owned())?;

    if body.get("error").is_some() {
        return Err("RPC endpoint returned an error response".to_owned());
    }

    Ok(body)
}

fn failed_result(
    url: &str,
    display_url: &str,
    latency_ms: u64,
    error: &str,
    is_mainnet: Option<bool>,
    is_premium: bool,
) -> RpcEndpointTestResult {
    RpcEndpointTestResult {
        url: url.to_owned(),
        display_url: display_url.to_owned(),
        success: false,
        latency_ms,
        error: Some(error.to_owned()),
        is_mainnet,
        version: None,
        is_premium,
    }
}

/// Test multiple endpoints concurrently.
pub async fn test_rpc_endpoints(urls: &[String]) -> Vec<RpcEndpointTestResult> {
    use futures::future::join_all;

    join_all(urls.iter().map(|url| test_rpc_endpoint(url))).await
}

/// Validate that an endpoint is on Solana mainnet.
pub async fn validate_mainnet(url: &str) -> Result<bool, String> {
    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("Failed to create client: {error}"))?;
    let body = rpc_request(&client, url, "getGenesisHash").await?;
    let genesis_hash = body
        .get("result")
        .and_then(|result| result.as_str())
        .ok_or("Missing genesis hash in response")?;

    Ok(genesis_hash == MAINNET_GENESIS_HASH)
}

/// Get the version of an RPC node.
pub async fn get_rpc_version(url: &str) -> Result<String, String> {
    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("Failed to create client: {error}"))?;
    let body = rpc_request(&client, url, "getVersion").await?;

    Ok(body
        .get("result")
        .and_then(|result| result.get("solana-core"))
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_owned())
}
