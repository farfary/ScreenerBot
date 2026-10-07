// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana settlement reader: signature verdicts from batched statuses and the block height, holdings from both token programs.

use std::str::FromStr;

use crate::chains::settlement::{Holding, SettlementReader, SignatureCheck, SignatureVerdict};
use crate::chains::solana::assets::ata::token_holding;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::commitment_config::CommitmentLevel;
use crate::chains::solana::solana_sdk::signature::Signature;
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionConfirmationStatus, TransactionStatus,
};
use crate::chains::{ChainId, Error, Result};

/// Blocks after its blockhash was fetched during which a transaction can still land.
pub(crate) const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// The commitment of the height an expiry bound is read from. A blockhash fetched before
/// submission is at most as recent as the confirmed tip, so the confirmed height plus the
/// validity window is never below its last valid height.
const EXPIRY_BOUND_COMMITMENT: CommitmentLevel = CommitmentLevel::Confirmed;

/// The commitment of the height compared against an expiry bound. Once the finalized height
/// has passed the bound, every block that could hold the transaction is finalized and
/// visible to `getTransaction`.
const EXPIRED_HEIGHT_COMMITMENT: CommitmentLevel = CommitmentLevel::Finalized;

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
        // Each status is kept with the slot its node answered at.
        let mut statuses = Vec::with_capacity(signatures.len());
        for chunk in signatures.chunks(MAX_SIGNATURES_PER_STATUS_READ) {
            let (read_slot, chunk_statuses) =
                rpc.get_signature_statuses(chunk)
                    .await
                    .map_err(|error| Error::SettlementRead {
                        chain: ChainId::Solana,
                        detail: error.to_string(),
                    })?;
            statuses.extend(chunk_statuses.into_iter().map(|status| (read_slot, status)));
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

        let needs_tip = checks
            .iter()
            .zip(&statuses)
            .any(|(check, (_, status))| status.is_none() && check.expiry_bound.is_some());
        let tip = if needs_tip {
            finalized_tip(rpc).await.ok()
        } else {
            None
        };

        let mut verdicts = Vec::with_capacity(checks.len());
        for ((check, signature), (read_slot, status)) in
            checks.iter().zip(&signatures).zip(&statuses)
        {
            let verdict =
                match classify_status(status.as_ref(), *read_slot, check.expiry_bound, tip) {
                    StatusRead::Decided(verdict) => verdict,
                    StatusRead::LookUpTransaction => {
                        transaction_lookup(rpc.get_transaction(signature).await).verdict()
                    }
                };
            verdicts.push(verdict);
        }
        Ok(verdicts)
    }

    async fn holding(&self, owner: &str, asset: &str) -> Result<Holding> {
        token_holding(owner, asset)
            .await
            .map_err(|error| Error::SettlementRead {
                chain: ChainId::Solana,
                detail: error.to_string(),
            })
    }

    async fn expiry_bound(&self) -> Result<u64> {
        let height = get_rpc_client()
            .get_block_height_with_commitment(EXPIRY_BOUND_COMMITMENT)
            .await
            .map_err(|error| Error::SettlementRead {
                chain: ChainId::Solana,
                detail: error.to_string(),
            })?;
        Ok(height.saturating_add(SOLANA_BLOCKHASH_VALIDITY_SLOTS))
    }
}

/// The finalized tip an expiry is judged against: a block height and the slot
/// it was read at, from one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalizedTip {
    pub slot: u64,
    pub block_height: u64,
}

/// Read the finalized tip.
pub(crate) async fn finalized_tip(rpc: &impl RpcClientMethods) -> crate::Result<FinalizedTip> {
    let (slot, block_height) = rpc
        .get_slot_and_block_height(EXPIRED_HEIGHT_COMMITMENT)
        .await?;
    Ok(FinalizedTip { slot, block_height })
}

/// What a signature status alone decides.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StatusRead {
    Decided(SignatureVerdict),
    /// The status is null after the expiry: only the transaction lookup decides.
    LookUpTransaction,
}

/// The one rule for what a signature status proves, shared by settlement and
/// by the swap settle loop.
///
/// A status present at any level means a node saw the signature, so it is
/// never absence: a failed status counts only at `confirmed` or above, since a
/// `processed` one may still be rolled back. A null status is evidence of
/// absence only once the finalized height has passed `expiry_bound` and the
/// node that answered with it had reached the slot that height was read at
/// (`read_slot`): a node behind that slot may simply not have the block that
/// holds the transaction yet. Even then the transaction lookup decides.
pub(crate) fn classify_status(
    status: Option<&TransactionStatus>,
    read_slot: u64,
    expiry_bound: Option<u64>,
    tip: Option<FinalizedTip>,
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
        None => match (expiry_bound, tip) {
            (Some(bound), Some(tip)) if tip.block_height > bound && read_slot >= tip.slot => {
                StatusRead::LookUpTransaction
            }
            _ => StatusRead::Decided(SignatureVerdict::Pending),
        },
    }
}

/// The outcome of a `getTransaction` lookup for an expired, unseen signature.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TransactionLookup {
    /// The chain holds no transaction for the signature.
    Missing,
    /// It executed; `error` is the chain's failure, when it failed.
    Executed { error: Option<String> },
    /// The lookup could not be read. Not evidence of anything.
    Unreadable,
}

impl TransactionLookup {
    pub(crate) fn verdict(&self) -> SignatureVerdict {
        match self {
            TransactionLookup::Missing => SignatureVerdict::NotLanded,
            TransactionLookup::Executed { error: Some(_) } => SignatureVerdict::FailedOnChain,
            TransactionLookup::Executed { error: None } => SignatureVerdict::Landed,
            TransactionLookup::Unreadable => SignatureVerdict::Pending,
        }
    }
}

pub(crate) fn transaction_lookup(
    result: crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>>,
) -> TransactionLookup {
    match result {
        Ok(None) => TransactionLookup::Missing,
        Ok(Some(transaction)) => match transaction.transaction.meta {
            Some(meta) => TransactionLookup::Executed {
                error: meta.err.map(|err| err.to_string()),
            },
            None => TransactionLookup::Unreadable,
        },
        Err(_) => TransactionLookup::Unreadable,
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

    fn tip(slot: u64, block_height: u64) -> Option<FinalizedTip> {
        Some(FinalizedTip { slot, block_height })
    }

    #[test]
    fn a_failed_status_at_confirmed_or_above_failed_on_chain() {
        for confirmation in [
            TransactionConfirmationStatus::Confirmed,
            TransactionConfirmationStatus::Finalized,
        ] {
            let read = classify_status(Some(&status(confirmation, failure())), 0, None, None);
            assert_eq!(decided(read), SignatureVerdict::FailedOnChain);
        }
    }

    #[test]
    fn a_clean_status_at_confirmed_or_above_landed() {
        for confirmation in [
            TransactionConfirmationStatus::Confirmed,
            TransactionConfirmationStatus::Finalized,
        ] {
            let read = classify_status(Some(&status(confirmation, None)), 0, None, None);
            assert_eq!(decided(read), SignatureVerdict::Landed);
        }
    }

    /// A node that returned any status saw the signature: past the expiry
    /// and failed or not, a `processed` status is never evidence of absence.
    #[test]
    fn a_processed_status_is_pending_even_past_the_expiry() {
        for err in [None, failure()] {
            let read = classify_status(
                Some(&status(TransactionConfirmationStatus::Processed, err)),
                900,
                Some(10),
                tip(800, 500),
            );
            assert_eq!(decided(read), SignatureVerdict::Pending);
        }
    }

    #[test]
    fn a_null_status_before_the_expiry_bound_is_pending() {
        assert_eq!(
            decided(classify_status(None, 900, Some(100), tip(800, 100))),
            SignatureVerdict::Pending,
            "the last valid block itself still counts"
        );
        assert_eq!(
            decided(classify_status(None, 900, Some(100), tip(800, 42))),
            SignatureVerdict::Pending
        );
    }

    #[test]
    fn a_null_status_without_a_bound_or_a_tip_is_pending() {
        assert_eq!(
            decided(classify_status(None, 900, None, tip(800, 1_000))),
            SignatureVerdict::Pending
        );
        assert_eq!(
            decided(classify_status(None, 900, Some(100), None)),
            SignatureVerdict::Pending
        );
    }

    /// A null status from a node that had not reached the slot the expiry was
    /// read at says nothing: that node may not hold the block with the
    /// transaction yet.
    #[test]
    fn a_null_status_from_a_node_behind_the_expiry_slot_is_pending() {
        assert_eq!(
            decided(classify_status(None, 799, Some(100), tip(800, 101))),
            SignatureVerdict::Pending
        );
    }

    #[test]
    fn a_null_status_past_the_expiry_bound_is_decided_by_the_transaction_lookup() {
        for read_slot in [800, 801] {
            assert_eq!(
                classify_status(None, read_slot, Some(100), tip(800, 101)),
                StatusRead::LookUpTransaction
            );
        }
    }

    #[test]
    fn an_expired_signature_with_no_transaction_did_not_land() {
        assert_eq!(
            transaction_lookup(Ok(None)).verdict(),
            SignatureVerdict::NotLanded
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
            TransactionLookup::Unreadable.verdict(),
            SignatureVerdict::Pending
        );
    }

    fn transaction(meta: serde_json::Value) -> EncodedConfirmedTransactionWithStatusMeta {
        serde_json::from_value(serde_json::json!({
            "slot": 1,
            "transaction": {
                "signatures": ["signature"],
                "message": {
                    "accountKeys": [],
                    "recentBlockhash": "11111111111111111111111111111111",
                    "instructions": []
                }
            },
            "meta": meta,
            "blockTime": null
        }))
        .expect("a transaction response decodes")
    }

    fn meta(err: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "err": err,
            "status": { "Ok": null },
            "fee": 5000,
            "preBalances": [],
            "postBalances": []
        })
    }

    #[test]
    fn a_found_transaction_is_decided_by_its_meta() {
        let failed = meta(serde_json::json!({ "InstructionError": [0, { "Custom": 1 }] }));
        let lookup = transaction_lookup(Ok(Some(transaction(failed))));
        assert!(matches!(
            lookup,
            TransactionLookup::Executed { error: Some(_) }
        ));
        assert_eq!(lookup.verdict(), SignatureVerdict::FailedOnChain);
        assert_eq!(
            transaction_lookup(Ok(Some(transaction(meta(serde_json::Value::Null))))).verdict(),
            SignatureVerdict::Landed
        );
        assert_eq!(
            transaction_lookup(Ok(Some(transaction(serde_json::Value::Null)))).verdict(),
            SignatureVerdict::Pending
        );
    }

    #[test]
    fn the_bound_is_read_at_the_confirmed_tip_and_compared_at_finality() {
        assert_eq!(EXPIRY_BOUND_COMMITMENT, CommitmentLevel::Confirmed);
        assert_eq!(EXPIRED_HEIGHT_COMMITMENT, CommitmentLevel::Finalized);
    }
}
