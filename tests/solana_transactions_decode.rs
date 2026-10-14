// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure wallet-watch transaction classification contracts.

use screenerbot::chains::solana::constants::{SOL_MINT, USDC_MINT};
use screenerbot::chains::solana::transactions::program_ids::JUPITER_V6_PROGRAM_ID;
use screenerbot::chains::solana::wallets::classify::classify_transaction_activity;
use screenerbot::transactions::Transaction;
use screenerbot::wallets::watch::{ActivityKind, SwapSide, TransferDirection};
use serde_json::{json, Value};

const SUBJECT: &str = "11111111111111111111111111111111";
const PRIMARY_MINT: &str = "PrimaryMint1111111111111111111111111111111";
const SECONDARY_MINT: &str = "SecondaryMint11111111111111111111111111111";

fn token_balance_raw(account_index: u32, mint: &str, raw_amount: i128) -> Value {
    json!({
        "accountIndex": account_index,
        "mint": mint,
        "owner": SUBJECT,
        "programId": null,
        "uiTokenAmount": {
            "amount": raw_amount.to_string(),
            "decimals": 6,
            "uiAmount": null,
            "uiAmountString": raw_amount.to_string()
        }
    })
}

fn decoded(
    sol_delta_lamports: i64,
    deltas: &[(&str, f64)],
    dex_program: Option<&str>,
) -> Transaction {
    let pre_sol = 10_000_000_000_u64;
    let post_sol = (pre_sol as i64 + sol_delta_lamports) as u64;
    let pre_tokens: Vec<Value> = deltas
        .iter()
        .enumerate()
        .map(|(index, (mint, delta))| {
            token_balance_raw(
                index as u32 + 1,
                mint,
                if *delta < 0.0 {
                    (delta.abs() * 1_000_000.0).round() as i128
                } else {
                    0
                },
            )
        })
        .collect();
    let post_tokens: Vec<Value> = deltas
        .iter()
        .enumerate()
        .map(|(index, (mint, delta))| {
            token_balance_raw(
                index as u32 + 1,
                mint,
                if *delta > 0.0 {
                    (*delta * 1_000_000.0).round() as i128
                } else {
                    0
                },
            )
        })
        .collect();
    let instructions = dex_program
        .map(|program_id| vec![json!({ "programId": program_id })])
        .unwrap_or_default();

    let mut account_keys = vec![SUBJECT.to_owned()];
    account_keys.extend((0..deltas.len()).map(|index| format!("OwnedATA{index}")));
    let mut pre_balances = vec![pre_sol];
    pre_balances.extend(std::iter::repeat_n(2_039_280, deltas.len()));
    let mut post_balances = vec![post_sol];
    post_balances.extend(std::iter::repeat_n(2_039_280, deltas.len()));
    let mut transaction = Transaction::new("signature".to_owned());
    transaction.success = true;
    transaction.raw_transaction_data = Some(json!({
        "slot": 42,
        "transaction": {
            "message": {
                "accountKeys": account_keys,
                "instructions": instructions
            },
            "signatures": ["signature"]
        },
        "meta": {
            "err": null,
            "preBalances": pre_balances,
            "postBalances": post_balances,
            "preTokenBalances": pre_tokens,
            "postTokenBalances": post_tokens,
            "fee": 5_000,
            "computeUnitsConsumed": null,
            "logMessages": [],
            "innerInstructions": []
        },
        "blockTime": 1
    }));
    transaction
}

#[test]
fn fee_noise_on_a_usdc_quoted_route_is_not_a_sol_swap() {
    let transaction = decoded(
        -5_000,
        &[(USDC_MINT, -25.0), (PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );

    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("no SOL-quoted leg for this subject"));
}

#[test]
fn a_plain_spl_transfer_is_never_a_swap() {
    let transaction = decoded(-5_000, &[(PRIMARY_MINT, 25.0)], None);

    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Transfer {
            mint,
            amount: 25.0,
            direction: TransferDirection::In,
        } if mint == PRIMARY_MINT
    ));
}

#[test]
fn a_dex_sol_swap_resolves_primary_leg_side_amount_and_venue() {
    let transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );

    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            mint,
            side: SwapSide::Buy,
            native_amount: 1.0,
            token_amount: 100.0,
            venue: Some(venue),
            price_native: Some(0.01),
        } if mint == PRIMARY_MINT && venue == "jupiter"
    ));
}

#[test]
fn null_ui_amount_still_classifies_from_raw_integer_amount() {
    let transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            token_amount: 100.0,
            ..
        }
    ));
}

#[test]
fn a_failed_transaction_is_dropped_before_classification() {
    let mut transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    transaction.success = false;

    assert!(classify_transaction_activity(SUBJECT, &transaction).is_none());
}

#[test]
fn every_multi_mint_swap_is_rejected_even_when_amounts_differ() {
    let ambiguous = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100_000.0), (SECONDARY_MINT, 0.000_001)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let (kind, reason) = classify_transaction_activity(SUBJECT, &ambiguous).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("ambiguous nonquote token legs"));
}

#[test]
fn native_sell_has_opposite_token_direction() {
    let transaction = decoded(
        999_995_000,
        &[(PRIMARY_MINT, -100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    assert!(matches!(
        classify_transaction_activity(SUBJECT, &transaction)
            .unwrap()
            .0,
        ActivityKind::Swap {
            side: SwapSide::Sell,
            native_amount: 1.0,
            token_amount: 100.0,
            ..
        }
    ));
}

#[test]
fn native_buy_with_ata_rent_keeps_swap_price() {
    let mut transaction = decoded(
        -1_002_044_280,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let meta = transaction
        .raw_transaction_data
        .as_mut()
        .unwrap()
        .get_mut("meta")
        .unwrap();
    meta["preTokenBalances"] = json!([]);
    meta["preBalances"] = json!([10_000_000_000_u64, 0_u64]);
    meta["postBalances"] = json!([8_997_955_720_u64, 2_039_280_u64]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            native_amount: 1.0,
            price_native: Some(0.01),
            ..
        }
    ));
}

#[test]
fn native_sell_with_ata_closure_keeps_swap_price() {
    let mut transaction = decoded(
        1_002_034_280,
        &[(PRIMARY_MINT, -100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let meta = transaction
        .raw_transaction_data
        .as_mut()
        .unwrap()
        .get_mut("meta")
        .unwrap();
    meta["postTokenBalances"] = json!([]);
    meta["preBalances"] = json!([10_000_000_000_u64, 2_039_280_u64]);
    meta["postBalances"] = json!([11_002_034_280_u64, 0_u64]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            side: SwapSide::Sell,
            native_amount: 1.0,
            ..
        }
    ));
}

#[test]
fn existing_wsol_account_supplies_native_quote_without_signer_movement() {
    let mut buy = decoded(
        -5_000,
        &[(SOL_MINT, -1.0), (PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    buy.raw_transaction_data.as_mut().unwrap()["meta"]["preBalances"] =
        json!([10_000_000_000_u64, 1_002_039_280_u64, 2_039_280_u64]);
    buy.raw_transaction_data.as_mut().unwrap()["meta"]["postBalances"] =
        json!([9_999_995_000_u64, 2_039_280_u64, 2_039_280_u64]);
    let (kind, _) = classify_transaction_activity(SUBJECT, &buy).unwrap();
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            side: SwapSide::Buy,
            native_amount: 1.0,
            ..
        }
    ));

    let mut sell = decoded(
        -5_000,
        &[(SOL_MINT, 1.0), (PRIMARY_MINT, -100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    sell.raw_transaction_data.as_mut().unwrap()["meta"]["preBalances"] =
        json!([10_000_000_000_u64, 2_039_280_u64, 2_039_280_u64]);
    sell.raw_transaction_data.as_mut().unwrap()["meta"]["postBalances"] =
        json!([9_999_995_000_u64, 1_002_039_280_u64, 2_039_280_u64]);
    let (kind, _) = classify_transaction_activity(SUBJECT, &sell).unwrap();
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            side: SwapSide::Sell,
            native_amount: 1.0,
            ..
        }
    ));
}

#[test]
fn wrapping_and_closing_wsol_do_not_create_native_swap_consideration() {
    let mut wrap = decoded(
        -1_000_005_000,
        &[(SOL_MINT, 1.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    wrap.raw_transaction_data.as_mut().unwrap()["meta"]["preBalances"] =
        json!([10_000_000_000_u64, 2_039_280_u64]);
    wrap.raw_transaction_data.as_mut().unwrap()["meta"]["postBalances"] =
        json!([8_999_995_000_u64, 1_002_039_280_u64]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &wrap).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("no SOL-quoted leg for this subject"));

    let mut close = decoded(
        999_995_000,
        &[(SOL_MINT, -1.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    close.raw_transaction_data.as_mut().unwrap()["meta"]["preBalances"] =
        json!([10_000_000_000_u64, 1_002_039_280_u64]);
    close.raw_transaction_data.as_mut().unwrap()["meta"]["postBalances"] =
        json!([10_999_995_000_u64, 2_039_280_u64]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &close).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("no SOL-quoted leg for this subject"));
}

#[test]
fn stable_token_movement_with_native_rent_is_not_a_swap_quote() {
    let mut transaction = decoded(
        -1_002_044_280,
        &[(USDC_MINT, -25.0), (PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let meta = transaction
        .raw_transaction_data
        .as_mut()
        .unwrap()
        .get_mut("meta")
        .unwrap();
    meta["preTokenBalances"] = json!([token_balance_raw(1, USDC_MINT, 25_000_000)]);
    meta["postTokenBalances"] = json!([
        token_balance_raw(1, USDC_MINT, 0),
        token_balance_raw(2, PRIMARY_MINT, 100_000_000)
    ]);
    meta["preBalances"] = json!([10_000_000_000_u64, 2_039_280_u64, 0_u64]);
    meta["postBalances"] = json!([8_997_955_720_u64, 2_039_280_u64, 2_039_280_u64]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("unsupported stable-quoted swap"));
}

#[test]
fn same_sign_token_and_native_deltas_are_not_a_swap() {
    let transaction = decoded(
        999_995_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("token and native movements have same sign"));
}

#[test]
fn unrelated_top_level_native_transfer_makes_swap_ambiguous() {
    let mut transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    transaction.raw_transaction_data.as_mut().unwrap()["transaction"]["message"]["instructions"] = json!([
        { "program": "system", "programId": "11111111111111111111111111111111", "parsed": {
            "type": "transfer", "info": { "source": SUBJECT, "destination": "ExternalCounterparty", "lamports": 500_000_000 }
        }},
        { "programId": JUPITER_V6_PROGRAM_ID }
    ]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("ambiguous independent native transfer"));
}

#[test]
fn malformed_raw_amount_and_missing_native_metadata_are_other() {
    for amount in [
        "bad",
        "-1",
        "+1",
        "",
        "170141183460469231731687303715884105728",
    ] {
        let mut malformed = decoded(
            -1_000_005_000,
            &[(PRIMARY_MINT, 100.0)],
            Some(JUPITER_V6_PROGRAM_ID),
        );
        malformed.raw_transaction_data.as_mut().unwrap()["meta"]["postTokenBalances"][0]
            ["uiTokenAmount"]["amount"] = json!(amount);
        assert!(
            matches!(
                classify_transaction_activity(SUBJECT, &malformed)
                    .unwrap()
                    .0,
                ActivityKind::Other
            ),
            "amount {amount:?} must be rejected"
        );
    }

    let mut missing = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    missing.raw_transaction_data.as_mut().unwrap()["meta"]["postBalances"] = Value::Null;
    assert!(matches!(
        classify_transaction_activity(SUBJECT, &missing).unwrap().0,
        ActivityKind::Other
    ));
}

#[test]
fn inconsistent_decimals_for_one_mint_are_other() {
    let mut transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    transaction.raw_transaction_data.as_mut().unwrap()["meta"]["postTokenBalances"][0]
        ["uiTokenAmount"]["decimals"] = json!(9);
    assert!(matches!(
        classify_transaction_activity(SUBJECT, &transaction)
            .unwrap()
            .0,
        ActivityKind::Other
    ));
}

#[test]
fn non_signer_subject_uses_its_own_native_delta_without_fee_adjustment() {
    let mut transaction = decoded(
        -1_000_000_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let raw = transaction.raw_transaction_data.as_mut().unwrap();
    raw["transaction"]["message"]["accountKeys"] = json!(["FeePayer", SUBJECT, "OwnedATA0"]);
    raw["meta"]["preBalances"] = json!([2_000_000_u64, 10_000_000_000_u64, 2_039_280_u64]);
    raw["meta"]["postBalances"] = json!([1_995_000_u64, 9_000_000_000_u64, 2_039_280_u64]);
    raw["meta"]["preTokenBalances"][0]["accountIndex"] = json!(2);
    raw["meta"]["postTokenBalances"][0]["accountIndex"] = json!(2);
    assert!(matches!(
        classify_transaction_activity(SUBJECT, &transaction)
            .unwrap()
            .0,
        ActivityKind::Swap {
            side: SwapSide::Buy,
            native_amount: 1.0,
            ..
        }
    ));
}

#[test]
fn system_funding_into_owned_wsol_account_is_not_an_unrelated_transfer() {
    let mut transaction = decoded(
        -1_000_005_000,
        &[(SOL_MINT, 1.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let raw = transaction.raw_transaction_data.as_mut().unwrap();
    raw["meta"]["preBalances"] = json!([10_000_000_000_u64, 2_039_280_u64]);
    raw["meta"]["postBalances"] = json!([8_999_995_000_u64, 1_002_039_280_u64]);
    raw["transaction"]["message"]["instructions"] = json!([
        { "program": "system", "programId": "11111111111111111111111111111111", "parsed": {
            "type": "transfer", "info": { "source": SUBJECT, "destination": "OwnedATA0", "lamports": 1_000_000_000 }
        }},
        { "programId": JUPITER_V6_PROGRAM_ID }
    ]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("no SOL-quoted leg for this subject"));
}
