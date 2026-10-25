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
use crate::wallets::watch::{ActivityKind, SwapSide, TransferDirection, UnresolvedSwapReason};

/// Below this, a SOL balance delta is indistinguishable from transaction-fee noise
/// (base fee + a moderate priority fee) rather than a real transfer.
const FEE_NOISE_LAMPORTS: i64 = 50_000;

/// Native movement a stable-quoted swap may carry beside its stable leg: rent for
/// a token account it opens or closes, plus fees. Larger movement means the swap
/// was also paid in native SOL and has no single quote.
const STABLE_NATIVE_RESIDUE_LAMPORTS: i64 = 10_000_000;

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
/// `Other` or `UnresolvedSwap` because an explicit rule below declined to call it a
/// `Swap`, `StableSwap` or `Transfer` (no DEX program, ambiguous token legs, a mixed
/// quote) rather than because nothing happened at all -- the funnel logs it so
/// "why didn't this copy" never has to be guessed at.
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
        let own_accounts = subject_accounts(raw, &account_keys, subject_address, &owned_indices);
        let native_leg_lamports = external_native_transfers(raw, subject_address, &own_accounts)
            .map(|external| sol_delta_lamports + external);
        classify_swap(native_leg_lamports, &token_deltas, &token_decimals, venue)
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

/// Every parsed instruction of the transaction, top level first, then inner ones.
fn parsed_instructions(raw: &serde_json::Value) -> impl Iterator<Item = &serde_json::Value> {
    let top_level = raw
        .pointer("/transaction/message/instructions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten();
    let inner = raw
        .pointer("/meta/innerInstructions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|group| {
            group
                .get("instructions")
                .and_then(serde_json::Value::as_array)
        })
        .flatten();
    top_level.chain(inner)
}

fn info_str<'a>(instruction: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    instruction
        .pointer(&format!("/parsed/info/{field}"))
        .and_then(serde_json::Value::as_str)
}

/// Addresses whose lamports are part of the subject's own balance sheet: token
/// accounts it holds before or after the transaction, and token accounts the
/// transaction opens for it. A temporary wSOL account is created, funded, swapped
/// through and closed inside one transaction, so no token-balance list names it.
fn subject_accounts(
    raw: &serde_json::Value,
    account_keys: &[String],
    subject_address: &str,
    owned_indices: &HashSet<usize>,
) -> HashSet<String> {
    let mut accounts = owned_indices
        .iter()
        .filter_map(|index| account_keys.get(*index).cloned())
        .collect::<HashSet<_>>();
    for instruction in parsed_instructions(raw) {
        let program = instruction
            .get("program")
            .and_then(serde_json::Value::as_str);
        let kind = instruction
            .pointer("/parsed/type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let opened_for_subject = match program {
            Some("spl-token" | "spl-token-2022") if kind.starts_with("initializeAccount") => {
                info_str(instruction, "owner") == Some(subject_address)
            }
            Some("spl-associated-token-account")
                if matches!(kind, "create" | "createIdempotent") =>
            {
                info_str(instruction, "wallet") == Some(subject_address)
            }
            _ => false,
        };
        if let Some(account) = info_str(instruction, "account").filter(|_| opened_for_subject) {
            accounts.insert(account.to_owned());
        }
    }
    accounts
}

/// Net lamports the subject moved through top-level system transfers to or from
/// accounts outside its own balance sheet, positive when the subject paid out. A
/// priority tip, a trading terminal's fee or a relayer refund rides beside the
/// swap as its own instruction and is no part of the traded amount; a payment into
/// the subject's own temporary wSOL account is the swap itself and stays in it.
/// `None` when a transfer cannot be read.
fn external_native_transfers(
    raw: &serde_json::Value,
    subject_address: &str,
    own_accounts: &HashSet<String>,
) -> Option<i128> {
    let instructions = raw
        .pointer("/transaction/message/instructions")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut external = 0_i128;
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
        let Some(kind) = instruction
            .pointer("/parsed/type")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if !matches!(kind, "transfer" | "transferWithSeed") {
            continue;
        }
        let source = info_str(instruction, "source")?;
        let destination = info_str(instruction, "destination")?;
        let lamports = instruction
            .pointer("/parsed/info/lamports")
            .and_then(serde_json::Value::as_u64)?;
        if source == subject_address && !own_accounts.contains(destination) {
            external += i128::from(lamports);
        } else if destination == subject_address && !own_accounts.contains(source) {
            external -= i128::from(lamports);
        }
    }
    Some(external)
}

fn is_stable(mint: &str) -> bool {
    mint == USDC_MINT || mint == USDT_MINT
}

fn decimal_amount(raw: i128, mint: &str, decimals: &HashMap<String, u8>) -> f64 {
    raw.unsigned_abs() as f64 / 10_f64.powi(i32::from(*decimals.get(mint).unwrap_or(&0)))
}

/// A DEX program is present. One non-quote token leg against native SOL resolves
/// to `Swap`; against exactly one stable coin, with the native movement inside
/// account-rent residue, to `StableSwap`. A swap-shaped transaction (legs moving in
/// both directions around at least one non-quote token) that fits neither becomes
/// `UnresolvedSwap` so its consumers can say why it was not copied; anything else,
/// such as wrapping SOL or claiming a token, stays `Other`.
fn classify_swap(
    native_leg_lamports: Option<i128>,
    token_deltas: &[(String, i128)],
    decimals: &HashMap<String, u8>,
    venue: Option<String>,
) -> (ActivityKind, Option<&'static str>) {
    let candidates: Vec<&(String, i128)> = token_deltas
        .iter()
        .filter(|(mint, _)| mint != SOL_MINT && !is_stable(mint))
        .collect();
    let stables: Vec<&(String, i128)> = token_deltas
        .iter()
        .filter(|(mint, _)| is_stable(mint))
        .collect();
    if candidates.is_empty() {
        return (ActivityKind::Other, Some("no nonquote token leg"));
    }
    let sole_mint = (candidates.len() == 1).then(|| candidates[0].0.clone());
    let unresolved = |reason: UnresolvedSwapReason, why: &'static str| {
        (
            ActivityKind::UnresolvedSwap {
                mint: sole_mint.clone(),
                reason,
                venue: venue.clone(),
            },
            Some(why),
        )
    };
    let Some(native_leg) = native_leg_lamports else {
        return unresolved(
            UnresolvedSwapReason::Unreadable,
            "unreadable top-level native transfer",
        );
    };

    let native_counts = native_leg.unsigned_abs() > FEE_NOISE_LAMPORTS as u128;
    let legs = candidates
        .iter()
        .chain(stables.iter())
        .map(|(_, delta)| *delta)
        .chain(native_counts.then_some(native_leg));
    let (mut gains, mut losses) = (false, false);
    for delta in legs {
        gains |= delta > 0;
        losses |= delta < 0;
    }
    if !(gains && losses) {
        return (ActivityKind::Other, Some("no opposing leg for the token"));
    }
    if candidates.len() > 1 || stables.len() > 1 {
        return unresolved(
            UnresolvedSwapReason::MultipleTokens,
            "more than one nonquote token leg",
        );
    }

    let (mint, token_delta) = candidates[0];
    let side = if *token_delta > 0 {
        SwapSide::Buy
    } else {
        SwapSide::Sell
    };
    let token_amount = decimal_amount(*token_delta, mint, decimals);

    if let Some((stable_mint, stable_delta)) = stables.first().copied() {
        if native_leg.unsigned_abs() > STABLE_NATIVE_RESIDUE_LAMPORTS as u128 {
            return unresolved(
                UnresolvedSwapReason::MixedQuote,
                "native and stable quote legs together",
            );
        }
        let stable_amount = decimal_amount(*stable_delta, stable_mint, decimals);
        if (*stable_delta > 0) == (*token_delta > 0)
            || !stable_amount.is_finite()
            || !token_amount.is_finite()
            || stable_amount <= 0.0
            || token_amount <= 0.0
        {
            return unresolved(
                UnresolvedSwapReason::Unreadable,
                "invalid stable swap amount",
            );
        }
        return (
            ActivityKind::StableSwap {
                mint: mint.clone(),
                side,
                stable_mint: stable_mint.clone(),
                stable_amount,
                token_amount,
                venue,
            },
            None,
        );
    }

    let sol_amount = native_leg.unsigned_abs() as f64 / LAMPORTS_PER_SOL as f64;
    let price_sol = sol_amount / token_amount;
    if !sol_amount.is_finite()
        || !token_amount.is_finite()
        || !price_sol.is_finite()
        || sol_amount <= 0.0
        || token_amount <= 0.0
        || price_sol <= 0.0
    {
        return unresolved(UnresolvedSwapReason::Unreadable, "invalid swap amount");
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
