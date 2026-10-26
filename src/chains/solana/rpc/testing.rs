// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
/// An endpoint is usable only when it is healthy and proves it is mainnet-beta. With a
/// `wallet_address` it must also answer the wallet reads the balance monitor makes
/// (`getBalance` and `getTokenAccountsByOwner` for both token programs): an endpoint
/// that passes the node checks but blocks those reads leaves the wallet unreadable.
pub async fn test_rpc_endpoint(url: &str, wallet_address: Option<&str>) -> RpcEndpointTestResult {
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

    probe_endpoint(&client, url, &display_url, is_premium, wallet_address).await
}

/// The checks of `test_rpc_endpoint` against `url` with `client`.
pub(super) async fn probe_endpoint(
    client: &reqwest::Client,
    url: &str,
    display_url: &str,
    is_premium: bool,
    wallet_address: Option<&str>,
) -> RpcEndpointTestResult {
    let display_url = display_url.to_owned();
    let started_at = Instant::now();
    let (health, genesis, version) = tokio::join!(
        rpc_request(client, url, "getHealth", None),
        rpc_request(client, url, "getGenesisHash", None),
        rpc_request(client, url, "getVersion", None)
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
            return failed_result(
                url,
                &display_url,
                latency_ms,
                &error.to_string(),
                None,
                is_premium,
            );
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

    if let Some(wallet) = wallet_address {
        if let Err(error) = probe_wallet_reads(client, url, wallet).await {
            return failed_result(
                url,
                &display_url,
                started_at.elapsed().as_millis() as u64,
                &format!("The endpoint refuses the wallet reads ScreenerBot needs: {error}"),
                Some(true),
                is_premium,
            );
        }
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

/// The wallet reads of the balance monitor, each named in the error of the first that
/// fails.
async fn probe_wallet_reads(
    client: &reqwest::Client,
    url: &str,
    wallet: &str,
) -> crate::chains::solana::Result<()> {
    use crate::chains::solana::constants::{SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
    use crate::chains::solana::Error;

    let token_accounts = |program_id: &str| serde_json::json!([wallet, { "programId": program_id }, { "encoding": "jsonParsed" }]);
    let (balance, spl, token_2022) = tokio::join!(
        rpc_request(client, url, "getBalance", Some(serde_json::json!([wallet]))),
        rpc_request(
            client,
            url,
            "getTokenAccountsByOwner",
            Some(token_accounts(SPL_TOKEN_PROGRAM_ID))
        ),
        rpc_request(
            client,
            url,
            "getTokenAccountsByOwner",
            Some(token_accounts(TOKEN_2022_PROGRAM_ID))
        )
    );
    for (method, result) in [
        ("getBalance", balance),
        ("getTokenAccountsByOwner", spl),
        ("getTokenAccountsByOwner", token_2022),
    ] {
        if let Err(error) = result {
            let detail = match error {
                Error::Rpc { detail, .. } => detail,
                other => other.to_string(),
            };
            return Err(Error::Rpc {
                operation: "test_wallet_read",
                detail: format!("{method}: {detail}"),
            });
        }
    }
    Ok(())
}

async fn rpc_request(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: Option<serde_json::Value>,
) -> crate::chains::solana::Result<serde_json::Value> {
    use crate::chains::solana::Error;

    let mut request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
    });
    if let Some(params) = params {
        request["params"] = params;
    }
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|error| {
            let detail = if error.is_timeout() {
                "RPC request timed out".to_owned()
            } else if error.is_connect() {
                "Could not connect to the RPC endpoint".to_owned()
            } else {
                "RPC request failed".to_owned()
            };
            Error::Rpc {
                operation: "test_request",
                detail,
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let summary = crate::errors::summarize_http_body(&body);
        let detail = if summary.is_empty() {
            format!("HTTP status: {status}")
        } else {
            format!("HTTP status: {status} ({summary})")
        };
        return Err(Error::Rpc {
            operation: "test_request",
            detail,
        });
    }

    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| Error::Rpc {
            operation: "test_request",
            detail: "RPC endpoint returned an invalid JSON response".to_owned(),
        })?;

    if body.get("error").is_some() {
        return Err(Error::Rpc {
            operation: "test_request",
            detail: "RPC endpoint returned an error response".to_owned(),
        });
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
pub async fn test_rpc_endpoints(
    urls: &[String],
    wallet_address: Option<&str>,
) -> Vec<RpcEndpointTestResult> {
    use futures::future::join_all;

    join_all(
        urls.iter()
            .map(|url| test_rpc_endpoint(url, wallet_address)),
    )
    .await
}

/// Validate that an endpoint is on Solana mainnet.
pub async fn validate_mainnet(url: &str) -> crate::chains::solana::Result<bool> {
    use crate::chains::solana::Error;

    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| Error::Rpc {
            operation: "create_client",
            detail: error.to_string(),
        })?;
    let body = rpc_request(&client, url, "getGenesisHash", None).await?;
    let genesis_hash = body
        .get("result")
        .and_then(|result| result.as_str())
        .ok_or_else(|| Error::Decode {
            payload: "rpc response",
            detail: "missing genesis hash in response".to_owned(),
        })?;

    Ok(genesis_hash == MAINNET_GENESIS_HASH)
}

/// Get the version of an RPC node.
pub async fn get_rpc_version(url: &str) -> crate::chains::solana::Result<String> {
    use crate::chains::solana::Error;

    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| Error::Rpc {
            operation: "create_client",
            detail: error.to_string(),
        })?;
    let body = rpc_request(&client, url, "getVersion", None).await?;

    Ok(body
        .get("result")
        .and_then(|result| result.get("solana-core"))
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_owned())
}
