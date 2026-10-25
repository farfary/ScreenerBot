// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure wallet-watch transaction classification contracts.

use screenerbot::chains::solana::constants::{SOL_MINT, USDC_MINT};
use screenerbot::chains::solana::transactions::program_ids::JUPITER_V6_PROGRAM_ID;
use screenerbot::chains::solana::wallets::classify::classify_transaction_activity;
use screenerbot::transactions::Transaction;
use screenerbot::wallets::watch::{
    ActivityKind, SwapSide, TransferDirection, UnresolvedSwapReason,
};
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

/// A real `getTransaction` (jsonParsed) result recorded from mainnet, with its log
/// lines dropped.
fn recorded(name: &str) -> Transaction {
    let path = format!(
        "{}/tests/fixtures/swaps/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut transaction = Transaction::new(
        raw["transaction"]["signatures"][0]
            .as_str()
            .unwrap()
            .to_owned(),
    );
    transaction.success = true;
    transaction.raw_transaction_data = Some(raw);
    transaction
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-9,
        "{actual} != {expected}"
    );
}

#[test]
fn a_usdc_quoted_route_resolves_a_stable_swap() {
    let transaction = decoded(
        -5_000,
        &[(USDC_MINT, -25.0), (PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );

    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::StableSwap {
            mint,
            side: SwapSide::Buy,
            stable_mint,
            stable_amount: 25.0,
            token_amount: 100.0,
            venue: Some(_),
        } if mint == PRIMARY_MINT && stable_mint == USDC_MINT
    ));
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
    assert!(matches!(
        kind,
        ActivityKind::UnresolvedSwap {
            mint: None,
            reason: UnresolvedSwapReason::MultipleTokens,
            ..
        }
    ));
    assert_eq!(reason, Some("more than one nonquote token leg"));
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
    assert_eq!(reason, Some("no nonquote token leg"));

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
    assert_eq!(reason, Some("no nonquote token leg"));
}

#[test]
fn stable_and_native_quote_legs_together_are_an_unresolved_mixed_quote() {
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
    assert!(matches!(
        kind,
        ActivityKind::UnresolvedSwap {
            mint: Some(ref mint),
            reason: UnresolvedSwapReason::MixedQuote,
            ..
        } if mint == PRIMARY_MINT
    ));
    assert_eq!(reason, Some("native and stable quote legs together"));
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
    assert_eq!(reason, Some("no opposing leg for the token"));
}

#[test]
fn a_top_level_tip_or_fee_beside_the_swap_is_not_part_of_its_amount() {
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
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            side: SwapSide::Buy,
            native_amount: 0.5,
            token_amount: 100.0,
            ..
        }
    ));
}

#[test]
fn a_payment_into_a_temporary_wsol_account_opened_for_the_subject_is_the_swap() {
    let mut transaction = decoded(
        -1_001_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let raw = transaction.raw_transaction_data.as_mut().unwrap();
    raw["transaction"]["message"]["instructions"] = json!([
        { "program": "system", "programId": "11111111111111111111111111111111", "parsed": {
            "type": "transfer", "info": { "source": SUBJECT, "destination": "TemporaryWsol", "lamports": 1_000_000_000 }
        }},
        { "program": "system", "programId": "11111111111111111111111111111111", "parsed": {
            "type": "transfer", "info": { "source": SUBJECT, "destination": "TipAccount", "lamports": 1_000_000 }
        }},
        { "programId": JUPITER_V6_PROGRAM_ID }
    ]);
    raw["meta"]["innerInstructions"] = json!([{ "index": 2, "instructions": [
        { "program": "spl-token", "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "parsed": {
            "type": "initializeAccount3", "info": { "account": "TemporaryWsol", "mint": SOL_MINT, "owner": SUBJECT }
        }}
    ]}]);
    let (kind, reason) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(
        kind,
        ActivityKind::Swap {
            side: SwapSide::Buy,
            native_amount: 1.0,
            ..
        }
    ));
}

#[test]
fn an_unreadable_top_level_transfer_leaves_the_swap_unresolved() {
    let mut transaction = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    transaction.raw_transaction_data.as_mut().unwrap()["transaction"]["message"]["instructions"] = json!([
        { "program": "system", "programId": "11111111111111111111111111111111", "parsed": {
            "type": "transfer", "info": { "source": SUBJECT }
        }},
        { "programId": JUPITER_V6_PROGRAM_ID }
    ]);
    let (kind, _) = classify_transaction_activity(SUBJECT, &transaction).unwrap();
    assert!(matches!(
        kind,
        ActivityKind::UnresolvedSwap {
            reason: UnresolvedSwapReason::Unreadable,
            ..
        }
    ));
}

#[test]
fn recorded_terminal_buy_excludes_the_terminal_fee() {
    let transaction = recorded("copy_terminal_fee_pump_buy");
    let (kind, reason) =
        classify_transaction_activity("D4xQoGuvX7i8xzNNRc9HK5N5MtULsC4vX9JFgqnEKKnb", &transaction)
            .unwrap();
    assert_eq!(reason, None);
    let ActivityKind::Swap {
        mint,
        side: SwapSide::Buy,
        native_amount,
        token_amount,
        ..
    } = kind
    else {
        panic!("expected a buy, got {kind:?}");
    };
    assert_eq!(mint, "GaW5SiXrKqxZPimyzou32nMMse72sGe7cN56rw5kpump");
    assert_close(native_amount, 0.012_210_109);
    assert_close(token_amount, 75_836.167_729);
}

#[test]
fn recorded_jupiter_buy_excludes_fee_and_tip_and_keeps_its_wsol_payment() {
    let transaction = recorded("copy_jupiter_tip_fee_temp_wsol_buy");
    let (kind, reason) =
        classify_transaction_activity("3VUNtVtjjx5ckUojT7UocJ5fbuAJRsNUXNfTBnPte9vC", &transaction)
            .unwrap();
    assert_eq!(reason, None);
    let ActivityKind::Swap {
        mint,
        side: SwapSide::Buy,
        native_amount,
        token_amount,
        ..
    } = kind
    else {
        panic!("expected a buy, got {kind:?}");
    };
    assert_eq!(mint, "H9kjkWCDm7oJZN2YbUQsbke3bqenVhHyGoexjPu7cUgT");
    assert_close(native_amount, 2.488_75);
    assert_close(token_amount, 39_329_807.365_078);
}

#[test]
fn recorded_relayed_usdc_trades_resolve_as_stable_swaps() {
    const WALLET: &str = "6qudAN2kV8mtCcYJxb5QQ6Vr15itdHHdeVbYm99NKMhy";
    const MINT: &str = "6mVSEFWhPszE2XHQpWqGsxbdTc5WAvvwAAsS7YA6pump";
    for (name, expected_side, expected_usdc) in [
        ("copy_usdc_relayed_buy", SwapSide::Buy, 250.313_033),
        ("copy_usdc_relayed_sell", SwapSide::Sell, 249.986_736),
    ] {
        let (kind, reason) = classify_transaction_activity(WALLET, &recorded(name)).unwrap();
        assert_eq!(reason, None, "{name}");
        let ActivityKind::StableSwap {
            mint,
            side,
            stable_mint,
            stable_amount,
            token_amount,
            ..
        } = kind
        else {
            panic!("{name}: expected a stable swap, got {kind:?}");
        };
        assert_eq!(mint, MINT);
        assert_eq!(side, expected_side);
        assert_eq!(stable_mint, USDC_MINT);
        assert_close(stable_amount, expected_usdc);
        assert_close(token_amount, 32_027_088.056_276);
    }
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
    assert_eq!(reason, Some("no nonquote token leg"));
}
