// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana settlement reader: signature verdicts from batched statuses and the block height, holdings from both token programs.

use std::str::FromStr;

use crate::chains::settlement::{Holding, SettlementReader, SignatureCheck, SignatureVerdict};
use crate::chains::solana::assets::ata::get_all_token_accounts;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods, TokenAccountInfo};
use crate::chains::solana::solana_sdk::signature::Signature;
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionConfirmationStatus, TransactionStatus,
};
use crate::chains::{ChainId, Error, RawAmount, Result};

/// Blocks after its blockhash was fetched during which a transaction can still land.
pub(crate) const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// The most signatures one `getSignatureStatuses` request accepts.
const MAX_SIGNATURES_PER_STATUS_READ: usize = 256;

/// The Solana implementation of [`SettlementReader`], reading through the process RPC client.
pub struct SolanaSettlement;

#[async_trait::async_trait]
impl SettlementReader for SolanaSettlement {
    async fn signature_verdicts(&self, checks: &[SignatureCheck]) -> Result<Vec<SignatureVerdict>> {
        if checks.is_empty() {
            return Ok(Vec::new());
        }
        let signatures = checks
            .iter()
            .map(|check| {
                Signature::from_str(&check.signature).map_err(|error| Error::SettlementRead {
                    chain: ChainId::Solana,
                    detail: format!("invalid signature '{}': {error}", check.signature),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let rpc = get_rpc_client();
        let mut statuses = Vec::with_capacity(signatures.len());
        for chunk in signatures.chunks(MAX_SIGNATURES_PER_STATUS_READ) {
            statuses.extend(rpc.get_signature_statuses(chunk).await.map_err(|error| {
                Error::SettlementRead {
                    chain: ChainId::Solana,
                    detail: error.to_string(),
                }
            })?);
        }
        if statuses.len() != checks.len() {
            return Err(Error::SettlementRead {
                chain: ChainId::Solana,
                detail: format!(
                    "{} statuses returned for {} signatures",
                    statuses.len(),
                    checks.len()
                ),
            });
        }

        let needs_height = checks
            .iter()
            .zip(&statuses)
            .any(|(check, status)| status.is_none() && check.expiry_bound.is_some());
        let block_height = if needs_height {
            rpc.get_block_height().await.ok()
        } else {
            None
        };

        let mut verdicts = Vec::with_capacity(checks.len());
        for ((check, signature), status) in checks.iter().zip(&signatures).zip(&statuses) {
            let verdict = match classify_status(status.as_ref(), check.expiry_bound, block_height) {
                StatusRead::Decided(verdict) => verdict,
                StatusRead::LookUpTransaction => {
                    lookup_verdict(transaction_lookup(rpc.get_transaction(signature).await))
                }
            };
            verdicts.push(verdict);
        }
        Ok(verdicts)
    }

    async fn holding(&self, owner: &str, asset: &str) -> Result<Holding> {
        let accounts =
            get_all_token_accounts(owner)
                .await
                .map_err(|error| Error::SettlementRead {
                    chain: ChainId::Solana,
                    detail: error.to_string(),
                })?;
        Ok(holding_of(&accounts, asset))
    }

    async fn expiry_bound(&self) -> Result<u64> {
        let height =
            get_rpc_client()
                .get_block_height()
                .await
                .map_err(|error| Error::SettlementRead {
                    chain: ChainId::Solana,
                    detail: error.to_string(),
                })?;
        Ok(height.saturating_add(SOLANA_BLOCKHASH_VALIDITY_SLOTS))
    }
}

/// What a signature status alone decides.
#[derive(Debug, PartialEq, Eq)]
enum StatusRead {
    Decided(SignatureVerdict),
    /// The status is null after the expiry bound: only the transaction lookup decides.
    LookUpTransaction,
}

/// A failed status counts only at `confirmed` or above; a `processed` status may still be
/// rolled back. A null status is undecided until the block height has passed the expiry bound.
fn classify_status(
    status: Option<&TransactionStatus>,
    expiry_bound: Option<u64>,
    block_height: Option<u64>,
) -> StatusRead {
    match status {
        Some(status) => StatusRead::Decided(match status.confirmation_status() {
            TransactionConfirmationStatus::Processed => SignatureVerdict::Pending,
            TransactionConfirmationStatus::Confirmed | TransactionConfirmationStatus::Finalized => {
                if status.err.is_some() {
                    SignatureVerdict::FailedOnChain
                } else {
                    SignatureVerdict::Landed
                }
            }
        }),
        None => match (expiry_bound, block_height) {
            (Some(bound), Some(height)) if height > bound => StatusRead::LookUpTransaction,
            _ => StatusRead::Decided(SignatureVerdict::Pending),
        },
    }
}

/// The outcome of a `getTransaction` lookup for an expired, unseen signature.
#[derive(Debug, PartialEq, Eq)]
enum TransactionLookup {
    Missing,
    Executed { failed: bool },
    Unreadable,
}

fn transaction_lookup(
    result: crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>>,
) -> TransactionLookup {
    match result {
        Ok(None) => TransactionLookup::Missing,
        Ok(Some(transaction)) => match transaction.transaction.meta {
            Some(meta) => TransactionLookup::Executed {
                failed: meta.err.is_some(),
            },
            None => TransactionLookup::Unreadable,
        },
        Err(_) => TransactionLookup::Unreadable,
    }
}

fn lookup_verdict(lookup: TransactionLookup) -> SignatureVerdict {
    match lookup {
        TransactionLookup::Missing => SignatureVerdict::NotLanded,
        TransactionLookup::Executed { failed: true } => SignatureVerdict::FailedOnChain,
        TransactionLookup::Executed { failed: false } => SignatureVerdict::Landed,
        TransactionLookup::Unreadable => SignatureVerdict::Pending,
    }
}

fn holding_of(accounts: &[TokenAccountInfo], asset: &str) -> Holding {
    let held = accounts.iter().filter(|account| account.mint == asset);
    Holding {
        amount: RawAmount::new(
            held.clone()
                .map(|account| u128::from(account.balance))
                .sum(),
        ),
        frozen: held.clone().any(|account| account.is_frozen),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::solana_sdk::instruction::InstructionError;
    use crate::chains::solana::solana_sdk::transaction::TransactionError;

    fn status(
        confirmation: TransactionConfirmationStatus,
        err: Option<TransactionError>,
    ) -> TransactionStatus {
        TransactionStatus {
            slot: 1,
            confirmations: None,
            status: match &err {
                Some(err) => Err(err.clone()),
                None => Ok(()),
            },
            err,
            confirmation_status: Some(confirmation),
        }
    }

    fn failure() -> Option<TransactionError> {
        Some(TransactionError::InstructionError(
            0,
            InstructionError::Custom(1),
        ))
    }

    fn decided(read: StatusRead) -> SignatureVerdict {
        match read {
            StatusRead::Decided(verdict) => verdict,
            StatusRead::LookUpTransaction => panic!("expected a decided verdict"),
        }
    }

    #[test]
    fn a_failed_status_at_confirmed_or_above_failed_on_chain() {
        for confirmation in [
            TransactionConfirmationStatus::Confirmed,
            TransactionConfirmationStatus::Finalized,
        ] {
            let read = classify_status(Some(&status(confirmation, failure())), None, None);
            assert_eq!(decided(read), SignatureVerdict::FailedOnChain);
        }
    }

    #[test]
    fn a_clean_status_at_confirmed_or_above_landed() {
        for confirmation in [
            TransactionConfirmationStatus::Confirmed,
            TransactionConfirmationStatus::Finalized,
        ] {
            let read = classify_status(Some(&status(confirmation, None)), None, None);
            assert_eq!(decided(read), SignatureVerdict::Landed);
        }
    }

    #[test]
    fn a_processed_status_is_pending_even_when_it_failed() {
        for err in [None, failure()] {
            let read = classify_status(
                Some(&status(TransactionConfirmationStatus::Processed, err)),
                Some(10),
                Some(500),
            );
            assert_eq!(decided(read), SignatureVerdict::Pending);
        }
    }

    #[test]
    fn a_null_status_before_the_expiry_bound_is_pending() {
        assert_eq!(
            decided(classify_status(None, Some(100), Some(100))),
            SignatureVerdict::Pending
        );
        assert_eq!(
            decided(classify_status(None, Some(100), Some(42))),
            SignatureVerdict::Pending
        );
    }

    #[test]
    fn a_null_status_without_a_bound_or_a_height_is_pending() {
        assert_eq!(
            decided(classify_status(None, None, Some(1_000))),
            SignatureVerdict::Pending
        );
        assert_eq!(
            decided(classify_status(None, Some(100), None)),
            SignatureVerdict::Pending
        );
    }

    #[test]
    fn a_null_status_past_the_expiry_bound_is_decided_by_the_transaction_lookup() {
        assert_eq!(
            classify_status(None, Some(100), Some(101)),
            StatusRead::LookUpTransaction
        );
    }

    #[test]
    fn an_expired_signature_with_no_transaction_did_not_land() {
        assert_eq!(
            lookup_verdict(transaction_lookup(Ok(None))),
            SignatureVerdict::NotLanded
        );
    }

    #[test]
    fn an_expired_signature_whose_transaction_is_found_is_decided_by_its_meta() {
        assert_eq!(
            lookup_verdict(TransactionLookup::Executed { failed: false }),
            SignatureVerdict::Landed
        );
        assert_eq!(
            lookup_verdict(TransactionLookup::Executed { failed: true }),
            SignatureVerdict::FailedOnChain
        );
    }

    #[test]
    fn an_unreadable_transaction_lookup_is_pending() {
        let error = crate::Error::Data(crate::errors::DataError::ParseError {
            data_type: "transaction".to_owned(),
            error: "unreachable".to_owned(),
        });
        assert_eq!(
            transaction_lookup(Err(error)),
            TransactionLookup::Unreadable
        );
        assert_eq!(
            lookup_verdict(TransactionLookup::Unreadable),
            SignatureVerdict::Pending
        );
    }

    fn account(mint: &str, balance: u64, is_frozen: bool) -> TokenAccountInfo {
        TokenAccountInfo {
            account: format!("{mint}-{balance}"),
            mint: mint.to_owned(),
            balance,
            decimals: 6,
            is_token_2022: false,
            is_nft: false,
            is_frozen,
        }
    }

    #[test]
    fn a_holding_sums_every_account_of_the_asset_and_flags_any_frozen_one() {
        let accounts = [
            account("asset", 700, false),
            account("other", 5_000, true),
            account("asset", 300, true),
        ];
        assert_eq!(
            holding_of(&accounts, "asset"),
            Holding {
                amount: RawAmount::new(1_000),
                frozen: true,
            }
        );
        assert_eq!(
            holding_of(&accounts[..1], "asset"),
            Holding {
                amount: RawAmount::new(700),
                frozen: false,
            }
        );
        assert_eq!(
            holding_of(&accounts, "absent"),
            Holding {
                amount: RawAmount::ZERO,
                frozen: false,
            }
        );
    }
}
