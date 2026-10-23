// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The ONE way this app talks to the ScreenerBot data service.
//!
//! ============================================================================
//! WHY THIS MODULE EXISTS
//! ============================================================================
//! Six subsystems read from screenerbot.io/data — candles, the native/USD
//! reference chart, the pool registry, Rugcheck reports, token decimals and
//! boosted-token identity. Each of them used to build its own URL, own timeout, own "was that
//! a 200?" check and own silent `None`. That was survivable while the service was
//! open; it stopped being survivable the moment the service started asking WHO
//! is calling, because six copies of an authentication rule is six chances to
//! get it wrong and no place at all to answer "why is my data missing?".
//!
//! So: one client. It resolves the endpoint, attaches the credential, states
//! this build's version, names the chain, classifies the answer, and publishes a
//! single availability state that the setup screen and Settings both read.
//!
//! ============================================================================
//! EVERY REQUEST NAMES ITS CHAIN
//! ============================================================================
//! `get_json` takes a `ChainId` and the one request builder appends it as the
//! `chain` query parameter; a caller never passes it. A refusal of that chain
//! (HTTP 400) concerns that request alone: the service already accepted the
//! session and the version before refusing, so the shared availability state
//! stays usable for every other chain and surface.
//!
//! ============================================================================
//! IT IS AN ACCELERATOR, NEVER A DEPENDENCY
//! ============================================================================
//! Every caller falls back to its direct provider on `None`, and that is a rule
//! rather than a coincidence. A signed-out install discovers tokens, prices
//! pools, charts candles and trades exactly as before — slower, against public
//! rate limits, with thinner history. Nothing here may become load-bearing for a
//! trade.
//!
//! ============================================================================
//! WHY A SIGNED-OUT INSTALL MAKES NO REQUEST AT ALL
//! ============================================================================
//! We know the answer before we ask: without an account there is no credential
//! to present, and the service will refuse. Sending the request anyway would
//! spend a round trip per token per refresh to be told something we already
//! knew, and would put a wall of 401s in the service's logs that says nothing.

pub mod access;

use std::future::Future;
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::chains::ChainId;

pub use access::{status, DataAccess, DataAccessStatus};

/// The app states its version so the service can retire a release. Kept in step
/// with the header the Data Server reads in `api/auth.rs`.
const VERSION_HEADER: &str = "x-screenerbot-version";

/// Which config section supplies the endpoint for this call.
///
/// Two sections exist because OHLCV and token data are separately switchable —
/// somebody debugging charts may turn the shared candle source off without
/// giving up the shared pool registry. They are read, never merged.
#[derive(Debug, Clone, Copy)]
pub enum Surface {
    /// `[tokens.sources.screenerbot_server]` — pools, Rugcheck, decimals, market.
    Tokens,
    /// `[ohlcv.sources.screenerbot_server]` — candles and the native/USD chart.
    Ohlcv,
}

impl Surface {
    /// `(endpoint, timeout)` when this surface is switched on and configured.
    ///
    /// The two sections are separate config STRUCTS, not two instances of one,
    /// so they are read separately rather than through a shared reference.
    fn settings(self) -> Option<(String, Duration)> {
        crate::config::with_config(|config| {
            let (enabled, endpoint, timeout_seconds) = match self {
                Surface::Tokens => {
                    let source = &config.tokens.sources.screenerbot_server;
                    (source.enabled, &source.endpoint, source.timeout_seconds)
                }
                Surface::Ohlcv => {
                    let source = &config.ohlcv.sources.screenerbot_server;
                    (source.enabled, &source.endpoint, source.timeout_seconds)
                }
            };
            let endpoint = endpoint.trim_end_matches('/').to_string();
            if !enabled || endpoint.is_empty() {
                return None;
            }
            Some((endpoint, Duration::from_secs(timeout_seconds)))
        })
    }
}

/// The service's machine-readable refusal codes, mapped to what we tell the user.
///
/// Matched on the code and never on the sentence: the sentence is written for a
/// person and will be improved, and a client that pattern-matched prose would
/// break the first time it was.
fn access_for_refusal(
    status: reqwest::StatusCode,
    code: &str,
    minimum: Option<String>,
) -> DataAccess {
    match code {
        "signin_required" | "token_invalid" | "token_expired" | "token_wrong_audience" => {
            DataAccess::SignedOut
        }
        "reauthorization_required" | "scope_missing" => DataAccess::ReauthorizationRequired,
        "version_unsupported" => DataAccess::VersionUnsupported {
            minimum: minimum.unwrap_or_else(|| "a newer release".to_string()),
        },
        _ => match status {
            reqwest::StatusCode::UNAUTHORIZED => DataAccess::SignedOut,
            reqwest::StatusCode::FORBIDDEN => DataAccess::ReauthorizationRequired,
            reqwest::StatusCode::UPGRADE_REQUIRED => DataAccess::VersionUnsupported {
                minimum: minimum.unwrap_or_else(|| "a newer release".to_string()),
            },
            // The access gate runs before any handler, so a 400 proves the service
            // is reachable and the session and version accepted; it refuses this
            // request alone (an unserved chain or a malformed parameter).
            reqwest::StatusCode::BAD_REQUEST => DataAccess::Ready,
            _ => DataAccess::Unreachable,
        },
    }
}

/// A refusal of the presented access token itself, which a refresh answers.
///
/// While a session exists this is a stale credential, never a sign-out: the
/// cached token expired (a long sleep, a clock step) or was replaced, and the
/// refresh token behind it is untouched.
fn is_stale_access_token(code: &str) -> bool {
    matches!(code, "token_expired" | "token_invalid")
}

/// Where the request credential comes from: the account session in the app,
/// a stub in tests.
trait Credential {
    fn access_token(&self) -> impl Future<Output = Option<String>> + Send;
    fn invalidate(&self, stale: &str);
    fn has_session(&self) -> bool;
}

struct AccountCredential;

impl Credential for AccountCredential {
    fn access_token(&self) -> impl Future<Output = Option<String>> + Send {
        crate::account::access_token()
    }

    fn invalidate(&self, stale: &str) {
        crate::account::invalidate_access_token(stale);
    }

    fn has_session(&self) -> bool {
        crate::account::is_signed_in()
    }
}

/// Pull the refusal code and, for a version refusal, the minimum it names.
///
/// The service writes the minimum into its sentence rather than a field, so it
/// is lifted out here rather than displayed as a whole server sentence inside an
/// app sentence.
fn refusal_code(body: &serde_json::Value) -> (String, Option<String>) {
    let code = body
        .get("code")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    let minimum = body
        .get("error")
        .and_then(|value| value.as_str())
        .and_then(|message| {
            message
                .split_whitespace()
                .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .map(|word| word.trim_end_matches(['.', ',']).to_string())
        });

    (code, minimum)
}

/// Stamps a request to the data service with the app's version. Every call to
/// the service goes through here: a request without it is refused before any
/// handler runs.
pub(crate) fn with_app_version(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request.header(VERSION_HEADER, crate::version::VERSION)
}

/// Builds a GET to the data service for `chain`: the app version header, the
/// caller's query pairs in order, then exactly one `chain` pair. The single place
/// a data service GET is built. A caller pair named `chain` would send the
/// parameter twice, so it panics in every build profile.
fn service_request(
    client: &reqwest::Client,
    url: &str,
    chain: ChainId,
    query: &[(&str, String)],
) -> reqwest::RequestBuilder {
    assert!(
        query.iter().all(|(key, _)| *key != "chain"),
        "the chain query parameter is appended by the request builder"
    );
    with_app_version(client.get(url))
        .query(query)
        .query(&[("chain", chain.as_str())])
}

/// GET a JSON payload from the data service for one chain.
///
/// `None` means "use your own provider", for every reason: switched off,
/// offline, signed out, refused, unreachable, or an answer we could not read.
/// The reason is published to `access` so exactly one place has to explain it.
pub async fn get_json<T: DeserializeOwned>(
    surface: Surface,
    chain: ChainId,
    path: &str,
    query: &[(&str, String)],
) -> Option<T> {
    let Some((endpoint, timeout)) = surface.settings() else {
        access::record(DataAccess::Disabled);
        return None;
    };

    if crate::connectivity::is_network_offline() {
        access::record(DataAccess::Offline);
        return None;
    }

    let url = format!("{endpoint}{path}");
    let client = crate::net::client();
    fetch_json(&AccountCredential, path, chain, |token| {
        service_request(&client, &url, chain, query)
            .bearer_auth(token)
            .timeout(timeout)
            .send()
    })
    .await
}

/// Send with the current credential and classify the answer.
///
/// A stale access token is dropped, refreshed once through the account's
/// single-flight lock, and the request retried once. A session that exists but
/// cannot produce a token right now (a transient refresh failure) is recorded
/// as a transport failure: the account is still signed in, and saying
/// otherwise would tell the user to sign in to fix a server hiccup.
async fn fetch_json<T, C, Request, RequestFuture>(
    credential: &C,
    path: &str,
    chain: ChainId,
    send: Request,
) -> Option<T>
where
    T: DeserializeOwned,
    C: Credential,
    Request: Fn(String) -> RequestFuture,
    RequestFuture: Future<Output = reqwest::Result<reqwest::Response>>,
{
    let mut retried = false;
    loop {
        // No credential, no request. See the module header.
        let Some(token) = credential.access_token().await else {
            if credential.has_session() {
                access::record_transport_failure();
            } else {
                access::record(DataAccess::SignedOut);
            }
            return None;
        };

        let response = match send(token.clone()).await {
            Ok(response) => response,
            Err(error) => {
                log::debug!("Data Server: {path} on {chain} failed: {error}");
                access::record_transport_failure();
                return None;
            }
        };

        let status = response.status();
        if !status.is_success() {
            // A refusal body is small and always JSON; an unreadable one is
            // treated as the status alone rather than as a transport failure.
            let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
            let (code, minimum) = refusal_code(&body);
            log::debug!("Data Server: {path} on {chain} refused with {status} code={code:?}");

            if is_stale_access_token(&code) && credential.has_session() {
                credential.invalidate(&token);
                if !retried {
                    retried = true;
                    continue;
                }
                access::record_transport_failure();
                return None;
            }

            match access_for_refusal(status, &code, minimum) {
                DataAccess::Unreachable => access::record_transport_failure(),
                refusal => access::record(refusal),
            }
            return None;
        }

        return match response.json::<T>().await {
            Ok(value) => {
                access::record(DataAccess::Ready);
                Some(value)
            }
            Err(error) => {
                // The service answered and we could not read it. That is our bug
                // or a shape change, not a permission problem, so it is logged
                // rather than reported to the user as an account state.
                log::debug!("Data Server: {path} on {chain} returned an unreadable body: {error}");
                access::record(DataAccess::Ready);
                None
            }
        };
    }
}

/// Is it worth making a request right now?
///
/// Used by callers that would otherwise loop — a chunked batch, seven
/// timeframes in a row — so one unusable state costs one check rather than one
/// refused round trip per item.
///
/// The local facts are RE-EVALUATED here rather than read off the last recorded
/// state, and that distinction is the whole correctness of this function.
/// Config, connectivity and the session all change without any call being made,
/// so a cached "no" for one of them would latch: nothing would call, so nothing
/// would record a new state, so nothing would ever call again. Only the two
/// refusals that genuinely cannot change without a sign-in or an upgrade are
/// taken from the recorded state — and `access::reset` clears those the moment
/// the session changes.
pub fn is_usable(surface: Surface) -> bool {
    if surface.settings().is_none() {
        return false;
    }
    if crate::connectivity::is_network_offline() {
        return false;
    }
    if !crate::account::is_signed_in() {
        return false;
    }

    !matches!(
        access::current(),
        DataAccess::ReauthorizationRequired | DataAccess::VersionUnsupported { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_service_request_carries_the_app_version() {
        let request = with_app_version(reqwest::Client::new().get("https://example.invalid/v1"))
            .build()
            .expect("request builds");
        assert_eq!(
            request
                .headers()
                .get(VERSION_HEADER)
                .map(|v| v.to_str().unwrap()),
            Some(crate::version::VERSION)
        );
    }

    fn chain_pairs(request: &reqwest::Request) -> Vec<String> {
        request
            .url()
            .query_pairs()
            .filter(|(key, _)| key == "chain")
            .map(|(_, value)| value.into_owned())
            .collect()
    }

    #[test]
    fn every_service_request_names_exactly_one_chain_after_the_caller_pairs() {
        let query = [("mint", "abc".to_string()), ("limit", "5".to_string())];
        for &chain in ChainId::ALL {
            let request = service_request(
                &reqwest::Client::new(),
                "https://example.invalid/v1/pools",
                chain,
                &query,
            )
            .build()
            .expect("request builds");
            assert_eq!(chain_pairs(&request), vec![chain.as_str().to_string()]);
            let pairs: Vec<(String, String)> = request
                .url()
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            assert_eq!(
                &pairs[..2],
                &[
                    ("mint".to_string(), "abc".to_string()),
                    ("limit".to_string(), "5".to_string()),
                ]
            );
            assert_eq!(
                request
                    .headers()
                    .get(VERSION_HEADER)
                    .map(|v| v.to_str().unwrap()),
                Some(crate::version::VERSION)
            );
        }
    }

    #[test]
    fn the_solana_candle_request_is_the_caller_query_followed_by_the_chain() {
        let query = [
            (
                "mint",
                "So11111111111111111111111111111111111111112".to_string(),
            ),
            ("pool", "pool-address".to_string()),
            ("timeframe", "1m".to_string()),
            ("limit", "1000".to_string()),
            ("stateful", "true".to_string()),
            ("before", "1700000000".to_string()),
        ];
        let request = service_request(
            &reqwest::Client::new(),
            "https://example.invalid/v1/ohlcv",
            ChainId::Solana,
            &query,
        )
        .build()
        .expect("request builds");
        let pairs: Vec<(String, String)> = request
            .url()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let mut expected: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        expected.push(("chain".to_string(), "solana".to_string()));
        assert_eq!(pairs, expected);
    }

    #[test]
    #[should_panic(expected = "appended by the request builder")]
    fn a_caller_query_may_not_name_the_chain() {
        let _ = service_request(
            &reqwest::Client::new(),
            "https://example.invalid/v1/pools",
            ChainId::Solana,
            &[("chain", "solana".to_string())],
        );
    }

    #[test]
    fn a_bad_request_refuses_that_request_and_keeps_the_service_usable() {
        for code in [
            "chain_unknown",
            "chain_not_served",
            "chain_not_served_by_route",
            "",
        ] {
            assert_eq!(
                access_for_refusal(reqwest::StatusCode::BAD_REQUEST, code, None),
                DataAccess::Ready,
                "{code}"
            );
        }
    }

    #[test]
    fn other_failure_statuses_still_count_as_unreachable() {
        for status in [
            reqwest::StatusCode::NOT_FOUND,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            reqwest::StatusCode::BAD_GATEWAY,
        ] {
            assert_eq!(
                access_for_refusal(status, "", None),
                DataAccess::Unreachable,
                "{status}"
            );
        }
        assert_eq!(
            access_for_refusal(reqwest::StatusCode::FORBIDDEN, "", None),
            DataAccess::ReauthorizationRequired
        );
        assert_eq!(
            access_for_refusal(reqwest::StatusCode::BAD_REQUEST, "signin_required", None),
            DataAccess::SignedOut
        );
    }

    #[test]
    fn refusal_codes_map_to_the_state_the_user_can_act_on() {
        let cases = [
            ("signin_required", DataAccess::SignedOut),
            ("token_expired", DataAccess::SignedOut),
            (
                "reauthorization_required",
                DataAccess::ReauthorizationRequired,
            ),
            ("scope_missing", DataAccess::ReauthorizationRequired),
        ];
        for (code, expected) in cases {
            assert_eq!(
                access_for_refusal(reqwest::StatusCode::UNAUTHORIZED, code, None),
                expected,
                "{code}"
            );
        }
    }

    #[test]
    fn an_unknown_code_falls_back_to_the_status_not_to_ready() {
        assert_eq!(
            access_for_refusal(reqwest::StatusCode::UNAUTHORIZED, "", None),
            DataAccess::SignedOut
        );
        assert_eq!(
            access_for_refusal(reqwest::StatusCode::BAD_GATEWAY, "", None),
            DataAccess::Unreachable
        );
        assert_eq!(
            access_for_refusal(reqwest::StatusCode::UPGRADE_REQUIRED, "", None),
            DataAccess::VersionUnsupported {
                minimum: "a newer release".to_string()
            }
        );
    }

    #[test]
    fn the_minimum_version_is_lifted_out_of_the_service_sentence() {
        let body = json!({
            "code": "version_unsupported",
            "error": "This ScreenerBot version is no longer served. Update to 0.3.1 or newer."
        });
        let (code, minimum) = refusal_code(&body);
        assert_eq!(code, "version_unsupported");
        assert_eq!(minimum.as_deref(), Some("0.3.1"));

        assert_eq!(
            access_for_refusal(reqwest::StatusCode::UPGRADE_REQUIRED, &code, minimum),
            DataAccess::VersionUnsupported {
                minimum: "0.3.1".to_string()
            }
        );
    }

    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// A session whose tokens are handed out in order, recording every
    /// invalidation the client asks for.
    struct StubCredential {
        tokens: Mutex<Vec<&'static str>>,
        invalidated: Mutex<Vec<String>>,
        signed_in: bool,
    }

    impl StubCredential {
        fn new(tokens: &[&'static str]) -> Self {
            let mut tokens = tokens.to_vec();
            tokens.reverse();
            Self {
                tokens: Mutex::new(tokens),
                invalidated: Mutex::new(Vec::new()),
                signed_in: true,
            }
        }
    }

    impl Credential for StubCredential {
        fn access_token(&self) -> impl Future<Output = Option<String>> + Send {
            let next = self.tokens.lock().unwrap().pop().map(str::to_string);
            async move { next }
        }

        fn invalidate(&self, stale: &str) {
            self.invalidated.lock().unwrap().push(stale.to_string());
        }

        fn has_session(&self) -> bool {
            self.signed_in
        }
    }

    const EXPIRED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 24\r\nConnection: close\r\n\r\n{\"code\":\"token_expired\"}";
    const SERVED: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"value\":7}";

    /// Answers each connection with the next response and records the
    /// `authorization` header it was sent.
    async fn serve(responses: &'static [&'static [u8]]) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&seen);
        tokio::spawn(async move {
            for response in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 4096];
                let read = socket.read(&mut request).await.unwrap();
                let text = String::from_utf8_lossy(&request[..read]).to_lowercase();
                if let Some(line) = text.lines().find(|line| line.starts_with("authorization:")) {
                    observed.lock().unwrap().push(line.trim().to_string());
                }
                socket.write_all(response).await.unwrap();
            }
        });
        (format!("http://{address}/v1/value"), seen)
    }

    #[derive(serde::Deserialize)]
    struct Value {
        value: u32,
    }

    #[tokio::test]
    async fn a_stale_access_token_is_refreshed_once_and_the_request_retried() {
        let (url, seen) = serve(&[EXPIRED, SERVED]).await;
        let credential = StubCredential::new(&["stale", "fresh"]);
        let client = reqwest::Client::new();

        let value: Option<Value> = fetch_json(&credential, "/v1/value", ChainId::Solana, |token| {
            client.get(&url).bearer_auth(token).send()
        })
        .await;

        assert_eq!(value.map(|v| v.value), Some(7));
        assert_eq!(*credential.invalidated.lock().unwrap(), vec!["stale"]);
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["authorization: bearer stale", "authorization: bearer fresh"]
        );
    }

    #[tokio::test]
    async fn a_session_whose_token_stays_stale_is_never_reported_signed_out() {
        let (url, _) = serve(&[EXPIRED, EXPIRED]).await;
        let credential = StubCredential::new(&["stale", "still-stale"]);
        let client = reqwest::Client::new();

        let value: Option<Value> = fetch_json(&credential, "/v1/value", ChainId::Solana, |token| {
            client.get(&url).bearer_auth(token).send()
        })
        .await;

        assert!(value.is_none());
        assert_eq!(credential.invalidated.lock().unwrap().len(), 2);
        assert_ne!(access::current(), DataAccess::SignedOut);
    }

    fn never_sent(_: String) -> std::future::Ready<reqwest::Result<reqwest::Response>> {
        unreachable!("no request is sent without a credential")
    }

    #[tokio::test]
    async fn a_session_without_a_usable_token_is_not_reported_signed_out() {
        let credential = StubCredential::new(&[]);
        let value: Option<Value> =
            fetch_json(&credential, "/v1/value", ChainId::Solana, never_sent).await;

        assert!(value.is_none());
        assert_ne!(access::current(), DataAccess::SignedOut);

        let signed_out = StubCredential {
            signed_in: false,
            ..StubCredential::new(&[])
        };
        let value: Option<Value> =
            fetch_json(&signed_out, "/v1/value", ChainId::Solana, never_sent).await;
        assert!(value.is_none());
        assert_eq!(access::current(), DataAccess::SignedOut);
    }

    #[test]
    fn only_token_refusals_count_as_a_stale_credential() {
        assert!(is_stale_access_token("token_expired"));
        assert!(is_stale_access_token("token_invalid"));
        for code in [
            "signin_required",
            "token_wrong_audience",
            "reauthorization_required",
            "",
        ] {
            assert!(!is_stale_access_token(code), "{code}");
        }
    }

    #[test]
    fn a_body_without_a_code_yields_no_code_and_no_minimum() {
        let (code, minimum) = refusal_code(&serde_json::Value::Null);
        assert!(code.is_empty());
        assert!(minimum.is_none());
    }
}
