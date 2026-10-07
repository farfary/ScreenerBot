// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Turning a [`SwapPlan`] into a landed, verified transaction.
//!
//! The step order is the money-safety contract of the engine:
//!
//! 1. preflight the wallet's balance — a swap it cannot afford never reaches RPC
//!    for a build it was always going to lose;
//! 2. build and sign — nothing is on the network yet;
//! 3. simulate — a mis-built instruction is rejected here for free, and the
//!    MEASURED compute cost tightens the limit the transaction actually requests;
//! 4. send — the point of no return; from here the outcome carries the
//!    signature, whatever the send itself answered;
//! 5. settle — poll for a landed signature, re-broadcasting the same signed
//!    transaction against network drops, until either it lands, it fails on
//!    chain at `Confirmed`, or its blockhash provably expires unseen (steps 4
//!    and 5 are the pre-send gate's `send_and_settle`);
//! 6. verify — read back what actually arrived.
//!
//! Anything that fails at steps 1-3 returns an error whose
//! [`DirectSwapError::submitted`] is false, and is safe to retry.
//!
//! Step 5 is why this module exists in its current shape. A transaction is
//! provably DEAD once the block height passes the `lastValidBlockHeight` of its
//! blockhash and the signature still has not been seen: the runtime rejects any
//! blockhash older than 151 blocks, so that transaction can never be included
//! after that point, by any node, ever. That is the ONE post-send outcome that is
//! definitively safe to retry — [`DirectSwapError::BlockhashExpired`], whose
//! `submitted()` is `false`. Every other post-send failure keeps `submitted()`
//! `true`, including [`DirectSwapError::ConfirmationTimeout`]: a timeout while the
//! blockhash is STILL valid means the transaction may yet land, and retrying it
//! risks buying the position twice.

use super::error::{DirectSwapError, DirectSwapResult};
use super::plan::SwapPlan;
use super::verify::{receipt_from_transaction, Receipt};
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    commitment_config::CommitmentLevel,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use crate::chains::solana::swaps::presend::{self, Settled, SwapNode, Verdict};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::swaps::NotSubmittedReason;
use std::time::{Duration, Instant};

/// A completed direct swap.
#[derive(Debug, Clone)]
pub struct DirectSwapOutcome {
    /// Transaction signature.
    pub signature: String,
    /// Amount of the input mint the wallet gave up, platform fee included.
    pub amount_in: u64,
    /// What the wallet received, per [`Receipt`].
    pub receipt: Receipt,
    /// Platform fee collected, in raw units of the fee mint.
    pub platform_fee: u64,
    /// The mint [`Self::platform_fee`] is denominated in, when a fee was
    /// collected at all. Carried alongside the amount because the fee rides
    /// whichever leg is a reference mint: on a TOKEN/USDC pair it is USDC, and a
    /// caller that assumes lamports would report a six-decimal figure as a
    /// nine-decimal one.
    pub platform_fee_mint: Option<Pubkey>,
    /// Wall-clock time from build to verified, in milliseconds.
    pub duration_ms: u64,
}

impl DirectSwapOutcome {
    /// The platform fee in LAMPORTS, or zero when it was collected in something
    /// else. Converting a USDC fee would need a price this module does not have,
    /// and a wrong number in a fee ledger is worse than an absent one.
    pub fn platform_fee_lamports(&self) -> u64 {
        match self.platform_fee_mint {
            Some(mint) if super::intent::is_wsol(&mint) => self.platform_fee,
            _ => 0,
        }
    }
}

/// How many times to re-read a confirmed transaction before giving up on an
/// exact receipt.
const RECEIPT_READ_ATTEMPTS: usize = 6;

/// Delay between those attempts.
const RECEIPT_READ_DELAY: Duration = Duration::from_millis(600);

/// Cached rent-exempt minimum for a 165-byte SPL token account. Fixed by the
/// runtime, so it is read once rather than once per swap.
static ATA_RENT_LAMPORTS: tokio::sync::OnceCell<u64> = tokio::sync::OnceCell::const_new();

/// Extra headroom, in percent, on top of the network fee the plan's own compute
/// budget determines. Deliberately conservative: a preflight that
/// under-estimates would let a swap through that then fails on chain, which is
/// the exact failure mode this preflight exists to avoid.
const NETWORK_FEE_HEADROOM_PCT: u64 = 20;

/// The lamports this specific plan will hand the network, with headroom.
///
/// This was a flat 10,000 constant, which is BELOW what every venue actually
/// pays at the default priority-fee price -- 10,850 for the cheapest and 31,000
/// for Meteora DLMM -- because Solana bills the prioritization fee on the
/// compute-unit LIMIT the transaction requests, and the config allows a price
/// 200x the default. Sizing it off the plan's own compute-budget instructions is
/// exact rather than a guess.
fn network_fee_cushion_lamports(plan: &SwapPlan) -> u64 {
    super::compute::network_fee_lamports(&plan.instructions)
        .saturating_mul(100 + NETWORK_FEE_HEADROOM_PCT)
        .saturating_div(100)
}

/// SPL token account size, used to size the ATA rent-exemption read.
const TOKEN_ACCOUNT_SIZE: usize = 165;

async fn ata_rent_lamports() -> DirectSwapResult<u64> {
    ATA_RENT_LAMPORTS
        .get_or_try_init(|| async {
            get_rpc_client()
                .get_minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_SIZE)
                .await
                .map_err(|e| DirectSwapError::NodeUnavailable {
                    operation: "getMinimumBalanceForRentExemption",
                    detail: format!("could not read the ATA rent-exempt minimum: {e}"),
                })
        })
        .await
        .copied()
}

/// Refuse a swap the wallet plainly cannot afford before it costs an RPC round
/// trip on a build that was always going to fail.
///
/// This is a preflight, not the safety mechanism -- `min_out` inside the swap
/// instruction is still what protects the trade once it is submitted. What this
/// catches is the case simulation reports as an opaque `SimulationRejected`
/// today: the wallet simply does not hold enough of the input mint, or enough
/// native SOL to cover rent and fees, and neither fact says anything about
/// whether the TOKEN is tradable.
pub async fn preflight_balance(plan: &SwapPlan, owner: &Pubkey) -> DirectSwapResult<()> {
    let rpc = get_rpc_client();
    let accounts = rpc
        .get_multiple_accounts(&[*owner, plan.input_account, plan.output_account])
        .await
        .map_err(|e| DirectSwapError::AccountUnavailable {
            address: *owner,
            detail: format!("could not read wallet balances for preflight: {e}"),
        })?;

    // The outer `Option` is the RESPONSE being short -- a malformed or
    // truncated `getMultipleAccounts` reply, which says nothing about the
    // wallet and must not be read as "zero balance". The inner `Option` is the
    // account genuinely not existing on chain, which for a lamport balance IS
    // zero -- a wallet that has never been funded has no account and no SOL.
    let owner_lamports = accounts
        .first()
        .ok_or_else(|| DirectSwapError::AccountUnavailable {
            address: *owner,
            detail: "getMultipleAccounts returned no entry for the preflight balance batch"
                .to_owned(),
        })?
        .as_ref()
        .map(|a| a.lamports)
        .unwrap_or(0);
    let input_account = accounts
        .get(1)
        .ok_or_else(|| DirectSwapError::AccountUnavailable {
            address: plan.input_account,
            detail: "getMultipleAccounts returned no input token-account entry".to_owned(),
        })?
        .as_ref();
    let output_exists = accounts
        .get(2)
        .ok_or_else(|| DirectSwapError::AccountUnavailable {
            address: plan.output_account,
            detail: "getMultipleAccounts returned no output token-account entry".to_owned(),
        })?
        .is_some();

    // Both ATA creations are idempotent, so only the ones that are genuinely
    // MISSING will actually draw rent. Charging for both unconditionally would
    // refuse swaps a wallet can plainly afford: two rent-exempt minimums is
    // about 0.004 SOL, which is larger than many entry sizes.
    let missing_accounts = u64::from(input_account.is_none()) + u64::from(!output_exists);
    let rent_cushion = if missing_accounts == 0 {
        0
    } else {
        ata_rent_lamports().await?.saturating_mul(missing_accounts)
    };

    let network_cushion = network_fee_cushion_lamports(plan);

    if super::intent::is_wsol(&plan.quote.input_mint) {
        let required = plan
            .quote
            .amount_in
            .saturating_add(rent_cushion)
            .saturating_add(network_cushion);
        if owner_lamports < required {
            return Err(DirectSwapError::InsufficientBalance {
                mint: plan.quote.input_mint,
                required,
                available: owner_lamports,
            });
        }
        return Ok(());
    }

    let token_balance = match input_account {
        None => 0,
        Some(account) => crate::chains::solana::layout::token_account_amount(&account.data)
            .ok_or_else(|| DirectSwapError::AccountUnavailable {
                address: plan.input_account,
                detail: "source token-account data is malformed".to_owned(),
            })?,
    };
    if token_balance < plan.quote.amount_in {
        return Err(DirectSwapError::InsufficientBalance {
            mint: plan.quote.input_mint,
            required: plan.quote.amount_in,
            available: token_balance,
        });
    }

    let required_lamports = rent_cushion.saturating_add(network_cushion);
    if owner_lamports < required_lamports {
        return Err(DirectSwapError::InsufficientBalance {
            mint: super::intent::wsol_mint(),
            required: required_lamports,
            available: owner_lamports,
        });
    }
    Ok(())
}

/// Read back what a CONFIRMED swap delivered.
///
/// The read asks at `confirmed`, the same commitment the swap was confirmed at.
/// Asking without a commitment lets the node apply its default of `finalized`,
/// roughly thirteen seconds behind the tip — the first real mainnet swap through
/// this engine landed successfully and was then reported as "not found" for
/// exactly that reason. Even at the right commitment, indexing trails
/// confirmation slightly, so the read is retried.
///
/// So the read is retried, and if it still cannot be had the swap is NOT failed.
/// Confirmation already proves success: the settle loop only returns success once
/// the signature status carries no error, and the pool programme itself refused
/// to return less than `min_out`. What is lost is only the exact amount, so the
/// receipt falls back to the guaranteed minimum and marks itself inexact rather
/// than inventing precision it does not have.
async fn read_receipt(
    signature: &str,
    owner: &crate::chains::solana::solana_sdk::pubkey::Pubkey,
    plan: &SwapPlan,
) -> DirectSwapResult<Receipt> {
    let rpc = get_rpc_client();
    let mut last_error = String::new();

    for attempt in 0..RECEIPT_READ_ATTEMPTS {
        match rpc
            .get_transaction_details_with_commitment(signature, CommitmentLevel::Confirmed)
            .await
        {
            Ok(details) => return receipt_from_transaction(signature, owner, plan, &details),
            Err(e) => last_error = e.to_string(),
        }
        if attempt + 1 < RECEIPT_READ_ATTEMPTS {
            tokio::time::sleep(RECEIPT_READ_DELAY).await;
        }
    }

    logger::warning(
        LogTag::Swap,
        &format!(
            "Swap {signature} confirmed but could not be read back after \
             {RECEIPT_READ_ATTEMPTS} attempts ({last_error}); reporting the \
             chain-guaranteed minimum of {} raw units",
            plan.quote.min_net_out
        ),
    );
    Ok(Receipt {
        received: plan.quote.min_net_out,
        exact: false,
        network_fee_lamports: 0,
        slot: 0,
    })
}

/// Run `plan` against a node WITHOUT submitting it.
///
/// The transaction is built unsigned with `owner` as the fee payer, which is
/// enough because `simulate_transaction` sends `sigVerify: false`: the node
/// executes the instructions against real account state and real balances but
/// nothing is signed and nothing lands.
///
/// This is the strongest check available for free. It exercises the exact
/// account list, the exact instruction data and the exact `min_out` that a real
/// swap would carry, against the live pool — so a wrong account order, a bad
/// discriminator, an under-sized compute budget or an unsatisfiable `min_out`
/// all surface here rather than costing a fee.
pub async fn simulate_plan(
    plan: &SwapPlan,
    owner: &Pubkey,
) -> DirectSwapResult<crate::chains::solana::rpc::types::SimulationOutcome> {
    let rpc = get_rpc_client();
    let blockhash =
        rpc.get_latest_blockhash()
            .await
            .map_err(|e| DirectSwapError::NodeUnavailable {
                operation: "getLatestBlockhash",
                detail: format!("no recent blockhash: {e}"),
            })?;

    let mut transaction = Transaction::new_with_payer(&plan.instructions, Some(owner));
    transaction.message.recent_blockhash = blockhash;
    // An unsigned transaction still needs a signature slot per required signer or
    // it fails to deserialise on the node.
    transaction.signatures =
        vec![Signature::default(); transaction.message.header.num_required_signatures as usize];

    match presend::gate(&VersionedTransaction::from(transaction)).await {
        Verdict::Cleared(outcome) | Verdict::Failed(outcome) => Ok(outcome),
        Verdict::Refused(reason) => Err(reason.into()),
        Verdict::NodeUnavailable { detail } => Err(DirectSwapError::SimulationUnavailable {
            detail: format!("simulation could not be run: {detail}"),
        }),
    }
}

/// Build, sign, simulate, send, settle and verify `plan`.
pub async fn execute_plan(
    plan: &SwapPlan,
    keypair: &Keypair,
) -> DirectSwapResult<DirectSwapOutcome> {
    let started = Instant::now();
    let rpc = get_rpc_client();
    let owner = keypair.pubkey();

    preflight_balance(plan, &owner).await?;

    // Asking at `Confirmed` rather than the bare `get_latest_blockhash` (which
    // asks at `Finalized`, ~32 slots / ~13s behind the tip) matters here: every
    // second of that gap is a second of the 151-block validity window this
    // transaction will never get to spend, and the settle loop depends on that
    // window being as long as the runtime actually allows.
    let (blockhash, last_valid_block_height) = rpc
        .get_latest_blockhash_with_commitment(CommitmentLevel::Confirmed)
        .await
        .map_err(|e| DirectSwapError::NodeUnavailable {
            operation: "getLatestBlockhash",
            detail: format!("no recent blockhash: {e}"),
        })?;

    let mut instructions = plan.instructions.clone();
    let mut transaction = VersionedTransaction::from(Transaction::new_signed_with_payer(
        &instructions,
        Some(&owner),
        &[keypair],
        blockhash,
    ));

    // The gate measures the transaction against the packet limit, then
    // simulates it. Simulation is not optional here: it is the last point at
    // which a mis-built instruction, a wrong account or an unaffordable swap
    // costs nothing, and its measured compute is what keeps the prioritization
    // fee honest, so a node that cannot simulate stops the swap.
    let outcome = match presend::gate(&transaction).await {
        Verdict::Cleared(outcome) => outcome,
        Verdict::Failed(outcome) => {
            return Err(DirectSwapError::SimulationRejected {
                detail: outcome.failure_detail(),
                logs: outcome.logs,
            });
        }
        Verdict::Refused(reason) => return Err(reason.into()),
        Verdict::NodeUnavailable { detail } => {
            return Err(DirectSwapError::SimulationUnavailable {
                detail: format!("simulation could not be run: {detail}"),
            });
        }
    };
    if let Some(units) = outcome.units_consumed {
        logger::debug(
            LogTag::System,
            &format!(
                "Direct swap simulation consumed {units} CU against a {} CU venue estimate",
                plan.venue_compute_units
            ),
        );

        // The prioritization fee is charged on the LIMIT the transaction
        // requests, not on what it actually consumes. The venue's static
        // estimate carries 30% headroom for the worst case; simulation just
        // measured the real cost for THIS swap, so a tighter limit sized off
        // that measurement stops paying for compute units the transaction
        // never uses. Only ever tighten, never raise: simulation runs
        // against slightly older state, and a real execution can cost more.
        let measured_limit = super::compute::compute_unit_limit_from_measured(units);
        if let Some(requested_limit) = super::compute::requested_compute_unit_limit(&instructions) {
            if measured_limit < requested_limit {
                instructions[0] = crate::chains::solana::solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(measured_limit);
                transaction = VersionedTransaction::from(Transaction::new_signed_with_payer(
                    &instructions,
                    Some(&owner),
                    &[keypair],
                    blockhash,
                ));
            }
        }
    }

    let timeout = Duration::from_secs(with_config(|cfg| {
        cfg.chains.solana.swaps.direct.confirmation_timeout_secs
    }));
    let signature_str = send_settled(rpc, &transaction, last_valid_block_height, timeout).await?;

    let receipt = read_receipt(&signature_str, &owner, plan).await?;

    Ok(DirectSwapOutcome {
        signature: signature_str,
        amount_in: plan.quote.amount_in,
        receipt,
        platform_fee: plan.quote.fee.amount,
        platform_fee_mint: plan.quote.fee.mint,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Send `transaction` and read the settle verdict in the engine's vocabulary.
///
/// Only a send the node refused as a request is [`DirectSwapError::SubmitFailed`];
/// every other send outcome is settled by the transaction's own signature, so a
/// send that may have been delivered is never mistaken for one that was not.
async fn send_settled<N: SwapNode>(
    node: &N,
    transaction: &VersionedTransaction,
    last_valid_block_height: u64,
    timeout: Duration,
) -> DirectSwapResult<String> {
    let (signature, settled) =
        presend::send_and_settle(node, transaction, Some(last_valid_block_height), timeout)
            .await
            .map_err(|reason| match reason {
                NotSubmittedReason::RequestRejected { detail } => {
                    DirectSwapError::SubmitFailed { detail }
                }
                other => DirectSwapError::from(other),
            })?;
    let signature = signature.to_string();
    match settled {
        Settled::Landed => Ok(signature),
        Settled::Reverted { detail } => {
            Err(DirectSwapError::TransactionFailed { signature, detail })
        }
        Settled::Expired {
            last_valid_block_height,
            current_block_height,
        } => Err(DirectSwapError::BlockhashExpired {
            signature,
            last_valid_block_height,
            current_block_height,
        }),
        Settled::Unsettled { waited_ms } => Err(DirectSwapError::ConfirmationTimeout {
            signature,
            waited_ms,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::solana_sdk::{hash::Hash, instruction::Instruction};
    use crate::chains::solana::solana_transaction_status::TransactionConfirmationStatus;
    use crate::chains::solana::swaps::presend::scripted_tests::{status, ScriptedNode, SendAnswer};
    use crate::rpc::RpcError;

    const LAST_VALID: u64 = 1_000;

    fn signed() -> VersionedTransaction {
        let payer = Keypair::new();
        VersionedTransaction::from(Transaction::new_signed_with_payer(
            &[Instruction {
                program_id: Pubkey::new_unique(),
                accounts: vec![],
                data: vec![1],
            }],
            Some(&payer.pubkey()),
            &[&payer],
            Hash::new_unique(),
        ))
    }

    /// A send that timed out after the request left the machine may have
    /// been delivered: the engine settles the transaction's own signature and
    /// never reports it as a submission that safely failed.
    #[tokio::test(start_paused = true)]
    async fn an_unproven_send_is_settled_by_its_signature_and_never_falls_back() {
        let transaction = signed();
        let expected = transaction.signatures[0].to_string();

        let pending = ScriptedNode::new(0, vec![SendAnswer::TimedOut], vec![Ok(None)], LAST_VALID);
        let error = send_settled(&pending, &transaction, LAST_VALID, Duration::from_secs(5))
            .await
            .expect_err("a signature nobody can see has not landed");
        assert!(matches!(error, DirectSwapError::ConfirmationTimeout { .. }));
        assert_eq!(error.settled_signature(), Some(expected.as_str()));
        assert!(error.submitted());
        assert!(!error.safe_to_fallback());

        let landed = ScriptedNode::new(
            0,
            vec![SendAnswer::TimedOut],
            vec![
                Ok(None),
                Ok(Some(status(TransactionConfirmationStatus::Confirmed, None))),
            ],
            LAST_VALID,
        );
        assert_eq!(
            send_settled(&landed, &transaction, LAST_VALID, Duration::from_secs(5))
                .await
                .expect("a delivered send that confirms is a swap"),
            expected
        );
    }

    /// Only chain evidence ends an unproven send as safe to send again: a
    /// node that answered and did not know the signature once the blockhash
    /// expired.
    #[tokio::test(start_paused = true)]
    async fn an_unproven_send_whose_blockhash_expired_unseen_is_the_one_safe_retry() {
        let transaction = signed();
        let node = ScriptedNode::new(
            0,
            vec![SendAnswer::TimedOut],
            vec![Ok(None)],
            LAST_VALID + 1,
        );
        let error = send_settled(&node, &transaction, LAST_VALID, Duration::from_secs(60))
            .await
            .expect_err("an expired transaction never landed");
        assert!(matches!(error, DirectSwapError::BlockhashExpired { .. }));
        assert_eq!(
            error.signature(),
            Some(transaction.signatures[0].to_string().as_str())
        );
        assert!(error.safe_to_fallback());

        let unreadable = ScriptedNode::new(
            0,
            vec![SendAnswer::TimedOut],
            vec![Err(crate::Error::Rpc(RpcError::Network {
                message: "reset".to_owned(),
                is_timeout: false,
            }))],
            LAST_VALID + 1,
        );
        let error = send_settled(
            &unreadable,
            &transaction,
            LAST_VALID,
            Duration::from_secs(30),
        )
        .await
        .expect_err("an unreadable node proves nothing");
        assert!(
            matches!(error, DirectSwapError::ConfirmationTimeout { .. }),
            "expiry without an answering node is not evidence: {error}"
        );
    }

    /// The one send outcome that is safe to route elsewhere: the node the
    /// request reached refused the request itself.
    #[tokio::test(start_paused = true)]
    async fn a_send_the_node_refused_as_a_request_is_a_safe_submit_failure() {
        let node = ScriptedNode::new(
            0,
            vec![SendAnswer::Fails(crate::Error::Rpc(
                RpcError::ProviderError {
                    code: -32602,
                    message: "invalid transaction".to_owned(),
                    data: None,
                },
            ))],
            vec![Ok(None)],
            LAST_VALID,
        );
        let error = send_settled(&node, &signed(), LAST_VALID, Duration::from_secs(5))
            .await
            .expect_err("a refused send is a failure");
        assert!(matches!(error, DirectSwapError::SubmitFailed { .. }));
        assert!(error.safe_to_fallback());
        assert_eq!(
            node.sent().len(),
            1,
            "nothing is re-broadcast after a refusal"
        );
    }

    #[test]
    fn the_requested_compute_limit_is_read_back_from_instruction_zero() {
        crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
        let ixs = super::super::compute::compute_budget_instructions(200_000);
        assert_eq!(
            super::super::compute::requested_compute_unit_limit(&ixs),
            Some(260_000)
        );
    }

    #[test]
    fn a_non_compute_budget_instruction_at_index_zero_is_not_misread() {
        let bogus = crate::chains::solana::solana_sdk::instruction::Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![9, 9, 9],
        };
        assert_eq!(
            super::super::compute::requested_compute_unit_limit(&[bogus]),
            None
        );
    }

    #[test]
    fn measured_tightening_only_ever_lowers_the_requested_limit() {
        let requested = 260_000u32;
        let measured_low = super::super::compute::compute_unit_limit_from_measured(50_000);
        let measured_high = super::super::compute::compute_unit_limit_from_measured(400_000);
        assert!(
            measured_low < requested,
            "a swap that used far less than requested should tighten"
        );
        assert!(
            measured_high > requested,
            "sanity: this measured value is deliberately above what was requested, \
             and execute_plan's own `if measured_limit < requested_limit` guard is \
             what stops it ever being applied"
        );
    }
}
