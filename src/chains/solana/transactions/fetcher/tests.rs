// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction fetcher tests.

use super::*;
use crate::chains::solana::error::Error as SolanaError;
use crate::errors::ErrorClass;
use crate::rpc::RpcError;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn detail_request_uses_configured_commitment() {
    for (config, expected) in [
        (FetcherConfig::default(), CommitmentLevel::Confirmed),
        (
            FetcherConfig {
                commitment: CommitmentConfig::finalized(),
                ..FetcherConfig::default()
            },
            CommitmentLevel::Finalized,
        ),
    ] {
        let fetcher = TransactionFetcher::with_config(config);
        let observed = Arc::new(Mutex::new(None));
        let request_observed = Arc::clone(&observed);
        let details = fetcher
            .request_transaction_details("signature", move |_, commitment| {
                *request_observed.lock().expect("request seam lock") = Some(commitment);
                std::future::ready(Ok(serde_json::from_value(serde_json::json!({
                    "slot": 1,
                    "transaction": { "message": { "accountKeys": [] }, "signatures": [] },
                    "meta": null
                }))
                .expect("transaction details fixture")))
            })
            .await
            .expect("request succeeds");
        assert_eq!(*observed.lock().expect("request seam lock"), Some(expected));
        assert_eq!(details.slot, 1);
    }
}

#[test]
fn transaction_fetcher_preserves_rpc_retry_classification() {
    let unsupported_version = SolanaError::RpcFailure {
        operation: "get_transaction_details",
        source: RpcError::ProviderError {
            code: -32015,
            message: "Transaction version (1) is not supported".to_owned(),
            data: None,
        },
    };
    assert!(!unsupported_version.is_retryable());

    let provider_fault = SolanaError::RpcFailure {
        operation: "get_transaction_details",
        source: RpcError::ProviderError {
            code: -32005,
            message: "node is unhealthy".to_owned(),
            data: None,
        },
    };
    assert!(provider_fault.is_retryable());
}
