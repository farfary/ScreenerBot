//! Transaction processor helpers — utility functions for instruction parsing and validation.
//
// Transaction processing pipeline - Helper functions
//
// This module contains standalone utility functions used by the transaction
// processor for various parsing and analysis tasks.
//
// Transfer detection and MEV tip helpers are in helpers_transfers.rs.
// WSOL/wrap operation helpers are in helpers_wsol.rs.
// Both are re-exported here so callers can continue using `super::helpers::*`.

use serde_json::Value;

// Re-export sibling helper modules so callers can use `super::helpers::*` unchanged.
pub(super) use super::helpers_transfers::*;
pub(super) use super::helpers_wsol::*;

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Extract account keys from a transaction message (legacy and v0 support)
pub(super) fn account_keys_from_message(message: &Value) -> Vec<String> {
    // Support multiple jsonParsed shapes for message.accountKeys
    // 1) Legacy/compact: array of strings
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        // Try strings first
        let mut keys: Vec<String> = array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !keys.is_empty() {
            return keys;
        }
        // Fallback: array of objects containing { pubkey, signer, writable, source }
        keys = array
            .iter()
            .filter_map(|v| {
                v.get("pubkey")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        if !keys.is_empty() {
            return keys;
        }
    }

    // 2) v0 format: object with staticAccountKeys and loadedAddresses
    if let Some(obj) = message.get("accountKeys").and_then(|v| v.as_object()) {
        let mut keys = Vec::new();

        // Static account keys
        if let Some(static_keys) = obj.get("staticAccountKeys").and_then(|v| v.as_array()) {
            // staticAccountKeys itself may be strings or objects with pubkey
            for item in static_keys {
                if let Some(s) = item.as_str() {
                    keys.push(s.to_string());
                } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
                    keys.push(pk.to_string());
                }
            }
        }

        // Loaded addresses: writable + readonly
        if let Some(loaded) = obj.get("loadedAddresses").and_then(|v| v.as_object()) {
            if let Some(writable) = loaded.get("writable").and_then(|v| v.as_array()) {
                for item in writable {
                    if let Some(s) = item.as_str() {
                        keys.push(s.to_string());
                    } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
                        keys.push(pk.to_string());
                    }
                }
            }
            if let Some(readonly) = loaded.get("readonly").and_then(|v| v.as_array()) {
                for item in readonly {
                    if let Some(s) = item.as_str() {
                        keys.push(s.to_string());
                    } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
                        keys.push(pk.to_string());
                    }
                }
            }
        }

        if !keys.is_empty() {
            return keys;
        }
    }

    Vec::new()
}

/// Parse UI token amount with graceful fallback to raw representation
pub(super) fn parse_ui_amount(amount: &crate::chains::solana::rpc::UiTokenAmount) -> f64 {
    // Try ui_amount first
    if let Some(ui_amount) = amount.ui_amount {
        return ui_amount;
    }

    // Fallback to amount string parsing with decimals
    if let Ok(raw_amount) = amount.amount.parse::<u64>() {
        return (raw_amount as f64) / (10f64).powi(amount.decimals as i32);
    }

    0.0
}

/// Extract token decimals from RPC transaction metadata (authoritative source)
/// Returns None if the token is not found in pre/post token balances
pub(super) fn extract_token_decimals_from_rpc(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    mint: &str,
) -> Option<u8> {
    let meta = tx_data.meta.as_ref()?;

    // Check post token balances first (most recent state)
    if let Some(post_balances) = &meta.post_token_balances {
        for balance in post_balances {
            if balance.mint == mint {
                return Some(balance.ui_token_amount.decimals);
            }
        }
    }

    // Fallback to pre token balances
    if let Some(pre_balances) = &meta.pre_token_balances {
        for balance in pre_balances {
            if balance.mint == mint {
                return Some(balance.ui_token_amount.decimals);
            }
        }
    }

    None
}

/// Resolve account keys vector (supports legacy array and v0 object forms)
pub(super) fn resolve_account_keys_vec(message: &serde_json::Value) -> Vec<String> {
    account_keys_from_message(message)
}
