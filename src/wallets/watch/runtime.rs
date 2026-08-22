//! The injected chain runtime wallet-watch execution depends on.
//!
//! Chain-neutral: this module holds a trait object registered by the application
//! composition root (`src/run/services.rs`, via `set_runtime_factory`) — it never
//! imports `crate::chains::solana`. The concrete Solana runtime lives in
//! `crate::chains::solana::wallets::runtime::build_runtime`, mirroring the
//! `swaps::registry` / `set_router_factory` seam.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;

use crate::transactions::types::{Subject, Transaction};

use super::types::{ActivityKind, WatchNotification};

/// One address's realtime notification subscription. Dropping the runtime's
/// `Box<dyn NotificationStream>` must end the underlying subscription, the same
/// contract `LogsSubscription` already has.
#[async_trait]
pub trait NotificationStream: Send {
    /// Waits for the next notification. `None` means the subscription ended
    /// (transport error, close, or process shutdown) — the funnel treats this
    /// exactly like the transport's own disconnect handling always has.
    async fn recv(&mut self) -> Option<WatchNotification>;
}

/// A live view of runtime connectivity, so the funnel can gap-fill on
/// reconnect without polling for it.
#[async_trait]
pub trait ConnectionWatch: Send {
    /// Waits for the connection state to change. `Err` means the underlying
    /// sender is gone (process shutting down) — nothing left to react to.
    async fn changed(&mut self) -> Result<(), ()>;
    /// The connection state as of the last observed change.
    fn is_connected(&self) -> bool;
}

/// The chain-specific execution the shared wallet-watch funnel is injected
/// with: address resolution, realtime subscription, signature paging, decode
/// and activity classification. `crate::wallets::watch` owns targets,
/// persistence, dedupe, scheduling and lifecycle; this trait owns everything
/// that has to know the wire format.
#[async_trait]
pub trait WalletWatchRuntime: Send + Sync {
    /// Validate a user-supplied address and resolve it to a chain-neutral
    /// subject. Rejects the target before any observation starts.
    fn resolve_subject(&self, address: &str) -> Result<Subject, String>;

    /// True while the shared realtime transport is connected.
    fn is_connected(&self) -> bool;

    /// A fresh connection-state watch, independent of `is_connected()`'s
    /// point-in-time read.
    fn connection_watch(&self) -> Box<dyn ConnectionWatch>;

    /// Subscribe to realtime notifications for one address. Ends when the
    /// returned handle is dropped.
    async fn subscribe(&self, address: &str) -> Result<Box<dyn NotificationStream>, String>;

    /// Fetch one page of signatures for `address`, newest first, restricted to
    /// the `(until, before]` range (both ends optional/open).
    async fn fetch_signatures_page(
        &self,
        address: &str,
        page_size: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<String>, String>;

    /// Decode one signature into a chain-neutral transaction record.
    /// `is_own` selects the own-wallet persistence policy (raw JSON retained)
    /// versus the watch-target policy (raw JSON dropped at decode time).
    async fn decode_transaction(
        &self,
        address: &str,
        signature: &str,
        is_own: bool,
    ) -> Result<Transaction, String>;

    /// Classify a decoded, successful transaction from `address`'s
    /// perspective. `None` means dedupe should still commit but nothing is
    /// recorded or broadcast; `Some((kind, skip_reason))` mirrors the pure
    /// classifier's contract.
    fn classify(
        &self,
        address: &str,
        transaction: &Transaction,
    ) -> Option<(ActivityKind, Option<&'static str>)>;
}

/// Global runtime instance (lazily built from the registered factory).
static RUNTIME: OnceLock<Arc<dyn WalletWatchRuntime>> = OnceLock::new();

/// Factory that builds the concrete runtime, registered once by the
/// application composition root before `WalletWatchService` can initialize.
static RUNTIME_FACTORY: OnceLock<fn() -> Arc<dyn WalletWatchRuntime>> = OnceLock::new();

/// Register the chain-owned runtime factory. Must be called once during boot
/// (see `crate::run::services::register_all_services`) before any watch
/// activity can occur.
pub fn set_runtime_factory(factory: fn() -> Arc<dyn WalletWatchRuntime>) {
    let _ = RUNTIME_FACTORY.set(factory);
}

/// Fallible global runtime access. Returns `None` when the composition root
/// has not registered a factory — integration tests and library callers that
/// reach watch APIs before boot. Never panics.
pub fn try_get_runtime() -> Option<Arc<dyn WalletWatchRuntime>> {
    if let Some(runtime) = RUNTIME.get() {
        return Some(Arc::clone(runtime));
    }
    let factory = RUNTIME_FACTORY.get()?;
    let _ = RUNTIME.set(factory());
    RUNTIME.get().map(Arc::clone)
}

/// Get the global runtime, initializing it from the registered factory on
/// first access. Returns a structured service-init error when no factory has
/// been registered — never panics.
pub fn get_runtime() -> crate::Result<Arc<dyn WalletWatchRuntime>> {
    try_get_runtime().ok_or_else(|| {
        crate::Error::Service(crate::errors::ServiceError::Initialize {
            service: "wallets.watch.runtime".to_owned(),
            message: "wallet-watch chain runtime has not been registered".to_owned(),
        })
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    //! A deterministic fake runtime, shared by wallet-watch unit tests that need
    //! to exercise the injected seam without a live chain.

    use std::collections::VecDeque;
    use std::sync::Mutex;

    use tokio::sync::mpsc;

    use crate::chains::{AccountId, ChainId};

    use super::*;

    /// Records every call the shared funnel makes, so a test can assert the
    /// exact chain/account identity it was handed.
    #[derive(Default)]
    pub struct FakeCalls {
        pub resolved: Vec<String>,
        pub subscribed: Vec<String>,
        pub decoded: Vec<(String, String, bool)>,
        pub classified: Vec<String>,
    }

    pub struct FakeNotificationStream {
        pub rx: mpsc::UnboundedReceiver<WatchNotification>,
    }

    #[async_trait]
    impl NotificationStream for FakeNotificationStream {
        async fn recv(&mut self) -> Option<WatchNotification> {
            self.rx.recv().await
        }
    }

    pub struct FakeConnectionWatch {
        pub rx: tokio::sync::watch::Receiver<bool>,
    }

    #[async_trait]
    impl ConnectionWatch for FakeConnectionWatch {
        async fn changed(&mut self) -> Result<(), ()> {
            self.rx.changed().await.map_err(|_| ())
        }

        fn is_connected(&self) -> bool {
            *self.rx.borrow()
        }
    }

    /// A deterministic in-memory runtime: valid addresses are whatever was
    /// registered via `with_target`, decode results and signature pages are
    /// pre-seeded, and every call is recorded in `calls`.
    pub struct FakeRuntime {
        pub connected: tokio::sync::watch::Sender<bool>,
        connected_rx: tokio::sync::watch::Receiver<bool>,
        valid_addresses: Vec<String>,
        pages: Mutex<std::collections::HashMap<String, VecDeque<Vec<String>>>>,
        decode_results: Mutex<std::collections::HashMap<String, Result<Transaction, String>>>,
        classify_results:
            Mutex<std::collections::HashMap<String, (ActivityKind, Option<&'static str>)>>,
        pub calls: Mutex<FakeCalls>,
    }

    impl FakeRuntime {
        pub fn new(valid_addresses: Vec<String>) -> Arc<Self> {
            let (tx, rx) = tokio::sync::watch::channel(true);
            Arc::new(Self {
                connected: tx,
                connected_rx: rx,
                valid_addresses,
                pages: Mutex::new(std::collections::HashMap::new()),
                decode_results: Mutex::new(std::collections::HashMap::new()),
                classify_results: Mutex::new(std::collections::HashMap::new()),
                calls: Mutex::new(FakeCalls::default()),
            })
        }

        pub fn queue_page(&self, address: &str, page: Vec<String>) {
            self.pages
                .lock()
                .unwrap()
                .entry(address.to_owned())
                .or_default()
                .push_back(page);
        }

        pub fn set_decode_result(&self, signature: &str, result: Result<Transaction, String>) {
            self.decode_results
                .lock()
                .unwrap()
                .insert(signature.to_owned(), result);
        }

        pub fn set_classify_result(
            &self,
            signature: &str,
            kind: ActivityKind,
            skip_reason: Option<&'static str>,
        ) {
            self.classify_results
                .lock()
                .unwrap()
                .insert(signature.to_owned(), (kind, skip_reason));
        }
    }

    #[async_trait]
    impl WalletWatchRuntime for FakeRuntime {
        fn resolve_subject(&self, address: &str) -> Result<Subject, String> {
            self.calls.lock().unwrap().resolved.push(address.to_owned());
            if !self.valid_addresses.iter().any(|a| a == address) {
                return Err(format!(
                    "not a valid address for this fake chain: {address}"
                ));
            }
            let account = AccountId::new(ChainId::Solana, address)
                .map_err(|e| format!("invalid account: {e}"))?;
            Ok(Subject::from_account(account))
        }

        fn is_connected(&self) -> bool {
            *self.connected_rx.borrow()
        }

        fn connection_watch(&self) -> Box<dyn ConnectionWatch> {
            Box::new(FakeConnectionWatch {
                rx: self.connected_rx.clone(),
            })
        }

        async fn subscribe(&self, address: &str) -> Result<Box<dyn NotificationStream>, String> {
            self.calls
                .lock()
                .unwrap()
                .subscribed
                .push(address.to_owned());
            let (_tx, rx) = mpsc::unbounded_channel();
            Ok(Box::new(FakeNotificationStream { rx }))
        }

        async fn fetch_signatures_page(
            &self,
            address: &str,
            _page_size: usize,
            _before: Option<&str>,
            _until: Option<&str>,
        ) -> Result<Vec<String>, String> {
            let mut pages = self.pages.lock().unwrap();
            Ok(pages
                .get_mut(address)
                .and_then(VecDeque::pop_front)
                .unwrap_or_default())
        }

        async fn decode_transaction(
            &self,
            address: &str,
            signature: &str,
            is_own: bool,
        ) -> Result<Transaction, String> {
            self.calls.lock().unwrap().decoded.push((
                address.to_owned(),
                signature.to_owned(),
                is_own,
            ));
            self.decode_results
                .lock()
                .unwrap()
                .get(signature)
                .cloned()
                .unwrap_or_else(|| Err(format!("no decode result seeded for {signature}")))
        }

        fn classify(
            &self,
            address: &str,
            transaction: &Transaction,
        ) -> Option<(ActivityKind, Option<&'static str>)> {
            self.calls
                .lock()
                .unwrap()
                .classified
                .push(address.to_owned());
            self.classify_results
                .lock()
                .unwrap()
                .get(&transaction.signature)
                .cloned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_lookup_is_fallible_and_never_panics_when_unregistered() {
        // This process-wide OnceLock may already be set by another test in the
        // same binary; the contract under test is "never panics", which holds
        // either way -- assert only the non-panicking, well-typed result shape.
        let result = try_get_runtime();
        match result {
            Some(_) => {}
            None => {
                assert!(
                    get_runtime().is_err(),
                    "unset runtime must be a domain error, not a panic"
                );
            }
        }
    }
}
