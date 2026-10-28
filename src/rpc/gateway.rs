// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Free transaction relaying for signed-in accounts.
//!
//! ============================================================================
//! WHY THIS IS NOT A PROVIDER IN THE POOL
//! ============================================================================
//! The obvious design is another `ProviderConfig` in the provider list. It is
//! the wrong one, and the reason is worth stating because it will look like an
//! omission later.
//!
//! Providers are chosen by `selector.rs`, which selects on health, latency and
//! priority — never on METHOD. A gateway in that list could be handed
//! `getMultipleAccounts` for pool polling, which runs roughly every 500 ms and
//! is almost the entire RPC cost of an installation. Teaching the selector about
//! methods to prevent that would put a capacity rule in a component whose job is
//! availability, and one refactor later the rule would be gone.
//!
//! So the gateway is an explicit FIRST ATTEMPT for a fixed list of submission
//! methods, checked here and nowhere else. Anything not on that list cannot
//! reach it, because no code path exists that would take it there.
//!
//! ============================================================================
//! UNDER OWN-RPC ACCESS IT NEVER BLOCKS A TRADE
//! ============================================================================
//! Every failure that proves nothing was delivered falls through, and the
//! caller carries on down the normal provider path with the user's own RPC. A
//! free service must not be able to stop somebody trading — not when it is
//! down, not when the quota is spent, not when the session has expired. A
//! failure after which the relay may already have delivered a send is the one
//! exception: it is reported as ambiguous so settlement reads the chain
//! before anything is rebuilt.
//!
//! Under service access the relay is the only submission path, used by the
//! service chain backend directly.

use std::time::Duration;

use serde_json::Value;

use crate::chains::access::{self as chain_access, ChainAccessMode, SUBMIT_SCOPE};
use crate::data_server::ServiceRefusal;
use crate::rpc::RpcError;

/// The only methods the gateway is ever asked for. The relay refuses any other
/// method, so asking would waste a round trip to be told so.
///
/// Submission and its immediate follow-up, and nothing that reads chain state
/// for pricing.
const RELAYED_METHODS: &[&str] = &[
    "sendTransaction",
    "getLatestBlockhash",
    "getSignatureStatuses",
    "simulateTransaction",
    "getFeeForMessage",
];

/// A hardcoded constant, like the account endpoint and for the same reason: a
/// config-editable relay URL would let anyone who can edit `config.toml`
/// redirect signed transactions to a server of their choosing. A signed
/// transaction cannot be altered, but it can be WITHHELD, and front-running a
/// swap you were handed early is worth real money.
const GATEWAY_URL: &str = "https://screenerbot.io/data/v1/rpc";

/// Short. If the relay is slow, the user's own RPC is right there.
const TIMEOUT: Duration = Duration::from_secs(12);

/// JSON-RPC "invalid params", the code a node gives a request it rejects
/// before reading it.
const INVALID_PARAMS: i64 = -32602;

/// Is this method one we would ever relay?
fn is_relayable(method: &str) -> bool {
    RELAYED_METHODS.contains(&method)
}

/// Should this call go through the gateway first?
///
/// The method is relayable, an account is signed in, that device holds the
/// submission scope, and either the chain is in service access mode or the
/// user switched relaying on. The scope check matters because scopes are
/// frozen at grant time on the server — a device authorised before this
/// feature existed does not acquire it by upgrading, and asking anyway would
/// produce a 403 on every swap.
pub fn should_relay(method: &str) -> bool {
    if !is_relayable(method) {
        return false;
    }

    let chosen = chain_access::active_mode() == ChainAccessMode::Service
        || crate::config::with_config(|config| config.account.use_gateway_rpc);
    if !chosen {
        return false;
    }

    crate::account::is_signed_in() && crate::account::has_scope(SUBMIT_SCOPE)
}

/// The relay call as sent: the account credential, the app version the service
/// requires before it answers, and the JSON-RPC method with its params.
fn relay_request(
    client: &reqwest::Client,
    token: &str,
    method: &str,
    params: &Value,
) -> reqwest::RequestBuilder {
    crate::data_server::with_app_version(client.post(GATEWAY_URL))
        .bearer_auth(token)
        .timeout(TIMEOUT)
        .json(&serde_json::json!({ "method": method, "params": params }))
}

/// What a relay attempt came to.
#[derive(Debug, Clone)]
pub enum Relayed {
    /// The gateway answered with a result.
    Answered(Value),
    /// A node answered with a JSON-RPC error (a preflight refusal, a bad
    /// blockhash), or the relay refused a parameter before dispatching.
    /// `after_delivery` is true when an earlier attempt of the same call may
    /// already have delivered it.
    Refused {
        error: RpcError,
        after_delivery: bool,
    },
    /// No node answer. `maybe_delivered` is true when the request may have
    /// been forwarded before the answer was lost.
    Unanswered { maybe_delivered: bool },
    /// Nothing was dispatched: offline, no session, a refused grant or
    /// version, or over the relay quota.
    NotAsked(ServiceRefusal),
}

/// Try the gateway and classify the outcome. The caller decides what each
/// outcome means for its access mode.
pub async fn relay(client: &reqwest::Client, method: &str, params: &Value) -> Relayed {
    if crate::connectivity::is_network_offline() {
        return Relayed::NotAsked(ServiceRefusal::Offline);
    }

    let Some(token) = crate::account::access_token().await else {
        return Relayed::NotAsked(ServiceRefusal::SignedOut);
    };

    let response = match relay_request(client, &token, method, params).send().await {
        Ok(response) => response,
        Err(error) => {
            log::debug!("RPC gateway: {method} failed in transport: {error}");
            return classify_transport_failure(error.is_connect());
        }
    };

    let status = response.status();
    let retry_after_header = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    let body = response.json::<Value>().await.ok();
    let relayed = classify_response(method, status, body.as_ref(), retry_after_header);
    if !matches!(relayed, Relayed::Answered(_)) {
        log::debug!("RPC gateway: {method} answered HTTP {status}: {relayed:?}");
    }
    relayed
}

/// A request that never got an HTTP answer. A connect failure proves nothing
/// left the machine; any other failure (a timeout above all) may have come
/// after the relay forwarded the request.
fn classify_transport_failure(is_connect: bool) -> Relayed {
    Relayed::Unanswered {
        maybe_delivered: !is_connect,
    }
}

/// Classifies an HTTP answer of the relay. Pure, so the table is testable.
fn classify_response(
    method: &str,
    status: reqwest::StatusCode,
    body: Option<&Value>,
    retry_after_header: Option<u64>,
) -> Relayed {
    let null = Value::Null;
    let body_value = body.unwrap_or(&null);
    let after_delivery = body_value.get("after_delivery").and_then(Value::as_bool);

    if status.is_success() {
        return match body_value.get("result") {
            Some(result) => Relayed::Answered(result.clone()),
            // Answered and unreadable: the relay may have forwarded it.
            None => Relayed::Unanswered {
                maybe_delivered: true,
            },
        };
    }

    let code = body_value.get("code").and_then(Value::as_str).unwrap_or("");
    match (status, code) {
        (reqwest::StatusCode::BAD_GATEWAY, "rpc_error") => {
            let rpc = body_value.get("rpc_error").unwrap_or(&null);
            Relayed::Refused {
                error: RpcError::ProviderError {
                    code: rpc.get("code").and_then(Value::as_i64).unwrap_or(0),
                    message: rpc
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    data: match rpc.get("data") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(text)) => Some(text.clone()),
                        Some(other) => Some(other.to_string()),
                    },
                },
                after_delivery: after_delivery.unwrap_or(false),
            }
        }
        (reqwest::StatusCode::BAD_GATEWAY, "relay_unavailable") => Relayed::Unanswered {
            maybe_delivered: after_delivery.unwrap_or(false),
        },
        // A body that predates the `code` field: no node answered, and for a
        // send the relay may already have forwarded it.
        (reqwest::StatusCode::BAD_GATEWAY, _) => Relayed::Unanswered {
            maybe_delivered: method == "sendTransaction",
        },
        (reqwest::StatusCode::BAD_REQUEST, "param_refused") => Relayed::Refused {
            error: RpcError::ProviderError {
                code: INVALID_PARAMS,
                message: body_value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("parameter refused by the relay")
                    .to_owned(),
                data: None,
            },
            after_delivery: false,
        },
        // Refused before any dispatch: auth, scope, version, quota, a
        // malformed request, or no node configured on the relay.
        (status, _)
            if status.is_client_error() || status == reqwest::StatusCode::SERVICE_UNAVAILABLE =>
        {
            let (code, minimum) = crate::data_server::refusal_code(body_value);
            match crate::data_server::classify_refusal(
                status,
                &code,
                minimum,
                body_value,
                retry_after_header,
            ) {
                ServiceRefusal::Unreachable => Relayed::Unanswered {
                    maybe_delivered: false,
                },
                refusal => Relayed::NotAsked(refusal),
            }
        }
        // Any other server failure may have come after the relay forwarded
        // the request.
        _ => Relayed::Unanswered {
            maybe_delivered: method == "sendTransaction",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relay_request_carries_the_credential_and_the_app_version() {
        let request = relay_request(
            &reqwest::Client::new(),
            "token",
            "sendTransaction",
            &serde_json::json!(["tx"]),
        )
        .build()
        .expect("request builds");
        let headers = request.headers();
        assert_eq!(
            headers
                .get("x-screenerbot-version")
                .map(|v| v.to_str().unwrap()),
            Some(crate::version::VERSION)
        );
        assert_eq!(
            headers
                .get(reqwest::header::AUTHORIZATION)
                .map(|v| v.to_str().unwrap()),
            Some("Bearer token")
        );
    }

    /// The short name of an outcome, for the classification table.
    fn shape(relayed: &Relayed) -> String {
        match relayed {
            Relayed::Answered(_) => "answered".to_owned(),
            Relayed::Refused {
                error,
                after_delivery,
            } => {
                let code = match error {
                    RpcError::ProviderError { code, .. } => *code,
                    _ => 0,
                };
                format!("refused({code},{after_delivery})")
            }
            Relayed::Unanswered { maybe_delivered } => format!("unanswered({maybe_delivered})"),
            Relayed::NotAsked(refusal) => format!("not_asked({refusal:?})"),
        }
    }

    /// Every relay answer classifies to the outcome that decides whether the
    /// call may fall through: only an outcome that proves nothing was
    /// delivered may.
    #[test]
    fn every_relay_answer_classifies_to_its_delivery_outcome() {
        use reqwest::StatusCode as S;
        let rpc_error = |after: bool| {
            serde_json::json!({
                "error": "Transaction simulation failed",
                "code": "rpc_error",
                "rpc_error": {"code": -32002, "message": "Transaction simulation failed", "data": {"logs": []}},
                "after_delivery": after
            })
        };
        let cases: Vec<(&str, S, Option<Value>, &str)> = vec![
            (
                "sendTransaction",
                S::OK,
                Some(serde_json::json!({"result": "sig"})),
                "answered",
            ),
            (
                "sendTransaction",
                S::OK,
                Some(serde_json::json!({})),
                "unanswered(true)",
            ),
            (
                "sendTransaction",
                S::BAD_GATEWAY,
                Some(rpc_error(false)),
                "refused(-32002,false)",
            ),
            (
                "sendTransaction",
                S::BAD_GATEWAY,
                Some(rpc_error(true)),
                "refused(-32002,true)",
            ),
            (
                "sendTransaction",
                S::BAD_GATEWAY,
                Some(serde_json::json!({"code": "relay_unavailable", "after_delivery": true})),
                "unanswered(true)",
            ),
            (
                "sendTransaction",
                S::BAD_GATEWAY,
                Some(serde_json::json!({"code": "relay_unavailable", "after_delivery": false})),
                "unanswered(false)",
            ),
            (
                "sendTransaction",
                S::BAD_GATEWAY,
                Some(serde_json::json!({"error": "upstream"})),
                "unanswered(true)",
            ),
            (
                "getSignatureStatuses",
                S::BAD_GATEWAY,
                Some(serde_json::json!({"error": "upstream"})),
                "unanswered(false)",
            ),
            (
                "sendTransaction",
                S::BAD_REQUEST,
                Some(
                    serde_json::json!({"code": "param_refused", "error": "skipPreflight is not relayed"}),
                ),
                "refused(-32602,false)",
            ),
            (
                "sendTransaction",
                S::UNAUTHORIZED,
                Some(serde_json::json!({"code": "signin_required"})),
                "not_asked(SignedOut)",
            ),
            (
                "sendTransaction",
                S::FORBIDDEN,
                Some(serde_json::json!({"code": "scope_missing"})),
                "not_asked(ReauthorizationRequired)",
            ),
            (
                "sendTransaction",
                S::UPGRADE_REQUIRED,
                Some(
                    serde_json::json!({"code": "version_unsupported", "error": "Update to 0.2.22 or newer."}),
                ),
                "not_asked(VersionUnsupported { minimum: \"0.2.22\" })",
            ),
            (
                "sendTransaction",
                S::TOO_MANY_REQUESTS,
                Some(serde_json::json!({"code": "quota_exceeded", "retry_after": 120})),
                "not_asked(Busy { retry_after: 120s })",
            ),
            (
                "sendTransaction",
                S::GATEWAY_TIMEOUT,
                None,
                "unanswered(true)",
            ),
            (
                "sendTransaction",
                S::SERVICE_UNAVAILABLE,
                None,
                "unanswered(false)",
            ),
        ];
        for (method, status, body, expected) in cases {
            assert_eq!(
                shape(&classify_response(method, status, body.as_ref(), None)),
                expected,
                "{method} {status} {body:?}"
            );
        }
    }

    /// A transport failure proves nothing was delivered only when the
    /// connection was never made; an app-side timeout may follow delivery.
    #[test]
    fn only_a_connect_failure_proves_nothing_was_delivered() {
        assert!(matches!(
            classify_transport_failure(true),
            Relayed::Unanswered {
                maybe_delivered: false
            }
        ));
        assert!(matches!(
            classify_transport_failure(false),
            Relayed::Unanswered {
                maybe_delivered: true
            }
        ));
    }
}
