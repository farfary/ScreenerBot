//! Subject-relative balance deltas — pure extraction from a decoded Solana transaction.
//!
//! Mirrors the parsing prologue in `chains::solana::wallets::classify` (account keys, pre/post
//! token balances filtered by owner) but keeps every mint's delta instead of
//! collapsing to one dominant swap leg. Produces [`SubjectAssetDelta`], the chain-neutral
//! type owned by `crate::transactions::deltas`.
//!
//! Deliberately does NOT use `Transaction.token_balance_changes` -- that field is
//! flattened across every account in the transaction (pool included) with no owner
//! attached, so it cannot answer "what did the SUBJECT'S balance do".

use std::collections::{HashMap, HashSet};

use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::transactions::analyzer::balance::account_keys_from_message;
use crate::chains::solana::transactions::analyzer::dex::extract_program_ids;
use crate::chains::solana::transactions::program_ids::detect_router_from_program_id;
use crate::chains::ChainId;
use crate::transactions::deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};
use crate::transactions::types::Transaction;

/// Below this, a native SOL delta is indistinguishable from transaction-fee noise.
const NATIVE_NOISE_LAMPORTS: i128 = 50_000;

/// Programs that never indicate a `DeFi` interaction on their own -- system-level
/// plumbing present in nearly every transaction (fee payment, compute budget,
/// SPL token transfers, ATA creation, sysvar reads).
const SYSTEM_LEVEL_PROGRAM_IDS: &[&str] = &[
    "11111111111111111111111111111111",
    "ComputeBudget111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    "Sysvar1nstructions1111111111111111111111111",
];

/// Extract every subject-relative asset delta from one decoded transaction. Pure,
/// never panics: a malformed or missing field simply yields fewer deltas.
pub fn extract_subject_deltas(
    subject_address: &str,
    transaction: &Transaction,
) -> Vec<SubjectAssetDelta> {
    if !transaction.success {
        return Vec::new();
    }

    let Some(raw) = transaction.raw_transaction_data.as_ref() else {
        return Vec::new();
    };
    let Ok(tx_data) =
        serde_json::from_value::<crate::chains::solana::rpc::TransactionDetails>(raw.clone())
    else {
        return Vec::new();
    };
    let Some(meta) = tx_data.meta.as_ref() else {
        return Vec::new();
    };

    let account_keys = account_keys_from_message(&tx_data.transaction.message);
    let Some(subject_index) = account_keys.iter().position(|k| k == subject_address) else {
        return Vec::new();
    };

    // The stored row's own columns are the authority whenever the blob is silent: a
    // cached JSON written by an older encoding can lack the timestamp entirely, and a
    // delta with no `block_time` gives its round no `opened_at`/`closed_at`, so the
    // position materialised from it is dated "now" and jumps to the top of the Closed
    // list ahead of trades that really are the wallet's most recent.
    let slot = Some(tx_data.slot).filter(|s| *s > 0).or(transaction.slot);
    let block_time = tx_data.block_time.or(transaction.block_time);
    let tx_index = raw
        .get("transactionIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let program_ids = extract_program_ids(&tx_data).unwrap_or_default();
    let venue = program_ids
        .iter()
        .find_map(|id| detect_router_from_program_id(id).map(str::to_owned));

    // ---- token deltas: sum raw amounts per mint across every subject-owned account ----
    let empty: Vec<crate::chains::solana::rpc::TokenBalance> = Vec::new();
    let pre = meta.pre_token_balances.as_ref().unwrap_or(&empty);
    let post = meta.post_token_balances.as_ref().unwrap_or(&empty);

    let mut pre_by_mint: HashMap<String, i128> = HashMap::new();
    let mut post_by_mint: HashMap<String, i128> = HashMap::new();
    let mut decimals_by_mint: HashMap<String, u8> = HashMap::new();
    // (account_index -> mint) for every subject-owned token account, used to correct
    // the native delta for ATA rent below.
    let mut owned_account_mints: HashMap<u32, String> = HashMap::new();

    for balance in pre.iter() {
        if balance.owner.as_deref() != Some(subject_address) {
            continue;
        }
        owned_account_mints.insert(balance.account_index, balance.mint.clone());
        let raw_amount = parse_raw_amount(&balance.ui_token_amount.amount);
        *pre_by_mint.entry(balance.mint.clone()).or_insert(0) += raw_amount;
        decimals_by_mint.insert(balance.mint.clone(), balance.ui_token_amount.decimals);
    }
    for balance in post.iter() {
        if balance.owner.as_deref() != Some(subject_address) {
            continue;
        }
        owned_account_mints.insert(balance.account_index, balance.mint.clone());
        let raw_amount = parse_raw_amount(&balance.ui_token_amount.amount);
        *post_by_mint.entry(balance.mint.clone()).or_insert(0) += raw_amount;
        decimals_by_mint.insert(balance.mint.clone(), balance.ui_token_amount.decimals);
    }

    let mut mints: HashSet<String> = HashSet::new();
    mints.extend(pre_by_mint.keys().cloned());
    mints.extend(post_by_mint.keys().cloned());

    let mut token_deltas: Vec<(String, i128)> = Vec::new();
    for mint in &mints {
        let before = pre_by_mint.get(mint).copied().unwrap_or(0);
        let after = post_by_mint.get(mint).copied().unwrap_or(0);
        let delta = after - before;
        if delta != 0 {
            token_deltas.push((mint.clone(), delta));
        }
    }

    // ---- native SOL delta ----
    let subject_pre = meta.pre_balances.get(subject_index).copied().unwrap_or(0) as i128;
    let subject_post = meta.post_balances.get(subject_index).copied().unwrap_or(0) as i128;
    let mut native_delta = subject_post - subject_pre;
    if subject_index == 0 {
        native_delta += meta.fee as i128;
    }

    // ATA rent correction: creating/closing a wallet-owned (non-wSOL) token account
    // moves rent out of/back into native SOL in the same transaction. That is not
    // swap consideration and must not inflate an entry price or sale proceeds.
    for (&account_index, mint) in owned_account_mints.iter() {
        if account_index as usize == subject_index || mint == SOL_MINT {
            continue;
        }
        let idx = account_index as usize;
        let acc_pre = meta.pre_balances.get(idx).copied().unwrap_or(0) as i128;
        let acc_post = meta.post_balances.get(idx).copied().unwrap_or(0) as i128;
        native_delta += acc_post - acc_pre;
    }

    // ---- classification (transaction-level, shared by every delta emitted) ----
    let has_token_delta = !token_deltas.is_empty();
    let native_above_noise = native_delta.unsigned_abs() > NATIVE_NOISE_LAMPORTS as u128;
    let kind = if venue.is_some() {
        DeltaKind::Trade
    } else if has_token_delta || native_above_noise {
        DeltaKind::Transfer
    } else if program_ids
        .iter()
        .any(|id| !SYSTEM_LEVEL_PROGRAM_IDS.contains(&id.as_str()))
    {
        DeltaKind::DeFi
    } else {
        DeltaKind::Other
    };

    let mut deltas = Vec::new();

    for (mint, delta) in token_deltas {
        let before = pre_by_mint.get(&mint).copied().unwrap_or(0);
        let after = post_by_mint.get(&mint).copied().unwrap_or(0);
        let decimals = decimals_by_mint.get(&mint).copied().unwrap_or(0);
        deltas.push(SubjectAssetDelta {
            chain: ChainId::Solana,
            wallet_address: subject_address.to_owned(),
            signature: transaction.signature.clone(),
            mint,
            slot,
            block_time,
            tx_index,
            delta_raw: delta,
            before_raw: u128::try_from(before).ok(),
            after_raw: u128::try_from(after).ok(),
            decimals,
            kind,
            venue: venue.clone(),
            fee_native_raw: Some(meta.fee),
            success: transaction.success,
        });
    }

    if native_above_noise {
        deltas.push(SubjectAssetDelta {
            chain: ChainId::Solana,
            wallet_address: subject_address.to_owned(),
            signature: transaction.signature.clone(),
            mint: NATIVE_SOL_SENTINEL.to_owned(),
            slot,
            block_time,
            tx_index,
            delta_raw: native_delta,
            before_raw: u128::try_from(subject_pre).ok(),
            after_raw: u128::try_from(subject_post).ok(),
            decimals: 9,
            kind,
            venue: venue.clone(),
            fee_native_raw: Some(meta.fee),
            success: transaction.success,
        });
    }

    deltas
}

/// Parse a raw token amount string (e.g. `"1500000"`) into `i128`. Never panics --
/// a malformed value contributes `0` rather than aborting extraction.
fn parse_raw_amount(amount: &str) -> i128 {
    amount.parse::<i128>().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::transactions::program_ids::RAYDIUM_CPMM_PROGRAM_ID;
    use serde_json::json;

    const SUBJECT: &str = "SubjectWa11etAddre55111111111111111111111";
    const POOL: &str = "Poo1Addre55111111111111111111111111111111";
    const FEE_PAYER: &str = "FeePayerAddre55111111111111111111111111111";
    const MINT_A: &str = "MintA111111111111111111111111111111111111";
    const MINT_B: &str = "MintB111111111111111111111111111111111111";

    fn token_balance(
        account_index: u32,
        owner: &str,
        mint: &str,
        amount: &str,
        decimals: u8,
    ) -> serde_json::Value {
        json!({
            "accountIndex": account_index,
            "mint": mint,
            "owner": owner,
            "uiTokenAmount": {
                "amount": amount,
                "decimals": decimals,
                "uiAmount": null,
                "uiAmountString": amount,
            }
        })
    }

    fn make_transaction(raw: serde_json::Value, success: bool) -> Transaction {
        let mut tx = Transaction::new("test-signature".to_owned());
        tx.success = success;
        tx.raw_transaction_data = Some(raw);
        tx
    }

    #[test]
    fn only_the_subjects_token_balances_produce_deltas() {
        let raw = json!({
            "slot": 100,
            "blockTime": 1000,
            "transactionIndex": 0,
            "transaction": {
                "signatures": ["test-signature"],
                "message": { "accountKeys": [SUBJECT, POOL] }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [1_000_000_000u64, 1_000_000_000u64],
                "postBalances": [1_000_000_000u64, 1_000_000_000u64],
                "preTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "0", 6),
                    token_balance(1, POOL, MINT_A, "5000000", 6),
                ],
                "postTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "1000000", 6),
                    token_balance(1, POOL, MINT_A, "4000000", 6),
                ],
            }
        });

        let tx = make_transaction(raw, true);
        let deltas = extract_subject_deltas(SUBJECT, &tx);

        let token_deltas: Vec<_> = deltas.iter().filter(|d| d.mint == MINT_A).collect();
        assert_eq!(token_deltas.len(), 1);
        assert_eq!(token_deltas[0].delta_raw, 1_000_000);
        assert_eq!(token_deltas[0].chain, ChainId::Solana);
        assert_eq!(token_deltas[0].fee_native_raw, Some(5_000));
    }

    #[test]
    fn the_fee_payers_fee_is_added_back_to_the_native_delta() {
        let raw = json!({
            "slot": 100,
            "blockTime": 1000,
            "transactionIndex": 0,
            "transaction": {
                "signatures": ["test-signature"],
                "message": { "accountKeys": [SUBJECT, POOL] }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [10_000_000_000u64, 0u64],
                "postBalances": [9_000_000_000u64, 0u64],
            }
        });

        let tx = make_transaction(raw, true);
        let deltas = extract_subject_deltas(SUBJECT, &tx);

        let native = deltas
            .iter()
            .find(|d| d.mint == NATIVE_SOL_SENTINEL)
            .expect("native delta present");
        // raw delta = -1_000_000_000, fee payer so add fee back => -999_995_000
        assert_eq!(native.delta_raw, -999_995_000);
    }

    #[test]
    fn ata_rent_is_not_counted_as_native_consideration() {
        let raw = json!({
            "slot": 100,
            "blockTime": 1000,
            "transactionIndex": 0,
            "transaction": {
                "signatures": ["test-signature"],
                "message": { "accountKeys": [FEE_PAYER, SUBJECT, "AtaAccount1111111111111111111111111111111"] }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [1_000_000_000u64, 10_000_000_000u64, 0u64],
                "postBalances": [999_995_000u64, 8_997_960_720u64, 2_039_280u64],
                "preTokenBalances": [],
                "postTokenBalances": [
                    token_balance(2, SUBJECT, MINT_A, "500000", 6),
                ],
            }
        });

        let tx = make_transaction(raw, true);
        let deltas = extract_subject_deltas(SUBJECT, &tx);

        let native = deltas
            .iter()
            .find(|d| d.mint == NATIVE_SOL_SENTINEL)
            .expect("native delta present");
        // Observed subject delta: 8_997_960_720 - 10_000_000_000 = -1_002_039_280.
        // Correction adds back the ATA's own +2_039_280 lamport change => -1_000_000_000.
        assert_eq!(native.delta_raw, -1_000_000_000);
    }

    #[test]
    fn a_multi_leg_swap_emits_one_delta_per_mint() {
        let raw = json!({
            "slot": 100,
            "blockTime": 1000,
            "transactionIndex": 0,
            "transaction": {
                "signatures": ["test-signature"],
                "message": {
                    "accountKeys": [SUBJECT, POOL],
                    "instructions": [
                        { "programId": RAYDIUM_CPMM_PROGRAM_ID, "accounts": [], "data": "" }
                    ]
                }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [1_000_000_000u64, 1_000_000_000u64],
                "postBalances": [1_000_000_000u64, 1_000_000_000u64],
                "preTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "1000000", 6),
                    token_balance(0, SUBJECT, MINT_B, "0", 6),
                ],
                "postTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "0", 6),
                    token_balance(0, SUBJECT, MINT_B, "2000000", 6),
                ],
            }
        });

        let tx = make_transaction(raw, true);
        let deltas = extract_subject_deltas(SUBJECT, &tx);

        assert_eq!(deltas.len(), 2, "expected exactly one delta per mint");
        let a = deltas.iter().find(|d| d.mint == MINT_A).unwrap();
        let b = deltas.iter().find(|d| d.mint == MINT_B).unwrap();
        assert_eq!(a.delta_raw, -1_000_000);
        assert_eq!(b.delta_raw, 2_000_000);
        assert_eq!(a.kind, DeltaKind::Trade);
        assert_eq!(b.venue.as_deref(), Some("raydium"));
    }

    #[test]
    fn a_failed_transaction_produces_no_deltas() {
        let raw = json!({
            "slot": 100,
            "blockTime": 1000,
            "transactionIndex": 0,
            "transaction": {
                "signatures": ["test-signature"],
                "message": { "accountKeys": [SUBJECT, POOL] }
            },
            "meta": {
                "err": { "InstructionError": [0, "Custom"] },
                "fee": 5000,
                "preBalances": [10_000_000_000u64, 0u64],
                "postBalances": [9_000_000_000u64, 0u64],
                "preTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "0", 6),
                ],
                "postTokenBalances": [
                    token_balance(0, SUBJECT, MINT_A, "1000000", 6),
                ],
            }
        });

        let tx = make_transaction(raw, false);
        let deltas = extract_subject_deltas(SUBJECT, &tx);
        assert!(deltas.is_empty());
    }

    /// A one-token buy, with the timestamp key spelled however the caller asks for.
    fn buy_raw(block_time_key: Option<&str>) -> serde_json::Value {
        let mut raw = json!({
            "slot": 100,
            "transaction": {
                "signatures": ["test-signature"],
                "message": { "accountKeys": [SUBJECT, POOL] }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [10_000_000_000u64, 0u64],
                "postBalances": [9_000_000_000u64, 0u64],
                "preTokenBalances": [token_balance(0, SUBJECT, MINT_A, "0", 6)],
                "postTokenBalances": [token_balance(0, SUBJECT, MINT_A, "1000000", 6)],
            }
        });
        if let Some(key) = block_time_key {
            raw[key] = json!(1_700_000_000i64);
        }
        raw
    }

    #[test]
    fn a_cached_blob_written_in_snake_case_still_carries_its_timestamp() {
        // Blobs cached before `TransactionDetails` renamed the field round-tripped it as
        // `block_time`. Losing it dated every position derived from the round to the
        // moment it was re-read, which sorted ancient trades to the top of Closed.
        let tx = make_transaction(buy_raw(Some("block_time")), true);
        let deltas = extract_subject_deltas(SUBJECT, &tx);

        assert!(!deltas.is_empty());
        assert!(deltas.iter().all(|d| d.block_time == Some(1_700_000_000)));
    }

    #[test]
    fn a_blob_with_no_timestamp_falls_back_to_the_stored_row() {
        let mut tx = make_transaction(buy_raw(None), true);
        tx.block_time = Some(1_700_000_001);

        let deltas = extract_subject_deltas(SUBJECT, &tx);

        assert!(!deltas.is_empty());
        assert!(deltas.iter().all(|d| d.block_time == Some(1_700_000_001)));
    }

    #[test]
    fn the_blobs_own_timestamp_wins_over_the_stored_row() {
        let mut tx = make_transaction(buy_raw(Some("blockTime")), true);
        tx.block_time = Some(1);

        let deltas = extract_subject_deltas(SUBJECT, &tx);

        assert!(deltas.iter().all(|d| d.block_time == Some(1_700_000_000)));
    }
}
