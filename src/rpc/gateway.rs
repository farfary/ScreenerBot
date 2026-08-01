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
//! IT NEVER BLOCKS A TRADE
//! ============================================================================
//! Every failure returns `None`, and the caller carries on down the normal
//! provider path with the user's own RPC. A free service must not be able to
//! stop somebody trading — not when it is down, not when the quota is spent, not
//! when the session has expired.

use std::time::Duration;

use serde_json::Value;

/// The only methods the gateway is ever asked for. Kept in step with the
/// server's own allowlist in `DataServer/src/api/app.rs`; anything else is
/// refused there anyway, and asking would waste a round trip to be told so.
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

/// Is this method one we would ever relay?
fn is_relayable(method: &str) -> bool {
    RELAYED_METHODS.contains(&method)
}

/// Should this call go through the gateway first?
///
/// Four conditions, all required: the method is relayable, the user switched it
/// on, an account is signed in, and that device holds the scope. The scope check
/// matters because scopes are frozen at grant time on the server — a device
/// authorised before this feature existed does not acquire it by upgrading, and
/// asking anyway would produce a 403 on every swap.
pub fn should_relay(method: &str) -> bool {
    if !is_relayable(method) {
        return false;
    }

    if !crate::config::with_config(|config| config.account.use_gateway_rpc) {
        return false;
    }

    crate::account::is_signed_in() && crate::account::has_scope("rpc:submit")
}

/// Try the gateway. `None` means "fall through to the user's own RPC", and is
/// the answer for every failure — including a successful HTTP response carrying
/// a JSON-RPC error, because a `sendTransaction` the relay could not complete
/// deserves one more attempt through a different route before it is called lost.
pub async fn relay(client: &reqwest::Client, method: &str, params: &Value) -> Option<Value> {
    if crate::connectivity::is_network_offline() {
        return None;
    }

    let token = crate::account::access_token().await?;

    let response = client
        .post(GATEWAY_URL)
        .bearer_auth(token)
        .timeout(TIMEOUT)
        .json(&serde_json::json!({ "method": method, "params": params }))
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        // Debug only. A spent quota or an expired grant is not something to put
        // in a trading log — the transaction still goes out on the user's own
        // RPC a moment later, and nothing was lost.
        log::debug!(
            "RPC gateway declined {method} (HTTP {}); using your own RPC",
            response.status()
        );
        return None;
    }

    let body = response.json::<Value>().await.ok()?;
    body.get("result").cloned()
}
