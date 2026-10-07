// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one pre-send gate every swap transaction passes before it is sent.
//!
//! Jupiter, Raptor and the direct pool engine, on the main wallet and on the
//! wallet tools alike, hand their built transaction here, and only this module
//! asks a node to simulate or send a swap (guarded by
//! `tests/architecture_boundaries.rs`). The wallet's plain transactions —
//! transfers, account closes, burns — are sent and settled here too
//! ([`send_and_settle_signed`]), under the same outcome rule. The gate answers three questions in a
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
//! 3. **Was it sent, and what became of it?** A send the one node it reached
//!    refused as a malformed request, or whose preflight that node ran and saw
//!    fail, provably never reached the chain. Any
//!    other send outcome cannot prove that — the RPC manager may already have
//!    delivered the same bytes through the relay or another provider — so the
//!    transaction's own signature is settled from chain state, and only that
//!    verdict ([`Settled`]) says whether it landed, reverted, provably expired
//!    or is still unknown.
//!
//! Every refusal is a [`NotSubmittedReason`]: the typed "nothing was sent" the
//! fallback chain decides from. Once a send was attempted, every outcome
//! carries the signature.

use std::future::Future;
use std::time::Duration;

use base64::Engine;
use tokio::time::Instant;

use crate::chains::solana::rpc::client::RpcClient;
use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::settlement::{
    classify_status, finalized_tip, transaction_lookup, FinalizedTip, StatusRead, TransactionLookup,
};
use crate::chains::solana::solana_packet::PACKET_DATA_SIZE;
use crate::chains::solana::solana_sdk::{
    compute_budget::id as compute_budget_program,
    message::VersionedMessage,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus,
};
use crate::chains::solana::swaps::cost_guard;
use crate::chains::solana::swaps::direct::compute::compute_unit_limit_from_measured;
use crate::chains::SignatureVerdict;
use crate::logger::{self, LogTag};
use crate::swaps::{NotSubmittedReason, Quote, SwapExecutionError};

/// How long a sent aggregator swap is settled before it is reported as
/// submitted but unconfirmed, and handed to verification.
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);

/// How long a plain wallet transaction is settled before its outcome is
/// reported as unknown.
const SIGNED_TRANSACTION_TIMEOUT: Duration = Duration::from_secs(60);

/// How often the settle loop polls the signature's status.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// How often the settle loop re-sends the same signed transaction. The
/// signature is identical every time, so a duplicate landing is impossible:
/// this only fights the network having dropped the earlier copy.
const REBROADCAST_INTERVAL: Duration = Duration::from_secs(2);

/// How long one re-broadcast may take. A copy that is not handed over by then
/// is abandoned for that round: the status polls that decide the swap must not
/// wait behind a slow or hanging send, and a dropped copy of the same signed
/// bytes changes nothing.
const REBROADCAST_TIMEOUT: Duration = Duration::from_secs(1);

/// How often the settle loop reads the finalized tip its expiry is judged
/// against.
const TIP_CHECK_INTERVAL: Duration = Duration::from_secs(4);

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
    simulation_verdict(get_rpc_client().simulate(transaction).await)
}

/// Why a send request returned no signature.
#[derive(Debug, Clone)]
enum SendFailure {
    /// Provably never sent: refused before or by the request itself.
    NotSent(NotSubmittedReason),
    /// The send failed in a way that cannot prove the transaction never
    /// reached a node.
    Unproven(crate::Error),
}

/// Classify a failed send request.
fn send_failure(error: crate::Error) -> SendFailure {
    match error {
        crate::Error::Rpc(rejection) if rejection.is_request_rejection() => {
            SendFailure::NotSent(NotSubmittedReason::RequestRejected {
                detail: rejection.to_string(),
            })
        }
        // The node simulated the send itself, saw it fail and never forwarded
        // it: the same answer a pre-send simulation gives.
        crate::Error::Rpc(refusal) if refusal.is_preflight_refusal() => {
            SendFailure::NotSent(NotSubmittedReason::SimulationFailed {
                detail: refusal.to_string(),
            })
        }
        other => SendFailure::Unproven(other),
    }
}

/// The node calls a swap's simulate, send and settle steps make. The shared
/// RPC client is the production node; tests drive the same steps against a
/// scripted one.
pub trait SwapNode: Sync {
    /// Simulate without signature verification.
    fn simulate(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl Future<Output = crate::Result<SimulationOutcome>> + Send;
    /// Hand signed bytes to the network.
    fn send(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl Future<Output = crate::Result<Signature>> + Send;
    /// One signature's status, `None` while the node does not know it, with
    /// the slot the node answered at.
    fn status(
        &self,
        signature: &Signature,
    ) -> impl Future<Output = crate::Result<(u64, Option<TransactionStatus>)>> + Send;
    /// The finalized tip an expiry is judged against.
    fn finalized_tip(&self) -> impl Future<Output = crate::Result<FinalizedTip>> + Send;
    /// The executed transaction for a signature, `None` when the chain holds none.
    fn transaction(
        &self,
        signature: &Signature,
    ) -> impl Future<Output = crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>>> + Send;
}

impl SwapNode for RpcClient {
    async fn simulate(
        &self,
        transaction: &VersionedTransaction,
    ) -> crate::Result<SimulationOutcome> {
        self.simulate_transaction(transaction).await
    }

    async fn send(&self, transaction: &VersionedTransaction) -> crate::Result<Signature> {
        self.send_transaction(transaction).await
    }

    async fn status(
        &self,
        signature: &Signature,
    ) -> crate::Result<(u64, Option<TransactionStatus>)> {
        let (slot, statuses) = self
            .get_signature_statuses(std::slice::from_ref(signature))
            .await?;
        Ok((slot, statuses.into_iter().next().flatten()))
    }

    async fn finalized_tip(&self) -> crate::Result<FinalizedTip> {
        finalized_tip(self).await
    }

    async fn transaction(
        &self,
        signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        self.get_transaction(signature).await
    }
}

/// What became of a transaction that was handed to a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// Confirmed without error.
    Landed,
    /// Confirmed, and the chain reports it failed: it moved nothing but its
    /// fee, and it can never land again.
    Reverted { detail: String },
    /// Never seen, and its blockhash provably expired: past the last block the
    /// blockhash was valid for, a node that had reached the expiry did not know
    /// the signature and the chain holds no transaction for it
    /// ([`crate::chains::solana::settlement::classify_status`]), so it can never
    /// land, by any node.
    Expired {
        last_valid_block_height: u64,
        current_block_height: u64,
    },
    /// Neither confirmed nor provably dead within the wait. It may still land.
    Unsettled { waited_ms: u64 },
}

/// Send a signed transaction and settle it.
///
/// Returns a reason only when the transaction provably never reached a node:
/// it does not measure, carries no signature, or the one node the request
/// reached refused the request itself or failed its preflight. Once the request may have reached a
/// node, the result is the transaction's own signature with a settle verdict,
/// whatever the send answered: an unanswered, failed or unreadable send is
/// settled from chain state exactly like an accepted one, so nothing that may
/// land is ever reported as a failure with no signature.
///
/// `last_valid_block_height` is the last block the transaction's blockhash is
/// valid for. Without it, a transaction that never lands ends
/// [`Settled::Unsettled`] rather than provably [`Settled::Expired`].
pub async fn send_and_settle<N: SwapNode>(
    node: &N,
    transaction: &VersionedTransaction,
    last_valid_block_height: Option<u64>,
    timeout: Duration,
) -> Result<(Signature, Settled), NotSubmittedReason> {
    measure_transaction(transaction)?;
    let signature =
        *transaction
            .signatures
            .first()
            .ok_or_else(|| NotSubmittedReason::BuildUnusable {
                detail: "the transaction carries no signature".to_owned(),
            })?;
    if let Err(error) = node.send(transaction).await {
        match send_failure(error) {
            SendFailure::NotSent(reason) => return Err(reason),
            SendFailure::Unproven(error) => logger::warning(
                LogTag::Swap,
                &format!(
                    "Send of {signature} returned no answer that proves it was not delivered \
                     ({error}); settling it from chain state"
                ),
            ),
        }
    }
    let settled = settle(
        node,
        transaction,
        &signature,
        last_valid_block_height,
        timeout,
    )
    .await;
    Ok((signature, settled))
}

/// What one read of the chain told the settle loop.
enum Poll {
    /// The transaction reached a terminal state.
    Settled(Settled),
    /// Not decided: unconfirmed, inside its validity window, or unreadable.
    /// An unreadable node is not evidence of anything.
    Pending,
}

/// Read the signature once and decide it by the settlement rule.
async fn poll<N: SwapNode>(
    node: &N,
    signature: &Signature,
    last_valid_block_height: Option<u64>,
    tip: Option<FinalizedTip>,
) -> Poll {
    let (read_slot, status) = match node.status(signature).await {
        Ok(read) => read,
        Err(e) => {
            logger::debug(
                LogTag::Swap,
                &format!(
                    "Could not read the status of {signature} this round; an unreadable node \
                     is not evidence the swap failed: {e}"
                ),
            );
            return Poll::Pending;
        }
    };
    let failure = |err: Option<String>| Settled::Reverted {
        detail: err.unwrap_or_default(),
    };
    match classify_status(status.as_ref(), read_slot, last_valid_block_height, tip) {
        StatusRead::Decided(SignatureVerdict::Landed) => Poll::Settled(Settled::Landed),
        StatusRead::Decided(SignatureVerdict::FailedOnChain) => Poll::Settled(failure(
            status
                .and_then(|status| status.err)
                .map(|err| err.to_string()),
        )),
        StatusRead::Decided(SignatureVerdict::Pending | SignatureVerdict::NotLanded) => {
            Poll::Pending
        }
        StatusRead::LookUpTransaction => {
            match transaction_lookup(node.transaction(signature).await) {
                TransactionLookup::Executed { error: None } => Poll::Settled(Settled::Landed),
                TransactionLookup::Executed { error } => Poll::Settled(failure(error)),
                TransactionLookup::Unreadable => Poll::Pending,
                TransactionLookup::Missing => {
                    let (Some(last_valid_block_height), Some(tip)) = (last_valid_block_height, tip)
                    else {
                        return Poll::Pending;
                    };
                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "Transaction {signature} is dead: its blockhash expired at block \
                             {last_valid_block_height}, the finalized block is {}, and no \
                             node holds the signature",
                            tip.block_height
                        ),
                    );
                    Poll::Settled(Settled::Expired {
                        last_valid_block_height,
                        current_block_height: tip.block_height,
                    })
                }
            }
        }
    }
}

/// Poll for a landed signature, re-broadcasting the same signed transaction
/// against network drops, until it lands, fails at `Confirmed`, its blockhash
/// provably expires with the signature unseen, or `timeout` runs out.
///
/// Whether the transaction is dead is decided by the settlement rule
/// ([`classify_status`]) against the last finalized tip read, which is always
/// read before the status it judges.
async fn settle<N: SwapNode>(
    node: &N,
    transaction: &VersionedTransaction,
    signature: &Signature,
    last_valid_block_height: Option<u64>,
    timeout: Duration,
) -> Settled {
    let deadline = Instant::now() + timeout;
    let mut last_rebroadcast = Instant::now();
    let mut last_tip_check = Instant::now();
    let mut tip: Option<FinalizedTip> = None;

    loop {
        if let Poll::Settled(settled) = poll(node, signature, last_valid_block_height, tip).await {
            return settled;
        }

        if Instant::now() >= deadline {
            return Settled::Unsettled {
                waited_ms: timeout.as_millis() as u64,
            };
        }

        if last_rebroadcast.elapsed() >= REBROADCAST_INTERVAL {
            last_rebroadcast = Instant::now();
            match tokio::time::timeout(REBROADCAST_TIMEOUT, node.send(transaction)).await {
                Err(_) => logger::debug(
                    LogTag::Swap,
                    &format!(
                        "Re-broadcast of {signature} took longer than {}ms and was abandoned \
                         for this round",
                        REBROADCAST_TIMEOUT.as_millis()
                    ),
                ),
                Ok(Ok(_)) => logger::info(
                    LogTag::Swap,
                    &format!(
                        "Re-broadcast {signature}: the network may have dropped the earlier copy"
                    ),
                ),
                Ok(Err(e)) => logger::debug(
                    LogTag::Swap,
                    &format!(
                        "Re-broadcast of {signature} was not accepted this round (an 'already \
                         processed' response is a good sign, not a failure): {e}"
                    ),
                ),
            }
        }

        if last_valid_block_height.is_some() && last_tip_check.elapsed() >= TIP_CHECK_INTERVAL {
            last_tip_check = Instant::now();
            if let Ok(read) = node.finalized_tip().await {
                tip = Some(read);
            }
        }

        tokio::time::sleep(STATUS_POLL_INTERVAL).await;
    }
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

/// Gate, sign, send and settle a transaction an aggregator built for `quote`.
///
/// Every failure before the send is a [`SwapExecutionError::NotSubmitted`].
/// Once sent, the outcome follows the settle verdict: a confirmed swap returns
/// its signature, a confirmed revert is
/// [`crate::chains::ExecutionFailure::Reverted`], a provably expired one is
/// [`crate::chains::ExecutionFailure::Expired`], and one that is neither
/// carries its signature as
/// [`crate::chains::ExecutionFailure::ConfirmationTimeout`], so it is
/// reconciled, never sent again. `last_valid_block_height` is the build's own
/// report of its blockhash's validity, when the aggregator gives one.
pub async fn submit_built_swap(
    router: &'static str,
    transaction_base64: &str,
    last_valid_block_height: Option<u64>,
    quote: &Quote,
    signer: &Keypair,
) -> crate::Result<Signature> {
    submit_on(
        get_rpc_client(),
        router,
        transaction_base64,
        last_valid_block_height,
        quote,
        signer,
        CONFIRMATION_TIMEOUT,
    )
    .await
}

async fn submit_on<N: SwapNode>(
    node: &N,
    router: &'static str,
    transaction_base64: &str,
    last_valid_block_height: Option<u64>,
    quote: &Quote,
    signer: &Keypair,
    timeout: Duration,
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
    // Only the wallet signs, in slot 0, after the compute limit is tightened.
    // A build that needs a second signature could never be accepted once its
    // message changes, and simulation (`sigVerify: false`) would not notice.
    if transaction.message.header().num_required_signatures != 1
        || transaction.signatures.len() != 1
    {
        return Err(not_submitted(NotSubmittedReason::BuildUnusable {
            detail: format!(
                "the transaction needs {} signatures and carries {} slots; only the wallet's \
                 own signature can be supplied",
                transaction.message.header().num_required_signatures,
                transaction.signatures.len()
            ),
        }));
    }
    logger::debug(
        LogTag::Swap,
        &format!(
            "{router} transaction is {} of {} bytes ({:?})",
            measured.bytes, measured.limit, measured.format
        ),
    );

    match simulation_verdict(node.simulate(&transaction).await) {
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

    transaction.signatures[0] = signer.sign_message(&transaction.message.serialize());

    let (signature, settled) =
        send_and_settle(node, &transaction, last_valid_block_height, timeout)
            .await
            .map_err(not_submitted)?;
    landed(signature, settled)
        .map_err(|failure| crate::Error::Solana(crate::chains::solana::Error::Execution(failure)))
}

/// The signature of a landed transaction, or the settle verdict as an
/// execution failure that names the signature.
fn landed(
    signature: Signature,
    settled: Settled,
) -> Result<Signature, crate::chains::ExecutionFailure> {
    let reference = signature.to_string();
    Err(match settled {
        Settled::Landed => return Ok(signature),
        Settled::Reverted { detail } => {
            crate::chains::ExecutionFailure::Reverted { reference, detail }
        }
        Settled::Expired {
            last_valid_block_height,
            current_block_height,
        } => crate::chains::ExecutionFailure::Expired {
            reference,
            last_valid_block_height,
            current_block_height,
        },
        Settled::Unsettled { waited_ms } => crate::chains::ExecutionFailure::ConfirmationTimeout {
            reference,
            waited_ms,
        },
    })
}

/// Send a signed wallet transaction that is not a swap — a transfer, an
/// account close, a burn — and settle it by its own signature.
///
/// The swap outcome rule holds: only a send that provably never reached the
/// chain is [`crate::chains::solana::Error::NotSent`]. Every other outcome
/// names the signature, as the landed signature or as an
/// [`crate::chains::ExecutionFailure`], so a failed or unanswered send is never
/// reported as a transaction that did nothing. `last_valid_block_height` is
/// the last block the transaction's blockhash is valid for.
pub async fn send_and_settle_signed(
    transaction: &Transaction,
    last_valid_block_height: u64,
) -> crate::chains::solana::Result<Signature> {
    send_and_settle_signed_on(
        get_rpc_client(),
        transaction,
        last_valid_block_height,
        SIGNED_TRANSACTION_TIMEOUT,
    )
    .await
}

async fn send_and_settle_signed_on<N: SwapNode>(
    node: &N,
    transaction: &Transaction,
    last_valid_block_height: u64,
    timeout: Duration,
) -> crate::chains::solana::Result<Signature> {
    let transaction = VersionedTransaction::from(transaction.clone());
    let (signature, settled) =
        send_and_settle(node, &transaction, Some(last_valid_block_height), timeout)
            .await
            .map_err(crate::chains::solana::Error::NotSent)?;
    Ok(landed(signature, settled)?)
}

#[cfg(test)]
pub(crate) mod scripted_tests;

#[cfg(test)]
mod tests;
