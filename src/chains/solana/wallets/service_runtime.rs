// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana wallet-watch runtime when chain access goes through the ScreenerBot service.
//!
//! Only the trading wallet is observed. Its activity arrives as changes of the
//! wallet version the service backend polls: each change reads the signatures
//! that landed since the last one and hands each to the shared funnel as a
//! notification. Paging, decoding and classification are the own-RPC
//! runtime's, which already read through the chain access facade.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::watch;

use super::runtime::SolanaWalletWatchRuntime;
use crate::chains::access::Capability;
use crate::chains::solana::access::ServiceBackend;
use crate::chains::ChainId;
use crate::transactions::types::{Subject, Transaction};
use crate::wallets::watch::runtime::{ConnectionWatch, NotificationStream, WalletWatchRuntime};
use crate::wallets::watch::{
    ActivityKind, SignaturePageItem, SuccessfulTransactionsPage, WatchNotification,
};
use crate::wallets::Error;

/// Signatures read per wallet version change.
const SIGNATURES_PER_CHANGE: usize = 100;

/// Build the service wallet-watch runtime over the service backend.
pub fn build_runtime(backend: &'static ServiceBackend) -> Arc<dyn WalletWatchRuntime> {
    backend.start_poller();
    Arc::new(ServiceWalletWatchRuntime {
        backend,
        decoder: SolanaWalletWatchRuntime,
    })
}

struct ServiceWalletWatchRuntime {
    backend: &'static ServiceBackend,
    decoder: SolanaWalletWatchRuntime,
}

/// `address` when it is the trading wallet; any other subject needs a chain
/// access that can watch other wallets.
fn trading_subject(address: &str, trading_wallet: Option<&str>) -> Result<(), Error> {
    if trading_wallet == Some(address) {
        return Ok(());
    }
    Err(Error::ChainRuntime {
        operation: "resolve_subject",
        detail: crate::chains::Error::CapabilityUnavailable {
            chain: ChainId::Solana,
            capability: Capability::WatchOtherWallets,
        }
        .to_string(),
    })
}

#[async_trait]
impl WalletWatchRuntime for ServiceWalletWatchRuntime {
    fn resolve_subject(&self, address: &str) -> Result<Subject, Error> {
        // The vault read is synchronous on any runtime flavour; the
        // key-cache bridge refuses on a current-thread runtime.
        let trading_wallet = crate::wallets::replacement::trading_wallet_address(ChainId::Solana)
            .ok()
            .flatten();
        trading_subject(address, trading_wallet.as_deref())?;
        self.decoder.resolve_subject(address)
    }

    fn is_connected(&self) -> bool {
        *self.backend.connection().borrow()
    }

    /// Every version change gap-fills, so a connected stream needs no poll.
    fn polls_while_connected(&self) -> bool {
        false
    }

    fn connection_watch(&self) -> Box<dyn ConnectionWatch> {
        Box::new(ServiceConnectionWatch {
            rx: self.backend.connection(),
        })
    }

    async fn subscribe(&self, address: &str) -> Result<Box<dyn NotificationStream>, Error> {
        let trading_wallet = crate::chains::solana::accounts::configured_address_async()
            .await
            .ok();
        trading_subject(address, trading_wallet.as_deref())?;
        // Activity before the subscription is the funnel's gap-fill; only
        // what lands after the newest signature now is notified.
        let newest = self
            .decoder
            .fetch_signatures_page(address, 1, None, None)
            .await?
            .into_iter()
            .next()
            .map(|item| item.signature);
        let mut versions = self.backend.subscribe_version();
        versions.mark_unchanged();
        Ok(Box::new(VersionNotificationStream {
            address: address.to_owned(),
            versions,
            last_seen: newest,
            pending: Vec::new(),
        }))
    }

    async fn fetch_signatures_page(
        &self,
        address: &str,
        page_size: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<SignaturePageItem>, Error> {
        self.decoder
            .fetch_signatures_page(address, page_size, before, until)
            .await
    }

    async fn supports_high_activity_mode(&self) -> bool {
        false
    }

    async fn fetch_successful_transactions_after(
        &self,
        _address: &str,
        _limit: usize,
        _after: Option<&str>,
    ) -> Result<SuccessfulTransactionsPage, Error> {
        Err(Error::ChainRuntime {
            operation: "fetch_successful_transactions_after",
            detail: crate::chains::Error::CapabilityUnavailable {
                chain: ChainId::Solana,
                capability: Capability::WatchOtherWallets,
            }
            .to_string(),
        })
    }

    async fn decode_transaction(
        &self,
        address: &str,
        signature: &str,
        is_own: bool,
    ) -> Result<Transaction, Error> {
        self.decoder
            .decode_transaction(address, signature, is_own)
            .await
    }

    fn classify(
        &self,
        address: &str,
        transaction: &Transaction,
    ) -> Option<(ActivityKind, Option<&'static str>)> {
        self.decoder.classify(address, transaction)
    }
}

struct ServiceConnectionWatch {
    rx: watch::Receiver<bool>,
}

#[async_trait]
impl ConnectionWatch for ServiceConnectionWatch {
    async fn changed(&mut self) -> Result<(), ()> {
        self.rx.changed().await.map_err(|_| ())
    }

    fn is_connected(&self) -> bool {
        *self.rx.borrow()
    }
}

/// Notifications of the trading wallet, one per signature that landed since
/// the last version change.
struct VersionNotificationStream {
    address: String,
    versions: watch::Receiver<u64>,
    last_seen: Option<String>,
    /// Signatures read but not yet handed out, newest first.
    pending: Vec<SignaturePageItem>,
}

#[async_trait]
impl NotificationStream for VersionNotificationStream {
    async fn recv(&mut self) -> Option<WatchNotification> {
        loop {
            if let Some(item) = self.pending.pop() {
                return Some(WatchNotification {
                    signature: item.signature,
                    failed: item.failed,
                });
            }
            self.versions.changed().await.ok()?;
            // A failed read ends the stream; the funnel resubscribes and
            // gap-fills exactly as after a transport disconnect.
            let page = SolanaWalletWatchRuntime
                .fetch_signatures_page(
                    &self.address,
                    SIGNATURES_PER_CHANGE,
                    None,
                    self.last_seen.as_deref(),
                )
                .await
                .ok()?;
            if let Some(newest) = page.first() {
                self.last_seen = Some(newest.signature.clone());
            }
            self.pending = page;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_trading_wallet_is_a_subject() {
        let wallet = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        assert!(trading_subject(wallet, Some(wallet)).is_ok());
        assert!(matches!(
            trading_subject("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU", Some(wallet)),
            Err(Error::ChainRuntime { .. })
        ));
        assert!(trading_subject(wallet, None).is_err());
    }

    /// A version change reads what landed after the last signature seen and
    /// hands it out oldest first.
    #[tokio::test]
    async fn a_version_change_emits_the_new_signatures_oldest_first() {
        let (sender, versions) = watch::channel(0_u64);
        let mut stream = VersionNotificationStream {
            address: String::new(),
            versions,
            last_seen: Some("seen".to_owned()),
            pending: vec![
                SignaturePageItem {
                    signature: "newer".to_owned(),
                    failed: false,
                },
                SignaturePageItem {
                    signature: "older".to_owned(),
                    failed: true,
                },
            ],
        };
        let first = stream.recv().await.unwrap();
        assert_eq!(first.signature, "older");
        assert!(first.failed);
        assert_eq!(stream.recv().await.unwrap().signature, "newer");
        // With nothing pending and the version sender gone, the stream ends.
        drop(sender);
        assert!(stream.recv().await.is_none());
    }
}
