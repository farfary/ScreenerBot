// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure classification: a decoded transaction -> one `ActivityKind`, subject-relative.
//!
//! The single most expensive decode bug available here is treating a token transfer
//! (airdrop, CEX deposit, wallet consolidation) as a sell. This module's one job is
//! to make that impossible: a `Swap` is only ever produced when a known DEX/
//! aggregator program is present in the transaction (`program_ids::
//! detect_router`) -- a balance delta alone, however swap-shaped, is
//! never sufficient.

use std::collections::{HashMap, HashSet};

use crate::chains::solana::constants::{
    LAMPORTS_PER_SOL, SOL_MINT, SYSTEM_PROGRAM_ID, USDC_MINT, USDT_MINT,
};
use crate::chains::solana::transactions::analyzer::balance::account_keys_from_message;
use crate::chains::solana::transactions::analyzer::dex::extract_program_ids;
use crate::chains::solana::transactions::deltas::{extract_subject_deltas, subject_native_delta};
use crate::chains::solana::transactions::program_ids::detect_router;
use crate::transactions::deltas::NATIVE_SOL_SENTINEL;
use crate::transactions::types::Transaction;
use crate::wallets::watch::{ActivityKind, SwapSide, TransferDirection};

/// Below this, a SOL balance delta is indistinguishable from transaction-fee noise
/// (base fee + a moderate priority fee) rather than a real transfer.
const FEE_NOISE_LAMPORTS: i64 = 50_000;

/// Returns whether the decoded transaction may have changed the subject's assets.
///
/// The predicate only rejects a transaction when complete balance metadata proves
/// that the subject has no SOL change after excluding its network fee and no owned
/// token-account change. Any missing or internally inconsistent metadata is kept
/// for decoding so an incomplete RPC response cannot suppress real activity.
pub fn subject_has_meaningful_effect(
    subject_address: &str,
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> bool {
    let Some(meta) = tx_data.meta.as_ref() else {
        return true;
    };

    let account_keys = account_keys_from_message(&tx_data.transaction.message);
    let subject_indices: Vec<usize> = account_keys
        .iter()
        .enumerate()
        .filter_map(|(index, key)| (key == subject_address).then_some(index))
        .collect();
    let [subject_index] = subject_indices.as_slice() else {
        return true;
    };

    let owned_indices = meta
        .pre_token_balances
        .iter()
        .chain(meta.post_token_balances.iter())
        .flatten()
        .filter(|balance| balance.owner.as_deref() == Some(subject_address))
        .map(|balance| balance.account_index as usize)
        .collect::<HashSet<_>>();
    let Some(sol_delta) = subject_native_delta(meta, *subject_index, owned_indices) else {
        return true;
    };
    if sol_delta != 0 {
        return true;
    }

    subject_token_balances_changed(meta, subject_address, account_keys.len()).unwrap_or(true)
}

/// `Ok(false)` proves every subject-owned token account kept its raw balance.
/// `Err(())` means the RPC metadata cannot establish that conclusion.
fn subject_token_balances_changed(
    meta: &crate::chains::solana::rpc::TransactionMeta,
    subject_address: &str,
    account_count: usize,
) -> Result<bool, ()> {
    let (Some(pre), Some(post)) = (&meta.pre_token_balances, &meta.post_token_balances) else {
        return Err(());
    };

    let mut balances = HashMap::<(u32, String), (Option<u128>, Option<u128>)>::new();
    for balance in pre {
        record_subject_token_balance(&mut balances, balance, subject_address, account_count, true)?;
    }
    for balance in post {
        record_subject_token_balance(
            &mut balances,
            balance,
            subject_address,
            account_count,
            false,
        )?;
    }

    Ok(balances
        .into_values()
        .any(|(pre, post)| pre.unwrap_or(0) != post.unwrap_or(0)))
}

fn record_subject_token_balance(
    balances: &mut HashMap<(u32, String), (Option<u128>, Option<u128>)>,
    balance: &crate::chains::solana::rpc::TokenBalance,
    subject_address: &str,
    account_count: usize,
    is_pre: bool,
) -> Result<(), ()> {
    if balance.account_index as usize >= account_count || balance.owner.is_none() {
        return Err(());
    }
    if balance.owner.as_deref() != Some(subject_address) {
        return Ok(());
    }

    let raw_amount = balance
        .ui_token_amount
        .amount
        .parse::<u128>()
        .map_err(|_| ())?;
    let entry = balances
        .entry((balance.account_index, balance.mint.clone()))
        .or_insert((None, None));
    let slot = if is_pre { &mut entry.0 } else { &mut entry.1 };
    if slot.replace(raw_amount).is_some() {
        return Err(());
    }
    Ok(())
}

/// Resolve one subject-relative `ActivityKind` from a decoded, successful
/// transaction. Pure -- reads only `transaction.raw_transaction_data`, which
/// `TransactionProcessor::decode()` always populates in memory on the object it
/// returns, regardless of the subject's persistence policy (retention only affects
/// what gets written to the database, not what is present on the decoded value).
///
/// Returns `(kind, skip_reason)`. `skip_reason` is `Some` whenever `kind` came back
/// `Other` because an explicit rule below declined to call it a `Swap` or `Transfer`
/// (no DEX program, ambiguous token legs, a route with no SOL-quoted leg) rather than
/// because nothing happened at all -- the funnel logs it so "why didn't this copy"
/// never has to be guessed at.
pub fn classify_transaction_activity(
    subject_address: &str,
    transaction: &Transaction,
) -> Option<(ActivityKind, Option<&'static str>)> {
    if !transaction.success {
        return None;
    }

    let Some(raw) = transaction.raw_transaction_data.as_ref() else {
        return Some((ActivityKind::Other, Some("no raw transaction data")));
    };
    let Ok(tx_data) =
        serde_json::from_value::<crate::chains::solana::rpc::TransactionDetails>(raw.clone())
    else {
        return Some((
            ActivityKind::Other,
            Some("raw transaction data did not parse"),
        ));
    };
    let Some(meta) = tx_data.meta.as_ref() else {
        return Some((ActivityKind::Other, Some("missing transaction meta")));
    };

    let account_keys = account_keys_from_message(&tx_data.transaction.message);
    let Some(subject_index) = account_keys.iter().position(|k| k == subject_address) else {
        return Some((ActivityKind::Other, Some("subject not among account keys")));
    };

    let Some(token_decimals) =
        validated_subject_token_decimals(meta, subject_address, account_keys.len())
    else {
        return Some((
            ActivityKind::Other,
            Some("invalid subject token balance metadata"),
        ));
    };
    let owned_indices = meta
        .pre_token_balances
        .iter()
        .chain(meta.post_token_balances.iter())
        .flatten()
        .filter(|balance| balance.owner.as_deref() == Some(subject_address))
        .map(|balance| balance.account_index as usize)
        .collect::<HashSet<_>>();
    let Some(sol_delta_lamports) =
        subject_native_delta(meta, subject_index, owned_indices.iter().copied())
    else {
        return Some((ActivityKind::Other, Some("missing native balance metadata")));
    };
    let raw_deltas = extract_subject_deltas(subject_address, transaction);
    let token_deltas = raw_deltas
        .into_iter()
        .filter(|delta| delta.mint != NATIVE_SOL_SENTINEL)
        .map(|delta| (delta.mint, delta.delta_raw))
        .filter(|(_, delta)| *delta != 0)
        .collect::<Vec<_>>();

    let venue = extract_program_ids(&tx_data)
        .ok()
        .and_then(|ids| detect_router(&ids).map(str::to_owned));

    Some(if venue.is_some() {
        if has_ambiguous_native_transfer(raw, &account_keys, subject_index, &owned_indices) {
            (
                ActivityKind::Other,
                Some("ambiguous independent native transfer"),
            )
        } else {
            classify_swap(sol_delta_lamports, &token_deltas, &token_decimals, venue)
        }
    } else {
        classify_transfer(sol_delta_lamports, &token_deltas, &token_decimals)
    })
}

/// Validate raw token-balance fields before the ledger extractor is used for sizing.
fn validated_subject_token_decimals(
    meta: &crate::chains::solana::rpc::TransactionMeta,
    subject_address: &str,
    account_count: usize,
) -> Option<HashMap<String, u8>> {
    let pre = meta.pre_token_balances.as_ref()?;
    let post = meta.post_token_balances.as_ref()?;
    let mut decimals = HashMap::new();
    let mut seen_pre = HashSet::new();
    let mut seen_post = HashSet::new();
    let mut account_mints = HashMap::<u32, String>::new();
    let mut raw_totals = HashMap::<String, (i128, i128)>::new();
    for (balances, seen, is_pre) in [(pre, &mut seen_pre, true), (post, &mut seen_post, false)] {
        for balance in balances {
            let owner = balance.owner.as_deref()?;
            if owner != subject_address {
                continue;
            }
            let index = balance.account_index as usize;
            let raw_amount = balance.ui_token_amount.amount.as_str();
            if raw_amount.is_empty() || !raw_amount.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let amount = raw_amount.parse::<i128>().ok()?;
            if amount < 0 {
                return None;
            }
            if index >= account_count
                || meta.pre_balances.get(index).is_none()
                || meta.post_balances.get(index).is_none()
                || !seen.insert(balance.account_index)
            {
                return None;
            }
            if account_mints
                .insert(balance.account_index, balance.mint.clone())
                .is_some_and(|previous| previous != balance.mint)
            {
                return None;
            }
            match decimals.insert(balance.mint.clone(), balance.ui_token_amount.decimals) {
                Some(previous) if previous != balance.ui_token_amount.decimals => return None,
                _ => {}
            }
            let total = raw_totals.entry(balance.mint.clone()).or_default();
            let amount_total = if is_pre { &mut total.0 } else { &mut total.1 };
            *amount_total = amount_total.checked_add(amount)?;
        }
    }
    if raw_totals
        .values()
        .any(|(pre, post)| post.checked_sub(*pre).is_none())
    {
        return None;
    }
    Some(decimals)
}

fn has_ambiguous_native_transfer(
    raw: &serde_json::Value,
    account_keys: &[String],
    subject_index: usize,
    owned_indices: &HashSet<usize>,
) -> bool {
    let Some(instructions) = raw
        .pointer("/transaction/message/instructions")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    for instruction in instructions {
        if instruction
            .get("program")
            .and_then(serde_json::Value::as_str)
            != Some("system")
            && instruction
                .get("programId")
                .and_then(serde_json::Value::as_str)
                != Some(SYSTEM_PROGRAM_ID)
        {
            continue;
        }
        let Some(parsed) = instruction.get("parsed") else {
            continue;
        };
        let Some(kind) = parsed.get("type").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if !matches!(kind, "transfer" | "transferWithSeed") {
            continue;
        }
        let Some(info) = parsed.get("info") else {
            return true;
        };
        let source = info.get("source").and_then(serde_json::Value::as_str);
        let destination = info.get("destination").and_then(serde_json::Value::as_str);
        let (Some(source), Some(destination)) = (source, destination) else {
            return true;
        };
        let subject = account_keys.get(subject_index).map(String::as_str);
        let owned_counterparty = |address: &str| {
            owned_indices
                .iter()
                .any(|index| account_keys.get(*index).map(String::as_str) == Some(address))
        };
        if (Some(source) == subject && !owned_counterparty(destination))
            || (Some(destination) == subject && !owned_counterparty(source))
        {
            return true;
        }
    }
    false
}

/// A DEX program is present. Resolve to `Swap` only when the subject has exactly one
/// non-wSOL token leg, a nonzero native SOL movement, and opposing movement signs.
/// Stable-quoted and multi-token routes remain `Other` because this activity shape
/// has no independent quote amount for those routes.
fn classify_swap(
    sol_delta_lamports: i128,
    token_deltas: &[(String, i128)],
    decimals: &HashMap<String, u8>,
    venue: Option<String>,
) -> (ActivityKind, Option<&'static str>) {
    if sol_delta_lamports.unsigned_abs() <= FEE_NOISE_LAMPORTS as u128 {
        return (
            ActivityKind::Other,
            Some("no SOL-quoted leg for this subject"),
        );
    }

    if token_deltas
        .iter()
        .any(|(mint, delta)| (mint == USDC_MINT || mint == USDT_MINT) && *delta != 0)
    {
        return (ActivityKind::Other, Some("unsupported stable-quoted swap"));
    }

    let candidates: Vec<&(String, i128)> = token_deltas
        .iter()
        .filter(|(mint, _)| mint != SOL_MINT)
        .collect();

    if candidates.len() != 1 {
        return (ActivityKind::Other, Some("ambiguous nonquote token legs"));
    }

    let (mint, token_delta) = candidates[0];
    if (*token_delta > 0) == (sol_delta_lamports > 0) {
        return (
            ActivityKind::Other,
            Some("token and native movements have same sign"),
        );
    }

    let side = if *token_delta > 0 {
        SwapSide::Buy
    } else {
        SwapSide::Sell
    };
    let sol_amount = sol_delta_lamports.unsigned_abs() as f64 / LAMPORTS_PER_SOL as f64;
    let token_amount = token_delta.unsigned_abs() as f64
        / 10_f64.powi(i32::from(*decimals.get(mint).unwrap_or(&0)));
    let price_sol = sol_amount / token_amount;
    if !sol_amount.is_finite()
        || !token_amount.is_finite()
        || !price_sol.is_finite()
        || sol_amount <= 0.0
        || token_amount <= 0.0
        || price_sol <= 0.0
    {
        return (ActivityKind::Other, Some("invalid swap amount"));
    }

    (
        ActivityKind::Swap {
            mint: mint.clone(),
            side,
            native_amount: sol_amount,
            token_amount,
            venue,
            price_native: Some(price_sol),
        },
        None,
    )
}

/// No DEX program is present. A single clear leg (one token delta, or a SOL delta
/// well above fee noise with no token delta at all) is a plain transfer; anything
/// else is left as `Other`. This is deliberately conservative: "a coincidental SOL
/// fee delta plus a token delta, no DEX program in sight" is exactly the
/// airdrop/CEX-deposit shape that must never come out as a sell, so it is not forced
/// into a `Transfer` either -- it is simply not confidently anything.
fn classify_transfer(
    sol_delta_lamports: i128,
    token_deltas: &[(String, i128)],
    decimals: &HashMap<String, u8>,
) -> (ActivityKind, Option<&'static str>) {
    if token_deltas.len() == 1 {
        let (mint, delta) = &token_deltas[0];
        let direction = if *delta > 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        return (
            ActivityKind::Transfer {
                mint: mint.clone(),
                amount: delta.unsigned_abs() as f64
                    / 10_f64.powi(i32::from(*decimals.get(mint).unwrap_or(&0))),
                direction,
            },
            None,
        );
    }

    if token_deltas.is_empty() && sol_delta_lamports.unsigned_abs() > FEE_NOISE_LAMPORTS as u128 {
        let direction = if sol_delta_lamports > 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        return (
            ActivityKind::Transfer {
                mint: SOL_MINT.to_owned(),
                amount: sol_delta_lamports.unsigned_abs() as f64 / LAMPORTS_PER_SOL as f64,
                direction,
            },
            None,
        );
    }

    (
        ActivityKind::Other,
        Some("no DEX program and no single clear transfer leg"),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const SUBJECT: &str = "Subject111111111111111111111111111111111";
    const FEE_PAYER: &str = "FeePayer11111111111111111111111111111111";

    fn details(meta: serde_json::Value) -> crate::chains::solana::rpc::TransactionDetails {
        serde_json::from_value(json!({
            "slot": 1,
            "transaction": {
                "message": { "accountKeys": [FEE_PAYER, SUBJECT] },
                "signatures": ["signature"]
            },
            "meta": meta,
            "blockTime": 1
        }))
        .expect("transaction fixture parses")
    }

    fn unchanged_meta() -> serde_json::Value {
        json!({
            "err": null,
            "fee": 5_000,
            "preBalances": [1_000_000, 2_000_000],
            "postBalances": [995_000, 2_000_000],
            "preTokenBalances": [],
            "postTokenBalances": [],
            "computeUnitsConsumed": null,
            "logMessages": [],
            "innerInstructions": []
        })
    }

    #[test]
    fn read_only_reference_has_no_subject_effect() {
        assert!(!subject_has_meaningful_effect(
            SUBJECT,
            &details(unchanged_meta())
        ));
    }

    #[test]
    fn fee_only_subject_transaction_has_no_subject_effect() {
        let mut transaction = details(unchanged_meta());
        transaction.transaction.message = json!({ "accountKeys": [SUBJECT] });
        transaction
            .meta
            .as_mut()
            .expect("fixture has metadata")
            .pre_balances = vec![1_000_000];
        transaction
            .meta
            .as_mut()
            .expect("fixture has metadata")
            .post_balances = vec![995_000];

        assert!(!subject_has_meaningful_effect(SUBJECT, &transaction));
    }

    #[test]
    fn sol_balance_change_has_subject_effect() {
        let mut transaction = details(unchanged_meta());
        transaction
            .meta
            .as_mut()
            .expect("fixture has metadata")
            .post_balances[1] = 2_000_001;

        assert!(subject_has_meaningful_effect(SUBJECT, &transaction));
    }

    #[test]
    fn delegated_token_change_has_subject_effect_without_subject_signature() {
        let mut meta = unchanged_meta();
        meta["preTokenBalances"] = json!([{
            "accountIndex": 1,
            "mint": "Mint1111111111111111111111111111111111111",
            "owner": SUBJECT,
            "uiTokenAmount": { "amount": "10", "decimals": 0, "uiAmount": 10.0 }
        }]);
        meta["postTokenBalances"] = json!([{
            "accountIndex": 1,
            "mint": "Mint1111111111111111111111111111111111111",
            "owner": SUBJECT,
            "uiTokenAmount": { "amount": "9", "decimals": 0, "uiAmount": 9.0 }
        }]);

        assert!(subject_has_meaningful_effect(SUBJECT, &details(meta)));
    }

    #[test]
    fn missing_or_ambiguous_metadata_fails_open() {
        let missing_meta = details(serde_json::Value::Null);
        assert!(subject_has_meaningful_effect(SUBJECT, &missing_meta));

        let mut ambiguous = unchanged_meta();
        ambiguous["preTokenBalances"] = json!([{
            "accountIndex": 99,
            "mint": "Mint1111111111111111111111111111111111111",
            "owner": SUBJECT,
            "uiTokenAmount": { "amount": "0", "decimals": 0, "uiAmount": 0.0 }
        }]);
        ambiguous["postTokenBalances"] = ambiguous["preTokenBalances"].clone();
        assert!(subject_has_meaningful_effect(SUBJECT, &details(ambiguous)));
    }
}
