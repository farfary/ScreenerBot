//! Pure wallet-watch transaction classification contracts.

use screenerbot::chains::solana::constants::USDC_MINT;
use screenerbot::chains::solana::transactions::program_ids::JUPITER_V6_PROGRAM_ID;
use screenerbot::chains::solana::wallets::classify::classify_transaction_activity;
use screenerbot::transactions::Transaction;
use screenerbot::wallets::watch::{ActivityKind, SwapSide, TransferDirection};
use serde_json::{json, Value};

const SUBJECT: &str = "11111111111111111111111111111111";
const PRIMARY_MINT: &str = "PrimaryMint1111111111111111111111111111111";
const SECONDARY_MINT: &str = "SecondaryMint11111111111111111111111111111";

fn token_balance(account_index: u32, mint: &str, ui_amount: f64) -> Value {
    json!({
        "accountIndex": account_index,
        "mint": mint,
        "owner": SUBJECT,
        "programId": null,
        "uiTokenAmount": {
            "amount": "0",
            "decimals": 6,
            "uiAmount": ui_amount,
            "uiAmountString": ui_amount.to_string()
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
            token_balance(
                index as u32 + 1,
                mint,
                if *delta < 0.0 { delta.abs() } else { 0.0 },
            )
        })
        .collect();
    let post_tokens: Vec<Value> = deltas
        .iter()
        .enumerate()
        .map(|(index, (mint, delta))| {
            token_balance(
                index as u32 + 1,
                mint,
                if *delta > 0.0 { *delta } else { 0.0 },
            )
        })
        .collect();
    let instructions = dex_program
        .map(|program_id| vec![json!({ "programId": program_id })])
        .unwrap_or_default();

    let mut transaction = Transaction::new("signature".to_owned());
    transaction.success = true;
    transaction.raw_transaction_data = Some(json!({
        "slot": 42,
        "transaction": {
            "message": {
                "accountKeys": [SUBJECT],
                "instructions": instructions
            },
            "signatures": ["signature"]
        },
        "meta": {
            "err": null,
            "preBalances": [pre_sol],
            "postBalances": [post_sol],
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
            sol_amount: 1.0,
            token_amount: 100.0,
            venue: Some(venue),
            price_sol: Some(0.01),
        } if mint == PRIMARY_MINT && venue == "jupiter"
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
fn multi_mint_dominance_threshold_is_strictly_above_half() {
    let boundary = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0), (SECONDARY_MINT, 50.0)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let (kind, reason) = classify_transaction_activity(SUBJECT, &boundary).unwrap();
    assert_eq!(reason, None);
    assert!(matches!(kind, ActivityKind::Swap { mint, .. } if mint == PRIMARY_MINT));

    let ambiguous = decoded(
        -1_000_005_000,
        &[(PRIMARY_MINT, 100.0), (SECONDARY_MINT, 50.000_001)],
        Some(JUPITER_V6_PROGRAM_ID),
    );
    let (kind, reason) = classify_transaction_activity(SUBJECT, &ambiguous).unwrap();
    assert!(matches!(kind, ActivityKind::Other));
    assert_eq!(reason, Some("ambiguous multi-hop: no single dominant mint"));
}
