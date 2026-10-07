// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one pre-send gate every swap transaction passes before it is sent.
//!
//! Jupiter, Raptor and the direct pool engine, on the main wallet and on the
//! wallet tools alike, hand their built transaction here, and only this module
//! asks a node to simulate or send a swap (guarded by
//! `tests/architecture_boundaries.rs`). The gate answers three questions in a
//! fixed order:
//!
//! 1. **Does it fit?** The serialized transaction is measured against what its
//!    message format may carry on the wire: `PACKET_DATA_SIZE`, 1,232 bytes,
//!    for both legacy and v0 messages. This is pure and costs nothing, and it is
//!    the only deterministic size check there is: an aggregator's own build
//!    simulation does not report size, and its account cap counts accounts while
//!    the packet counts bytes (a lookup-table account costs one byte, a static
//!    one thirty-two). A message version the locked SDK cannot represent is
//!    refused as [`NotSubmittedReason::UnsupportedFormat`] rather than guessed at.
//! 2. **Does the node accept it?** Simulation. A node that ran it and saw it
//!    fail, or that decoded the request and refused it
//!    ([`crate::rpc::RpcError::is_request_rejection`]), has answered, and the
//!    answer is a refusal. Only a node that could not answer at all is
//!    [`Verdict::NodeUnavailable`], the one verdict a caller may proceed
//!    through.
//! 3. **Was it sent?** A send a node refused as a malformed request provably
//!    never reached the chain. Any other send failure cannot prove that — the
//!    RPC manager may already have delivered the same bytes through another
//!    provider — and is handed back unchanged.
//!
//! Every refusal is a [`NotSubmittedReason`]: the typed "nothing was sent" the
//! fallback chain decides from.

use std::time::Duration;

use base64::Engine;

use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_packet::PACKET_DATA_SIZE;
use crate::chains::solana::solana_sdk::{
    commitment_config::CommitmentLevel,
    compute_budget::id as compute_budget_program,
    message::VersionedMessage,
    signature::{Keypair, Signature, Signer},
    transaction::VersionedTransaction,
};
use crate::chains::solana::swaps::cost_guard;
use crate::chains::solana::swaps::direct::compute::compute_unit_limit_from_measured;
use crate::logger::{self, LogTag};
use crate::swaps::{NotSubmittedReason, Quote, SwapExecutionError};

/// How long a sent aggregator swap is polled for confirmation before it is
/// reported as submitted but unconfirmed, and handed to verification.
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);

/// The high bit of a message's first byte marks a versioned message; the low
/// seven bits are the version.
const MESSAGE_VERSION_PREFIX: u8 = 0x80;

/// Bytes of one ed25519 signature on the wire.
const SIGNATURE_BYTES: usize = 64;

/// The message formats this build can send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    Legacy,
    V0,
}

impl WireFormat {
    /// The most bytes a serialized transaction of this format may occupy.
    pub const fn size_limit(self) -> usize {
        match self {
            WireFormat::Legacy | WireFormat::V0 => PACKET_DATA_SIZE,
        }
    }
}

/// A serialized transaction that fits its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    pub format: WireFormat,
    pub bytes: usize,
    pub limit: usize,
}

/// A compact-u16 length prefix: up to three bytes of seven bits each. Returns
/// the value and how many bytes it took.
fn short_vec_len(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut value = 0usize;
    for (index, byte) in bytes.iter().take(3).enumerate() {
        value |= usize::from(byte & 0x7f) << (7 * index);
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
    }
    None
}

/// The message format of serialized transaction bytes, read off the prefix
/// byte that follows the signatures, without decoding the message.
fn wire_format(bytes: &[u8]) -> Result<WireFormat, NotSubmittedReason> {
    let truncated = || NotSubmittedReason::BuildUnusable {
        detail: "the transaction bytes end before its message".to_owned(),
    };
    let (signatures, prefix_len) = short_vec_len(bytes).ok_or_else(truncated)?;
    let message_start = signatures
        .checked_mul(SIGNATURE_BYTES)
        .and_then(|signature_bytes| signature_bytes.checked_add(prefix_len))
        .ok_or_else(truncated)?;
    let prefix = *bytes.get(message_start).ok_or_else(truncated)?;
    if prefix & MESSAGE_VERSION_PREFIX == 0 {
        return Ok(WireFormat::Legacy);
    }
    match prefix & !MESSAGE_VERSION_PREFIX {
        0 => Ok(WireFormat::V0),
        version => Err(NotSubmittedReason::UnsupportedFormat { version }),
    }
}

/// Measure serialized transaction bytes against their format's limit.
pub fn measure(bytes: &[u8]) -> Result<Measured, NotSubmittedReason> {
    let format = wire_format(bytes)?;
    let limit = format.size_limit();
    if bytes.len() > limit {
        return Err(NotSubmittedReason::TransactionTooLarge {
            bytes: bytes.len(),
            limit,
        });
    }
    Ok(Measured {
        format,
        bytes: bytes.len(),
        limit,
    })
}

/// Measure a transaction exactly as it would be sent.
pub fn measure_transaction(
    transaction: &VersionedTransaction,
) -> Result<Measured, NotSubmittedReason> {
    let bytes = bincode::serialize(transaction).map_err(|e| NotSubmittedReason::BuildUnusable {
        detail: format!("the transaction does not serialize: {e}"),
    })?;
    measure(&bytes)
}

/// What the gate concluded about a transaction before it was sent.
#[derive(Debug, Clone)]
pub enum Verdict {
    /// It fits its format and simulated without error.
    Cleared(SimulationOutcome),
    /// It fits, and the node ran it and saw it fail. The outcome carries the
    /// program error and logs.
    Failed(SimulationOutcome),
    /// It was refused before a node ran it: too large, an unsupported format,
    /// or a request the node refused to decode.
    Refused(NotSubmittedReason),
    /// No node could answer. The only verdict a caller may proceed through:
    /// an unreadable node is not evidence about the transaction.
    NodeUnavailable { detail: String },
}

/// The verdict a simulation result amounts to.
pub fn simulation_verdict(result: crate::Result<SimulationOutcome>) -> Verdict {
    match result {
        Ok(outcome) if outcome.succeeded() => Verdict::Cleared(outcome),
        Ok(outcome) => Verdict::Failed(outcome),
        Err(crate::Error::Rpc(error)) if error.is_request_rejection() => {
            Verdict::Refused(NotSubmittedReason::RequestRejected {
                detail: error.to_string(),
            })
        }
        Err(error) => Verdict::NodeUnavailable {
            detail: error.to_string(),
        },
    }
}

/// The program error plus the last log lines, which name the failing program.
pub fn simulation_failure_detail(outcome: &SimulationOutcome) -> String {
    let skip = outcome.logs.len().saturating_sub(3);
    let last_logs = outcome.logs[skip..].join(" | ");
    format!("{} {last_logs}", outcome.failure_detail())
        .trim()
        .to_owned()
}

/// Measure `transaction`, then simulate it.
pub async fn gate(transaction: &VersionedTransaction) -> Verdict {
    if let Err(reason) = measure_transaction(transaction) {
        return Verdict::Refused(reason);
    }
    simulation_verdict(get_rpc_client().simulate_transaction(transaction).await)
}

/// Why a send returned no signature.
#[derive(Debug, Clone)]
pub enum SendFailure {
    /// Provably never sent: refused before or by the request itself.
    NotSent(NotSubmittedReason),
    /// The send failed in a way that cannot prove the transaction never
    /// reached a node.
    Unproven(crate::Error),
}

/// Classify a failed send request.
pub fn send_failure(error: crate::Error) -> SendFailure {
    match error {
        crate::Error::Rpc(rejection) if rejection.is_request_rejection() => {
            SendFailure::NotSent(NotSubmittedReason::RequestRejected {
                detail: rejection.to_string(),
            })
        }
        other => SendFailure::Unproven(other),
    }
}

/// Send a signed transaction, measuring it first so nothing unmeasured is ever
/// handed to a node.
pub async fn send(transaction: &VersionedTransaction) -> Result<Signature, SendFailure> {
    measure_transaction(transaction).map_err(SendFailure::NotSent)?;
    get_rpc_client()
        .send_transaction(transaction)
        .await
        .map_err(send_failure)
}

/// Lower the compute-unit limit a compiled transaction requests to what its
/// simulation measured, plus the margin the direct engine applies.
///
/// The prioritization fee is charged on the limit a transaction REQUESTS, and an
/// aggregator whose own build simulation failed falls back to the runtime
/// ceiling of 1,400,000 units. Only ever lowers: simulation ran against
/// slightly older state, and a real execution can cost more. Returns the
/// requested and the tightened limit when it changed anything.
pub fn tighten_compute_unit_limit(
    transaction: &mut VersionedTransaction,
    units_consumed: u64,
) -> Option<(u32, u32)> {
    let program = compute_budget_program();
    let measured = compute_unit_limit_from_measured(units_consumed);
    let (keys, instructions) = match &mut transaction.message {
        VersionedMessage::Legacy(message) => (&message.account_keys, &mut message.instructions),
        VersionedMessage::V0(message) => (&message.account_keys, &mut message.instructions),
    };
    let instruction = instructions.iter_mut().find(|instruction| {
        keys.get(usize::from(instruction.program_id_index)) == Some(&program)
            && instruction.data.len() == 5
            && instruction.data[0] == 2
    })?;
    let requested = u32::from_le_bytes(instruction.data[1..5].try_into().ok()?);
    if measured >= requested {
        return None;
    }
    instruction.data[1..5].copy_from_slice(&measured.to_le_bytes());
    Some((requested, measured))
}

/// Gate, sign, send and confirm a transaction an aggregator built for `quote`.
///
/// Every failure before the send is a [`SwapExecutionError::NotSubmitted`]. A
/// sent transaction whose confirmation runs out carries its signature as
/// [`crate::chains::ExecutionFailure::ConfirmationTimeout`], so it is
/// reconciled, never sent again.
pub async fn submit_built_swap(
    router: &'static str,
    transaction_base64: &str,
    quote: &Quote,
    signer: &Keypair,
) -> crate::Result<Signature> {
    let not_submitted = |reason| {
        crate::Error::Swaps(SwapExecutionError::NotSubmitted {
            router: router.to_owned(),
            reason,
        })
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(transaction_base64)
        .map_err(|e| {
            not_submitted(NotSubmittedReason::BuildUnusable {
                detail: format!("the transaction is not base64: {e}"),
            })
        })?;
    let measured = measure(&bytes).map_err(not_submitted)?;
    let mut transaction: VersionedTransaction = bincode::deserialize(&bytes).map_err(|e| {
        not_submitted(NotSubmittedReason::BuildUnusable {
            detail: format!("the transaction does not decode: {e}"),
        })
    })?;
    if transaction.message.static_account_keys().first() != Some(&signer.pubkey()) {
        return Err(not_submitted(NotSubmittedReason::BuildUnusable {
            detail: "the transaction's fee payer is not the signing wallet".to_owned(),
        }));
    }
    logger::debug(
        LogTag::Swap,
        &format!(
            "{router} transaction is {} of {} bytes ({:?})",
            measured.bytes, measured.limit, measured.format
        ),
    );

    match simulation_verdict(get_rpc_client().simulate_transaction(&transaction).await) {
        Verdict::Cleared(outcome) => {
            // Built on the aggregator's host, so nothing on our side has checked
            // what it does with the wallet's lamports.
            cost_guard::check(router, &outcome, quote).await?;
            if let Some(units) = outcome.units_consumed {
                if let Some((requested, tightened)) =
                    tighten_compute_unit_limit(&mut transaction, units)
                {
                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "{router} compute limit tightened from {requested} to {tightened} \
                             units after simulation measured {units}"
                        ),
                    );
                }
            }
        }
        Verdict::Failed(outcome) => {
            return Err(not_submitted(NotSubmittedReason::SimulationFailed {
                detail: simulation_failure_detail(&outcome),
            }));
        }
        Verdict::Refused(reason) => return Err(not_submitted(reason)),
        Verdict::NodeUnavailable { detail } => logger::warning(
            LogTag::Swap,
            &format!("{router} preflight simulation unavailable, proceeding: {detail}"),
        ),
    }

    let signature = signer.sign_message(&transaction.message.serialize());
    match transaction.signatures.first_mut() {
        Some(slot) => *slot = signature,
        None => transaction.signatures.push(signature),
    }

    let sent = send(&transaction).await.map_err(|failure| match failure {
        SendFailure::NotSent(reason) => not_submitted(reason),
        SendFailure::Unproven(error) => error,
    })?;

    if get_rpc_client()
        .confirm_transaction(&sent, CommitmentLevel::Confirmed, CONFIRMATION_TIMEOUT)
        .await?
    {
        Ok(sent)
    } else {
        Err(crate::Error::Solana(
            crate::chains::solana::Error::Execution(
                crate::chains::ExecutionFailure::ConfirmationTimeout {
                    reference: sent.to_string(),
                    waited_ms: CONFIRMATION_TIMEOUT.as_millis() as u64,
                },
            ),
        ))
    }
}

#[cfg(test)]
mod tests;
