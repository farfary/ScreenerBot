// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Whether a swap transaction a third party built may be signed with the
//! wallet's key, and the refusal when it may not.
//!
//! An aggregator builds the transaction on its own host and the wallet signs
//! it here. Slippage protects only the OUTPUT of that swap: it guarantees a
//! minimum number of tokens and nothing else. It is blind to what else the
//! transaction does with the wallet — a top-level transfer to any address, an
//! approval, an authority change, or lamports that leave the wallet alongside
//! the swap, such as rent for an account a venue keeps for itself, which only
//! that venue can ever close. The quote does not mention any of it, and the
//! simulation clears all of it, because it succeeds.
//!
//! Three layers answer the question before anything is signed, and none of
//! them knows any particular venue:
//!
//! 1. **The message** ([`message`]). Every top-level instruction must be one a
//!    swap needs: compute budget, the wallet's own associated token accounts,
//!    a wrap into the wallet's wrapped-SOL account, the platform fee, a tip,
//!    the token program's sync and close back to the wallet, and the quoting
//!    router's own programs. Anything else is refused. This layer needs no
//!    node and always runs.
//! 2. **The inner instructions** ([`inner`]) of the simulated run. Every
//!    lamport the wallet sends out is classified as recoverable rent for one
//!    of the wallet's OWN token accounts, rent locked in an account another
//!    program or another authority controls, or a payment to a plain address.
//!    Any approval, authority change or assignment of the wallet's accounts is
//!    a grant, refused whatever its size.
//! 3. **Conservation** ([`balances`]) over the wallet's own accounts in the
//!    simulated run: no mint the wallet holds may fall beyond the trade, the
//!    output must arrive, and the lamports the run takes must be explained by
//!    the trade plus the allowance.
//!
//! A build that spends the native asset is never signed without a simulation;
//! a sale is never trapped by a node outage, and proceeds on the message layer
//! alone. `chains.solana.swaps.cost_guard.enabled` and the sale's
//! slippage-widened allowance govern only the comparison of attributed costs,
//! what the inner-instruction assessment explains. The message, the grants
//! and conservation of everything the assessment does not explain always run,
//! with the standard allowance.
//!
//! **Stability rule for the cost comparison: only a node that ANSWERED may
//! block a trade on cost.** An owner lookup that fails or a label the
//! aggregator cannot name leaves the cost unattributed, and a node that omits
//! the balance fields or the owner of a token balance skips only the
//! conservation layer. An unreadable node is
//! not evidence of a cost.

mod balances;
mod error;
mod inner;
mod message;
#[cfg(test)]
mod tests;

pub use error::BuildRefusal;
pub use inner::{assess_inner_instructions, attribute_unresolved, SwapCostAssessment};

use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;

use crate::chains::solana::access::{AccountControl, MintLookup};
use crate::chains::solana::constants::USDC_MINT;
use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use crate::chains::solana::swaps::presend::SwapNode;
use crate::chains::solana::swaps::revenue;
use crate::chains::solana::{Error, Result};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::swaps::{NotSubmittedReason, Quote, SwapExecutionError};

/// The caller's tolerance, read from `chains.solana.swaps.cost_guard`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostGuardSettings {
    pub enabled: bool,
    pub max_extra_cost_pct: f64,
    pub always_allow_below_lamports: u64,
    pub retry_excluding_venue: bool,
}

impl CostGuardSettings {
    /// The live configuration.
    pub fn current() -> Self {
        with_config(|cfg| Self {
            enabled: cfg.chains.solana.swaps.cost_guard.enabled,
            max_extra_cost_pct: cfg.chains.solana.swaps.cost_guard.max_extra_cost_pct,
            always_allow_below_lamports: cfg
                .chains
                .solana
                .swaps
                .cost_guard
                .always_allow_below_lamports,
            retry_excluding_venue: cfg.chains.solana.swaps.cost_guard.retry_excluding_venue,
        })
    }
}

/// What a router's build may carry for one quote: bounds read from the
/// router's own configuration, the platform fee and the cost allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildExpectations {
    /// The compute-unit price the router is configured to pay, in
    /// micro-lamports. A build may not pay more.
    pub max_compute_unit_price: u64,
    /// The most the build may pay the fee wallet.
    pub fee_bound: u64,
    /// The most the build may spend outside the trade, in lamports.
    pub allowance: u64,
}

impl BuildExpectations {
    /// The expectations for `quote`. A router without registered expectations
    /// is refused: its builds cannot be bounded.
    pub fn for_quote(
        quote: &Quote,
        settings: &CostGuardSettings,
    ) -> std::result::Result<Self, BuildRefusal> {
        let max_compute_unit_price = with_config(|cfg| match quote.router_id.as_str() {
            "jupiter" => Some(cfg.chains.solana.swaps.jupiter.priority_fee_micro_lamports),
            "raptor" => Some(cfg.chains.solana.swaps.raptor.priority_fee_micro_lamports),
            _ => None,
        })
        .ok_or_else(|| BuildRefusal::RouterUnregistered {
            router: quote.router_id.clone(),
        })?;
        let quote_bounds = |error: Error| BuildRefusal::QuoteBounds {
            detail: error.to_string(),
        };
        let trade_value = trade_value_lamports(quote).map_err(quote_bounds)?;
        let fee_bound = quote
            .platform_fee_lamports
            .unwrap_or(0)
            .max(revenue::platform_fee_amount(trade_value));
        let allowance = trade_allowance_lamports(quote, settings).map_err(quote_bounds)?;
        Ok(Self {
            max_compute_unit_price,
            fee_bound,
            allowance,
        })
    }
}

/// The most a trade of `trade_value_lamports` may spend on accounts it will
/// never get back.
///
/// Two dials, deliberately: a PROPORTION, so the same venue deposit that is
/// absurd on a 0.005 SOL trade is acceptable on a large one, and a FLOOR, so
/// dust never blocks a swap. A trade whose value cannot be expressed in
/// lamports (neither leg is the native asset) has only the floor.
pub fn allowance_lamports(trade_value_lamports: u64, settings: &CostGuardSettings) -> u64 {
    let proportional = if settings.max_extra_cost_pct > 0.0 {
        (trade_value_lamports as f64 * settings.max_extra_cost_pct / 100.0).round() as u64
    } else {
        0
    };
    proportional.max(settings.always_allow_below_lamports)
}

/// The allowance for one quote.
///
/// A trade that spends the native asset gets [`allowance_lamports`] of its
/// value. A sale for the native asset gets the larger of that and the share of
/// its expected proceeds its own slippage already gives up, so a venue cost
/// refuses a sale only when it is out of proportion even to the slippage the
/// seller accepted.
pub fn trade_allowance_lamports(quote: &Quote, settings: &CostGuardSettings) -> Result<u64> {
    let trade_value = trade_value_lamports(quote)?;
    let standard = allowance_lamports(trade_value, settings);
    let adapter = crate::chains::adapter();
    if adapter.is_native_asset(&quote.input_mint) || !adapter.is_native_asset(&quote.output_mint) {
        return Ok(standard);
    }
    let slippage_share = u128::from(trade_value) * u128::from(quote.slippage_bps)
        / u128::from(revenue::BPS_DENOMINATOR);
    let slippage_share = u64::try_from(slippage_share).map_err(|_| Error::InstructionBuild {
        instruction: "swap cost guard",
        detail: "the slippage share of the proceeds exceeds the Solana u64 limit".to_owned(),
    })?;
    Ok(standard.max(slippage_share))
}

/// The trade's own size in lamports, when one of its legs is the native asset.
pub fn trade_value_lamports(quote: &Quote) -> Result<u64> {
    let adapter = crate::chains::adapter();
    if adapter.is_native_asset(&quote.input_mint) {
        u64::try_from(quote.input_amount).map_err(|_| Error::InstructionBuild {
            instruction: "swap cost guard",
            detail: "input amount exceeds the Solana u64 limit".to_owned(),
        })
    } else if adapter.is_native_asset(&quote.output_mint) {
        u64::try_from(quote.output_amount).map_err(|_| Error::InstructionBuild {
            instruction: "swap cost guard",
            detail: "output amount exceeds the Solana u64 limit".to_owned(),
        })
    } else {
        Ok(0)
    }
}

/// What the pre-send gate learned by simulating the build.
#[derive(Debug, Clone, Copy)]
pub enum Simulated<'a> {
    /// A node ran the build and it succeeded.
    Ran(&'a SimulationOutcome),
    /// No node could answer the simulation.
    Unavailable { detail: &'a str },
}

/// Why [`decide`] refused a build.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refusal {
    /// The build does something with the wallet a swap never needs.
    Unusable(BuildRefusal),
    /// Nothing simulated a build that spends the native asset.
    Unavailable(String),
    /// The run spends more outside the trade than the allowance. The address
    /// is the largest single charge, empty when none is attributed.
    Cost {
        extra: u64,
        allowance: u64,
        venue_address: String,
    },
}

/// Refuse a third-party build that the wallet must not sign.
///
/// The one decision point: the pre-send gate only reports what its
/// simulation saw. Every refusal is [`SwapExecutionError::NotSubmitted`] and
/// happens before anything is signed, so it costs no fee and the caller is
/// free to route the trade another way.
pub async fn check<N: SwapNode>(
    node: &N,
    router: &'static str,
    transaction: &VersionedTransaction,
    simulated: Simulated<'_>,
    quote: &Quote,
) -> crate::Result<()> {
    let settings = CostGuardSettings::current();
    let not_submitted = |reason| {
        crate::Error::Swaps(SwapExecutionError::NotSubmitted {
            router: router.to_owned(),
            reason,
        })
    };
    let output_fee = match simulated {
        Simulated::Ran(_) => output_transfer_fee(node, quote).await,
        Simulated::Unavailable { .. } => None,
    };
    match decide(
        router,
        transaction,
        simulated,
        quote,
        &settings,
        output_fee,
        |addresses| resolve_owners(node, addresses),
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(Refusal::Unusable(refusal)) => {
            let detail = refusal.to_string();
            logger::warning(
                LogTag::Swap,
                &format!("{router} transaction refused before signing: {detail}"),
            );
            Err(not_submitted(NotSubmittedReason::BuildUnusable { detail }))
        }
        Err(Refusal::Unavailable(detail)) => {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "{router} transaction refused: it spends SOL and no node could simulate it: \
                     {detail}"
                ),
            );
            Err(not_submitted(NotSubmittedReason::SimulationUnavailable {
                detail,
            }))
        }
        Err(Refusal::Cost {
            extra,
            allowance,
            venue_address,
        }) => {
            // Name the venue the way people and the aggregator both know it;
            // fall back to the address when the aggregator has no name for it.
            let venue = venue_display_name(&venue_address).await;
            crate::swaps::progress::report_swap_stage(crate::swaps::SwapStage::CostRejected {
                router: router.to_owned(),
                venue: venue.clone(),
                extra_lamports: extra,
            })
            .await;
            logger::warning(
                LogTag::Swap,
                &format!(
                    "{router} transaction refused: it would spend {:.6} SOL outside the trade, \
                     the largest share to {venue}, above the {:.6} SOL allowed for this trade",
                    crate::chains::solana::constants::lamports_to_sol(extra),
                    crate::chains::solana::constants::lamports_to_sol(allowance),
                ),
            );
            Err(not_submitted(NotSubmittedReason::CostExceeded {
                extra_raw: extra,
                venue,
                venue_address,
            }))
        }
    }
}

/// The verdict of [`check`], with the owner lookup supplied by the caller.
async fn decide<F, Fut>(
    router: &str,
    transaction: &VersionedTransaction,
    simulated: Simulated<'_>,
    quote: &Quote,
    settings: &CostGuardSettings,
    output_transfer_fee: Option<u64>,
    resolve: F,
) -> std::result::Result<(), Refusal>
where
    F: FnOnce(Vec<String>) -> Fut,
    Fut: Future<Output = Option<HashMap<String, AccountControl>>>,
{
    let expectations = BuildExpectations::for_quote(quote, settings).map_err(Refusal::Unusable)?;
    let loaded = match simulated {
        Simulated::Ran(outcome) => outcome.loaded_addresses.as_ref(),
        Simulated::Unavailable { .. } => None,
    };
    let keys = message::ResolvedKeys::new(&transaction.message, loaded);
    message::inspect(transaction, &keys, quote, &expectations).map_err(Refusal::Unusable)?;

    let outcome = match simulated {
        Simulated::Ran(outcome) => outcome,
        Simulated::Unavailable { detail } => {
            if crate::chains::adapter().is_native_asset(&quote.input_mint) {
                return Err(Refusal::Unavailable(detail.to_owned()));
            }
            logger::warning(
                LogTag::Swap,
                &format!("{router} preflight simulation unavailable, proceeding: {detail}"),
            );
            return Ok(());
        }
    };

    let wallet = quote.wallet_address.as_str();
    let mut assessment = assess_inner_instructions(wallet, &outcome.inner_instructions);
    if !assessment.grants.is_empty() {
        return Err(Refusal::Unusable(BuildRefusal::GrantsControl {
            grants: assessment.grants,
        }));
    }
    balances::conserve_tokens(outcome, quote, output_transfer_fee).map_err(Refusal::Unusable)?;

    let legs = balances::NativeLegs::of(quote).map_err(Refusal::Unusable)?;
    let trade_value = trade_value_lamports(quote).map_err(|error| {
        Refusal::Unusable(BuildRefusal::QuoteBounds {
            detail: error.to_string(),
        })
    })?;
    let standard_allowance = allowance_lamports(trade_value, settings);
    if settings.enabled && !assessment.unresolved.is_empty() {
        let addresses = assessment
            .unresolved
            .iter()
            .map(|(address, _)| address.clone())
            .collect();
        // An owner lookup that failed leaves these accounts unattributed: the
        // lamport conservation below still bounds what the run takes.
        if let Some(owners) = resolve(addresses).await {
            attribute_unresolved(&mut assessment, wallet, &owners);
        }
    }

    let extra = assessment.extra_cost_lamports().saturating_add(
        assessment
            .fee_wallet_lamports
            .saturating_sub(expectations.fee_bound),
    );
    let venue_address = assessment.largest_charge().unwrap_or_default().to_owned();
    let allowance = expectations.allowance;
    if settings.enabled && extra > allowance {
        return Err(Refusal::Cost {
            extra,
            allowance,
            venue_address,
        });
    }
    // The lamports the assessment explains are conceded here; the standard
    // allowance covers the rest. An account whose owner was not resolved is
    // still an explained outflow: an unanswered lookup never blocks a trade.
    // The sale's slippage share is not added: the lamport baseline is the
    // quoted minimum, which already concedes it.
    let unresolved_lamports = assessment
        .unresolved
        .iter()
        .fold(0u64, |total, (_, lamports)| total.saturating_add(*lamports));
    let conservation = standard_allowance.saturating_add(
        assessment
            .extra_cost_lamports()
            .saturating_add(assessment.fee_wallet_lamports)
            .saturating_add(unresolved_lamports),
    );
    balances::conserve_lamports(outcome, wallet, legs, conservation).map_err(|extra| {
        Refusal::Cost {
            extra,
            allowance: conservation,
            venue_address,
        }
    })?;

    if settings.enabled && extra > 0 {
        logger::info(
            LogTag::Swap,
            &format!(
                "{router} transaction spends {:.6} SOL outside the trade, within the {:.6} SOL allowed",
                crate::chains::solana::constants::lamports_to_sol(extra),
                crate::chains::solana::constants::lamports_to_sol(allowance),
            ),
        );
    }
    Ok(())
}

/// Transfer fee the output mint charges on the quoted minimum, when its
/// Token-2022 account and current epoch can be read from `node`. An output
/// that is native, USDC or not a Token-2022 mint, and any read that fails,
/// leave the fee unsubtracted.
async fn output_transfer_fee<N: SwapNode>(node: &N, quote: &Quote) -> Option<u64> {
    if crate::chains::adapter().is_native_asset(&quote.output_mint)
        || quote.output_mint == USDC_MINT
    {
        return None;
    }
    if let Err(error) = Pubkey::from_str(&quote.output_mint) {
        logger::debug(
            LogTag::Swap,
            &format!("Could not parse output mint for transfer-fee check: {error}"),
        );
        return None;
    }
    let schedule = match node
        .mint_facts(std::slice::from_ref(&quote.output_mint))
        .await
    {
        Ok(lookups) => lookups
            .into_iter()
            .next()
            .and_then(MintLookup::into_facts)
            .and_then(|facts| facts.transfer_fee)?,
        Err(error) => {
            logger::debug(
                LogTag::Swap,
                &format!("Could not read output mint for transfer-fee check: {error}"),
            );
            return None;
        }
    };
    let epoch = match node.epoch().await {
        Ok(epoch) => epoch,
        Err(error) => {
            logger::debug(
                LogTag::Swap,
                &format!("Could not read epoch for output transfer-fee check: {error}"),
            );
            return None;
        }
    };
    let minimum = match u64::try_from(quote.minimum_output_amount) {
        Ok(minimum) => minimum,
        Err(error) => {
            logger::debug(
                LogTag::Swap,
                &format!("Could not convert output minimum for transfer-fee check: {error}"),
            );
            return None;
        }
    };
    let fee = schedule.fee_at(epoch, minimum);
    if fee.is_none() {
        logger::debug(LogTag::Swap, "Could not calculate output transfer fee");
    }
    fee
}

/// How a venue is named to the user: the aggregator's own label when it has
/// one, otherwise the address, shortened so it fits a toast.
async fn venue_display_name(program: &str) -> String {
    if program.is_empty() {
        return "a third-party program".to_owned();
    }
    if let Some(label) =
        crate::chains::solana::swaps::routers::venue_label_for_program(program).await
    {
        return label;
    }
    match program.len() > 12 {
        true => format!("{}…{}", &program[..4], &program[program.len() - 4..]),
        false => program.to_owned(),
    }
}

/// Who controls each of the funded accounts the instructions left
/// unattributed, `None` when the read failed. An account the node answered
/// as missing is absent from the map: it does not exist.
async fn resolve_owners<N: SwapNode>(
    node: &N,
    addresses: Vec<String>,
) -> Option<HashMap<String, AccountControl>> {
    let addresses: Vec<String> = addresses
        .into_iter()
        .filter(|address| Pubkey::from_str(address).is_ok())
        .collect();
    if addresses.is_empty() {
        return Some(HashMap::new());
    }
    match node.account_controls(&addresses).await {
        Ok(controls) => Some(controls),
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("Could not read the owners of accounts a swap funds: {e}"),
            );
            None
        }
    }
}
