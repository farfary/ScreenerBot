// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The signing gate against recorded builds, synthetic messages and simulated
//! runs, decided without any node or network call.

use std::collections::HashMap;

use base64::Engine;
use serde_json::{json, Value};

use super::*;
use crate::chains::solana::constants::{
    JUPITER_V6_PROGRAM_ID, LAMPORTS_PER_SOL, SOL_MINT, SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID, USDC_MINT,
};
use crate::chains::solana::rpc::types::LoadedAddresses;
use crate::chains::solana::solana_sdk::{
    account::Account,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::{v0, AddressLookupTableAccount, Message, VersionedMessage},
    signature::Signature,
};
use crate::chains::solana::solana_system_interface::instruction::transfer as system_transfer;
use crate::chains::solana::spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use crate::chains::solana::spl_token;
use crate::chains::solana::swaps::presend::scripted_tests::ScriptedNode;
use crate::chains::solana::swaps::revenue::FEE_WALLET;
use crate::chains::solana::transactions::program_ids::{
    is_jupiter_program_id, KNOWN_PROGRAMS, RAPTOR_PROGRAM_ID,
};

const WALLET: &str = "6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv";
const VENUE: &str = "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp";
const VENUE_ACCOUNT: &str = "GVHuUhKQYx3LfoquXK9gmCmEBUM3nMaeFHuonhYASiZt";
const OUR_ATA: &str = "CUzfpKAieLAgVzNjXM4mH2fuh9vTrExnGKGooEUKY8TK";
const STRANGER: &str = "AgmLJBMDN6vCXqVqHUcgDqXKRZKPQvmdKfKhVpVLRZ2r";
const MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// Recorded `lite-api.jup.ag` builds of a 0.05 SOL buy of PUMP for `WALLET`.
const JUPITER_BUILDS: &str =
    include_str!("../../../../../tests/fixtures/swaps/jupiter_sol_pump_builds.json");

fn group(instructions: Vec<Value>) -> Value {
    json!({"index": 6, "instructions": instructions})
}

fn system(kind: &str, info: Value) -> Value {
    json!({
        "program": "system",
        "programId": SYSTEM_PROGRAM_ID,
        "parsed": {"type": kind, "info": info},
        "stackHeight": 2,
    })
}

fn token(kind: &str, info: Value) -> Value {
    json!({
        "program": "spl-token",
        "programId": SPL_TOKEN_PROGRAM_ID,
        "parsed": {"type": kind, "info": info},
        "stackHeight": 2,
    })
}

fn transfer(source: &str, destination: &str, lamports: u64) -> Value {
    system(
        "transfer",
        json!({"source": source, "destination": destination, "lamports": lamports}),
    )
}

fn create_account(source: &str, new_account: &str, owner: &str, lamports: u64) -> Value {
    system(
        "createAccount",
        json!({
            "source": source,
            "newAccount": new_account,
            "owner": owner,
            "lamports": lamports,
            "space": 165,
        }),
    )
}

fn initialize_account(account: &str, owner: &str) -> Value {
    token(
        "initializeAccount3",
        json!({"account": account, "mint": MINT, "owner": owner}),
    )
}

fn settings() -> CostGuardSettings {
    CostGuardSettings {
        enabled: true,
        max_extra_cost_pct: 1.0,
        always_allow_below_lamports: 100_000,
        retry_excluding_venue: true,
    }
}

fn wallet() -> Pubkey {
    WALLET.parse().unwrap()
}

fn key(address: &str) -> Pubkey {
    address.parse().unwrap()
}

fn quote(router: &str, input_mint: &str, output_mint: &str, input: u64, output: u64) -> Quote {
    crate::config::utils::install_default_config();
    Quote {
        chain: crate::chains::ChainId::Solana,
        router_id: router.to_owned(),
        router_name: router.to_owned(),
        input_mint: input_mint.to_owned(),
        output_mint: output_mint.to_owned(),
        input_amount: input.into(),
        output_amount: output.into(),
        minimum_output_amount: output.into(),
        price_impact_pct: 0.1,
        platform_fee_lamports: None,
        estimated_network_fee_lamports: None,
        slippage_bps: 100,
        route_plan: "test".to_owned(),
        swap_mode: crate::swaps::SwapMode::ExactIn,
        wallet_address: WALLET.to_owned(),
        exclude_dexes: None,
        execution_data: Vec::new(),
    }
}

/// A 0.05 SOL buy of 950 units of `MINT`. Its fee bound is 250,000 lamports
/// and its allowance 500,000.
fn buy(router: &str) -> Quote {
    quote(router, SOL_MINT, MINT, 50_000_000, 950)
}

/// A sale of 1,000 units of `MINT` for at least 0.0495 SOL.
fn sale(router: &str) -> Quote {
    Quote {
        output_amount: 50_000_000u64.into(),
        ..quote(router, MINT, SOL_MINT, 1_000, 49_500_000)
    }
}

fn budget() -> Vec<Instruction> {
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        ComputeBudgetInstruction::set_compute_unit_price(50_000),
    ]
}

fn trade(program: &str) -> Instruction {
    Instruction {
        program_id: key(program),
        accounts: vec![AccountMeta::new(wallet(), true)],
        data: vec![1, 2, 3],
    }
}

/// An unsigned legacy build paid by `WALLET`.
fn build(instructions: &[Instruction]) -> VersionedTransaction {
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::Legacy(Message::new_with_blockhash(
            instructions,
            Some(&wallet()),
            &Hash::new_unique(),
        )),
    }
}

/// The budget, `extra` and a Jupiter trade.
fn jupiter_build(extra: Vec<Instruction>) -> VersionedTransaction {
    let mut instructions = budget();
    instructions.extend(extra);
    instructions.push(trade(JUPITER_V6_PROGRAM_ID));
    build(&instructions)
}

fn wrapped_sol_account() -> Pubkey {
    get_associated_token_address_with_program_id(&wallet(), &key(SOL_MINT), &spl_token::id())
}

fn ran(groups: Vec<Value>) -> SimulationOutcome {
    SimulationOutcome {
        inner_instructions: groups,
        ..Default::default()
    }
}

async fn verdict(
    transaction: &VersionedTransaction,
    simulated: Simulated<'_>,
    quote: &Quote,
    owners: HashMap<String, AccountControl>,
) -> std::result::Result<(), Refusal> {
    decide(
        "Jupiter",
        transaction,
        simulated,
        quote,
        &settings(),
        None,
        move |_| async move { Some(owners) },
    )
    .await
}

fn is_unusable(verdict: &std::result::Result<(), Refusal>) -> bool {
    matches!(verdict, Err(Refusal::Unusable(_)))
}

// ----------------------------------------------------------------------------
// Inner instructions
// ----------------------------------------------------------------------------

/// The shape that cost real money: an aggregator's route transfers rent out
/// of the trader's wallet and the venue's program then takes the account
/// over, all inside one swap the quote described as 0.005 SOL.
#[test]
fn rent_a_venue_takes_over_mid_swap_is_charged_to_that_venue() {
    let groups = vec![group(vec![
        transfer(WALLET, VENUE_ACCOUNT, 13_045_440),
        system("allocate", json!({"account": VENUE_ACCOUNT, "space": 2440})),
        system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
    ])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 13_045_440);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
    assert_eq!(assessment.recoverable_rent_lamports, 0);
    assert!(assessment.unresolved.is_empty());
}

#[test]
fn rent_for_the_wallets_own_token_accounts_is_recoverable_not_a_cost() {
    let groups = vec![group(vec![
        create_account(WALLET, OUR_ATA, SPL_TOKEN_PROGRAM_ID, 1_488_440),
        initialize_account(OUR_ATA, WALLET),
        create_account(WALLET, VENUE_ACCOUNT, TOKEN_2022_PROGRAM_ID, 1_539_240),
        initialize_account(VENUE_ACCOUNT, WALLET),
    ])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 0);
    assert_eq!(assessment.recoverable_rent_lamports, 3_027_680);
    assert_eq!(assessment.venue_program, None);
}

#[test]
fn an_account_created_straight_into_a_third_party_program_is_charged() {
    let groups = vec![group(vec![create_account(
        WALLET,
        VENUE_ACCOUNT,
        VENUE,
        2_500_000,
    )])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 2_500_000);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
}

#[test]
fn lamports_somebody_else_pays_are_never_charged_to_this_wallet() {
    let groups = vec![group(vec![
        transfer(STRANGER, VENUE_ACCOUNT, 13_045_440),
        system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
        create_account(STRANGER, OUR_ATA, SPL_TOKEN_PROGRAM_ID, 1_488_440),
    ])];

    assert_eq!(
        assess_inner_instructions(WALLET, &groups),
        SwapCostAssessment::default()
    );
}

#[test]
fn a_transfer_with_no_assign_is_reported_unresolved_rather_than_guessed_at() {
    let groups = vec![group(vec![transfer(WALLET, VENUE_ACCOUNT, 900_000)])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 0);
    assert_eq!(
        assessment.unresolved,
        vec![(VENUE_ACCOUNT.to_owned(), 900_000)]
    );
}

#[test]
fn an_unresolved_account_is_settled_by_the_owner_it_has_on_chain() {
    let mut third_party = SwapCostAssessment {
        unresolved: vec![(VENUE_ACCOUNT.to_owned(), 900_000)],
        ..Default::default()
    };
    attribute_unresolved(
        &mut third_party,
        WALLET,
        &HashMap::from([(
            VENUE_ACCOUNT.to_owned(),
            AccountControl::Program(VENUE.to_owned()),
        )]),
    );
    assert_eq!(third_party.unrecoverable_lamports, 900_000);
    assert_eq!(third_party.venue_program.as_deref(), Some(VENUE));

    let mut token_account = SwapCostAssessment {
        unresolved: vec![(OUR_ATA.to_owned(), 1_488_440)],
        ..Default::default()
    };
    attribute_unresolved(
        &mut token_account,
        WALLET,
        &HashMap::from([(
            OUR_ATA.to_owned(),
            AccountControl::TokenAccount {
                program: SPL_TOKEN_PROGRAM_ID.to_owned(),
                authority: WALLET.to_owned(),
            },
        )]),
    );
    assert_eq!(token_account.unrecoverable_lamports, 0);
    assert_eq!(token_account.recoverable_rent_lamports, 1_488_440);

    // A tip to a plain wallet, and an account no node knows, are both left
    // alone: neither is rent locked behind someone else's program.
    let mut plain = SwapCostAssessment {
        unresolved: vec![
            (VENUE_ACCOUNT.to_owned(), 10_000),
            (OUR_ATA.to_owned(), 20_000),
        ],
        ..Default::default()
    };
    attribute_unresolved(
        &mut plain,
        WALLET,
        &HashMap::from([(
            VENUE_ACCOUNT.to_owned(),
            AccountControl::Program(SYSTEM_PROGRAM_ID.to_owned()),
        )]),
    );
    assert_eq!(plain.unrecoverable_lamports, 0);
    assert_eq!(plain.venue_program, None);
    assert_eq!(plain.extra_cost_lamports(), 30_000);
}

/// A plain system account holds no program's state, but lamports sent to
/// one are gone all the same: a route may tip or pay a fee, never move a
/// multiple of the trade out of the wallet.
#[test]
fn lamports_sent_to_a_plain_wallet_beyond_the_allowance_are_a_cost() {
    let trade_lamports = 50_000_000;
    let allowance = allowance_lamports(trade_lamports, &settings());
    let owners = HashMap::from([(
        STRANGER.to_owned(),
        AccountControl::Program(SYSTEM_PROGRAM_ID.to_owned()),
    )]);

    // The size of a platform fee on the trade stays inside the allowance.
    let mut fee =
        assess_inner_instructions(WALLET, &[group(vec![transfer(WALLET, STRANGER, 250_000)])]);
    attribute_unresolved(&mut fee, WALLET, &owners);
    assert!(fee.extra_cost_lamports() <= allowance);

    let mut drain = assess_inner_instructions(
        WALLET,
        &[group(vec![transfer(WALLET, STRANGER, 5_000_000_000)])],
    );
    attribute_unresolved(&mut drain, WALLET, &owners);
    assert!(
        drain.extra_cost_lamports() > allowance,
        "5 SOL sent to a plain wallet during a 0.05 SOL trade must exceed the {allowance} \
         lamport allowance, got {}",
        drain.extra_cost_lamports()
    );
}

#[test]
fn charges_from_instructions_and_from_chain_state_merge_under_one_venue() {
    let mut assessment = SwapCostAssessment {
        unrecoverable_lamports: 13_045_440,
        venue_program: Some(VENUE.to_owned()),
        unresolved: vec![(OUR_ATA.to_owned(), 1_000_000)],
        ..Default::default()
    };

    attribute_unresolved(
        &mut assessment,
        WALLET,
        &HashMap::from([(
            OUR_ATA.to_owned(),
            AccountControl::Program(VENUE.to_owned()),
        )]),
    );

    assert_eq!(assessment.unrecoverable_lamports, 14_045_440);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
}

#[test]
fn the_venue_named_is_the_one_that_took_the_most() {
    let smaller = "TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH";
    let groups = vec![group(vec![
        create_account(WALLET, OUR_ATA, smaller, 2_000_000),
        create_account(WALLET, VENUE_ACCOUNT, VENUE, 13_045_440),
    ])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 15_045_440);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
}

#[test]
fn instructions_are_read_across_every_inner_group_and_odd_shapes_are_survived() {
    let groups = vec![
        json!({"index": 2, "instructions": []}),
        json!({"index": 3}),
        json!("not a group"),
        group(vec![
            // A token-program instruction: not a lamport movement.
            json!({
                "program": "spl-token",
                "programId": SPL_TOKEN_PROGRAM_ID,
                "parsed": {"type": "transfer", "info": {"amount": "4975000"}},
            }),
            // Unparsed inner instruction (no `parsed` field at all).
            json!({"programId": SYSTEM_PROGRAM_ID, "accounts": [], "data": "3Bxs"}),
            // Lamports as a string, which some nodes emit.
            system(
                "transfer",
                json!({"source": WALLET, "destination": VENUE_ACCOUNT, "lamports": "13045440"}),
            ),
            system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
        ]),
    ];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 13_045_440);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
}

#[test]
fn a_run_that_moves_nothing_out_of_the_wallet_costs_nothing() {
    let groups: Vec<Value> = vec![group(vec![json!({
        "program": "spl-token",
        "programId": SPL_TOKEN_PROGRAM_ID,
        "parsed": {"type": "transfer", "info": {"amount": "4975000"}},
    })])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert!(assessment.is_empty());
    assert_eq!(assess_inner_instructions(WALLET, &[]), assessment);
}

/// A token account the wallet pays for but another authority controls is
/// closable only by that authority. The recorded case: a Raptor buy created
/// such an account for its own authority out of the wallet's lamports.
#[tokio::test]
async fn a_token_account_created_for_another_authority_is_charged_to_that_authority() {
    const ACCOUNT: &str = "AcEyePa7h3LcuNhvNZSxVbMp2sU6JFUe3yBtkqQ1UtvW";
    const AUTHORITY: &str = "G32FX97JkKjz1XQw6yeR4Yp3rJGtCUtu38fyemGVzBcy";
    let groups = vec![group(vec![
        create_account(WALLET, ACCOUNT, SPL_TOKEN_PROGRAM_ID, 1_488_440),
        token(
            "initializeAccount3",
            json!({
                "account": ACCOUNT,
                "mint": "AT13ipG8K4HDyEJm3H1ZVQV2Bw8sihewoLQReSMsYo3o",
                "owner": AUTHORITY,
            }),
        ),
    ])];

    let assessment = assess_inner_instructions(WALLET, &groups);
    assert_eq!(assessment.unrecoverable_lamports, 1_488_440);
    assert_eq!(assessment.recoverable_rent_lamports, 0);
    assert_eq!(assessment.venue_program.as_deref(), Some(AUTHORITY));

    let quote = quote("raptor", SOL_MINT, MINT, 4_000_000, 950);
    let mut instructions = budget();
    instructions.push(trade(RAPTOR_PROGRAM_ID));
    let outcome = ran(groups);
    let refused = decide(
        "Raptor",
        &build(&instructions),
        Simulated::Ran(&outcome),
        &quote,
        &CostGuardSettings::current(),
        None,
        |_| async { Some(HashMap::new()) },
    )
    .await;
    assert_eq!(
        refused,
        Err(Refusal::Cost {
            extra: 1_488_440,
            allowance: 100_000,
            venue_address: AUTHORITY.to_owned(),
        })
    );
}

#[test]
fn a_token_account_created_and_never_initialized_is_a_cost() {
    let groups = vec![group(vec![create_account(
        WALLET,
        OUR_ATA,
        SPL_TOKEN_PROGRAM_ID,
        1_488_440,
    )])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.unrecoverable_lamports, 1_488_440);
    assert_eq!(assessment.recoverable_rent_lamports, 0);
}

#[test]
fn a_token_account_created_and_closed_back_to_the_wallet_costs_nothing() {
    let groups = vec![group(vec![
        create_account(WALLET, OUR_ATA, SPL_TOKEN_PROGRAM_ID, 2_039_280),
        initialize_account(OUR_ATA, WALLET),
        token(
            "closeAccount",
            json!({"account": OUR_ATA, "destination": WALLET, "owner": WALLET}),
        ),
    ])];

    let assessment = assess_inner_instructions(WALLET, &groups);

    assert_eq!(assessment.extra_cost_lamports(), 0);
    assert_eq!(assessment.recoverable_rent_lamports, 0);
    assert!(assessment.grants.is_empty());
}

#[test]
fn an_approval_is_a_grant_and_one_revoked_in_the_run_is_not() {
    let approve = token(
        "approve",
        json!({"source": OUR_ATA, "delegate": STRANGER, "owner": WALLET, "amount": "1000"}),
    );
    let granted = assess_inner_instructions(WALLET, &[group(vec![approve.clone()])]);
    assert_eq!(granted.grants.len(), 1);

    let revoked = assess_inner_instructions(
        WALLET,
        &[group(vec![
            approve,
            token("revoke", json!({"source": OUR_ATA, "owner": WALLET})),
        ])],
    );
    assert!(revoked.grants.is_empty());
}

#[test]
fn an_authority_change_or_an_assignment_of_the_wallet_is_a_grant() {
    let authority = assess_inner_instructions(
        WALLET,
        &[group(vec![token(
            "setAuthority",
            json!({
                "account": OUR_ATA,
                "authority": WALLET,
                "newAuthority": STRANGER,
                "authorityType": "accountOwner",
            }),
        )])],
    );
    assert_eq!(authority.grants.len(), 1);

    let assigned = assess_inner_instructions(
        WALLET,
        &[group(vec![system(
            "assign",
            json!({"account": WALLET, "owner": VENUE}),
        )])],
    );
    assert_eq!(assigned.grants.len(), 1);
}

#[test]
fn lamports_into_an_existing_token_account_of_another_authority_are_not_recoverable() {
    let mut assessment =
        assess_inner_instructions(WALLET, &[group(vec![transfer(WALLET, OUR_ATA, 700_000)])]);
    attribute_unresolved(
        &mut assessment,
        WALLET,
        &HashMap::from([(
            OUR_ATA.to_owned(),
            AccountControl::TokenAccount {
                program: SPL_TOKEN_PROGRAM_ID.to_owned(),
                authority: VENUE.to_owned(),
            },
        )]),
    );

    assert_eq!(assessment.recoverable_rent_lamports, 0);
    assert_eq!(assessment.unrecoverable_lamports, 700_000);
    assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
}

/// The platform fee is part of the trade up to its bound; only what a run
/// pays the fee wallet beyond it counts against the allowance.
#[tokio::test]
async fn the_fee_wallet_fee_is_free_up_to_its_bound_and_a_cost_beyond_it() {
    let quote = buy("jupiter");
    let transaction = jupiter_build(Vec::new());

    let fee = ran(vec![group(vec![transfer(WALLET, FEE_WALLET, 250_000)])]);
    assert_eq!(
        verdict(&transaction, Simulated::Ran(&fee), &quote, HashMap::new()).await,
        Ok(())
    );

    let overcharge = ran(vec![group(vec![transfer(WALLET, FEE_WALLET, 2_000_000)])]);
    assert_eq!(
        verdict(
            &transaction,
            Simulated::Ran(&overcharge),
            &quote,
            HashMap::new()
        )
        .await,
        Err(Refusal::Cost {
            extra: 1_750_000,
            allowance: 500_000,
            venue_address: String::new(),
        })
    );
}

/// A failed owner lookup is not evidence of a cost: the unattributed transfer
/// is left out of the comparison instead of being charged as a payment.
#[tokio::test]
async fn a_failed_owner_lookup_leaves_the_cost_unattributed() {
    let outcome = ran(vec![group(vec![transfer(WALLET, VENUE_ACCOUNT, 900_000)])]);
    let unanswered = decide(
        "Jupiter",
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &buy("jupiter"),
        &settings(),
        None,
        |_| async { None },
    )
    .await;
    assert_eq!(unanswered, Ok(()));

    let answered = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &buy("jupiter"),
        HashMap::new(),
    )
    .await;
    assert!(matches!(
        answered,
        Err(Refusal::Cost { extra: 900_000, .. })
    ));
}

// ----------------------------------------------------------------------------
// The message
// ----------------------------------------------------------------------------

#[test]
fn the_recorded_jupiter_builds_pass_the_inspection() {
    let builds: Value = serde_json::from_str(JUPITER_BUILDS).expect("fixture is JSON");
    assert_eq!(builds["user"], WALLET);
    let quote = buy("jupiter");
    let expectations = BuildExpectations::for_quote(&quote, &settings()).unwrap();
    for route in ["single_hop", "multi_hop"] {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(builds[route]["transaction"].as_str().unwrap())
            .unwrap();
        let transaction: VersionedTransaction = bincode::deserialize(&bytes).unwrap();
        let keys = message::ResolvedKeys::new(&transaction.message, None);
        assert_eq!(
            message::inspect(&transaction, &keys, &quote, &expectations),
            Ok(()),
            "{route}"
        );
    }
}

#[tokio::test]
async fn a_build_of_only_what_a_swap_needs_passes() {
    let transaction = jupiter_build(vec![
        create_associated_token_account_idempotent(
            &wallet(),
            &wallet(),
            &key(MINT),
            &spl_token::id(),
        ),
        system_transfer(&wallet(), &wrapped_sol_account(), 50_000_000),
        spl_token::instruction::sync_native(&spl_token::id(), &wrapped_sol_account()).unwrap(),
        system_transfer(&wallet(), &key(FEE_WALLET), 250_000),
        spl_token::instruction::close_account(
            &spl_token::id(),
            &wrapped_sol_account(),
            &wallet(),
            &wallet(),
            &[],
        )
        .unwrap(),
    ]);
    let outcome = SimulationOutcome::default();

    assert_eq!(
        verdict(
            &transaction,
            Simulated::Ran(&outcome),
            &buy("jupiter"),
            HashMap::new()
        )
        .await,
        Ok(())
    );
}

#[tokio::test]
async fn a_top_level_instruction_a_swap_does_not_need_is_refused() {
    let quote = buy("jupiter");
    let wallet = wallet();
    let stranger = key(STRANGER);
    let account = key(OUR_ATA);
    let price = with_config(|cfg| cfg.chains.solana.swaps.jupiter.priority_fee_micro_lamports);
    let cases: Vec<(&str, VersionedTransaction)> = vec![
        (
            "a transfer outside the trade",
            jupiter_build(vec![system_transfer(&wallet, &stranger, 1_000)]),
        ),
        ("a program of no router", jupiter_build(vec![trade(VENUE)])),
        (
            "a program of another router",
            jupiter_build(vec![trade(RAPTOR_PROGRAM_ID)]),
        ),
        (
            "a compute price above the configured one",
            build(&[
                ComputeBudgetInstruction::set_compute_unit_price(price + 1),
                trade(JUPITER_V6_PROGRAM_ID),
            ]),
        ),
        (
            "an associated account for another owner",
            jupiter_build(vec![create_associated_token_account_idempotent(
                &wallet,
                &stranger,
                &key(MINT),
                &spl_token::id(),
            )]),
        ),
        (
            "a close that pays another address",
            jupiter_build(vec![spl_token::instruction::close_account(
                &spl_token::id(),
                &account,
                &stranger,
                &wallet,
                &[],
            )
            .unwrap()]),
        ),
        (
            "an approval",
            jupiter_build(vec![spl_token::instruction::approve(
                &spl_token::id(),
                &account,
                &stranger,
                &wallet,
                &[],
                1,
            )
            .unwrap()]),
        ),
        (
            "an authority change",
            jupiter_build(vec![spl_token::instruction::set_authority(
                &spl_token::id(),
                &account,
                Some(&stranger),
                spl_token::instruction::AuthorityType::AccountOwner,
                &wallet,
                &[],
            )
            .unwrap()]),
        ),
        (
            "a wrap larger than the input",
            jupiter_build(vec![system_transfer(
                &wallet,
                &wrapped_sol_account(),
                50_000_001,
            )]),
        ),
        (
            "a fee above the platform fee",
            jupiter_build(vec![system_transfer(&wallet, &key(FEE_WALLET), 250_001)]),
        ),
    ];
    let outcome = SimulationOutcome::default();
    for (case, transaction) in cases {
        let refused = verdict(
            &transaction,
            Simulated::Ran(&outcome),
            &quote,
            HashMap::new(),
        )
        .await;
        assert!(is_unusable(&refused), "{case}: {refused:?}");
    }
}

/// A v0 build whose wrap destination is loaded from a lookup table.
fn wrap_through_a_lookup_table() -> VersionedTransaction {
    let table = AddressLookupTableAccount {
        key: Pubkey::new_unique(),
        addresses: vec![wrapped_sol_account()],
    };
    let mut instructions = budget();
    instructions.push(system_transfer(
        &wallet(),
        &wrapped_sol_account(),
        50_000_000,
    ));
    instructions.push(trade(JUPITER_V6_PROGRAM_ID));
    let message = v0::Message::try_compile(&wallet(), &instructions, &[table], Hash::new_unique())
        .expect("the message compiles");
    assert_eq!(message.address_table_lookups.len(), 1);
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(message),
    }
}

#[tokio::test]
async fn a_lookup_table_account_is_trusted_only_once_the_simulation_resolved_it() {
    let transaction = wrap_through_a_lookup_table();
    let quote = buy("jupiter");

    let resolved = SimulationOutcome {
        loaded_addresses: Some(LoadedAddresses {
            writable: vec![wrapped_sol_account().to_string()],
            readonly: Vec::new(),
        }),
        ..Default::default()
    };
    assert_eq!(
        verdict(
            &transaction,
            Simulated::Ran(&resolved),
            &quote,
            HashMap::new()
        )
        .await,
        Ok(())
    );

    let unresolved = SimulationOutcome::default();
    let refused = verdict(
        &transaction,
        Simulated::Ran(&unresolved),
        &quote,
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");

    // Counts that do not match the message's lookups are not trusted either.
    let mismatched = SimulationOutcome {
        loaded_addresses: Some(LoadedAddresses {
            writable: vec![wrapped_sol_account().to_string(), STRANGER.to_owned()],
            readonly: Vec::new(),
        }),
        ..Default::default()
    };
    let refused = verdict(
        &transaction,
        Simulated::Ran(&mismatched),
        &quote,
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");

    // The resolved address is what the instruction is judged by.
    let elsewhere = SimulationOutcome {
        loaded_addresses: Some(LoadedAddresses {
            writable: vec![STRANGER.to_owned()],
            readonly: Vec::new(),
        }),
        ..Default::default()
    };
    let refused = verdict(
        &transaction,
        Simulated::Ran(&elsewhere),
        &quote,
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");
}

#[test]
fn a_router_without_registered_expectations_is_refused() {
    let quote = buy("unknown");
    assert!(BuildExpectations::for_quote(&quote, &settings()).is_err());
}

// ----------------------------------------------------------------------------
// Conservation
// ----------------------------------------------------------------------------

fn token_balance(index: u32, mint: &str, amount: u64) -> Value {
    json!({
        "accountIndex": index,
        "mint": mint,
        "owner": WALLET,
        "programId": SPL_TOKEN_PROGRAM_ID,
        "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6},
    })
}

/// A run with the given lamport and token balances and a 5,000-lamport fee.
fn balances(
    pre: Vec<u64>,
    post: Vec<u64>,
    pre_tokens: Value,
    post_tokens: Value,
) -> SimulationOutcome {
    SimulationOutcome {
        fee: Some(5_000),
        pre_balances: Some(pre),
        post_balances: Some(post),
        pre_token_balances: Some(serde_json::from_value(pre_tokens).unwrap()),
        post_token_balances: Some(serde_json::from_value(post_tokens).unwrap()),
        ..Default::default()
    }
}

const START: u64 = LAMPORTS_PER_SOL;

#[tokio::test]
async fn a_run_that_takes_more_lamports_than_the_trade_explains_is_refused() {
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000 - 5_000_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 950)]),
    );
    assert_eq!(
        verdict(
            &jupiter_build(Vec::new()),
            Simulated::Ran(&outcome),
            &buy("jupiter"),
            HashMap::new()
        )
        .await,
        Err(Refusal::Cost {
            extra: 5_000_000,
            allowance: 500_000,
            venue_address: String::new(),
        })
    );
}

#[tokio::test]
async fn a_run_that_delivers_less_than_the_minimum_is_refused() {
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 949)]),
    );
    let refused = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &buy("jupiter"),
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");
}

#[test]
fn a_token_2022_output_fee_is_subtracted_from_the_minimum() {
    let quote = Quote {
        minimum_output_amount: 1_000u64.into(),
        ..buy("jupiter")
    };
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 990)]),
    );
    assert_eq!(
        balances::conserve_tokens(&outcome, &quote, Some(10)),
        Ok(())
    );

    let below = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 989)]),
    );
    assert!(balances::conserve_tokens(&below, &quote, Some(10)).is_err());
}

#[test]
fn a_plain_spl_output_keeps_the_full_quoted_minimum() {
    let quote = Quote {
        minimum_output_amount: 1_000u64.into(),
        ..buy("jupiter")
    };
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 989)]),
    );
    assert!(balances::conserve_tokens(&outcome, &quote, None).is_err());
}

#[tokio::test]
async fn a_run_that_lowers_a_mint_the_trade_does_not_spend_is_refused() {
    let other = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let outcome = balances(
        vec![START, 0, 0],
        vec![START - 50_000_000 - 5_000, 0, 0],
        json!([token_balance(1, MINT, 0), token_balance(2, other, 500)]),
        json!([token_balance(1, MINT, 950), token_balance(2, other, 0)]),
    );
    let refused = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &buy("jupiter"),
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");
}

#[tokio::test]
async fn rent_the_wallet_pays_into_its_own_new_account_nets_out() {
    let rent = 2_039_280;
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000 - rent, rent],
        json!([]),
        json!([token_balance(1, MINT, 950)]),
    );
    assert_eq!(
        verdict(
            &jupiter_build(Vec::new()),
            Simulated::Ran(&outcome),
            &buy("jupiter"),
            HashMap::new()
        )
        .await,
        Ok(())
    );
}

#[tokio::test]
async fn a_node_without_balance_fields_skips_conservation() {
    let outcome = SimulationOutcome {
        fee: Some(5_000),
        pre_balances: Some(vec![START]),
        post_balances: Some(vec![0]),
        ..Default::default()
    };
    assert_eq!(
        verdict(
            &jupiter_build(Vec::new()),
            Simulated::Ran(&outcome),
            &buy("jupiter"),
            HashMap::new()
        )
        .await,
        Ok(())
    );
}

/// Option A for a sale: the allowance is the larger of the standard share and
/// the share of the proceeds the quote's slippage already gives up.
#[test]
fn a_sale_is_allowed_the_share_of_its_proceeds_its_slippage_gives_up() {
    let settings = settings();
    let wide = Quote {
        slippage_bps: 300,
        ..sale("jupiter")
    };
    assert_eq!(
        trade_allowance_lamports(&wide, &settings).unwrap(),
        1_500_000
    );

    let narrow = Quote {
        slippage_bps: 10,
        ..sale("jupiter")
    };
    assert_eq!(
        trade_allowance_lamports(&narrow, &settings).unwrap(),
        500_000
    );

    let buy = Quote {
        slippage_bps: 300,
        ..buy("jupiter")
    };
    assert_eq!(trade_allowance_lamports(&buy, &settings).unwrap(), 500_000);
}

// ----------------------------------------------------------------------------
// No simulation
// ----------------------------------------------------------------------------

#[tokio::test]
async fn an_unavailable_simulation_refuses_a_build_that_spends_sol() {
    let refused = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Unavailable { detail: "reset" },
        &buy("jupiter"),
        HashMap::new(),
    )
    .await;
    assert_eq!(refused, Err(Refusal::Unavailable("reset".to_owned())));
}

#[tokio::test]
async fn an_unavailable_simulation_lets_a_clean_sale_proceed_and_still_inspects_it() {
    assert_eq!(
        verdict(
            &jupiter_build(Vec::new()),
            Simulated::Unavailable { detail: "reset" },
            &sale("jupiter"),
            HashMap::new(),
        )
        .await,
        Ok(())
    );

    let refused = verdict(
        &jupiter_build(vec![system_transfer(&wallet(), &key(STRANGER), 1_000)]),
        Simulated::Unavailable { detail: "reset" },
        &sale("jupiter"),
        HashMap::new(),
    )
    .await;
    assert!(is_unusable(&refused), "{refused:?}");
}

// ----------------------------------------------------------------------------
// Allowance
// ----------------------------------------------------------------------------

#[test]
fn the_allowance_is_a_share_of_the_trade_with_a_floor_under_it() {
    let settings = settings();
    // 1% of a 1 SOL trade, well above the floor.
    assert_eq!(allowance_lamports(1_000_000_000, &settings), 10_000_000);
    // 1% of a 0.005 SOL trade is 50,000 -- the floor wins.
    assert_eq!(allowance_lamports(5_000_000, &settings), 100_000);
    // A trade with no native leg has only the floor to go on.
    assert_eq!(allowance_lamports(0, &settings), 100_000);

    let strict = CostGuardSettings {
        max_extra_cost_pct: 0.0,
        always_allow_below_lamports: 0,
        ..settings
    };
    assert_eq!(allowance_lamports(1_000_000_000, &strict), 0);

    let generous = CostGuardSettings {
        max_extra_cost_pct: 100.0,
        ..settings
    };
    assert_eq!(allowance_lamports(5_000_000, &generous), 5_000_000);
}

/// The real numbers from the trade that exposed this: a 0.005 SOL buy whose
/// route locked 0.01304544 SOL of rent. It must be refused at the default
/// settings, and accepted once the same deposit is small against the trade.
#[test]
fn the_real_deposit_is_refused_on_a_small_trade_and_allowed_on_a_large_one() {
    let settings = settings();
    let deposit = 13_045_440;

    assert!(deposit > allowance_lamports(5_000_000, &settings));
    // Against 2 SOL the same one-off deposit is inside the 1% allowance.
    assert!(deposit <= allowance_lamports(2_000_000_000, &settings));
}

/// A Token-2022 mint account that charges `basis_points` of a transfer, at
/// most `maximum_fee`, from epoch 0.
fn token_2022_mint_with_fee(basis_points: u16, maximum_fee: u64) -> Account {
    use crate::chains::solana::spl_token_2022::extension::{
        transfer_fee::{TransferFee, TransferFeeConfig},
        BaseStateWithExtensionsMut, ExtensionType, StateWithExtensionsMut,
    };
    use crate::chains::solana::spl_token_2022::state::Mint;

    let length =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
            .unwrap();
    let mut data = vec![0u8; length];
    let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();
    state.base = Mint {
        is_initialized: true,
        decimals: 6,
        ..Mint::default()
    };
    state.pack_base();
    state.init_account_type().unwrap();
    let fee = TransferFee {
        epoch: 0u64.into(),
        maximum_fee: maximum_fee.into(),
        transfer_fee_basis_points: basis_points.into(),
    };
    let config = state.init_extension::<TransferFeeConfig>(true).unwrap();
    config.older_transfer_fee = fee;
    config.newer_transfer_fee = fee;
    Account {
        lamports: 1_000_000,
        data,
        owner: crate::chains::solana::spl_token_2022::id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn node_holding(mint_account: Option<Account>, epoch: Option<u64>) -> ScriptedNode {
    let accounts = mint_account
        .map(|account| HashMap::from([(MINT.parse().unwrap(), account)]))
        .unwrap_or_default();
    ScriptedNode::new(0, Vec::new(), Vec::new(), 0).with_chain_state(accounts, epoch)
}

/// The transfer fee is read through the node the swap runs against, never the
/// process-wide client, so a single-threaded runtime serves it.
#[tokio::test]
async fn the_output_transfer_fee_is_read_from_the_node_the_swap_runs_against() {
    let quote = quote("jupiter", SOL_MINT, MINT, 50_000_000, 1_000_000);
    let node = node_holding(Some(token_2022_mint_with_fee(100, 20_000)), Some(5));
    assert_eq!(output_transfer_fee(&node, &quote).await, Some(10_000));

    let capped = node_holding(Some(token_2022_mint_with_fee(100, 4_000)), Some(5));
    assert_eq!(output_transfer_fee(&capped, &quote).await, Some(4_000));
}

/// A mint that is absent, not Token-2022 or whose epoch cannot be read leaves
/// the fee unsubtracted, and the quoted minimum is then held strictly.
#[tokio::test]
async fn an_unreadable_output_mint_or_epoch_subtracts_no_transfer_fee() {
    let quote = quote("jupiter", SOL_MINT, MINT, 50_000_000, 1_000_000);

    assert_eq!(
        output_transfer_fee(&node_holding(None, Some(5)), &quote).await,
        None
    );

    let mut plain = token_2022_mint_with_fee(100, 20_000);
    plain.owner = SPL_TOKEN_PROGRAM_ID.parse().unwrap();
    assert_eq!(
        output_transfer_fee(&node_holding(Some(plain), Some(5)), &quote).await,
        None
    );

    let no_epoch = node_holding(Some(token_2022_mint_with_fee(100, 20_000)), None);
    assert_eq!(output_transfer_fee(&no_epoch, &quote).await, None);
}

#[tokio::test]
async fn a_native_or_usdc_output_reads_no_mint() {
    let node = node_holding(Some(token_2022_mint_with_fee(100, 20_000)), Some(5));
    assert_eq!(
        output_transfer_fee(&node, &sale("jupiter")).await,
        None,
        "native output"
    );
    let usdc = quote("jupiter", SOL_MINT, USDC_MINT, 50_000_000, 1_000_000);
    assert_eq!(output_transfer_fee(&node, &usdc).await, None);
}

// ----------------------------------------------------------------------------
// Recorded and adversarial runs across all three layers
// ----------------------------------------------------------------------------

/// A Raptor sale of 1.721433 JUP for `WALLET`, recorded on chain with its
/// inner instructions and balances: a split route through four venues, the
/// platform fee paid by Raptor's own CPI into the fee wallet.
const RAPTOR_SALE: &str =
    include_str!("../../../../../tests/fixtures/swaps/raptor_split_route_sell.json");

/// The recorded run as the simulation of the same build would report it.
fn recorded_run(recorded: &Value) -> SimulationOutcome {
    let meta = &recorded["meta"];
    SimulationOutcome {
        fee: meta["fee"].as_u64(),
        pre_balances: Some(serde_json::from_value(meta["preBalances"].clone()).unwrap()),
        post_balances: Some(serde_json::from_value(meta["postBalances"].clone()).unwrap()),
        pre_token_balances: Some(serde_json::from_value(meta["preTokenBalances"].clone()).unwrap()),
        post_token_balances: Some(
            serde_json::from_value(meta["postTokenBalances"].clone()).unwrap(),
        ),
        inner_instructions: serde_json::from_value(meta["innerInstructions"].clone()).unwrap(),
        ..Default::default()
    }
}

/// A legitimate Raptor build must clear the message, the inner instructions
/// and conservation, or every one-click sale through it is refused.
#[tokio::test]
async fn a_recorded_raptor_sale_passes_every_layer() {
    const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";
    let recorded: Value = serde_json::from_str(RAPTOR_SALE).expect("fixture is JSON");
    assert_eq!(
        recorded["transaction"]["message"]["accountKeys"][0]["pubkey"],
        WALLET
    );
    let outcome = recorded_run(&recorded);

    // The recorded top level: its compute budget, the wrapped-SOL account
    // created idempotently for the wallet, and the Raptor route.
    let transaction = build(&[
        ComputeBudgetInstruction::set_compute_unit_limit(208_875),
        ComputeBudgetInstruction::set_compute_unit_price(50_000),
        create_associated_token_account_idempotent(
            &wallet(),
            &wallet(),
            &key(SOL_MINT),
            &spl_token::id(),
        ),
        trade(RAPTOR_PROGRAM_ID),
    ]);
    // Gross proceeds 4,944,664 lamports, 24,723 of them the platform fee;
    // the floor is 1% below the net.
    let quote = Quote {
        output_amount: 4_919_941u64.into(),
        minimum_output_amount: 4_870_742u64.into(),
        platform_fee_lamports: Some(24_723),
        ..quote("raptor", JUP, SOL_MINT, 1_721_433, 0)
    };

    let verdict = decide(
        "Raptor",
        &transaction,
        Simulated::Ran(&outcome),
        &quote,
        &settings(),
        None,
        |_| async { Some(HashMap::new()) },
    )
    .await;
    assert_eq!(verdict, Ok(()));
}

/// A sale with 3% slippage: 0.05 SOL expected, 0.0485 SOL guaranteed, so its
/// allowance is the 1,500,000 lamports that slippage gives up.
fn wide_sale() -> Quote {
    Quote {
        slippage_bps: 300,
        output_amount: 50_000_000u64.into(),
        ..quote("jupiter", MINT, SOL_MINT, 1_000, 48_500_000)
    }
}

/// A run of `wide_sale` whose swap delivers the full expected proceeds while
/// `taken` lamports leave the wallet's controlled accounts.
fn sale_run_losing(taken: u64, groups: Vec<Value>) -> SimulationOutcome {
    SimulationOutcome {
        inner_instructions: groups,
        ..balances(
            vec![START, 2_039_280],
            vec![START + 50_000_000 - taken - 5_000, 2_039_280],
            json!([token_balance(1, MINT, 1_000)]),
            json!([token_balance(1, MINT, 0)]),
        )
    }
}

/// The same loss is one verdict whichever instruction carries it. The lamport
/// check measures against the slippage floor, which already concedes the
/// slippage share; adding the allowance on top concedes it a second time.
#[tokio::test]
async fn a_sale_loss_is_refused_alike_as_a_lamport_or_a_wrapped_sol_transfer() {
    const ELSEWHERE: &str = "7WMCUyZXrDxNjAWjuh2r7g8CdT2guqvrmdUPAnGdLsfG";
    let quote = wide_sale();
    let allowance = trade_allowance_lamports(&quote, &settings()).unwrap();
    assert_eq!(allowance, 1_500_000);
    let taken = 2_900_000;

    let as_lamports = sale_run_losing(taken, vec![group(vec![transfer(WALLET, STRANGER, taken)])]);
    let visible = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&as_lamports),
        &quote,
        HashMap::from([(
            STRANGER.to_owned(),
            AccountControl::Program(SYSTEM_PROGRAM_ID.to_owned()),
        )]),
    )
    .await;
    assert_eq!(
        visible,
        Err(Refusal::Cost {
            extra: taken,
            allowance,
            venue_address: STRANGER.to_owned(),
        })
    );

    let as_wrapped_sol = sale_run_losing(
        taken,
        vec![group(vec![token(
            "transfer",
            json!({
                "source": wrapped_sol_account().to_string(),
                "destination": ELSEWHERE,
                "authority": WALLET,
                "amount": taken.to_string(),
            }),
        )])],
    );
    let hidden = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&as_wrapped_sol),
        &quote,
        HashMap::new(),
    )
    .await;
    assert!(
        matches!(hidden, Err(Refusal::Cost { .. })),
        "{taken} lamports taken beyond the expected proceeds are refused as a lamport transfer \
         but signed as a wrapped-SOL transfer, against a {allowance} lamport allowance: \
         {hidden:?}"
    );
}

/// Whether the proceeds arrive is conservation, not a cost judgement: a buy
/// whose tokens never arrive is refused with the cost comparison off, and a
/// sale whose lamports never arrive must be refused the same way.
#[tokio::test]
async fn the_proceeds_must_reach_the_wallet_with_the_cost_comparison_off() {
    let off = CostGuardSettings {
        enabled: false,
        ..settings()
    };
    let decide_with = |settings: CostGuardSettings, outcome: SimulationOutcome, quote: Quote| async move {
        decide(
            "Jupiter",
            &jupiter_build(Vec::new()),
            Simulated::Ran(&outcome),
            &quote,
            &settings,
            None,
            |_| async { Some(HashMap::new()) },
        )
        .await
    };

    let buy_delivered_elsewhere = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000, 0],
        json!([token_balance(1, MINT, 0)]),
        json!([token_balance(1, MINT, 0)]),
    );
    let refused = decide_with(off, buy_delivered_elsewhere, buy("jupiter")).await;
    assert!(is_unusable(&refused), "{refused:?}");

    // The 1,000 units leave the wallet; the 0.0495 SOL minimum goes elsewhere.
    let sale_delivered_elsewhere = || {
        balances(
            vec![START, 2_039_280],
            vec![START - 5_000, 2_039_280],
            json!([token_balance(1, MINT, 1_000)]),
            json!([token_balance(1, MINT, 0)]),
        )
    };
    let on = decide_with(settings(), sale_delivered_elsewhere(), sale("jupiter")).await;
    assert!(on.is_err(), "{on:?}");

    let signed = decide_with(off, sale_delivered_elsewhere(), sale("jupiter")).await;
    assert!(
        signed.is_err(),
        "a sale whose 49,500,000 lamport minimum never reached the wallet was cleared with the \
         cost comparison off: {signed:?}"
    );
}

/// A node that leaves out the optional `owner` of a token balance has given
/// no reading of the wallet's accounts, and an absent reading is not evidence
/// that the output went elsewhere.
#[tokio::test]
async fn a_node_that_omits_the_balance_owner_does_not_refuse_a_delivered_buy() {
    let unowned = |amount: u64| {
        json!({
            "accountIndex": 1,
            "mint": MINT,
            "programId": SPL_TOKEN_PROGRAM_ID,
            "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6},
        })
    };
    let rent = 2_039_280;
    let outcome = balances(
        vec![START, 0],
        vec![START - 50_000_000 - 5_000 - rent, rent],
        json!([]),
        json!([unowned(950)]),
    );
    assert_eq!(
        verdict(
            &jupiter_build(Vec::new()),
            Simulated::Ran(&outcome),
            &buy("jupiter"),
            HashMap::new()
        )
        .await,
        Ok(()),
        "the buy delivered its 950 units; only the node's balance owner is missing"
    );
}

/// Rent a venue keeps, inside what the sale's slippage already concedes and
/// with the full proceeds delivered, is not a conservation failure.
#[tokio::test]
async fn a_venue_rent_within_the_sale_slippage_allowance_passes_when_the_proceeds_arrive() {
    let rent = 1_000_000;
    let quote = wide_sale();
    assert!(
        rent > allowance_lamports(trade_value_lamports(&quote).unwrap(), &settings()),
        "the rent must exceed the standard allowance to exercise the widened one"
    );
    let outcome = sale_run_losing(
        rent,
        vec![group(vec![create_account(
            WALLET,
            VENUE_ACCOUNT,
            VENUE,
            rent,
        )])],
    );
    let verdict = verdict(
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &quote,
        HashMap::new(),
    )
    .await;
    assert_eq!(verdict, Ok(()));
}

/// The disabled setting lifts the cap on attributed costs and nothing else:
/// the proceeds still reach the wallet, so the sale passes.
#[tokio::test]
async fn with_the_cost_comparison_off_a_large_venue_rent_passes_when_the_proceeds_arrive() {
    let rent = 5_000_000;
    let off = CostGuardSettings {
        enabled: false,
        ..settings()
    };
    let outcome = sale_run_losing(
        rent,
        vec![group(vec![create_account(
            WALLET,
            VENUE_ACCOUNT,
            VENUE,
            rent,
        )])],
    );
    let verdict = decide(
        "Jupiter",
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &wide_sale(),
        &off,
        None,
        |_| async { Some(HashMap::new()) },
    )
    .await;
    assert_eq!(verdict, Ok(()));
}

/// An outflow whose owner lookup went unanswered is still an explained
/// outflow: conservation concedes it, so an unreadable node cannot block a
/// sale whose proceeds arrive.
#[tokio::test]
async fn an_unanswered_owner_lookup_never_turns_a_sale_into_a_conservation_refusal() {
    let charge = 2_500_000;
    let quote = wide_sale();
    let outcome = sale_run_losing(
        charge,
        vec![group(vec![transfer(WALLET, VENUE_ACCOUNT, charge)])],
    );
    let verdict = decide(
        "Jupiter",
        &jupiter_build(Vec::new()),
        Simulated::Ran(&outcome),
        &quote,
        &settings(),
        None,
        |_| async { None },
    )
    .await;
    assert_eq!(verdict, Ok(()));
}

/// The guard admits a top-level program in a third-party-built swap only when the
/// registry names it with the quote's router id. A registry entry that carried a quote
/// router id (`jupiter`, `raptor`, `direct`) without being that router's own program would
/// widen what a signed swap may call.
#[test]
fn no_registered_program_widens_a_quote_router() {
    for known in KNOWN_PROGRAMS {
        match known.router {
            "jupiter" => assert!(
                is_jupiter_program_id(known.id),
                "{} is labelled jupiter but is not a Jupiter program",
                known.id
            ),
            "raptor" => assert_eq!(
                known.id, RAPTOR_PROGRAM_ID,
                "{} is labelled raptor but is not the Raptor program",
                known.id
            ),
            "direct" => panic!("{} carries the direct router id", known.id),
            _ => {}
        }
    }
}
