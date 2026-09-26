//! Pure classification: a decoded transaction -> one `ActivityKind`, subject-relative.
//!
//! The single most expensive decode bug available here is treating a token transfer
//! (airdrop, CEX deposit, wallet consolidation) as a sell. This module's one job is
//! to make that impossible: a `Swap` is only ever produced when a known DEX/
//! aggregator program is present in the transaction (`program_ids::
//! detect_router_from_program_id`) -- a balance delta alone, however swap-shaped, is
//! never sufficient. See PLAN.md §6.4.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::chains::solana::constants::lamports_to_sol;
use crate::chains::solana::constants::{SOL_MINT, USDC_MINT, USDT_MINT};
use crate::chains::solana::transactions::analyzer::balance::account_keys_from_message;
use crate::chains::solana::transactions::analyzer::dex::extract_program_ids;
use crate::chains::solana::transactions::program_ids::detect_router_from_program_id;
use crate::transactions::types::Transaction;
use crate::wallets::watch::{ActivityKind, SwapSide, TransferDirection};

/// Below this, a SOL balance delta is indistinguishable from transaction-fee noise
/// (base fee + a moderate priority fee) rather than a real transfer.
const FEE_NOISE_LAMPORTS: i64 = 50_000;

/// Floating-point tolerance for "no change" when comparing UI token/SOL amounts.
const EPSILON: f64 = 1e-9;

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

    let (Some(pre_sol), Some(post_sol)) = (
        meta.pre_balances.get(*subject_index),
        meta.post_balances.get(*subject_index),
    ) else {
        return true;
    };
    let mut sol_delta = i128::from(*post_sol) - i128::from(*pre_sol);
    if *subject_index == 0 {
        sol_delta += i128::from(meta.fee);
    }
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
/// (no DEX program, ambiguous multi-hop, a route with no SOL-quoted leg) rather than
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

    let observed_sol_delta = subject_sol_delta_lamports(meta, subject_index);
    // Account key 0 is the fee payer. Its observed balance delta includes the base
    // and priority fee, which is not part of the quoted swap leg. Add the fee back
    // to recover the trade/transfer delta before classification and sizing.
    let sol_delta_lamports = if subject_index == 0 {
        observed_sol_delta.saturating_add(meta.fee.min(i64::MAX as u64) as i64)
    } else {
        observed_sol_delta
    };
    let token_deltas = subject_token_deltas(meta, subject_address);

    let venue = extract_program_ids(&tx_data).ok().and_then(|ids| {
        ids.iter()
            .find_map(|id| detect_router_from_program_id(id).map(str::to_owned))
    });

    Some(if venue.is_some() {
        classify_swap(sol_delta_lamports, &token_deltas, venue)
    } else {
        classify_transfer(sol_delta_lamports, &token_deltas)
    })
}

/// The subject's own SOL balance change, in lamports. `0` when the subject's account
/// index cannot be matched against the pre/post balance arrays (should not happen --
/// `subject_index` was resolved from the same account-keys list those arrays are
/// indexed by -- but a mismatched jsonParsed response must never panic).
fn subject_sol_delta_lamports(
    meta: &crate::chains::solana::rpc::TransactionMeta,
    subject_index: usize,
) -> i64 {
    let pre = meta.pre_balances.get(subject_index).copied().unwrap_or(0) as i64;
    let post = meta.post_balances.get(subject_index).copied().unwrap_or(0) as i64;
    post - pre
}

/// `(mint, ui delta)` for every token balance change owned by `subject_address`.
/// Mirrors `analyzer::balance::extract_token_balance_changes`'s pre/post merge, but
/// filtered to one owner instead of flattening every account in the transaction --
/// that flattening is exactly what makes the shared `Transaction.token_balance_
/// changes` field unusable for a subject-relative decision (the owning account is
/// discarded once every account's changes are merged into one list).
fn subject_token_deltas(
    meta: &crate::chains::solana::rpc::TransactionMeta,
    subject_address: &str,
) -> Vec<(String, f64)> {
    let empty = Vec::new();
    let pre = meta.pre_token_balances.as_ref().unwrap_or(&empty);
    let post = meta.post_token_balances.as_ref().unwrap_or(&empty);

    let mut pre_map: HashMap<(u32, &str), f64> = HashMap::new();
    for balance in pre {
        if balance.owner.as_deref() == Some(subject_address) {
            pre_map.insert(
                (balance.account_index, balance.mint.as_str()),
                balance.ui_token_amount.ui_amount.unwrap_or(0.0),
            );
        }
    }

    let mut post_map: HashMap<(u32, &str), f64> = HashMap::new();
    for balance in post {
        if balance.owner.as_deref() == Some(subject_address) {
            post_map.insert(
                (balance.account_index, balance.mint.as_str()),
                balance.ui_token_amount.ui_amount.unwrap_or(0.0),
            );
        }
    }

    let mut keys: HashSet<(u32, &str)> = HashSet::new();
    keys.extend(pre_map.keys().copied());
    keys.extend(post_map.keys().copied());

    let mut deltas: HashMap<String, f64> = HashMap::new();
    for key in keys {
        let pre_ui = pre_map.get(&key).copied().unwrap_or(0.0);
        let post_ui = post_map.get(&key).copied().unwrap_or(0.0);
        let delta = post_ui - pre_ui;
        if delta.abs() > EPSILON {
            *deltas.entry(key.1.to_owned()).or_insert(0.0) += delta;
        }
    }

    deltas.into_iter().collect()
}

/// A DEX program is present. Resolve to `Swap` only when the subject has exactly one
/// dominant non-SOL, non-stable leg AND a nonzero SOL-side delta -- V1 handles
/// SOL-quoted legs only (a USDC-quoted route, or a route with no SOL leg at all for
/// this subject, is skipped with a reason rather than mis-sized).
fn classify_swap(
    sol_delta_lamports: i64,
    token_deltas: &[(String, f64)],
    venue: Option<String>,
) -> (ActivityKind, Option<&'static str>) {
    if sol_delta_lamports.abs() <= FEE_NOISE_LAMPORTS {
        return (
            ActivityKind::Other,
            Some("no SOL-quoted leg for this subject"),
        );
    }

    let candidates: Vec<&(String, f64)> = token_deltas
        .iter()
        .filter(|(mint, _)| mint != SOL_MINT && mint != USDC_MINT && mint != USDT_MINT)
        .collect();

    if candidates.is_empty() {
        return (ActivityKind::Other, Some("no non-stable token leg found"));
    }

    // Ambiguous multi-hop: more than one candidate mint and no single dominant leg
    // (the runner-up is within half the magnitude of the leader) -- never guess.
    if candidates.len() >= 2 {
        let mut magnitudes: Vec<f64> = candidates.iter().map(|(_, d)| d.abs()).collect();
        magnitudes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
        if magnitudes[1] > magnitudes[0] * 0.5 {
            return (
                ActivityKind::Other,
                Some("ambiguous multi-hop: no single dominant mint"),
            );
        }
    }

    let (mint, token_delta) = candidates
        .into_iter()
        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(Ordering::Equal))
        .expect("candidates is non-empty");

    let side = if *token_delta > 0.0 {
        SwapSide::Buy
    } else {
        SwapSide::Sell
    };
    let sol_amount = lamports_to_sol(sol_delta_lamports.unsigned_abs());
    let token_amount = token_delta.abs();
    let price_sol = (token_amount > EPSILON).then_some(sol_amount / token_amount);

    (
        ActivityKind::Swap {
            mint: mint.clone(),
            side,
            sol_amount,
            token_amount,
            venue,
            price_sol,
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
    sol_delta_lamports: i64,
    token_deltas: &[(String, f64)],
) -> (ActivityKind, Option<&'static str>) {
    if token_deltas.len() == 1 {
        let (mint, delta) = &token_deltas[0];
        let direction = if *delta > 0.0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        return (
            ActivityKind::Transfer {
                mint: mint.clone(),
                amount: delta.abs(),
                direction,
            },
            None,
        );
    }

    if token_deltas.is_empty() && sol_delta_lamports.abs() > FEE_NOISE_LAMPORTS {
        let direction = if sol_delta_lamports > 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        return (
            ActivityKind::Transfer {
                mint: SOL_MINT.to_owned(),
                amount: lamports_to_sol(sol_delta_lamports.unsigned_abs()),
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
