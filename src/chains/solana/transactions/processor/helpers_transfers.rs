//! Transaction processor helpers — transfer detection and MEV tip scanning.
//
// Functions for finding system transfers and MEV/Jito tips in parsed transactions.

/// Find the largest parsed system transfer amount from the wallet in inner/outer instructions
pub(super) fn find_largest_system_transfer_from_wallet(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    let mut best: Option<u64> = None;

    // Helper to process a single instruction value
    let mut consider_ix = |ix: &serde_json::Value| {
        // Prefer parsed format
        if let Some(parsed) = ix.get("parsed") {
            let ix_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if let Some(info) = parsed.get("info") {
                let source = info
                    .get("source")
                    .and_then(|v| v.as_str())
                    .or_else(|| info.get("from").and_then(|v| v.as_str()))
                    .unwrap_or_default();
                if source == wallet_key {
                    let lamports = info
                        .get("lamports")
                        .and_then(|v| v.as_u64())
                        .or_else(|| info.get("amount").and_then(|v| v.as_u64()));
                    if let Some(lamports) = lamports {
                        if ix_type == "transfer" || ix_type == "createAccount" {
                            if best.is_none_or(|b| lamports > b) {
                                best = Some(lamports);
                            }
                        }
                    }
                }
            }
        }
    };

    // Outer instructions
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

    // Inner instructions
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

    best
}

/// Sum explicit MEV/Jito tip lamports sent from wallet by scanning parsed instructions
pub(super) fn find_mev_tips_from_wallet(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    use crate::chains::solana::transactions::program_ids::is_mev_tip_address;
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
                    if source == wallet_key && is_mev_tip_address(dest) {
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

/// Sum all system transfers from wallet to a specific account address
pub(super) fn sum_system_transfers_to_account_from_wallet(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
    dest_account: &str,
) -> Option<u64> {
    let mut total: u64 = 0;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
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
                    if source == wallet_key && dest == dest_account {
                        if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
                            total = total.saturating_add(lamports);
                        } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
                            total = total.saturating_add(amount);
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
    if total > 0 {
        Some(total)
    } else {
        None
    }
}

/// Find the largest system transfer amount sent from the wallet to any destination excluding known tip addresses
pub(super) fn find_largest_system_transfer_from_wallet_excluding_tips(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    wallet_key: &str,
) -> Option<u64> {
    use crate::chains::solana::transactions::program_ids::is_mev_tip_address;
    let mut best: u64 = 0;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
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
                    if source == wallet_key && !is_mev_tip_address(dest) {
                        if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
                            if lamports > best {
                                best = lamports;
                            }
                        } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
                            if amount > best {
                                best = amount;
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
    if best > 0 {
        Some(best)
    } else {
        None
    }
}

/// Lightweight instruction scan for MEV/Jito tips (outer + inner), returning SOL units
pub(super) fn detect_mev_tips_from_instructions_light(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> f64 {
    use crate::chains::solana::transactions::program_ids::is_mev_tip_address;
    let mut total_lamports: u64 = 0;
    let mut consider_ix = |ix: &serde_json::Value| {
        if let Some(parsed) = ix.get("parsed") {
            if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
                if let Some(info) = parsed.get("info") {
                    let dest = info
                        .get("destination")
                        .and_then(|v| v.as_str())
                        .or_else(|| info.get("to").and_then(|v| v.as_str()))
                        .unwrap_or_default();
                    if is_mev_tip_address(dest) {
                        if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
                            total_lamports = total_lamports.saturating_add(lamports);
                        } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
                            total_lamports = total_lamports.saturating_add(amount);
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
    (total_lamports as f64) / 1_000_000_000.0
}
