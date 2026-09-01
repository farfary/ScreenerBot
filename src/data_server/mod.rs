//! The ONE way this app talks to the ScreenerBot data service.
//!
//! ============================================================================
//! WHY THIS MODULE EXISTS
//! ============================================================================
//! Six subsystems read from screenerbot.io/data — candles, the SOL/USD reference
//! chart, the pool registry, Rugcheck reports, token decimals and boosted-token
//! identity. Each of them used to build its own URL, own timeout, own "was that
//! a 200?" check and own silent `None`. That was survivable while the service was
//! open; it stopped being survivable the moment the service started asking WHO
//! is calling, because six copies of an authentication rule is six chances to
//! get it wrong and no place at all to answer "why is my data missing?".
//!
//! So: one client. It resolves the endpoint, attaches the credential, states
//! this build's version, classifies the answer, and publishes a single
//! availability state that the setup screen and Settings both read.
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

use std::time::Duration;

use serde::de::DeserializeOwned;

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
    /// `[ohlcv.sources.screenerbot_server]` — candles and the SOL/USD chart.
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
            _ => DataAccess::Unreachable,
        },
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

/// GET a JSON payload from the data service.
///
/// `None` means "use your own provider", for every reason: switched off,
/// offline, signed out, refused, unreachable, or an answer we could not read.
/// The reason is published to `access` so exactly one place has to explain it.
pub async fn get_json<T: DeserializeOwned>(
    surface: Surface,
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

    // No credential, no request. See the module header.
    let Some(token) = crate::account::access_token().await else {
        access::record(DataAccess::SignedOut);
        return None;
    };

    let url = format!("{endpoint}{path}");
    let response = crate::net::client()
        .get(&url)
        .bearer_auth(token)
        .header(VERSION_HEADER, crate::version::VERSION)
        .query(query)
        .timeout(timeout)
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            log::debug!("Data Server: {path} failed: {error}");
            access::record(DataAccess::Unreachable);
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        // A refusal body is small and always JSON; an unreadable one is treated
        // as the status alone rather than as a transport failure.
        let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        let (code, minimum) = refusal_code(&body);
        access::record(access_for_refusal(status, &code, minimum));
        return None;
    }

    match response.json::<T>().await {
        Ok(value) => {
            access::record(DataAccess::Ready);
            Some(value)
        }
        Err(error) => {
            // The service answered and we could not read it. That is our bug or a
            // shape change, not a permission problem, so it is logged rather than
            // reported to the user as an account state.
            log::debug!("Data Server: {path} returned an unreadable body: {error}");
            access::record(DataAccess::Ready);
            None
        }
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

    #[test]
    fn a_body_without_a_code_yields_no_code_and_no_minimum() {
        let (code, minimum) = refusal_code(&serde_json::Value::Null);
        assert!(code.is_empty());
        assert!(minimum.is_none());
    }
}
