// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Settlement of queued verifications through the chain runtime: the reader for the positions store's chain, expiry bounds for submissions, and what each signature verdict means for a position.

use std::sync::Arc;

use crate::chains::{runtime_for, RawAmount, SettlementReader, SignatureCheck, SignatureVerdict};
use crate::logger::{self, LogTag};

use super::db::with_positions_database;
use super::queue::VerificationItem;
use super::round_state::is_dust;
use super::state::{get_position_by_id, is_position_open, POSITIONS};
use super::transitions::{NotLandedEvidence, PositionTransition};
use super::types::VerificationKind;
use super::{Error, Result};

/// The settlement reader of the chain whose positions the store holds.
pub(crate) async fn reader() -> Result<Arc<dyn SettlementReader>> {
    let chain = with_positions_database(|db| Ok(db.chain())).await?;
    let runtime = runtime_for(chain).ok_or(crate::chains::Error::ChainNotEnabled { chain })?;
    Ok(runtime.settlement())
}

/// The bound after which a transaction submitted now can no longer land, or `None` when it
/// cannot be read; without a bound an unseen signature stays pending.
pub(crate) async fn submission_expiry_bound() -> Option<u64> {
    let bound = match reader().await {
        Ok(reader) => reader.expiry_bound().await.map_err(Error::from),
        Err(error) => Err(error),
    };
    bound
        .inspect_err(|error| {
            logger::warning(
                LogTag::Positions,
                &format!("Submission expiry bound unavailable: {error}"),
            );
        })
        .ok()
}

/// One verdict per item, in order, from one batched read.
pub(crate) async fn signature_verdicts(
    items: &[VerificationItem],
) -> Result<Vec<SignatureVerdict>> {
    let checks: Vec<SignatureCheck> = items
        .iter()
        .map(|item| SignatureCheck {
            signature: item.signature.clone(),
            expiry_bound: item.expiry_height,
        })
        .collect();
    Ok(reader().await?.signature_verdicts(&checks).await?)
}

/// What a signature verdict means for the verification of one queued item.
#[derive(Debug, Clone)]
pub(crate) enum Disposition {
    /// Keep verifying. `swap_confirmed` is true when the chain confirmed the swap landed, so
    /// the item is never settled as not landed again.
    Requeue { swap_confirmed: bool },
    /// Stop verifying and apply the transition.
    Apply(PositionTransition),
}

/// The disposition of `item` under `verdict`. A swap that failed on chain or did not land
/// moved nothing, so no branch writes a position off. An entry that did not land is
/// removed only when the wallet's holding attributable to it is known to be dust:
/// `attributable_is_dust` is `None` when that holding could not be read.
pub(crate) fn disposition(
    item: &VerificationItem,
    verdict: SignatureVerdict,
    attributable_is_dust: Option<bool>,
) -> Disposition {
    let Some(position_id) = item.position_id else {
        return Disposition::Requeue {
            swap_confirmed: verdict == SignatureVerdict::Landed,
        };
    };
    let evidence = match verdict {
        SignatureVerdict::Landed => {
            return Disposition::Requeue {
                swap_confirmed: true,
            }
        }
        SignatureVerdict::Pending => {
            return Disposition::Requeue {
                swap_confirmed: false,
            }
        }
        SignatureVerdict::FailedOnChain => NotLandedEvidence::FailedOnChain,
        SignatureVerdict::NotLanded => NotLandedEvidence::Expired,
    };
    let reason = match evidence {
        NotLandedEvidence::FailedOnChain => "The transaction failed on chain",
        NotLandedEvidence::Expired => "The transaction expired without landing",
    }
    .to_owned();

    let transition = match item.kind {
        VerificationKind::Entry if item.is_dca => PositionTransition::DcaFailed {
            position_id,
            dca_signature: item.signature.clone(),
            reason,
        },
        VerificationKind::Entry => {
            if evidence == NotLandedEvidence::Expired && attributable_is_dust != Some(true) {
                return Disposition::Requeue {
                    swap_confirmed: false,
                };
            }
            PositionTransition::RemoveOrphanEntry {
                position_id,
                signature: item.signature.clone(),
                evidence,
            }
        }
        VerificationKind::Exit if item.is_partial_exit => PositionTransition::PartialExitFailed {
            position_id,
            reason,
        },
        VerificationKind::Exit => PositionTransition::ExitFailedClearForRetry {
            position_id,
            exit_signature: item.signature.clone(),
        },
    };
    Disposition::Apply(transition)
}

/// Whether the wallet's holding of the item's mint, less what the other open positions of
/// the mint hold, is dust against what the item's position bought. `None` when the holding
/// or the position cannot be read.
pub(crate) async fn entry_attributable_is_dust(item: &VerificationItem) -> Option<bool> {
    let position_id = item.position_id?;
    let position = get_position_by_id(position_id).await?;
    let wallet = crate::utils::get_wallet_address().ok()?;
    let holding = match reader().await {
        Ok(reader) => reader
            .holding(&wallet, &item.mint)
            .await
            .map_err(Error::from),
        Err(error) => Err(error),
    }
    .inspect_err(|error| {
        logger::warning(
            LogTag::Positions,
            &format!("Holding of {} unavailable: {error}", item.mint),
        );
    })
    .ok()?;

    let held_by_others = POSITIONS
        .read()
        .await
        .iter()
        .filter(|p| p.mint == item.mint && p.id != Some(position_id) && is_position_open(p))
        .filter_map(|p| p.remaining_token_amount.or(p.token_amount))
        .fold(RawAmount::ZERO, |sum, held| {
            sum.checked_add(held).unwrap_or(RawAmount::new(u128::MAX))
        });
    let attributable = holding
        .amount
        .checked_sub(held_by_others)
        .unwrap_or(RawAmount::ZERO);
    Some(is_dust(
        attributable,
        position.token_amount.unwrap_or(RawAmount::ZERO),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_VERDICTS: [SignatureVerdict; 4] = [
        SignatureVerdict::Landed,
        SignatureVerdict::FailedOnChain,
        SignatureVerdict::Pending,
        SignatureVerdict::NotLanded,
    ];

    fn entry() -> VerificationItem {
        VerificationItem::new(
            "entry-sig".to_owned(),
            "mint".to_owned(),
            Some(7),
            VerificationKind::Entry,
            Some(100),
        )
    }

    fn dca() -> VerificationItem {
        VerificationItem::new_dca("dca-sig".to_owned(), "mint".to_owned(), Some(7), Some(100))
    }

    fn partial_exit() -> VerificationItem {
        VerificationItem::new_partial_exit(
            "partial-sig".to_owned(),
            "mint".to_owned(),
            Some(7),
            RawAmount::new(500),
            50.0,
            Some(100),
        )
    }

    fn full_exit() -> VerificationItem {
        VerificationItem::new(
            "exit-sig".to_owned(),
            "mint".to_owned(),
            Some(7),
            VerificationKind::Exit,
            Some(100),
        )
    }

    fn requeued(disposition: Disposition) -> bool {
        match disposition {
            Disposition::Requeue { swap_confirmed } => swap_confirmed,
            Disposition::Apply(transition) => panic!("expected a requeue, got {transition:?}"),
        }
    }

    fn applied(disposition: Disposition) -> PositionTransition {
        match disposition {
            Disposition::Apply(transition) => transition,
            Disposition::Requeue { .. } => panic!("expected a transition"),
        }
    }

    #[test]
    fn a_landed_swap_of_any_kind_is_requeued_as_confirmed() {
        for item in [entry(), dca(), partial_exit(), full_exit()] {
            for dust in [None, Some(true), Some(false)] {
                assert!(requeued(disposition(&item, SignatureVerdict::Landed, dust)));
            }
        }
    }

    #[test]
    fn a_pending_swap_of_any_kind_is_requeued_unconfirmed() {
        for item in [entry(), dca(), partial_exit(), full_exit()] {
            for dust in [None, Some(true), Some(false)] {
                assert!(!requeued(disposition(
                    &item,
                    SignatureVerdict::Pending,
                    dust
                )));
            }
        }
    }

    #[test]
    fn an_entry_that_failed_on_chain_is_removed_with_that_evidence() {
        for dust in [None, Some(true), Some(false)] {
            assert!(matches!(
                applied(disposition(&entry(), SignatureVerdict::FailedOnChain, dust)),
                PositionTransition::RemoveOrphanEntry {
                    position_id: 7,
                    ref signature,
                    evidence: NotLandedEvidence::FailedOnChain,
                } if signature == "entry-sig"
            ));
        }
    }

    #[test]
    fn an_entry_that_did_not_land_is_removed_only_when_its_holding_is_known_dust() {
        assert!(matches!(
            applied(disposition(
                &entry(),
                SignatureVerdict::NotLanded,
                Some(true)
            )),
            PositionTransition::RemoveOrphanEntry {
                position_id: 7,
                evidence: NotLandedEvidence::Expired,
                ..
            }
        ));
        for dust in [None, Some(false)] {
            assert!(!requeued(disposition(
                &entry(),
                SignatureVerdict::NotLanded,
                dust
            )));
        }
    }

    #[test]
    fn a_dca_that_failed_or_did_not_land_is_marked_failed() {
        for verdict in [SignatureVerdict::FailedOnChain, SignatureVerdict::NotLanded] {
            for dust in [None, Some(true), Some(false)] {
                assert!(matches!(
                    applied(disposition(&dca(), verdict, dust)),
                    PositionTransition::DcaFailed { position_id: 7, ref dca_signature, .. }
                        if dca_signature == "dca-sig"
                ));
            }
        }
    }

    #[test]
    fn a_partial_exit_that_failed_or_did_not_land_is_marked_failed() {
        for verdict in [SignatureVerdict::FailedOnChain, SignatureVerdict::NotLanded] {
            assert!(matches!(
                applied(disposition(&partial_exit(), verdict, None)),
                PositionTransition::PartialExitFailed { position_id: 7, .. }
            ));
        }
    }

    #[test]
    fn a_full_exit_that_failed_or_did_not_land_is_cleared_for_retry() {
        for verdict in [SignatureVerdict::FailedOnChain, SignatureVerdict::NotLanded] {
            assert!(matches!(
                applied(disposition(&full_exit(), verdict, None)),
                PositionTransition::ExitFailedClearForRetry { position_id: 7, ref exit_signature }
                    if exit_signature == "exit-sig"
            ));
        }
    }

    #[test]
    fn a_dca_never_removes_its_position() {
        for verdict in ALL_VERDICTS {
            for dust in [None, Some(true), Some(false)] {
                assert!(!matches!(
                    disposition(&dca(), verdict, dust),
                    Disposition::Apply(PositionTransition::RemoveOrphanEntry { .. })
                ));
            }
        }
    }

    #[test]
    fn no_verdict_writes_a_position_off() {
        for item in [entry(), dca(), partial_exit(), full_exit()] {
            for verdict in ALL_VERDICTS {
                for dust in [None, Some(true), Some(false)] {
                    assert!(!matches!(
                        disposition(&item, verdict, dust),
                        Disposition::Apply(
                            PositionTransition::ExitPermanentFailureSynthetic { .. }
                        )
                    ));
                }
            }
        }
    }
}
