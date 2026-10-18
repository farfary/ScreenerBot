// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Conservation over the wallet's own accounts in a simulated run: what the
//! run takes from the wallet, and what it delivers to it.
//!
//! The controlled set is the wallet itself (index 0, the fee payer) plus every
//! token account a pre or post token balance names the wallet as owner of.
//! Rent the wallet pays into its own new accounts stays inside that set, so
//! it nets out. A node that omits a balance field, or the owner of any token
//! balance, skips the check: some nodes do not return them, and their absence
//! is not evidence of a cost.

use std::collections::{BTreeMap, BTreeSet};

use super::error::BuildRefusal;
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::rpc::types::{SimulationOutcome, TokenBalance};
use crate::logger::{self, LogTag};
use crate::swaps::Quote;

/// The token balances of a run, when the node returned both lists and named
/// the owner of every balance in them.
fn token_balances(outcome: &SimulationOutcome) -> Option<(&[TokenBalance], &[TokenBalance])> {
    match (&outcome.pre_token_balances, &outcome.post_token_balances) {
        (Some(pre), Some(post)) => {
            if pre
                .iter()
                .chain(post)
                .any(|balance| balance.owner.is_none())
            {
                logger::debug(
                    LogTag::Swap,
                    "Simulation returned a token balance without an owner; the balance check is \
                     skipped",
                );
                return None;
            }
            Some((pre, post))
        }
        _ => {
            logger::debug(
                LogTag::Swap,
                "Simulation returned no token balances; the balance check is skipped",
            );
            None
        }
    }
}

/// The net change of every mint the wallet holds, summed over the wallet's own
/// token accounts. `None` when an amount is not an integer.
fn token_deltas(
    wallet: &str,
    pre: &[TokenBalance],
    post: &[TokenBalance],
) -> Option<BTreeMap<String, i128>> {
    let mut deltas: BTreeMap<String, i128> = BTreeMap::new();
    for (balances, sign) in [(pre, -1i128), (post, 1i128)] {
        for balance in balances
            .iter()
            .filter(|balance| balance.owner.as_deref() == Some(wallet))
        {
            let amount: i128 = balance.ui_token_amount.amount.parse().ok()?;
            *deltas.entry(balance.mint.clone()).or_default() += sign * amount;
        }
    }
    Some(deltas)
}

/// Refuse a run that leaves the wallet with less of any mint than the trade
/// explains: the input may fall by at most the quoted input, the output must
/// rise by at least the quoted minimum, and every other mint must not fall.
/// Wrapped SOL is native and is left to [`conserve_lamports`].
pub fn conserve_tokens(
    outcome: &SimulationOutcome,
    quote: &Quote,
    output_transfer_fee: Option<u64>,
) -> Result<(), BuildRefusal> {
    let Some((pre, post)) = token_balances(outcome) else {
        return Ok(());
    };
    let Some(deltas) = token_deltas(&quote.wallet_address, pre, post) else {
        logger::debug(
            LogTag::Swap,
            "Simulation returned a token amount that is not an integer; the balance check is \
             skipped",
        );
        return Ok(());
    };
    let adapter = crate::chains::adapter();
    let input_native = adapter.is_native_asset(&quote.input_mint);
    let output_native = adapter.is_native_asset(&quote.output_mint);

    if !output_native {
        let received = deltas.get(&quote.output_mint).copied().unwrap_or(0);
        let minimum = quote
            .minimum_output_amount
            .raw()
            .saturating_sub(u128::from(output_transfer_fee.unwrap_or(0)));
        let minimum = i128::try_from(minimum).map_err(|_| BuildRefusal::QuoteAmountOutOfRange {
            leg: "minimum",
            limit: "i128 range",
        })?;
        if received < minimum {
            return Err(BuildRefusal::OutputBelowMinimum {
                received,
                mint: quote.output_mint.clone(),
                minimum,
            });
        }
    }
    for (mint, delta) in &deltas {
        if mint == SOL_MINT || *delta >= 0 || (!output_native && mint == &quote.output_mint) {
            continue;
        }
        let spent = delta.unsigned_abs();
        if !input_native && mint == &quote.input_mint {
            if spent > quote.input_amount.raw() {
                return Err(BuildRefusal::InputAboveQuote {
                    spent,
                    mint: mint.clone(),
                    input: quote.input_amount.raw(),
                });
            }
            continue;
        }
        return Err(BuildRefusal::UnspentMintTaken {
            spent,
            mint: mint.clone(),
        });
    }
    Ok(())
}

/// The trade's native legs: the input it spends and the minimum it must
/// return, each zero when that leg is not the native asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLegs {
    pub input: u64,
    pub minimum_output: u64,
}

impl NativeLegs {
    /// The native legs of `quote`, or the refusal detail when one does not
    /// fit the Solana u64 range.
    pub fn of(quote: &Quote) -> Result<Self, BuildRefusal> {
        let adapter = crate::chains::adapter();
        let input = match adapter.is_native_asset(&quote.input_mint) {
            true => u64::try_from(quote.input_amount).map_err(|_| {
                BuildRefusal::QuoteAmountOutOfRange {
                    leg: "input",
                    limit: "Solana u64 limit",
                }
            })?,
            false => 0,
        };
        let minimum_output = match adapter.is_native_asset(&quote.output_mint) {
            true => u64::try_from(quote.minimum_output_amount).map_err(|_| {
                BuildRefusal::QuoteAmountOutOfRange {
                    leg: "minimum",
                    limit: "Solana u64 limit",
                }
            })?,
            false => 0,
        };
        Ok(Self {
            input,
            minimum_output,
        })
    }
}

/// Refuse a run whose lamports are not explained by the trade plus
/// `allowance`. The error is the lamports the run spends beyond the trade.
///
/// With Δ the lamport change of the controlled set and `fee` the network fee
/// (already deducted in the post-balances), the run must satisfy
/// `Δ + fee + allowance ≥ legs.minimum_output − legs.input`. One inequality
/// covers buys, sales and token swaps alike.
pub fn conserve_lamports(
    outcome: &SimulationOutcome,
    wallet: &str,
    legs: NativeLegs,
    allowance: u64,
) -> Result<(), u64> {
    let (Some(pre), Some(post), Some(fee)) =
        (&outcome.pre_balances, &outcome.post_balances, outcome.fee)
    else {
        logger::debug(
            LogTag::Swap,
            "Simulation returned no lamport balances or fee; the lamport check is skipped",
        );
        return Ok(());
    };
    let Some((pre_tokens, post_tokens)) = token_balances(outcome) else {
        return Ok(());
    };
    let controlled: BTreeSet<usize> = std::iter::once(0)
        .chain(
            pre_tokens
                .iter()
                .chain(post_tokens)
                .filter(|balance| balance.owner.as_deref() == Some(wallet))
                .map(|balance| balance.account_index as usize),
        )
        .collect();
    let mut delta: i128 = 0;
    for index in controlled {
        let (Some(before), Some(after)) = (pre.get(index), post.get(index)) else {
            logger::debug(
                LogTag::Swap,
                &format!(
                    "Simulation token balance names account {index}, outside its lamport \
                     balances; the lamport check is skipped"
                ),
            );
            return Ok(());
        };
        delta += i128::from(*after) - i128::from(*before);
    }

    let spent_beyond_trade =
        i128::from(legs.minimum_output) - i128::from(legs.input) - (delta + i128::from(fee));
    if spent_beyond_trade > i128::from(allowance) {
        // Every term is a u64 lamport figure; a sum past u64 is reported as the
        // largest figure the refusal can carry.
        return Err(u64::try_from(spent_beyond_trade).unwrap_or(u64::MAX));
    }
    Ok(())
}
