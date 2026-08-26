//! Proving what a confirmed swap actually delivered.
//!
//! Two different guarantees, and they must not be confused:
//!
//! * SAFETY comes from `min_out` inside the swap instruction. The pool program
//!   itself refuses to return less, so a transaction that confirmed WITHOUT an
//!   error already proves at least the minimum arrived. Nothing measured here
//!   can make a confirmed swap unsafe after the fact.
//! * ACCOUNTING comes from this module. P&L, fills and position sizing need the
//!   amount that actually landed, not the estimate.
//!
//! The two output shapes are measured differently, and only one is exact:
//!
//! * A TOKEN output is exact. The transaction's own pre/post token balances for
//!   the owner and mint give the delta to the raw unit, and a delta below the
//!   guaranteed minimum is a hard [`DirectSwapError::OutputNotReceived`].
//! * A NATIVE SOL output is approximate, because the WSOL account is closed in
//!   the same transaction: the balance to read no longer exists, so the only
//!   evidence is the owner's lamport delta, which also carries the transaction
//!   fee and any account rent that moved. It is reported, never used to fail a
//!   swap the chain already accepted — a false alarm there would report a
//!   successful sell as a failure.

use super::error::{DirectSwapError, DirectSwapResult};
use super::plan::SwapPlan;
use crate::chains::solana::rpc::types::TransactionDetails;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// What a confirmed swap delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    /// Raw units of the output mint that reached the wallet.
    pub received: u64,
    /// Whether [`Self::received`] is an exact on-chain measurement. False for a
    /// native-SOL output, where the figure is a lamport delta net of costs.
    pub exact: bool,
    /// Network fee the transaction paid, in lamports.
    pub network_fee_lamports: u64,
    /// Slot the transaction landed in.
    pub slot: u64,
}

/// Read the receipt out of a confirmed transaction.
///
/// `signature` is only used to build errors; the caller has already confirmed it.
pub fn receipt_from_transaction(
    signature: &str,
    owner: &Pubkey,
    plan: &SwapPlan,
    details: &TransactionDetails,
) -> DirectSwapResult<Receipt> {
    let meta = details
        .meta
        .as_ref()
        .ok_or_else(|| DirectSwapError::TransactionFailed {
            signature: signature.to_owned(),
            detail: "the confirmed transaction carries no metadata to verify against".to_owned(),
        })?;

    if let Some(err) = &meta.err {
        return Err(DirectSwapError::TransactionFailed {
            signature: signature.to_owned(),
            detail: err.to_string(),
        });
    }

    if plan.output_is_native {
        // The wrapped account is gone by now; the lamport delta is all there is.
        let pre = meta.pre_balances.first().copied().unwrap_or(0);
        let post = meta.post_balances.first().copied().unwrap_or(0);
        let gained = post.saturating_sub(pre).saturating_add(meta.fee);
        return Ok(Receipt {
            received: gained,
            exact: false,
            network_fee_lamports: meta.fee,
            slot: details.slot,
        });
    }

    let owner_str = owner.to_string();
    let mint_str = plan.quote.output_mint.to_string();
    let balance_of = |balances: &Option<Vec<crate::chains::solana::rpc::types::TokenBalance>>| {
        balances
            .as_ref()
            .map(|list| {
                list.iter()
                    .filter(|b| {
                        b.mint == mint_str && b.owner.as_deref() == Some(owner_str.as_str())
                    })
                    .filter_map(|b| b.ui_token_amount.amount.parse::<u128>().ok())
                    .sum::<u128>()
            })
            .unwrap_or(0)
    };

    let pre = balance_of(&meta.pre_token_balances);
    let post = balance_of(&meta.post_token_balances);
    let received = post.saturating_sub(pre) as u64;

    if received < plan.quote.min_net_out {
        return Err(DirectSwapError::OutputNotReceived {
            signature: signature.to_owned(),
            expected_minimum: plan.quote.min_net_out,
            received,
        });
    }

    Ok(Receipt {
        received,
        exact: true,
        network_fee_lamports: meta.fee,
        slot: details.slot,
    })
}
