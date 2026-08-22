//! The Solana implementation of `crate::wallets::watch::runtime::WalletWatchRuntime`.
//!
//! Owns every Solana-specific mechanic the shared wallet-watch funnel needs:
//! address/subject resolution, the realtime subscription transport, signature
//! paging, transaction decode, and activity classification. Registered once by
//! the composition root (`crate::run::services`) via `set_runtime_factory`,
//! mirroring `crate::chains::solana::swaps::routers::build_routers`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::chains::solana::rpc::{self, ConnectionState};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::transactions::fetcher::TransactionFetcher;
use crate::chains::solana::transactions::processor::TransactionProcessor;
use crate::chains::solana::transactions::subject;
use crate::transactions::types::{Subject, Transaction};
use crate::wallets::watch::runtime::{ConnectionWatch, NotificationStream, WalletWatchRuntime};
use crate::wallets::watch::{ActivityKind, WatchNotification};

use super::classify;

/// Build the Solana wallet-watch runtime. Registered as the factory passed to
/// `crate::wallets::watch::runtime::set_runtime_factory`.
pub fn build_runtime() -> Arc<dyn WalletWatchRuntime> {
    Arc::new(SolanaWalletWatchRuntime)
}

struct SolanaWalletWatchRuntime;

fn parse_pubkey(address: &str) -> Result<Pubkey, String> {
    address
        .parse::<Pubkey>()
        .map_err(|e| format!("Invalid Solana address '{address}': {e}"))
}

#[async_trait]
impl WalletWatchRuntime for SolanaWalletWatchRuntime {
    fn resolve_subject(&self, address: &str) -> Result<Subject, String> {
        subject::try_from_address(address).map_err(|e| e.to_string())
    }

    fn is_connected(&self) -> bool {
        *rpc::connection_state().borrow() == ConnectionState::Connected
    }

    fn connection_watch(&self) -> Box<dyn ConnectionWatch> {
        Box::new(SolanaConnectionWatch {
            rx: rpc::connection_state(),
        })
    }

    async fn subscribe(&self, address: &str) -> Result<Box<dyn NotificationStream>, String> {
        let sub = rpc::subscribe_logs_mentions(address)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Box::new(SolanaNotificationStream { sub }))
    }

    async fn fetch_signatures_page(
        &self,
        address: &str,
        page_size: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let pubkey = parse_pubkey(address)?;
        TransactionFetcher::new()
            .fetch_signatures_page(pubkey, page_size, before, until)
            .await
    }

    async fn decode_transaction(
        &self,
        address: &str,
        signature: &str,
        is_own: bool,
    ) -> Result<Transaction, String> {
        let pubkey = parse_pubkey(address)?;
        let processor = if is_own {
            TransactionProcessor::new(pubkey)
        } else {
            TransactionProcessor::new_for_watch_target(pubkey)
        };
        processor.decode(signature).await
    }

    fn classify(
        &self,
        address: &str,
        transaction: &Transaction,
    ) -> Option<(ActivityKind, Option<&'static str>)> {
        classify::classify_transaction_activity(address, transaction)
    }
}

struct SolanaConnectionWatch {
    rx: tokio::sync::watch::Receiver<ConnectionState>,
}

#[async_trait]
impl ConnectionWatch for SolanaConnectionWatch {
    async fn changed(&mut self) -> Result<(), ()> {
        self.rx.changed().await.map_err(|_| ())
    }

    fn is_connected(&self) -> bool {
        *self.rx.borrow() == ConnectionState::Connected
    }
}

struct SolanaNotificationStream {
    sub: rpc::LogsSubscription,
}

#[async_trait]
impl NotificationStream for SolanaNotificationStream {
    async fn recv(&mut self) -> Option<WatchNotification> {
        let event = self.sub.recv().await?;
        Some(WatchNotification {
            signature: event.signature,
            failed: event.failed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_subject_accepts_valid_and_rejects_invalid_addresses() {
        let runtime = SolanaWalletWatchRuntime;
        let pubkey = Pubkey::new_unique();

        let subject = runtime
            .resolve_subject(&pubkey.to_string())
            .expect("valid pubkey resolves");
        assert_eq!(subject.address(), pubkey.to_string());
        assert_eq!(subject.chain(), crate::chains::ChainId::Solana);

        assert!(runtime.resolve_subject("not-a-valid-pubkey").is_err());
    }
}
