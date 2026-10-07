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
    }
}

/// Only a node that answered may refuse a transaction; every other failure is
/// the one fail-open verdict. A send is refused on exactly the same answers.
#[test]
fn only_a_node_that_answered_refuses_a_transaction() {
    let provider = |code: i64| RpcError::ProviderError {
        code,
        message: "base64 encoded VersionedTransaction too large".to_owned(),
        data: None,
    };
    let rows: Vec<(RpcError, bool)> = vec![
        (
            RpcError::RateLimited {
                provider_id: "p".to_owned(),
                retry_after: None,
            },
            false,
        ),
        (
            RpcError::Network {
                message: "reset".to_owned(),
                is_timeout: true,
            },
            false,
        ),
        (provider(-32602), true),
        (provider(-32600), true),
        (provider(-32700), true),
        (provider(-32002), false),
        (provider(-32005), false),
        (
            RpcError::Timeout {
                provider_id: "p".to_owned(),
                after: Duration::from_secs(1),
            },
            false,
        ),
        (
            RpcError::CircuitOpen {
                provider_id: "p".to_owned(),
                retry_after: Duration::from_secs(1),
            },
            false,
        ),
        (RpcError::NoProvidersAvailable { last_error: None }, false),
        (
            RpcError::AccountNotFound {
                pubkey: "k".to_owned(),
            },
            false,
        ),
        (
            RpcError::InvalidResponse {
                message: "garbled".to_owned(),
            },
            false,
        ),
        (
            RpcError::Configuration {
                message: "bad".to_owned(),
            },
            false,
        ),
        (RpcError::Other("other".to_owned()), false),
    ];
    let mut covered: Vec<&str> = rows.iter().map(|(error, _)| rpc_variant(error)).collect();
    covered.sort_unstable();
    covered.dedup();
    assert_eq!(covered.len(), 10, "every RpcError variant needs a row");

    for (error, refused) in rows {
        let label = error.to_string();
        let verdict = simulation_verdict(Err(crate::Error::Rpc(error.clone())));
        match (&verdict, refused) {
            (Verdict::Refused(NotSubmittedReason::RequestRejected { .. }), true)
            | (Verdict::NodeUnavailable { .. }, false) => {}
            _ => panic!("{label}: unexpected verdict {verdict:?}"),
        }
        let send = send_failure(crate::Error::Rpc(error));
        match (&send, refused) {
            (SendFailure::NotSent(NotSubmittedReason::RequestRejected { .. }), true)
            | (SendFailure::Unproven(_), false) => {}
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
