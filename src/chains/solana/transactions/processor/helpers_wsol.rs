//! Transaction processor helpers — WSOL/wrap operations.
//
// Functions for detecting WSOL wraps, sync-native deposits, and wallet WSOL ATA resolution.

use serde_json::Value;

use crate::chains::solana::transactions::program_ids::*;
use crate::transactions::utils::*;

use super::helpers::account_keys_from_message;
use super::helpers::resolve_account_keys_vec;
use super::helpers_transfers::sum_system_transfers_to_account_from_wallet;

/// Find the WSOL token amount that left the wallet (owner-aggregated token change) as UI amount
pub(super) fn find_owner_wsol_change_ui(
    analysis: &crate::chains::solana::transactions::analyzer::CompleteAnalysis,
    wallet_key: &str,
) -> Option<f64> {
    analysis
        .balance
        .token_changes
        .get(wallet_key)
        .and_then(|changes| {
            changes
                .iter()
                .find(|c| c.mint == WSOL_MINT && c.change < 0.0)
                .map(|c| c.change.abs())
        })
}

/// Find lamports deposited into the wallet's WSOL ATA by inspecting pre/post balances at that account index
pub(super) fn find_wsol_wrap_deposit_lamports(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    let meta = tx_data.meta.as_ref()?;
    let empty_vec: Vec<crate::chains::solana::rpc::TokenBalance> = Vec::new();
    let pre_token = meta.pre_token_balances.as_ref().unwrap_or(&empty_vec);
    let post_empty: Vec<crate::chains::solana::rpc::TokenBalance> = Vec::new();
    let post_token = meta.post_token_balances.as_ref().unwrap_or(&post_empty);

    // Build candidate account indices for WSOL accounts owned by wallet
    let mut indices: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for bal in pre_token.iter().chain(post_token.iter()) {
        if bal.mint == WSOL_MINT {
            indices.insert(bal.account_index);
        }
    }
    if indices.is_empty() {
        return None;
    }

    let pre = &meta.pre_balances;
    let post = &meta.post_balances;
    let mut best: u64 = 0;
    for idx in indices {
        let i = idx as usize;
        if i >= pre.len() || i >= post.len() {
            continue;
        }
        let pre_b = pre[i];
        let post_b = post[i];
        if post_b > pre_b {
            let delta = post_b - pre_b;
            if delta > best {
                best = delta;
            }
        }
    }
    if best > 0 {
        Some(best)
    } else {
        None
    }
}

/// Detect wrap deposit by looking for Token Program syncNative instructions and measuring lamport delta on that account index
pub(super) fn find_wrap_deposit_via_sync_native(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> Option<u64> {
    let meta = tx_data.meta.as_ref()?;
    let pre = &meta.pre_balances;
    let post = &meta.post_balances;
    let keys = account_keys_from_message(&tx_data.transaction.message);

    let mut best: u64 = 0;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if let Some(ix_type) = parsed.get("type").and_then(|v| v.as_str()) {
                if ix_type == "syncNative" {
                    if let Some(info) = parsed.get("info") {
                        if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
                            if let Some(index) = keys.iter().position(|k| k == account) {
                                if index < pre.len() && index < post.len() {
                                    let pre_b = pre[index];
                                    let post_b = post[index];
                                    if post_b > pre_b {
                                        let delta = post_b - pre_b;
                                        if delta > best {
                                            best = delta;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    if let Some(ixs) = tx_data
        .transaction
        .message
        .get("instructions")
        .and_then(|v| v.as_array())
    {
        for ix in ixs {
            consider_ix(ix);
        }
    }
    if let Some(inner) = &meta.inner_instructions {
        for group in inner {
            if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
                for ix in ixs {
                    consider_ix(ix);
                }
            }
        }
    }

    if best > 0 {
        Some(best)
    } else {
        None
    }
}

/// Collect wallet-owned WSOL ATA addresses from pre/post token balances
pub(super) fn get_wallet_wsol_ata_addresses(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Vec<String> {
    let meta = match tx_data.meta.as_ref() {
        Some(m) => m,
        None => {
            return Vec::new();
        }
    };
    let message = &tx_data.transaction.message;
    let account_keys = resolve_account_keys_vec(message);

    let mut indices: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if let Some(pre) = &meta.pre_token_balances {
        for bal in pre {
            if bal.mint == WSOL_MINT && bal.owner.as_deref() == Some(wallet_key) {
                indices.insert(bal.account_index);
            }
        }
    }
    if let Some(post) = &meta.post_token_balances {
        for bal in post {
            if bal.mint == WSOL_MINT && bal.owner.as_deref() == Some(wallet_key) {
                indices.insert(bal.account_index);
            }
        }
    }

    // Also look for createIdempotent that targets a WSOL ATA owned by the wallet; include that account even if
    // it has zero pre/post token balance entries (e.g., created and closed within the same tx)
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if parsed.get("type").and_then(|v| v.as_str()) == Some("createIdempotent") {
                if let Some(info) = parsed.get("info") {
                    let account = info.get("account").and_then(|v| v.as_str());
                    let mint = info.get("mint").and_then(|v| v.as_str());
                    let wallet = info.get("wallet").and_then(|v| v.as_str());
                    if let Some(acc) = account {
                        if mint == Some(WSOL_MINT) && wallet == Some(wallet_key) {
                            if let Some(index) = account_keys.iter().position(|k| k == acc) {
                                indices.insert(index as u32);
                            }
                        }
                    }
                }
            }
        }
    };
    if let Some(ixs) = tx_data
        .transaction
        .message
        .get("instructions")
        .and_then(|v| v.as_array())
    {
        for ix in ixs {
            consider_ix(ix);
        }
    }
    if let Some(inner) = &meta.inner_instructions {
        for group in inner {
            if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
                for ix in ixs {
                    consider_ix(ix);
                }
            }
        }
    }

    let mut addrs = Vec::new();
    for idx in indices {
        let i = idx as usize;
        if i < account_keys.len() {
            addrs.push(account_keys[i].clone());
        }
    }
    addrs
}

/// Find wrap deposit by summing system transfers from wallet to their WSOL ATA addresses
pub(super) fn find_wrap_deposit_via_sys_transfers_to_wsol_atas(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    let wsol_atas = get_wallet_wsol_ata_addresses(tx_data, wallet_key);
    if wsol_atas.is_empty() {
        return None;
    }

    let mut total: u64 = 0;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            let ix_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if ix_type == "transfer" {
                if let Some(info) = parsed.get("info") {
                    let source = info
                        .get("source")
                        .and_then(|v| v.as_str())
                        .or_else(|| info.get("from").and_then(|v| v.as_str()))
                        .unwrap_or_default();
                    let dest = info
                        .get("destination")
                        .and_then(|v| v.as_str())
                        .or_else(|| info.get("to").and_then(|v| v.as_str()))
                        .unwrap_or_default();
                    if source == wallet_key && wsol_atas.iter().any(|a| a == dest) {
                        if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
                            total = total.saturating_add(lamports);
                        }
                    }
                }
            }
        }
    };

    if let Some(instructions) = tx_data
        .transaction
        .message
        .get("instructions")
        .and_then(|v| v.as_array())
    {
        for ix in instructions {
            consider_ix(ix);
        }
    }
    if let Some(meta) = &tx_data.meta {
        if let Some(inner) = &meta.inner_instructions {
            for group in inner {
                if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
                    for ix in ixs {
                        consider_ix(ix);
                    }
                }
            }
        }
    }

    if total > 0 {
        Some(total)
    } else {
        None
    }
}

/// Detect syncNative account and its lamport delta; returns (account_pubkey, delta_lamports)
pub(super) fn find_wrap_sync_account_and_delta(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> Option<(String, u64)> {
    let meta = tx_data.meta.as_ref()?;
    let pre = &meta.pre_balances;
    let post = &meta.post_balances;
    let keys = account_keys_from_message(&tx_data.transaction.message);

    let mut result: Option<(String, u64)> = None;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if let Some(ix_type) = parsed.get("type").and_then(|v| v.as_str()) {
                if ix_type == "syncNative" {
                    if let Some(info) = parsed.get("info") {
                        if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
                            if let Some(index) = keys.iter().position(|k| k == account) {
                                if index < pre.len() && index < post.len() {
                                    let pre_b = pre[index];
                                    let post_b = post[index];
                                    // Even if delta is zero (e.g., wrap then full spend and close), the account is still the WSOL ATA
                                    let delta = if post_b > pre_b { post_b - pre_b } else { 0 };
                                    result = Some((account.to_string(), delta));
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    if let Some(ixs) = tx_data
        .transaction
        .message
        .get("instructions")
        .and_then(|v| v.as_array())
    {
        for ix in ixs {
            consider_ix(ix);
        }
    }
    if let Some(meta) = &tx_data.meta {
        if let Some(inner) = &meta.inner_instructions {
            for group in inner {
                if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
                    for ix in ixs {
                        consider_ix(ix);
                    }
                }
            }
        }
    }

    result
}

/// Detect wrap deposit by matching the syncNative account with explicit system transfer from wallet
pub(super) fn find_wrap_deposit_via_transfer_to_sync_account(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    let (sync_account, _delta) = find_wrap_sync_account_and_delta(tx_data)?;
    sum_system_transfers_to_account_from_wallet(tx_data, wallet_key, &sync_account)
}

/// Sum uiAmount across all inner instructions where the parsed info indicates a
/// transferChecked of WSOL (So1111...), returning SOL units. Robust to split/referral fees.
pub(super) fn sum_inner_wsol_transferchecked_ui(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> f64 {
    let meta = match tx_data.meta.as_ref() {
        Some(m) => m,
        None => {
            return 0.0;
        }
    };
    let inner = match meta.inner_instructions.as_ref() {
        Some(v) => v,
        None => {
            return 0.0;
        }
    };
    let mut total_ui: f64 = 0.0;
    for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
            for ix in ixs {
                if let Some(parsed) = ix.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        let mint = info
                            .get("mint")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        if mint == crate::transactions::utils::WSOL_MINT {
                            if let Some(token_amount) = info.get("tokenAmount") {
                                if let Some(ui) =
                                    token_amount.get("uiAmount").and_then(|v| v.as_f64())
                                {
                                    if ui > 0.0 {
                                        total_ui += ui;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    total_ui
}

/// Sum WSOL inner transfers (transfer or transferChecked) that credit the wallet's WSOL ATAs, returning SOL units
pub(super) fn sum_inner_wsol_transfers_ui_to_wallet(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> f64 {
    let meta = match tx_data.meta.as_ref() {
        Some(m) => m,
        None => {
            return 0.0;
        }
    };
    let inner = match meta.inner_instructions.as_ref() {
        Some(v) => v,
        None => {
            return 0.0;
        }
    };
    // Resolve wallet's WSOL ATA addresses within this tx context
    let wallet_wsol_atas = get_wallet_wsol_ata_addresses(tx_data, wallet_key);
    if wallet_wsol_atas.is_empty() {
        return 0.0;
    }

    let mut total_ui: f64 = 0.0;
    for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
            for ix in ixs {
                if let Some(parsed) = ix.get("parsed") {
                    let ix_type = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if ix_type == "transfer" || ix_type == "transferChecked" {
                        if let Some(info) = parsed.get("info") {
                            let mint = info
                                .get("mint")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();
                            if mint == crate::transactions::utils::WSOL_MINT {
                                let dest = info
                                    .get("destination")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| info.get("to").and_then(|v| v.as_str()))
                                    .unwrap_or_default();
                                if !dest.is_empty() && wallet_wsol_atas.iter().any(|a| a == dest) {
                                    // Prefer tokenAmount.uiAmount (transferChecked). Fallback to amount lamports (transfer)
                                    let mut ui_amount = 0.0f64;
                                    if let Some(token_amount) = info.get("tokenAmount") {
                                        if let Some(ui) =
                                            token_amount.get("uiAmount").and_then(|v| v.as_f64())
                                        {
                                            ui_amount = ui;
                                        }
                                    }
                                    if ui_amount == 0.0 {
                                        if let Some(raw) =
                                            info.get("amount").and_then(|v| v.as_str())
                                        {
                                            if let Ok(lamports) = raw.parse::<u64>() {
                                                ui_amount = (lamports as f64) / 1_000_000_000.0;
                                            }
                                        } else if let Some(raw_num) =
                                            info.get("amount").and_then(|v| v.as_u64())
                                        {
                                            ui_amount = (raw_num as f64) / 1_000_000_000.0;
                                        }
                                    }
                                    if ui_amount > 0.0 {
                                        total_ui += ui_amount;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    total_ui
}
