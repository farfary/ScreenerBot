//! Referral activation — the ONLY code in ScreenerBot that reports anything.
//!
//! ============================================================================
//! READ THIS BEFORE CHANGING ANYTHING HERE.
//! ============================================================================
//! ScreenerBot has no telemetry. This service is the single exception, it is
//! OPT-IN, and it stays silent until the user types a referral code into
//! Settings. With `referral.code` empty, `is_enabled()` returns false, the
//! service never starts, and no request is ever constructed.
//!
//! When it does run it sends ONE JSON body:
//!
//!     { referral_code, proofs: [{ wallet, issued_at, signature }], platform, version }
//!
//! and nothing else. Not balances, not positions, not trades, not P&L, not the
//! config, not an IP we choose to include, not a machine id.
//!
//! Each wallet signs a short, domain-separated statement binding its PUBLIC
//! address to the referral code and current time. Private keys stay local and
//! no transaction is created. The proof is required because a public address
//! alone says nothing about who chose the referral code.
//!
//! This repository is public, which is what makes the paragraph above worth
//! writing: it is checkable rather than promised.

#[cfg(test)]
use crate::chains::solana::solana_sdk::signer::Signer;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::config::get_config_clone;
use crate::services::{Service, ServiceHealth};

/// Give the wallet and connectivity services time to settle before the first
/// announce. A referral that arrives 30 seconds late costs nothing; one that
/// races wallet loading reports an empty wallet list and attributes nothing.
const STARTUP_DELAY: Duration = Duration::from_secs(30);

/// One attempt is enough. This is not a heartbeat, and the re-announce timer
/// is the retry: failing quietly and trying again tomorrow is the correct
/// behaviour for something the user does not need to think about.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Serialize)]
struct ActivationRequest {
    referral_code: String,
    proofs: Vec<ActivationProof>,
    platform: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ActivationProof {
    wallet: String,
    issued_at: u64,
    signature: String,
}

fn activation_message(wallet: &str, code: &str, issued_at: u64) -> String {
    format!(
        "ScreenerBot Referral Activation v1\n\
         Domain: screenerbot.io\n\
         Wallet: {wallet}\n\
         Referral Code: {code}\n\
         Issued At: {issued_at}"
    )
}

pub struct ReferralService;

#[async_trait]
impl Service for ReferralService {
    fn name(&self) -> &'static str {
        "referral"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// The consent gate. No code, no service, no request — the task is never
    /// even spawned.
    fn is_enabled(&self) -> bool {
        if !crate::global::is_initialization_complete() {
            return false;
        }
        crate::config::with_config(|config| config.referral.is_enabled())
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = tokio::spawn(monitor.instrument(async move {
            run_referral_loop(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}

async fn run_referral_loop(shutdown: Arc<Notify>) {
    tokio::select! {
        _ = tokio::time::sleep(STARTUP_DELAY) => {}
        _ = shutdown.notified() => return,
    }

    loop {
        announce_once().await;

        let hours = get_config_clone().referral.reannounce_hours;
        if hours == 0 {
            // Announcing once was the whole job. Wait for shutdown rather than
            // spinning, so the task stays parked instead of busy-looping.
            shutdown.notified().await;
            return;
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(hours * 3600)) => {}
            _ = shutdown.notified() => return,
        }
    }
}

async fn announce_once() {
    // Offline is a normal state for this bot (see the connectivity module).
    // Trying anyway would log a DNS error the user cannot act on.
    if crate::connectivity::is_network_offline() {
        return;
    }

    let config = get_config_clone();
    let referral = &config.referral;

    // Re-checked here and not only in is_enabled(): the code can be cleared
    // from the dashboard while the loop is parked between announces, and that
    // must take effect at the next tick rather than at the next restart.
    if !referral.is_enabled() {
        return;
    }

    let code = referral.normalized_code();
    if code.is_empty() {
        return;
    }

    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let proofs = match crate::chains::solana::accounts::sign_message_for_active_wallets(|address| {
        activation_message(address, &code, issued_at)
    })
    .await
    {
        Ok(signed) => signed
            .into_iter()
            .take(25)
            .map(|(address, signature)| ActivationProof {
                wallet: address,
                issued_at,
                signature,
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            log::debug!("Referral: could not prove wallet ownership: {error}");
            Vec::new()
        }
    };

    // No wallet means nothing to attribute. Sending the code alone would tell
    // the server a referral exists that it can never match to any revenue, so
    // it would inflate that referrer's signup count and never their earnings.
    if proofs.is_empty() {
        log::debug!("Referral: no wallets yet, nothing to announce");
        return;
    }

    let body = ActivationRequest {
        referral_code: code,
        proofs,
        platform: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
    };

    let client = match reqwest::Client::builder().timeout(REQUEST_TIMEOUT).build() {
        Ok(client) => client,
        Err(error) => {
            log::debug!("Referral: could not build HTTP client: {error}");
            return;
        }
    };

    match client.post(&referral.endpoint).json(&body).send().await {
        Ok(response) if response.status().is_success() => {
            log::info!("Referral code registered with screenerbot.io");
        }
        Ok(response) => {
            // A rejected code is worth telling the user about ONCE, at info, so
            // a typo does not silently cost their referrer every future fee.
            log::info!(
                "Referral: screenerbot.io did not accept the code (HTTP {}). Check it in Settings.",
                response.status()
            );
        }
        Err(error) => {
            // Never a warning: the network being unavailable is not a fault the
            // user needs to see in their trading log.
            log::debug!("Referral: announce failed, will retry later: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::solana_sdk::signature::Keypair;

    #[test]
    fn activation_proof_is_bound_to_wallet_code_and_time() {
        let keypair = Keypair::new();
        let wallet = keypair.pubkey().to_string();
        let message = activation_message(&wallet, "CREATOR42", 1_800_000_000);
        let signature = keypair.sign_message(message.as_bytes());

        assert!(signature.verify(keypair.pubkey().as_ref(), message.as_bytes()));
        assert!(!signature.verify(
            keypair.pubkey().as_ref(),
            activation_message(&wallet, "ATTACKER", 1_800_000_000).as_bytes()
        ));
    }
}
