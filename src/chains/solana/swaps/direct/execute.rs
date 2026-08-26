//! Turning a [`SwapPlan`] into a landed, verified transaction.
//!
//! The step order is the money-safety contract of the engine:
//!
//! 1. build and sign — nothing is on the network yet;
//! 2. simulate — a mis-built instruction is rejected here for free;
//! 3. send — the point of no return;
//! 4. confirm — bounded wait;
//! 5. verify — read back what actually arrived.
//!
//! Anything that fails at steps 1-2 returns an error whose
//! [`DirectSwapError::submitted`] is false, and is safe to retry. Everything from
//! step 3 onward returns `submitted() == true`, INCLUDING a confirmation timeout:
//! a timed-out swap may still land, and retrying it buys the position twice.

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
    /// Wall-clock time from build to verified, in milliseconds.
    pub duration_ms: u64,
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
    let blockhash = rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| DirectSwapError::Build {
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
        .map_err(|e| DirectSwapError::SubmitFailed {
            detail: format!("simulation could not be run: {e}"),
        })
}

/// Build, sign, simulate, send, confirm and verify `plan`.
pub async fn execute_plan(
    plan: &SwapPlan,
    keypair: &Keypair,
) -> DirectSwapResult<DirectSwapOutcome> {
    let started = Instant::now();
    let rpc = get_rpc_client();
    let owner = keypair.pubkey();

    let blockhash = rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| DirectSwapError::Build {
            detail: format!("no recent blockhash: {e}"),
        })?;

    let transaction = VersionedTransaction::from(Transaction::new_signed_with_payer(
        &plan.instructions,
        Some(&owner),
        &[keypair],
        blockhash,
    ));

    if with_config(|cfg| cfg.swaps.direct.simulate_before_send) {
        let outcome = rpc.simulate_transaction(&transaction).await.map_err(|e| {
            DirectSwapError::SubmitFailed {
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
        }
    }

    let signature = rpc
        .send_transaction(&transaction)
        .await
        .map_err(|e| DirectSwapError::SubmitFailed {
            detail: e.to_string(),
        })?
        .to_string();

    let timeout = Duration::from_secs(with_config(|cfg| {
        cfg.swaps.direct.confirmation_timeout_secs
    }));
    let parsed = signature
        .parse()
        .map_err(|e| DirectSwapError::SubmitFailed {
            detail: format!("node returned an unparsable signature {signature}: {e}"),
        })?;

    let confirmed = rpc
        .confirm_transaction(&parsed, CommitmentLevel::Confirmed, timeout)
        .await
        .map_err(|_| DirectSwapError::ConfirmationTimeout {
            signature: signature.clone(),
            waited_ms: timeout.as_millis() as u64,
        })?;
    if !confirmed {
        return Err(DirectSwapError::ConfirmationTimeout {
            signature: signature.clone(),
            waited_ms: timeout.as_millis() as u64,
        });
    }

    let details = rpc.get_transaction_details(&signature).await.map_err(|e| {
        // Confirmed but unreadable: the swap happened, so this must stay a
        // submitted-class error rather than anything a caller might retry.
        DirectSwapError::TransactionFailed {
            signature: signature.clone(),
            detail: format!("confirmed transaction could not be read back: {e}"),
        }
    })?;

    let receipt = receipt_from_transaction(&signature, &owner, plan, &details)?;

    Ok(DirectSwapOutcome {
        signature,
        amount_in: plan.quote.amount_in,
        receipt,
        platform_fee: plan.quote.fee.amount,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}
