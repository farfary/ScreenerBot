//! Keeps the ScreenerBot account session alive.
//!
//! Two jobs, both background: renew the access token before it expires, and —
//! only when the user has switched it on — offer the wallet sign-in once at
//! startup.
//!
//! It is deliberately incapable of blocking anything. The service starts whether
//! or not an account exists, does nothing at all when signed out, and never
//! reports unhealthy for a network failure: being offline is a normal state for
//! this bot, and an account subsystem must never be the reason a trading engine
//! looks broken.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::services::{Service, ServiceHealth};

/// Let the wallet and connectivity services settle first. A wallet sign-in that
/// races wallet loading asks the server about an address that is not there yet.
const STARTUP_DELAY: Duration = Duration::from_secs(20);

/// How often to consider refreshing. The token lives 15 minutes and
/// `access_token()` renews inside its margin on demand, so this loop exists to
/// keep a session warm while the app sits idle, not to be the primary path.
const TICK: Duration = Duration::from_secs(5 * 60);

pub struct AccountService;

#[async_trait]
impl Service for AccountService {
    fn name(&self) -> &'static str {
        "account"
    }

    fn priority(&self) -> i32 {
        210
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Runs in preview mode too. Signing in is one of the few useful things to
    /// do before a wallet and RPC exist, and refusing to run until setup is
    /// complete would make the account panel on the setup screen inert.
    fn is_enabled(&self) -> bool {
        crate::global::is_preview_or_full()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        // Reads the encrypted store; no network. Safe to do on the boot path.
        crate::account::initialize();
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = tokio::spawn(monitor.instrument(async move {
            run_account_loop(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Signed out is not unhealthy. Neither is offline.
        ServiceHealth::Healthy
    }
}

async fn run_account_loop(shutdown: Arc<Notify>) {
    tokio::select! {
        _ = tokio::time::sleep(STARTUP_DELAY) => {}
        _ = shutdown.notified() => return,
    }

    maybe_auto_wallet_signin().await;

    loop {
        if crate::account::is_signed_in() && !crate::connectivity::is_network_offline() {
            // `access_token()` refreshes when inside the margin and is a no-op
            // otherwise, so this is the whole of the keep-warm job.
            let _ = crate::account::access_token().await;
        }

        tokio::select! {
            _ = tokio::time::sleep(TICK) => {}
            _ = shutdown.notified() => return,
        }
    }
}

/// Sign in with the trading wallet, but only because the user asked for it.
///
/// The config default is false. This function exists so that someone who has
/// deliberately turned it on does not have to click a button on every launch —
/// it is a convenience for a decision already made, never the decision itself.
async fn maybe_auto_wallet_signin() {
    if crate::account::is_signed_in() {
        return;
    }

    if crate::connectivity::is_network_offline() {
        return;
    }

    let enabled = crate::config::with_config(|config| config.account.auto_wallet_signin);
    if !enabled {
        return;
    }

    // `create: false`. Automatic sign-in may adopt an EXISTING account; it may
    // never bring one into being.
    match crate::account::sign_in_with_wallet(false).await {
        Ok(()) => {}
        Err(error) => {
            // Debug, not warning: the ordinary reason this fails is that the
            // wallet has no account yet, which is not a fault.
            log::debug!("Account: automatic wallet sign-in did not proceed: {error}");
        }
    }
}
