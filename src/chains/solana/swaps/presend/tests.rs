// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The pre-send gate against recorded aggregator builds and synthetic
//! transactions at the packet boundary.

use super::*;
use crate::chains::solana::solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::{v0, Message},
    pubkey::Pubkey,
};
use crate::rpc::RpcError;

/// Recorded `lite-api.jup.ag` builds of the same 0.05 SOL buy of PUMP: a
/// three-venue route and the single-venue route of the same trade.
const BUILDS: &str =
    include_str!("../../../../../tests/fixtures/swaps/jupiter_sol_pump_builds.json");

fn recorded(route: &str) -> Vec<u8> {
    let builds: serde_json::Value = serde_json::from_str(BUILDS).expect("fixture is JSON");
    let transaction = builds[route]["transaction"]
        .as_str()
        .expect("fixture carries the transaction");
    base64::engine::general_purpose::STANDARD
        .decode(transaction)
        .expect("fixture is base64")
}

fn decoded(bytes: &[u8]) -> VersionedTransaction {
    bincode::deserialize(bytes).expect("fixture decodes")
}

#[test]
fn the_recorded_multi_hop_build_is_refused_and_the_single_hop_build_fits() {
    let multi_hop = recorded("multi_hop");
    assert_eq!(multi_hop.len(), 1329);
    assert!(matches!(
        measure(&multi_hop),
        Err(NotSubmittedReason::TransactionTooLarge {
            bytes: 1329,
            limit: 1232
        })
    ));

    let single_hop = recorded("single_hop");
    assert_eq!(
        measure(&single_hop).expect("the single-hop build fits"),
        Measured {
            format: WireFormat::V0,
            bytes: 628,
            limit: 1232
        }
    );
    // Decoding and re-serializing changes nothing: the transaction measured is
    // the transaction that would be sent.
    assert_eq!(
        measure_transaction(&decoded(&single_hop)).unwrap().bytes,
        628
    );
}

/// The refusal happens before any node is asked: no RPC client exists in this
/// test, and the gate never reaches for one.
#[tokio::test]
async fn the_gate_refuses_an_oversized_build_without_any_rpc() {
    let transaction = decoded(&recorded("multi_hop"));
    assert!(matches!(
        gate(&transaction).await,
        Verdict::Refused(NotSubmittedReason::TransactionTooLarge { bytes: 1329, .. })
    ));
}

/// A legacy and a v0 transaction padded to exactly `target` serialized bytes.
fn padded(target: usize, versioned: bool) -> VersionedTransaction {
    let payer = Pubkey::new_unique();
    let program = Pubkey::new_unique();
    let build = |data_len: usize| {
        let instruction = Instruction {
            program_id: program,
            accounts: vec![],
            data: vec![7; data_len],
        };
        let message = if versioned {
            VersionedMessage::V0(
                v0::Message::try_compile(&payer, &[instruction], &[], Hash::default())
                    .expect("compiles"),
            )
        } else {
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&payer)))
        };
        VersionedTransaction {
            signatures: vec![Signature::default()],
            message,
        }
    };
    let base = bincode::serialize(&build(1000)).unwrap().len();
    let transaction = build(1000 + target - base);
    assert_eq!(bincode::serialize(&transaction).unwrap().len(), target);
    transaction
}

#[test]
fn a_transaction_of_exactly_the_packet_size_fits_and_one_byte_more_does_not() {
    for versioned in [false, true] {
        let at_limit = padded(1232, versioned);
        assert_eq!(measure_transaction(&at_limit).unwrap().bytes, 1232);

        let over = padded(1233, versioned);
        assert!(
            matches!(
                measure_transaction(&over),
                Err(NotSubmittedReason::TransactionTooLarge {
                    bytes: 1233,
                    limit: 1232
                })
            ),
            "versioned: {versioned}"
        );
    }
    assert_eq!(WireFormat::Legacy.size_limit(), 1232);
    assert_eq!(WireFormat::V0.size_limit(), 1232);
}

/// A message version the locked SDK cannot represent is refused, never sized
/// against a limit that is not its own; bytes that end before the message are
/// unusable.
#[test]
fn an_unknown_message_version_or_truncated_bytes_are_refused() {
    let mut bytes = vec![1u8];
    bytes.extend_from_slice(&[0u8; 64]);
    bytes.push(0x81);
    assert!(matches!(
        measure(&bytes),
        Err(NotSubmittedReason::UnsupportedFormat { version: 1 })
    ));
    assert!(matches!(
        measure(&[1u8, 0, 0]),
        Err(NotSubmittedReason::BuildUnusable { .. })
    ));
    assert!(matches!(
        measure(&[0xff, 0xff, 0xff]),
        Err(NotSubmittedReason::BuildUnusable { .. })
    ));
}

/// Every RPC error variant a simulation can fail with, by name. Exhaustive, so a
/// new variant cannot compile without a row in the verdict table.
fn rpc_variant(error: &RpcError) -> &'static str {
    match error {
        RpcError::RateLimited { .. } => "RateLimited",
        RpcError::Network { .. } => "Network",
        RpcError::ProviderError { .. } => "ProviderError",
        RpcError::Timeout { .. } => "Timeout",
        RpcError::CircuitOpen { .. } => "CircuitOpen",
        RpcError::NoProvidersAvailable { .. } => "NoProvidersAvailable",
        RpcError::AccountNotFound { .. } => "AccountNotFound",
        RpcError::InvalidResponse { .. } => "InvalidResponse",
        RpcError::Configuration { .. } => "Configuration",
        RpcError::Other(_) => "Other",
        RpcError::RefusedAfterDelivery { .. } => "RefusedAfterDelivery",
    }
}

/// How a node's answer reads, as a simulation and as a first send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The node refused the request itself.
    Rejected,
    /// The node ran the send's preflight and saw it fail; a simulation never
    /// answers this way.
    PreflightFailed,
    /// Not an answer about the transaction.
    Unanswered,
}

/// Only a node that answered may refuse a transaction; every other failure is
/// the one fail-open verdict. A send is refused on the same answers, and also
/// when the node's own preflight failed, unless that preflight found the
/// transaction already processed.
#[test]
fn only_a_node_that_answered_refuses_a_transaction() {
    let provider = |code: i64| RpcError::ProviderError {
        code,
        message: "base64 encoded VersionedTransaction too large".to_owned(),
        data: None,
    };
    let rows: Vec<(RpcError, Answer)> = vec![
        (
            RpcError::RateLimited {
                provider_id: "p".to_owned(),
                retry_after: None,
            },
            Answer::Unanswered,
        ),
        (
            RpcError::Network {
                message: "reset".to_owned(),
                is_timeout: true,
            },
            Answer::Unanswered,
        ),
        (provider(-32602), Answer::Rejected),
        (provider(-32600), Answer::Rejected),
        (provider(-32700), Answer::Rejected),
        (provider(-32002), Answer::PreflightFailed),
        (
            RpcError::ProviderError {
                code: -32002,
                message: "Transaction simulation failed: This transaction has already been \
                          processed"
                    .to_owned(),
                data: None,
            },
            Answer::Unanswered,
        ),
        (
            RpcError::ProviderError {
                code: -32002,
                message: "Transaction simulation failed".to_owned(),
                data: Some("{\"err\":\"AlreadyProcessed\"}".to_owned()),
            },
            Answer::Unanswered,
        ),
        (provider(-32005), Answer::Unanswered),
        (
            RpcError::Timeout {
                provider_id: "p".to_owned(),
                after: Duration::from_secs(1),
            },
            Answer::Unanswered,
        ),
        (
            RpcError::CircuitOpen {
                provider_id: "p".to_owned(),
                retry_after: Duration::from_secs(1),
            },
            Answer::Unanswered,
        ),
        (
            RpcError::NoProvidersAvailable { last_error: None },
            Answer::Unanswered,
        ),
        (
            RpcError::AccountNotFound {
                pubkey: "k".to_owned(),
            },
            Answer::Unanswered,
        ),
        (
            RpcError::InvalidResponse {
                message: "garbled".to_owned(),
            },
            Answer::Unanswered,
        ),
        (
            RpcError::Configuration {
                message: "bad".to_owned(),
            },
            Answer::Unanswered,
        ),
        (RpcError::Other("other".to_owned()), Answer::Unanswered),
        // A refusal after an attempt that may have delivered the transaction
        // answers only for its own node.
        (
            RpcError::RefusedAfterDelivery {
                earlier_attempts: 1,
                refusal: Box::new(provider(-32602)),
            },
            Answer::Unanswered,
        ),
    ];
    let mut covered: Vec<&str> = rows.iter().map(|(error, _)| rpc_variant(error)).collect();
    covered.sort_unstable();
    covered.dedup();
    assert_eq!(covered.len(), 11, "every RpcError variant needs a row");

    for (error, answer) in rows {
        let label = error.to_string();
        let verdict = simulation_verdict(Err(crate::Error::Rpc(error.clone())));
        match (&verdict, answer) {
            (Verdict::Refused(NotSubmittedReason::RequestRejected { .. }), Answer::Rejected)
            | (Verdict::NodeUnavailable { .. }, Answer::PreflightFailed | Answer::Unanswered) => {}
            _ => panic!("{label}: unexpected verdict {verdict:?}"),
        }
        let send = send_failure(crate::Error::Rpc(error));
        match (&send, answer) {
            (
                SendFailure::NotSent(NotSubmittedReason::RequestRejected { .. }),
                Answer::Rejected,
            )
            | (
                SendFailure::NotSent(NotSubmittedReason::SimulationFailed { .. }),
                Answer::PreflightFailed,
            )
            | (SendFailure::Unproven(_), Answer::Unanswered) => {}
            _ => panic!("{label}: unexpected send failure {send:?}"),
        }
    }

    // A failure outside the RPC channel is never evidence either.
    assert!(matches!(
        simulation_verdict(Err(crate::Error::internal_error("serialize"))),
        Verdict::NodeUnavailable { .. }
    ));
}

#[test]
fn a_simulation_the_node_ran_is_cleared_or_failed_by_its_own_result() {
    let outcome = |err: Option<serde_json::Value>| SimulationOutcome {
        err,
        logs: vec![
            "Program log: one".to_owned(),
            "Program log: two".to_owned(),
            "Program log: three".to_owned(),
            "Program X failed: custom program error: 0x1771".to_owned(),
        ],
        units_consumed: Some(120_000),
        inner_instructions: vec![],
    };
    assert!(matches!(
        simulation_verdict(Ok(outcome(None))),
        Verdict::Cleared(_)
    ));
    let Verdict::Failed(failed) = simulation_verdict(Ok(outcome(Some(
        serde_json::json!({"InstructionError": [3, {"Custom": 6001}]}),
    )))) else {
        panic!("a node that ran the transaction and saw it fail has answered");
    };
    let detail = simulation_failure_detail(&failed);
    assert!(detail.contains("6001"), "{detail}");
    assert!(detail.contains("0x1771"), "{detail}");
    assert!(!detail.contains("Program log: one"), "{detail}");
}

/// Jupiter falls back to the 1,400,000-unit ceiling when its own build
/// simulation fails, and the recorded multi-hop build carries exactly that. The
/// limit is tightened to the measured figure in place, without changing the
/// transaction's size, and never raised: the recorded single-hop build already
/// requests 163,950 units, below what 150,000 measured units plus margin come to.
#[test]
fn a_recorded_ceiling_limit_is_tightened_to_the_measured_units() {
    let mut ceiling = decoded(&recorded("multi_hop"));
    let tightened = compute_unit_limit_from_measured(150_000);
    assert_eq!(
        tighten_compute_unit_limit(&mut ceiling, 150_000),
        Some((1_400_000, tightened))
    );
    assert_eq!(bincode::serialize(&ceiling).unwrap().len(), 1329);
    assert_eq!(
        tighten_compute_unit_limit(&mut ceiling, 150_000),
        None,
        "a limit already at the measured figure is left alone"
    );

    let mut sized = decoded(&recorded("single_hop"));
    assert_eq!(
        tighten_compute_unit_limit(&mut sized, 150_000),
        None,
        "a measurement above the requested limit never raises it"
    );
    assert_eq!(
        tighten_compute_unit_limit(&mut sized, 100_000),
        Some((163_950, compute_unit_limit_from_measured(100_000)))
    );

    let mut without_budget = padded(600, true);
    assert_eq!(
        tighten_compute_unit_limit(&mut without_budget, 10_000),
        None
    );
}

/// Tightening rewrites only the four limit bytes of the one
/// `SetComputeUnitLimit` instruction. Every account key, every lookup, the
/// blockhash and every other instruction — the platform fee transfer and the
/// route included — stay byte-for-byte the build the aggregator made.
#[test]
fn tightening_leaves_every_byte_outside_the_compute_limit_identical() {
    let original = decoded(&recorded("multi_hop"));
    let mut tightened = original.clone();
    tighten_compute_unit_limit(&mut tightened, 150_000).expect("the ceiling is tightened");
    let (VersionedMessage::V0(before), VersionedMessage::V0(after)) =
        (&original.message, &tightened.message)
    else {
        panic!("the recorded build is a v0 message");
    };
    assert_eq!(before.header, after.header);
    assert_eq!(before.account_keys, after.account_keys);
    assert_eq!(before.recent_blockhash, after.recent_blockhash);
    assert_eq!(before.address_table_lookups, after.address_table_lookups);
    assert_eq!(before.instructions.len(), after.instructions.len());
    let program = compute_budget_program();
    let mut changed = 0;
    for (old, new) in before.instructions.iter().zip(&after.instructions) {
        assert_eq!(old.program_id_index, new.program_id_index);
        assert_eq!(old.accounts, new.accounts);
        if old.data == new.data {
            continue;
        }
        changed += 1;
        assert_eq!(
            before.account_keys[usize::from(old.program_id_index)],
            program
        );
        assert_eq!(old.data[0], 2, "only SetComputeUnitLimit changes");
        assert_eq!(old.data.len(), new.data.len());
        assert_eq!(old.data[0], new.data[0]);
    }
    assert_eq!(changed, 1);
}

// ----------------------------------------------------------------------------
// Send and settle: once a send was attempted, the outcome carries the signature
// ----------------------------------------------------------------------------

use super::scripted_tests::{status, ScriptedNode, SendAnswer};
use crate::chains::solana::solana_sdk::{
    compute_budget::ComputeBudgetInstruction, transaction::TransactionError,
};
use crate::chains::solana::solana_transaction_status::TransactionConfirmationStatus as Level;

const CEILING: u32 = 1_400_000;

/// What an aggregator hands back for `signer`: a compute ceiling, a priority
/// price, the platform fee transfer and the trade, unsigned.
fn aggregator_build(signer: &Keypair, fee_wallet: &Pubkey) -> VersionedTransaction {
    let message = Message::new_with_blockhash(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(CEILING),
            ComputeBudgetInstruction::set_compute_unit_price(50_000),
            // The platform fee: lamports from the wallet to the fee account.
            Instruction {
                program_id: Pubkey::new_unique(),
                accounts: vec![
                    crate::chains::solana::solana_sdk::instruction::AccountMeta::new(
                        signer.pubkey(),
                        true,
                    ),
                    crate::chains::solana::solana_sdk::instruction::AccountMeta::new(
                        *fee_wallet,
                        false,
                    ),
                ],
                data: 25_000u64.to_le_bytes().to_vec(),
            },
            Instruction {
                program_id: Pubkey::new_unique(),
                accounts: vec![],
                data: vec![1, 2, 3],
            },
        ],
        Some(&signer.pubkey()),
        &Hash::new_unique(),
    );
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::Legacy(message),
    }
}

fn base64_of(transaction: &VersionedTransaction) -> String {
    base64::engine::general_purpose::STANDARD.encode(bincode::serialize(transaction).unwrap())
}

fn quote_for(signer: &Keypair) -> Quote {
    crate::config::utils::install_default_config();
    Quote {
        chain: crate::chains::ChainId::Solana,
        router_id: "jupiter".to_owned(),
        router_name: "Jupiter".to_owned(),
        input_mint: "So11111111111111111111111111111111111111112".to_owned(),
        output_mint: Pubkey::new_unique().to_string(),
        input_amount: 50_000_000u64.into(),
        output_amount: 1_000u64.into(),
        minimum_output_amount: 950u64.into(),
        price_impact_pct: 0.1,
        platform_fee_lamports: Some(25_000),
        estimated_network_fee_lamports: None,
        slippage_bps: 100,
        route_plan: "test".to_owned(),
        swap_mode: crate::swaps::SwapMode::ExactIn,
        wallet_address: signer.pubkey().to_string(),
        exclude_dexes: None,
        execution_data: Vec::new(),
    }
}

async fn submit(
    node: &ScriptedNode,
    build: &VersionedTransaction,
    signer: &Keypair,
    last_valid_block_height: Option<u64>,
) -> crate::Result<Signature> {
    submit_on(
        node,
        "Jupiter",
        &base64_of(build),
        last_valid_block_height,
        &quote_for(signer),
        signer,
        Duration::from_secs(5),
    )
    .await
}

/// The send times out after delivery and the transaction may well land: the
/// signature signed into slot 0 comes back as a reconcilable outcome, so the
/// swap is handed to verification instead of being written off, and nothing
/// may be sent in its place.
#[tokio::test(start_paused = true)]
async fn an_unproven_aggregator_send_keeps_its_signature_for_reconciliation() {
    let signer = Keypair::new();
    let build = aggregator_build(&signer, &Pubkey::new_unique());
    let node = ScriptedNode::new(150_000, vec![SendAnswer::TimedOut], vec![Ok(None)], 0);
    let error = submit(&node, &build, &signer, None)
        .await
        .expect_err("an unseen signature is not a confirmed swap");

    let sent = node.sent();
    let signed = sent[0].signatures[0];
    assert!(signed.verify(signer.pubkey().as_ref(), &sent[0].message.serialize()));
    assert_eq!(
        crate::swaps::unconfirmed_swap_signature(&error),
        Some(signed.to_string())
    );
    assert!(!crate::swaps::is_fallback_safe(&error));
    assert!(
        sent.iter().all(|copy| copy == &sent[0]),
        "only the same signed bytes are ever re-broadcast"
    );

    // A node that accepted the send but answered with something that is not a
    // signature has still been handed the transaction.
    let garbled = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Fails(crate::Error::Data(
            crate::errors::DataError::ParseError {
                data_type: "signature".to_owned(),
                error: "invalid response format".to_owned(),
            },
        ))],
        vec![Ok(None)],
        0,
    );
    let error = submit(&garbled, &build, &signer, None)
        .await
        .expect_err("unconfirmed");
    assert_eq!(
        crate::swaps::unconfirmed_swap_signature(&error),
        Some(garbled.sent()[0].signatures[0].to_string())
    );
}

/// The same unproven send settles to a swap when its signature confirms, and
/// to the one safe retry when the build's blockhash provably expired unseen.
#[tokio::test(start_paused = true)]
async fn an_unproven_aggregator_send_settles_from_chain_state() {
    let signer = Keypair::new();
    let build = aggregator_build(&signer, &Pubkey::new_unique());

    let landed = ScriptedNode::new(
        150_000,
        vec![SendAnswer::TimedOut],
        vec![Ok(None), Ok(Some(status(Level::Confirmed, None)))],
        0,
    );
    let signature = submit(&landed, &build, &signer, None)
        .await
        .expect("a delivered send that confirms is a swap");
    assert_eq!(signature, landed.sent()[0].signatures[0]);

    let expired = ScriptedNode::new(150_000, vec![SendAnswer::TimedOut], vec![Ok(None)], 501);
    let error = submit(&expired, &build, &signer, Some(500))
        .await
        .expect_err("an expired transaction never landed");
    assert!(matches!(
        error,
        crate::Error::Solana(crate::chains::solana::Error::Execution(
            crate::chains::ExecutionFailure::Expired { .. }
        ))
    ));
    assert_eq!(crate::swaps::unconfirmed_swap_signature(&error), None);
    assert!(crate::swaps::is_fallback_safe(&error));
}

/// A swap the chain reverted at `Confirmed` moved nothing and can never land:
/// it is the one sent outcome a caller may quote and send again.
#[tokio::test(start_paused = true)]
async fn a_swap_reverted_at_confirmed_may_be_sent_again() {
    let signer = Keypair::new();
    let node = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Accepted],
        vec![Ok(Some(status(
            Level::Confirmed,
            Some(TransactionError::AccountInUse),
        )))],
        0,
    );
    let error = submit(
        &node,
        &aggregator_build(&signer, &Pubkey::new_unique()),
        &signer,
        None,
    )
    .await
    .expect_err("reverted");
    assert!(matches!(
        error,
        crate::Error::Solana(crate::chains::solana::Error::Execution(
            crate::chains::ExecutionFailure::Reverted { .. }
        ))
    ));
    assert!(crate::swaps::is_fallback_safe(&error));
}

/// The send carries the build exactly: only the compute limit moves, so the
/// platform fee transfer reaches the node byte-for-byte as the router built it.
#[tokio::test(start_paused = true)]
async fn the_sent_swap_differs_from_its_build_only_in_the_compute_limit() {
    let signer = Keypair::new();
    let fee_wallet = Pubkey::new_unique();
    let build = aggregator_build(&signer, &fee_wallet);
    let node = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Accepted],
        vec![Ok(Some(status(Level::Confirmed, None)))],
        0,
    );
    submit(&node, &build, &signer, None)
        .await
        .expect("the swap lands");
    let sent = node.sent();
    assert_eq!(sent.len(), 1, "a confirmed swap is sent once");
    let (VersionedMessage::Legacy(built), VersionedMessage::Legacy(sent)) =
        (&build.message, &sent[0].message)
    else {
        panic!("legacy in, legacy out");
    };
    assert_eq!(built.header, sent.header);
    assert_eq!(built.account_keys, sent.account_keys);
    assert_eq!(built.recent_blockhash, sent.recent_blockhash);
    assert_eq!(built.instructions[1..], sent.instructions[1..]);
    assert_eq!(
        built.instructions[0].accounts,
        sent.instructions[0].accounts
    );
    assert_eq!(
        sent.instructions[0].data[1..5],
        compute_unit_limit_from_measured(150_000).to_le_bytes()
    );
    let fee = &sent.instructions[2];
    assert_eq!(
        sent.account_keys[usize::from(fee.accounts[1])],
        fee_wallet,
        "the fee still goes to the fee account"
    );
}

/// The gate only ever supplies the wallet's own signature, after it has
/// rewritten the message; a build that needs a second signer is refused before
/// anything is simulated or sent.
#[tokio::test]
async fn a_build_that_needs_a_second_signer_is_refused_before_the_send() {
    let signer = Keypair::new();
    let co_signer = Pubkey::new_unique();
    let message = Message::new_with_blockhash(
        &[Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                crate::chains::solana::solana_sdk::instruction::AccountMeta::new(co_signer, true),
            ],
            data: vec![1],
        }],
        Some(&signer.pubkey()),
        &Hash::new_unique(),
    );
    assert_eq!(message.header.num_required_signatures, 2);
    let build = VersionedTransaction {
        signatures: vec![Signature::default(); 2],
        message: VersionedMessage::Legacy(message),
    };
    let node = ScriptedNode::new(1, vec![SendAnswer::Accepted], vec![Ok(None)], 0);
    let error = submit(&node, &build, &signer, None)
        .await
        .expect_err("refused");
    assert!(matches!(
        error,
        crate::Error::Swaps(SwapExecutionError::NotSubmitted {
            reason: NotSubmittedReason::BuildUnusable { .. },
            ..
        })
    ));
    assert!(crate::swaps::is_fallback_safe(&error));
    assert!(node.sent().is_empty());
}

/// A node that cannot simulate is not evidence about the transaction: the
/// swap proceeds with exactly one send, untightened, because there is no
/// measurement to tighten from.
#[tokio::test(start_paused = true)]
async fn an_unavailable_simulation_proceeds_with_one_untightened_send() {
    let signer = Keypair::new();
    let node = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Accepted],
        vec![Ok(Some(status(Level::Confirmed, None)))],
        0,
    )
    .without_simulation(RpcError::Network {
        message: "reset".to_owned(),
        is_timeout: false,
    });
    submit(
        &node,
        &aggregator_build(&signer, &Pubkey::new_unique()),
        &signer,
        None,
    )
    .await
    .expect("the swap proceeds and lands");
    let sent = node.sent();
    assert_eq!(sent.len(), 1);
    let VersionedMessage::Legacy(message) = &sent[0].message else {
        panic!("legacy");
    };
    assert_eq!(message.instructions[0].data[1..5], CEILING.to_le_bytes());
}

/// A failure seen at `Processed` on an abandoned fork, then the same
/// signature confirmed cleanly, is a landed swap.
#[tokio::test(start_paused = true)]
async fn a_processed_failure_on_an_abandoned_fork_is_not_terminal() {
    let signer = Keypair::new();
    let mut transaction = aggregator_build(&signer, &Pubkey::new_unique());
    transaction.signatures[0] = signer.sign_message(&transaction.message.serialize());
    let node = ScriptedNode::new(
        0,
        vec![SendAnswer::Accepted],
        vec![
            Ok(Some(status(
                Level::Processed,
                Some(TransactionError::AccountInUse),
            ))),
            Ok(Some(status(Level::Confirmed, None))),
        ],
        0,
    );
    let (signature, settled) =
        send_and_settle(&node, &transaction, Some(1_000), Duration::from_secs(5))
            .await
            .expect("sent");
    assert_eq!(signature, transaction.signatures[0]);
    assert_eq!(settled, Settled::Landed);
}

/// A first send whose preflight the node ran and saw fail was never
/// forwarded: the swap is resendable after exactly one send, so an exit can
/// climb its slippage ladder and an entry holds no pending slot. The same
/// refusal after an attempt that may have delivered the transaction proves
/// nothing, and the swap is reconciled by its signature.
#[tokio::test(start_paused = true)]
async fn a_failed_send_preflight_is_never_sent_only_from_the_first_node() {
    let slippage = || RpcError::ProviderError {
        code: -32002,
        message: "Transaction simulation failed: Error processing Instruction 2: custom \
                  program error: 0x1771"
            .to_owned(),
        data: None,
    };
    let signer = Keypair::new();
    let build = aggregator_build(&signer, &Pubkey::new_unique());

    let refused = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Fails(crate::Error::Rpc(slippage()))],
        vec![Ok(None)],
        0,
    );
    let error = submit(&refused, &build, &signer, Some(1_000))
        .await
        .expect_err("a refused send is not a swap");
    assert!(matches!(
        crate::swaps::failed_swap(&error),
        crate::swaps::FailedSwap::Resendable
    ));
    assert_eq!(
        refused.sent().len(),
        1,
        "nothing is re-broadcast after a refusal"
    );

    let after_delivery = ScriptedNode::new(
        150_000,
        vec![SendAnswer::Fails(crate::Error::Rpc(
            RpcError::RefusedAfterDelivery {
                earlier_attempts: 1,
                refusal: Box::new(slippage()),
            },
        ))],
        vec![Ok(None)],
        0,
    );
    let error = submit(&after_delivery, &build, &signer, Some(1_000))
        .await
        .expect_err("an unseen signature is not a confirmed swap");
    assert!(matches!(
        crate::swaps::failed_swap(&error),
        crate::swaps::FailedSwap::Reconcile { signature }
            if signature == after_delivery.sent()[0].signatures[0].to_string()
    ));
}

/// Settles `node`'s answers for a transaction whose blockhash was valid to
/// block 500, on a node whose finalized tip is past it.
async fn settled_past_expiry(node: &ScriptedNode) -> Settled {
    let signer = Keypair::new();
    let mut transaction = aggregator_build(&signer, &Pubkey::new_unique());
    transaction.signatures[0] = signer.sign_message(&transaction.message.serialize());
    let (_, settled) = send_and_settle(node, &transaction, Some(500), Duration::from_secs(30))
        .await
        .expect("sent");
    settled
}

/// A node that returned a status for the signature saw it, at whatever
/// level: past the expiry, a `processed` status keeps the swap unknown.
#[tokio::test(start_paused = true)]
async fn a_signature_seen_at_processed_is_never_declared_expired() {
    let node = ScriptedNode::new(
        0,
        vec![SendAnswer::Accepted],
        vec![Ok(Some(status(Level::Processed, None)))],
        501,
    );
    assert!(matches!(
        settled_past_expiry(&node).await,
        Settled::Unsettled { .. }
    ));
}

/// A status reader that had not reached the slot the expiry was read at may
/// not hold the block with the transaction yet: its null proves nothing.
#[tokio::test(start_paused = true)]
async fn a_status_reader_behind_the_expiry_never_proves_a_swap_dead() {
    let node =
        ScriptedNode::new(0, vec![SendAnswer::Accepted], vec![Ok(None)], 501).reading_at(90, 100);
    assert!(matches!(
        settled_past_expiry(&node).await,
        Settled::Unsettled { .. }
    ));
}

/// A null status past the expiry is checked against the chain's own record
/// of the transaction before the swap is declared dead.
#[tokio::test(start_paused = true)]
async fn an_unseen_signature_the_chain_executed_is_decided_by_its_transaction() {
    let landed = ScriptedNode::new(0, vec![SendAnswer::Accepted], vec![Ok(None)], 501)
        .holding(serde_json::Value::Null);
    assert_eq!(settled_past_expiry(&landed).await, Settled::Landed);

    let reverted = ScriptedNode::new(0, vec![SendAnswer::Accepted], vec![Ok(None)], 501)
        .holding(serde_json::json!({ "InstructionError": [0, { "Custom": 1 }] }));
    assert!(matches!(
        settled_past_expiry(&reverted).await,
        Settled::Reverted { .. }
    ));

    let missing = ScriptedNode::new(0, vec![SendAnswer::Accepted], vec![Ok(None)], 501);
    assert!(matches!(
        settled_past_expiry(&missing).await,
        Settled::Expired {
            last_valid_block_height: 500,
            current_block_height: 501
        }
    ));
}
