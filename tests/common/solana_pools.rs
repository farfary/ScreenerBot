// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana adapter for the recorded pool cases: loads cases, builds the account map a decoder
//! receives, and holds the SPL layout helpers the pool suites share.

use super::pool_cases::{self, RecordedCase, SolanaAccount};
use screenerbot::chains::solana::pools::fetcher::AccountData;
use screenerbot::chains::solana::pools::types::ProgramKind;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

/// The chain directory of the Solana cases under `tests/fixtures/pools/`.
pub const CHAIN: &str = "solana";

/// SPL mint layout: `decimals` follows the 36-byte mint authority option and the 8-byte supply.
/// Token-2022 keeps the same base layout.
pub const MINT_BASE_LEN: usize = 82;
pub const TOKEN_ACCOUNT_BASE_LEN: usize = 165;
/// Token-2022 `AccountType` byte that follows the base layout when extensions exist.
const ACCOUNT_TYPE_MINT: u8 = 1;
const ACCOUNT_TYPE_TOKEN_ACCOUNT: u8 = 2;

pub type SolanaCase = RecordedCase<SolanaAccount>;

pub fn pubkey(address: &str) -> Pubkey {
    Pubkey::from_str(address).unwrap_or_else(|e| panic!("invalid pubkey {address}: {e}"))
}

/// Load one Solana case by venue slug and case name.
pub fn load_case(venue: &str, name: &str) -> SolanaCase {
    pool_cases::load_case(CHAIN, venue, name)
}

/// Every Solana case recorded for `purpose`, as `(case name, case)`, sorted by venue then name.
pub fn cases_for_purpose(purpose: &str) -> Vec<(String, SolanaCase)> {
    pool_cases::load_chain_cases::<SolanaAccount>(CHAIN)
        .into_iter()
        .filter(|(_, _, case)| case.purpose == purpose)
        .map(|(_, name, case)| (name, case))
        .collect()
}

/// The program kind a case's pool classifies as, checked against the case's own venue slug.
pub fn program_kind(case: &SolanaCase) -> ProgramKind {
    let owner = &case
        .accounts
        .get(&case.pool)
        .unwrap_or_else(|| panic!("case for {} lacks its pool account", case.pool))
        .owner;
    let kind = ProgramKind::classify(&pubkey(owner));
    assert_eq!(
        kind.protocol_slug(),
        case.venue,
        "pool {} classifies as a different program kind",
        case.pool
    );
    kind
}

/// The account map a decoder receives, built from the recorded accounts.
pub fn decoder_accounts(case: &SolanaCase) -> HashMap<String, AccountData> {
    case.accounts
        .iter()
        .map(|(address, account)| {
            let data = AccountData {
                pubkey: pubkey(address),
                data: account.data.clone(),
                slot: case.slot,
                fetched_at: Instant::now(),
                lamports: account.lamports,
                owner: pubkey(&account.owner),
            };
            (address.clone(), data)
        })
        .collect()
}

fn is_token_program(owner: &str) -> bool {
    owner == screenerbot::chains::solana::spl_token::id().to_string()
        || owner == screenerbot::chains::solana::spl_token_2022::id().to_string()
}

/// The mint an SPL account refers to: itself for a mint, its `mint` field for a token
/// account. `None` for any other account.
pub fn mint_of(address: &str, account: &SolanaAccount) -> Option<String> {
    if !is_token_program(&account.owner) {
        return None;
    }
    let data = &account.data;
    let account_type = data.get(TOKEN_ACCOUNT_BASE_LEN).copied();
    if data.len() == MINT_BASE_LEN || account_type == Some(ACCOUNT_TYPE_MINT) {
        return Some(address.to_owned());
    }
    if data.len() == TOKEN_ACCOUNT_BASE_LEN || account_type == Some(ACCOUNT_TYPE_TOKEN_ACCOUNT) {
        let mint: [u8; 32] = data[..32].try_into().expect("token account holds a mint");
        return Some(Pubkey::new_from_array(mint).to_string());
    }
    None
}
