// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain access route types — the mode switch request and its answer.

use crate::chains::access::ChainAccessMode;
use serde::{Deserialize, Serialize};

/// `POST /api/chain/access`. `Debug` is implemented by hand so an RPC URL, which may
/// carry a provider key, never reaches a log line.
#[derive(Deserialize)]
pub struct SwitchChainAccessRequest {
    pub mode: ChainAccessMode,
    /// The endpoints to test and save when switching to own RPC.
    #[serde(default)]
    pub rpc_urls: Vec<String>,
}

impl std::fmt::Debug for SwitchChainAccessRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwitchChainAccessRequest")
            .field("mode", &self.mode)
            .field("rpc_urls", &self.rpc_urls.len())
            .finish()
    }
}

/// The saved mode, which takes effect at the scheduled restart.
#[derive(Debug, Serialize)]
pub struct SwitchChainAccessResponse {
    pub mode: ChainAccessMode,
    pub restart_required: bool,
    pub instance_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_request_never_prints_an_rpc_url() {
        let request: SwitchChainAccessRequest = serde_json::from_value(serde_json::json!({
            "mode": "own_rpc",
            "rpc_urls": ["https://rpc.example.com/?api-key=provider-secret"],
        }))
        .expect("decode the request");
        let output = format!("{request:?}");
        assert!(!output.contains("provider-secret"), "{output}");
        assert_eq!(request.mode, ChainAccessMode::OwnRpc);

        let service: SwitchChainAccessRequest =
            serde_json::from_value(serde_json::json!({ "mode": "service" }))
                .expect("a service switch needs no endpoint");
        assert!(service.rpc_urls.is_empty());
    }
}
