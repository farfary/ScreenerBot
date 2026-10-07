// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Settlement of queued verifications through the chain runtime: the reader for the positions store's chain, expiry bounds for submissions, and what each signature verdict means for a position.

use std::sync::Arc;

use crate::chains::{
    runtime_for, Holding, RawAmount, SettlementReader, SignatureCheck, SignatureVerdict,
};
use crate::logger::{self, LogTag};

use super::db::get_store_chain;
use super::queue::VerificationItem;
use super::round_state::{attributable_is_dust, expected_acquisition};
use super::state::{get_position_by_id, is_position_open, POSITIONS};
use super::transitions::{NotLandedEvidence, PositionTransition};
use super::types::VerificationKind;
use super::{Error, Result};

/// The settlement reader of the chain whose positions the store holds.
pub(crate) async fn reader() -> Result<Arc<dyn SettlementReader>> {
    let chain = get_store_chain().await?;
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

/// The holding of `owner` in `mint` on the chain whose positions the store holds.
pub(crate) async fn holding(owner: &str, mint: &str) -> Result<Holding> {
    Ok(reader().await?.holding(owner, mint).await?)
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
    /// The swap did not land, but that cannot be acted on yet: the read is the first of its
    /// kind, or an entry's attributable holding is not known to be dust. Read again after a
    /// widening backoff.
    Defer,
    /// Stop verifying and apply the transition.
    Apply(PositionTransition),
}

/// The disposition of `item` under `verdict`, the single owner of what a settled signature
/// means for a position. A swap the chain already confirmed is never settled as not landed,
/// whatever a later read finds: a node that no longer serves the signature reads a landed
/// swap as unseen. A swap that failed on chain or did not land moved nothing, so no branch
/// writes a position off. A not-landed read is final only when the previous read of the item
/// found the same, so one lagging provider cannot decide it. An entry that did not land is
/// removed only when the wallet's holding attributable to it is known to be dust:
/// `attributable_is_dust` is `None` when that holding could not be read.
pub(crate) fn disposition(
    item: &VerificationItem,
    verdict: SignatureVerdict,
    attributable_is_dust: Option<bool>,
) -> Disposition {
    let Some(position_id) = item.position_id else {
        return Disposition::Requeue {
            swap_confirmed: item.swap_confirmed || verdict == SignatureVerdict::Landed,
        };
    };
    if item.swap_confirmed {
        return Disposition::Requeue {
            swap_confirmed: true,
        };
    }
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
        SignatureVerdict::NotLanded if !item.not_landed_seen() => return Disposition::Defer,
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
                return Disposition::Defer;
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
/// the mint hold, is dust against the acquisition the entry price implies. `None` when the
/// position, the token's decimals or the holding cannot be read, or when that acquisition is
/// too small to tell a holding from dust.
pub(crate) async fn entry_attributable_is_dust(item: &VerificationItem) -> Option<bool> {
    let position = get_position_by_id(item.position_id?).await?;
    let chain = get_store_chain().await.ok()?;
    let expected = expected_acquisition(
        position.entry_size_native,
        position.entry_price,
        crate::tokens::get_decimals(chain, &item.mint).await,
    )?;
    let wallet = crate::utils::get_wallet_address().ok()?;
    let holding = holding(&wallet, &item.mint)
        .await
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
        .filter(|p| p.mint == item.mint && p.id != position.id && is_position_open(p))
        .filter_map(|p| p.remaining_token_amount.or(p.token_amount))
        .fold(RawAmount::ZERO, |sum, held| {
            sum.checked_add(held).unwrap_or(RawAmount::new(u128::MAX))
        });
    attributable_is_dust(holding.amount, held_by_others, Some(expected))
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

    /// The item after a previous settlement read found its swap did not land.
    fn seen(item: VerificationItem) -> VerificationItem {
        let deferred = item.deferred();
        assert!(deferred.not_landed_seen());
        deferred
    }

    fn items() -> [VerificationItem; 4] {
        [entry(), dca(), partial_exit(), full_exit()]
    }

    fn requeued(disposition: Disposition) -> bool {
        match disposition {
            Disposition::Requeue { swap_confirmed } => swap_confirmed,
            other => panic!("expected a requeue, got {other:?}"),
        }
    }

    fn applied(disposition: Disposition) -> PositionTransition {
        match disposition {
            Disposition::Apply(transition) => transition,
            other => panic!("expected a transition, got {other:?}"),
        }
    }

    fn is_deferred(disposition: Disposition) -> bool {
        matches!(disposition, Disposition::Defer)
    }

    #[test]
    fn a_landed_swap_of_any_kind_is_requeued_as_confirmed() {
        for item in items()
            .into_iter()
            .flat_map(|item| [item.clone(), seen(item)])
        {
            for dust in [None, Some(true), Some(false)] {
                assert!(requeued(disposition(&item, SignatureVerdict::Landed, dust)));
            }
        }
    }

    #[test]
    fn a_pending_swap_of_any_kind_is_requeued_unconfirmed() {
        for item in items()
            .into_iter()
            .flat_map(|item| [item.clone(), seen(item)])
        {
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
    fn a_first_not_landed_read_of_any_kind_is_deferred() {
        for item in items() {
            for dust in [None, Some(true), Some(false)] {
                assert!(
                    is_deferred(disposition(&item, SignatureVerdict::NotLanded, dust)),
                    "{} acted on one not-landed read",
                    item.signature
                );
            }
        }
    }

    #[test]
    fn an_entry_that_failed_on_chain_is_removed_with_that_evidence() {
        for item in [entry(), seen(entry())] {
            for dust in [None, Some(true), Some(false)] {
                assert!(matches!(
                    applied(disposition(&item, SignatureVerdict::FailedOnChain, dust)),
                    PositionTransition::RemoveOrphanEntry {
                        position_id: 7,
                        ref signature,
                        evidence: NotLandedEvidence::FailedOnChain,
                    } if signature == "entry-sig"
                ));
            }
        }
    }

    #[test]
    fn an_entry_that_did_not_land_twice_is_removed_only_when_its_holding_is_known_dust() {
        assert!(matches!(
            applied(disposition(
                &seen(entry()),
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
            assert!(is_deferred(disposition(
                &seen(entry()),
                SignatureVerdict::NotLanded,
                dust
            )));
        }
    }

    #[test]
    fn a_dca_that_failed_or_did_not_land_twice_is_marked_failed() {
        for (item, verdict) in [
            (dca(), SignatureVerdict::FailedOnChain),
            (seen(dca()), SignatureVerdict::FailedOnChain),
            (seen(dca()), SignatureVerdict::NotLanded),
        ] {
            for dust in [None, Some(true), Some(false)] {
                assert!(matches!(
                    applied(disposition(&item, verdict, dust)),
                    PositionTransition::DcaFailed { position_id: 7, ref dca_signature, .. }
                        if dca_signature == "dca-sig"
                ));
            }
        }
    }

    #[test]
    fn a_partial_exit_that_failed_or_did_not_land_twice_is_marked_failed() {
        for (item, verdict) in [
            (partial_exit(), SignatureVerdict::FailedOnChain),
            (seen(partial_exit()), SignatureVerdict::NotLanded),
        ] {
            assert!(matches!(
                applied(disposition(&item, verdict, None)),
                PositionTransition::PartialExitFailed { position_id: 7, .. }
            ));
        }
    }

    #[test]
    fn a_full_exit_that_failed_or_did_not_land_twice_is_cleared_for_retry() {
        for (item, verdict) in [
            (full_exit(), SignatureVerdict::FailedOnChain),
            (seen(full_exit()), SignatureVerdict::NotLanded),
        ] {
            assert!(matches!(
                applied(disposition(&item, verdict, None)),
                PositionTransition::ExitFailedClearForRetry { position_id: 7, ref exit_signature }
                    if exit_signature == "exit-sig"
            ));
        }
    }

    /// A swap the chain confirmed is never settled as not landed by any later read, of any
    /// kind and at any count of earlier not-landed reads.
    #[test]
    fn a_confirmed_swap_is_requeued_as_confirmed_under_every_verdict() {
        for mut item in items()
            .into_iter()
            .flat_map(|item| [item.clone(), seen(item)])
        {
            item.swap_confirmed = true;
            for verdict in ALL_VERDICTS {
                for dust in [None, Some(true), Some(false)] {
                    assert!(
                        requeued(disposition(&item, verdict, dust)),
                        "{verdict:?} on confirmed {} (reads {})",
                        item.signature,
                        item.not_landed_reads
                    );
                }
            }
        }
    }

    #[test]
    fn an_item_without_a_position_is_never_applied() {
        for mut item in items()
            .into_iter()
            .flat_map(|item| [item.clone(), seen(item)])
        {
            item.position_id = None;
            for verdict in ALL_VERDICTS {
                for dust in [None, Some(true), Some(false)] {
                    assert!(!matches!(
                        disposition(&item, verdict, dust),
                        Disposition::Apply(_)
                    ));
                }
            }
        }
    }

    #[test]
    fn a_dca_never_removes_its_position() {
        for item in [dca(), seen(dca())] {
            for verdict in ALL_VERDICTS {
                for dust in [None, Some(true), Some(false)] {
                    assert!(!matches!(
                        disposition(&item, verdict, dust),
                        Disposition::Apply(PositionTransition::RemoveOrphanEntry { .. })
                    ));
                }
            }
        }
    }

    /// No verdict writes a position off or books anything: every transition a settlement
    /// applies only removes an entry that never landed or clears a swap that did not land.
    #[test]
    fn every_applied_transition_only_undoes_a_swap_that_did_not_land() {
        for item in items()
            .into_iter()
            .flat_map(|item| [item.clone(), seen(item)])
        {
            for verdict in ALL_VERDICTS {
                for dust in [None, Some(true), Some(false)] {
                    if let Disposition::Apply(transition) = disposition(&item, verdict, dust) {
                        assert!(
                            matches!(
                                transition,
                                PositionTransition::RemoveOrphanEntry { .. }
                                    | PositionTransition::DcaFailed { .. }
                                    | PositionTransition::PartialExitFailed { .. }
                                    | PositionTransition::ExitFailedClearForRetry { .. }
                            ),
                            "{verdict:?} on {} applied {transition:?}",
                            item.signature
                        );
                    }
                }
            }
        }
    }
}
