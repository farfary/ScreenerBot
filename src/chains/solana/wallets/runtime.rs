// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana implementation of `crate::wallets::watch::runtime::WalletWatchRuntime`.
//!
//! Owns every Solana-specific mechanic the shared wallet-watch funnel needs:
//! address/subject resolution, the realtime subscription transport, signature
//! paging, transaction decode, and activity classification. Registered once by
//! the composition root (`crate::run::services`) via `set_runtime_factory`,
//! mirroring `crate::chains::solana::swaps::routers::build_routers`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::chains::access::Capability;
use crate::chains::solana::access::own_rpc_client;
use crate::chains::solana::rpc::{self, ConnectionState, RpcClientMethods};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::transactions::fetcher::TransactionFetcher;
use crate::chains::solana::transactions::processor::TransactionProcessor;
use crate::chains::solana::transactions::subject;
use crate::transactions::types::{Subject, Transaction};
use crate::wallets::watch::runtime::{ConnectionWatch, NotificationStream, WalletWatchRuntime};
use crate::wallets::watch::{
    ActivityKind, SignaturePageItem, StreamEvent, StreamMode, SuccessfulTransactionPageItem,
    SuccessfulTransactionsPage, WatchNotification,
};
use crate::wallets::Error;

use super::account_changes::{self, AccountChangeStream};
use super::classify;

/// Build the Solana wallet-watch runtime. Registered as the factory passed to
/// `crate::wallets::watch::runtime::set_runtime_factory`.
pub fn build_runtime() -> Arc<dyn WalletWatchRuntime> {
    Arc::new(SolanaWalletWatchRuntime)
}

pub(super) struct SolanaWalletWatchRuntime;

/// The current slot on the endpoints that watch other wallets, `None` when none
/// answers.
pub(super) async fn watch_slot() -> Option<u64> {
    let client = own_rpc_client(Capability::WatchOtherWallets).ok()?;
    client.get_slot().await.ok()
}

fn parse_pubkey(address: &str) -> Result<Pubkey, Error> {
    address.parse::<Pubkey>().map_err(|e| Error::ChainRuntime {
        operation: "parse_pubkey",
        detail: format!("Invalid Solana address '{address}': {e}"),
    })
}

#[async_trait]
impl WalletWatchRuntime for SolanaWalletWatchRuntime {
    fn resolve_subject(&self, address: &str) -> Result<Subject, Error> {
        subject::try_from_address(address).map_err(|e| Error::ChainRuntime {
            operation: "resolve_subject",
            detail: e.to_string(),
        })
    }

    fn is_connected(&self) -> bool {
        *rpc::connection_state().borrow() == ConnectionState::Connected
    }

    fn connection_watch(&self) -> Box<dyn ConnectionWatch> {
        Box::new(SolanaConnectionWatch {
            rx: rpc::connection_state(),
        })
    }

    async fn subscribe(
        &self,
        address: &str,
        mode: StreamMode,
    ) -> Result<Box<dyn NotificationStream>, Error> {
        let topic = match mode {
            StreamMode::References => rpc::Topic::Mentions(address.to_owned()),
            StreamMode::OwnChanges => rpc::Topic::Account(address.to_owned()),
        };
        let pubkey = parse_pubkey(address)?;
        let sub = rpc::subscribe(topic)
            .await
            .map_err(|e| Error::ChainRuntime {
                operation: "subscribe",
                detail: e.to_string(),
            })?;
        Ok(match mode {
            StreamMode::References => Box::new(SolanaNotificationStream { sub }),
            StreamMode::OwnChanges => Box::new(AccountChangeStream::new(address, pubkey, sub)),
        })
    }

    async fn fetch_signatures_page(
        &self,
        address: &str,
        page_size: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<SignaturePageItem>, Error> {
        let pubkey = parse_pubkey(address)?;
        TransactionFetcher::new()
            .fetch_signature_info_page(pubkey, page_size, before, until)
            .await
            .map(|page| {
                let proof = account_changes::Proof::for_address(address);
                page.into_iter()
                    .map(|info| SignaturePageItem {
                        signature: info.signature.to_string(),
                        failed: info.err.is_some(),
                        block_time: info.block_time,
                        verdict: proof.verdict(info.slot, info.block_time),
                    })
                    .collect()
            })
            .map_err(|e| Error::ChainRuntime {
                operation: "fetch_signatures_page",
                detail: e.to_string(),
            })
    }

    async fn supports_high_activity_mode(&self) -> bool {
        match own_rpc_client(Capability::WatchOtherWallets) {
            Ok(client) => {
                client
                    .manager()
                    .has_enabled_provider_kind(crate::rpc::ProviderKind::Helius)
                    .await
            }
            Err(_) => false,
        }
    }

    fn high_activity_provider(&self) -> Option<&'static str> {
        Some("helius")
    }

    async fn fetch_successful_transactions_after(
        &self,
        address: &str,
        limit: usize,
        after: Option<&str>,
    ) -> Result<SuccessfulTransactionsPage, Error> {
        let pubkey = parse_pubkey(address)?;
        let after = after
            .map(|signature| {
                signature.parse::<crate::chains::solana::solana_sdk::signature::Signature>()
            })
            .transpose()
            .map_err(|error| Error::ChainRuntime {
                operation: "parse_helius_cursor",
                detail: error.to_string(),
            })?;
        let page = own_rpc_client(Capability::WatchOtherWallets)
            .map_err(|error| Error::ChainRuntime {
                operation: "fetch_successful_transactions_after",
                detail: error.to_string(),
            })?
            .get_helius_successful_transactions_after(&pubkey, limit.min(1000), after.as_ref())
            .await
            .map_err(|error| Error::ChainRuntime {
                operation: "fetch_successful_transactions_after",
                detail: error.to_string(),
            })?;
        let processor = TransactionProcessor::new_for_watch_target(pubkey);
        let mut items = Vec::with_capacity(page.transactions.len());
        for details in page.transactions {
            let signature = details
                .transaction
                .signatures
                .first()
                .cloned()
                .ok_or_else(|| Error::ChainRuntime {
                    operation: "fetch_successful_transactions_after",
                    detail: "successful provider item had no signature".to_owned(),
                })?;
            let transaction = if classify::subject_has_meaningful_effect(address, &details) {
                processor
                    .decode_details(&signature, &details)
                    .await
                    .map(Some)
                    .map_err(|error| match error.classify() {
                        Some(
                            failure @ (crate::chains::ExecutionFailure::IndexingDelay { .. }
                            | crate::chains::ExecutionFailure::NotFound { .. }),
                        ) => Error::ChainExecution(failure),
                        _ => Error::ChainRuntime {
                            operation: "decode_helius_transaction",
                            detail: error.to_string(),
                        },
                    })
            } else {
                Ok(None)
            };
            items.push(SuccessfulTransactionPageItem {
                signature,
                transaction,
            });
        }
        Ok(SuccessfulTransactionsPage {
            items,
            has_more: page.pagination_token.is_some(),
        })
    }

    async fn decode_transaction(
        &self,
        address: &str,
        signature: &str,
        is_own: bool,
    ) -> Result<Transaction, Error> {
        let pubkey = parse_pubkey(address)?;
        let processor = if is_own {
            TransactionProcessor::new(pubkey)
        } else {
            TransactionProcessor::new_for_watch_target(pubkey)
        };
        processor
            .decode(signature)
            .await
            .map_err(|e| match e.classify() {
                // A deferral cause travels as the typed neutral classification so the
                // wallet-watch funnel branches on the variant, never on message text
                // or a coded operation string.
                Some(
                    failure @ (crate::chains::ExecutionFailure::IndexingDelay { .. }
                    | crate::chains::ExecutionFailure::NotFound { .. }),
                ) => Error::ChainExecution(failure),
                _ => Error::ChainRuntime {
                    operation: "decode_transaction",
                    detail: e.to_string(),
                },
            })
    }

    fn moves_subject(&self, address: &str, transaction: &Transaction) -> bool {
        transaction
            .raw_transaction_data
            .as_ref()
            .and_then(|raw| {
                serde_json::from_value::<crate::chains::solana::rpc::TransactionDetails>(
                    raw.clone(),
                )
                .ok()
            })
            .is_none_or(|details| classify::subject_has_meaningful_effect(address, &details))
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
    sub: rpc::Subscription,
}

#[async_trait]
impl NotificationStream for SolanaNotificationStream {
    async fn recv(&mut self) -> Option<StreamEvent> {
        loop {
            match self.sub.recv().await? {
                rpc::TopicEvent::Confirmed => return Some(StreamEvent::Live),
                rpc::TopicEvent::Mention(mention) => {
                    return Some(StreamEvent::Notification(WatchNotification {
                        signature: mention.signature,
                        failed: mention.failed,
                    }))
                }
                rpc::TopicEvent::AccountChanged { .. } => {}
            }
        }
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

    /// A transaction of `fee_payer` whose account keys include `subject`, with each
    /// key's lamports moving by `deltas`.
    fn referencing(subject: &str, fee_payer: &str, deltas: [i64; 2]) -> Transaction {
        let mut transaction = Transaction::new("signature".to_owned());
        transaction.success = true;
        transaction.raw_transaction_data = Some(serde_json::json!({
            "slot": 42,
            "transaction": {
                "message": { "accountKeys": [fee_payer, subject], "instructions": [] },
                "signatures": ["signature"]
            },
            "meta": {
                "err": null,
                "fee": 5_000,
                "preBalances": [10_000_000_000_u64, 10_000_000_000_u64],
                "postBalances": [
                    (10_000_000_000_i64 + deltas[0]) as u64,
                    (10_000_000_000_i64 + deltas[1]) as u64
                ],
                "preTokenBalances": [],
                "postTokenBalances": [],
                "innerInstructions": [],
                "logMessages": []
            },
            "blockTime": 1
        }));
        transaction
    }

    #[test]
    fn a_reference_that_moves_none_of_the_subjects_balances_is_not_its_activity() {
        let runtime = SolanaWalletWatchRuntime;
        let subject = Pubkey::new_unique().to_string();
        let other = Pubkey::new_unique().to_string();

        assert!(!runtime.moves_subject(&subject, &referencing(&subject, &other, [-5_000, 0])));
        assert!(runtime.moves_subject(
            &subject,
            &referencing(&subject, &other, [-5_000, 250_000_000])
        ));
        let mut undecodable = referencing(&subject, &other, [-5_000, 0]);
        undecodable.raw_transaction_data = None;
        assert!(
            runtime.moves_subject(&subject, &undecodable),
            "without raw data the reference is kept"
        );
    }
}
