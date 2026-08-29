//! Turning a [`SwapPlan`] into a landed, verified transaction.
//!
//! The step order is the money-safety contract of the engine:
//!
//! 1. preflight the wallet's balance — a swap it cannot afford never reaches RPC
//!    for a build it was always going to lose;
//! 2. build and sign — nothing is on the network yet;
//! 3. simulate — a mis-built instruction is rejected here for free, and the
//!    MEASURED compute cost tightens the limit the transaction actually requests;
//! 4. send — the point of no return;
//! 5. settle — poll for a landed signature, re-broadcasting the same signed
//!    transaction against network drops, until either it lands, it fails on
//!    chain, or its blockhash provably expires unseen;
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
use crate::chains::solana::rpc::client::RpcClient;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    commitment_config::CommitmentLevel,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
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

/// How often the settle loop polls `getSignatureStatuses`.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// How often the settle loop re-sends the same signed transaction. The signature
/// is identical every time, so a duplicate landing is impossible: this only
/// fights the network having dropped the earlier copy.
const REBROADCAST_INTERVAL: Duration = Duration::from_secs(2);

/// How often the settle loop checks whether the blockhash has expired.
const BLOCK_HEIGHT_CHECK_INTERVAL: Duration = Duration::from_secs(4);

/// Extra status reads, spaced a second apart, once the blockhash is seen to have
/// expired -- a transaction can land in the very last valid block and take a
/// moment to index.
const POST_EXPIRY_STATUS_ATTEMPTS: usize = 2;
const POST_EXPIRY_STATUS_DELAY: Duration = Duration::from_secs(1);

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
    let input_account = accounts.get(1).and_then(|a| a.as_ref());
    let output_exists = accounts.get(2).and_then(|a| a.as_ref()).is_some();

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

    let token_balance = input_account
        .map(|a| super::venues::layout::token_account_amount(&a.data).unwrap_or(0))
        .unwrap_or(0);
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

    rpc.simulate_transaction(&VersionedTransaction::from(transaction))
        .await
        .map_err(|e| DirectSwapError::SimulationUnavailable {
            detail: format!("simulation could not be run: {e}"),
        })
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
    // transaction will never get to spend, and the settle loop below depends on
    // that window being as long as the runtime actually allows.
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

    if with_config(|cfg| cfg.swaps.direct.simulate_before_send) {
        let outcome = rpc.simulate_transaction(&transaction).await.map_err(|e| {
            DirectSwapError::SimulationUnavailable {
                detail: format!("simulation could not be run: {e}"),
            }
        })?;
        if !outcome.succeeded() {
            return Err(DirectSwapError::SimulationRejected {
                detail: outcome.failure_detail(),
                logs: outcome.logs,
            });
        }
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
            if let Some(requested_limit) =
                super::compute::requested_compute_unit_limit(&instructions)
            {
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
    }

    let signature =
        rpc.send_transaction(&transaction)
            .await
            .map_err(|e| DirectSwapError::SubmitFailed {
                detail: e.to_string(),
            })?;
    let signature_str = signature.to_string();

    let timeout = Duration::from_secs(with_config(|cfg| {
        cfg.swaps.direct.confirmation_timeout_secs
    }));

    settle(
        rpc,
        &transaction,
        &signature,
        &signature_str,
        last_valid_block_height,
        timeout,
    )
    .await?;

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

/// Read the `SetComputeUnitLimit` value plan.rs guarantees is instruction index
/// zero, so the measured-usage tightening in `execute_plan` knows what it would
/// be replacing.

/// Terminal outcomes the settle loop can reach from one signature-status read.
/// Kept as a pure function of the SDK's own status/height types so the
/// blockhash-expiry decision can be unit tested without a live node.
fn status_outcome(
    status: &crate::chains::solana::solana_transaction_status::TransactionStatus,
    signature_str: &str,
) -> Option<DirectSwapResult<()>> {
    use crate::chains::solana::solana_transaction_status::TransactionConfirmationStatus;

    if let Some(err) = &status.err {
        return Some(Err(DirectSwapError::TransactionFailed {
            signature: signature_str.to_owned(),
            detail: err.to_string(),
        }));
    }
    match status.confirmation_status() {
        TransactionConfirmationStatus::Confirmed | TransactionConfirmationStatus::Finalized => {
            Some(Ok(()))
        }
        TransactionConfirmationStatus::Processed => None,
    }
}

/// Whether a transaction whose signature has not been seen is now provably dead:
/// the current block height has passed the last block its blockhash was valid
/// for. Extracted as a pure predicate so the settle loop's core decision is unit
/// tested without a live node.
fn blockhash_has_expired(current_block_height: u64, last_valid_block_height: u64) -> bool {
    current_block_height > last_valid_block_height
}

/// Poll for a landed signature, re-broadcasting the same signed transaction
/// against network drops, until it lands, fails on chain, or its blockhash
/// provably expires with the signature still unseen.
async fn settle(
    rpc: &'static RpcClient,
    transaction: &VersionedTransaction,
    signature: &Signature,
    signature_str: &str,
    last_valid_block_height: u64,
    timeout: Duration,
) -> DirectSwapResult<()> {
    let deadline = Instant::now() + timeout;
    let mut last_rebroadcast = Instant::now();
    let mut last_height_check = Instant::now();

    loop {
        if let Poll::Settled(outcome) = poll_status_once(rpc, signature, signature_str).await {
            return outcome;
        }

        if Instant::now() >= deadline {
            return Err(DirectSwapError::ConfirmationTimeout {
                signature: signature_str.to_owned(),
                waited_ms: timeout.as_millis() as u64,
            });
        }

        if last_rebroadcast.elapsed() >= REBROADCAST_INTERVAL {
            last_rebroadcast = Instant::now();
            match rpc.send_transaction(transaction).await {
                Ok(_) => logger::info(
                    LogTag::Swap,
                    &format!(
                        "Re-broadcast {signature_str}: the network may have dropped the earlier copy"
                    ),
                ),
                Err(e) => logger::debug(
                    LogTag::Swap,
                    &format!(
                        "Re-broadcast of {signature_str} was not accepted this round \
                         (an 'already processed' response is a good sign, not a failure): {e}"
                    ),
                ),
            }
        }

        if last_height_check.elapsed() >= BLOCK_HEIGHT_CHECK_INTERVAL {
            last_height_check = Instant::now();
            if let Ok(height) = rpc.get_block_height().await {
                if blockhash_has_expired(height, last_valid_block_height) {
                    // A transaction can land in the very last valid block and
                    // take a moment to index -- give it two more chances before
                    // declaring it dead.
                    //
                    // Declaring death requires POSITIVE evidence of absence: a
                    // node that answered and did not know the signature. If
                    // every read in this window was unreadable, the expiry
                    // proves nothing we can act on, and the loop keeps waiting
                    // for the outer timeout rather than telling the caller a
                    // possibly-landed swap is safe to retry.
                    let mut confirmed_absent = false;
                    for _ in 0..POST_EXPIRY_STATUS_ATTEMPTS {
                        tokio::time::sleep(POST_EXPIRY_STATUS_DELAY).await;
                        match poll_status_once(rpc, signature, signature_str).await {
                            Poll::Settled(outcome) => return outcome,
                            Poll::Pending => confirmed_absent = true,
                            Poll::Unreadable => {}
                        }
                    }
                    if !confirmed_absent {
                        logger::warning(
                            LogTag::Swap,
                            &format!(
                                "Blockhash for {signature_str} expired, but no node could be \
                                 read to confirm the signature is absent; treating the outcome \
                                 as unknown rather than retryable"
                            ),
                        );
                        continue;
                    }
                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "Transaction {signature_str} is dead: blockhash expired at block \
                             {last_valid_block_height}, current block is {height}, and the \
                             signature was never seen. Safe to retry."
                        ),
                    );
                    return Err(DirectSwapError::BlockhashExpired {
                        signature: signature_str.to_owned(),
                        last_valid_block_height,
                        current_block_height: height,
                    });
                }
            }
        }

        tokio::time::sleep(STATUS_POLL_INTERVAL).await;
    }
}

/// What one signature-status read told the settle loop.
enum Poll {
    /// The transaction reached a terminal state.
    Settled(DirectSwapResult<()>),
    /// The node answered, and the signature is not yet in a terminal state.
    /// Absence here is EVIDENCE — it is what the blockhash-expiry verdict rests
    /// on.
    Pending,
    /// The node could not be asked. This is not evidence of anything: the
    /// transaction may be sitting confirmed on a chain we simply cannot read
    /// right now.
    Unreadable,
}

/// Read the signature's status once and translate it into a settle-loop
/// decision.
///
/// A failed read is deliberately NOT an error. Once the transaction is on the
/// network, returning [`DirectSwapError::SubmitFailed`] here would report
/// `submitted() == false` for a swap that may well have landed, and the caller
/// would retry it — buying the position twice. An RPC that cannot be reached
/// says nothing about what the chain did.
async fn poll_status_once(
    rpc: &'static RpcClient,
    signature: &Signature,
    signature_str: &str,
) -> Poll {
    match rpc
        .get_signature_statuses(std::slice::from_ref(signature))
        .await
    {
        Ok(statuses) => match statuses
            .into_iter()
            .next()
            .flatten()
            .and_then(|status| status_outcome(&status, signature_str))
        {
            Some(outcome) => Poll::Settled(outcome),
            None => Poll::Pending,
        },
        Err(e) => {
            logger::debug(
                LogTag::Swap,
                &format!(
                    "Could not read the status of {signature_str} this round; \
                     an unreadable node is not evidence the swap failed: {e}"
                ),
            );
            Poll::Unreadable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::solana_transaction_status::{
        TransactionConfirmationStatus, TransactionStatus,
    };

    fn status(confirmation: Option<TransactionConfirmationStatus>) -> TransactionStatus {
        TransactionStatus {
            slot: 1,
            confirmations: None,
            status: Ok(()),
            err: None,
            confirmation_status: confirmation,
        }
    }

    #[test]
    fn a_processed_only_status_is_not_yet_a_settle_outcome() {
        assert!(status_outcome(
            &status(Some(TransactionConfirmationStatus::Processed)),
            "sig"
        )
        .is_none());
    }

    #[test]
    fn a_confirmed_or_finalized_status_settles_successfully() {
        assert!(matches!(
            status_outcome(
                &status(Some(TransactionConfirmationStatus::Confirmed)),
                "sig"
            ),
            Some(Ok(()))
        ));
        assert!(matches!(
            status_outcome(
                &status(Some(TransactionConfirmationStatus::Finalized)),
                "sig"
            ),
            Some(Ok(()))
        ));
    }

    #[test]
    fn an_on_chain_error_settles_as_transaction_failed_not_a_timeout() {
        let mut s = status(Some(TransactionConfirmationStatus::Confirmed));
        s.err =
            Some(crate::chains::solana::solana_sdk::transaction::TransactionError::AccountInUse);
        assert!(matches!(
            status_outcome(&s, "sig"),
            Some(Err(DirectSwapError::TransactionFailed { .. }))
        ));
    }

    #[test]
    fn a_blockhash_is_expired_only_once_the_height_strictly_passes_the_last_valid_one() {
        assert!(
            !blockhash_has_expired(100, 100),
            "the last valid block itself still counts"
        );
        assert!(!blockhash_has_expired(99, 100));
        assert!(blockhash_has_expired(101, 100));
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
