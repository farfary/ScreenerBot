// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana adapter for the recorded pool cases: loads cases, builds the account map a decoder
//! receives, replays a case to a venue through [`CaseAccounts`], and holds the SPL layout
//! helpers the pool suites share.

use super::pool_cases::{self, RecordedCase, SolanaAccount};
use async_trait::async_trait;
use screenerbot::chains::solana::pools::fetcher::AccountData;
use screenerbot::chains::solana::pools::types::ProgramKind;
use screenerbot::chains::solana::rpc::types::TokenAccountInfo;
use screenerbot::chains::solana::solana_sdk::account::Account;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct::{
    self, AccountReader, DirectSwapResult, PoolMarket,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
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

/// Byte offsets of an SPL token account: `mint` at 0, `owner` at 32, `amount` at 64, `state` at 108.
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;
const TOKEN_ACCOUNT_AMOUNT_OFFSET: usize = 64;
const TOKEN_ACCOUNT_STATE_OFFSET: usize = 108;
const TOKEN_ACCOUNT_STATE_FROZEN: u8 = 2;
/// Offset of `decimals` in an SPL mint, which Token-2022 shares.
const MINT_DECIMALS_OFFSET: usize = 44;

/// A recorded case served to a venue exactly as a node would serve it: an address the case holds
/// answers with the recorded account, any other address answers as a missing account.
pub struct CaseAccounts {
    pool: Pubkey,
    target_mint: Pubkey,
    accounts: HashMap<Pubkey, Account>,
}

impl CaseAccounts {
    pub fn new(case: &SolanaCase) -> Self {
        let accounts = case
            .accounts
            .iter()
            .map(|(address, account)| {
                let account = Account {
                    lamports: account.lamports,
                    data: account.data.clone(),
                    owner: pubkey(&account.owner),
                    executable: false,
                    rent_epoch: 0,
                };
                (pubkey(address), account)
            })
            .collect();
        Self {
            pool: pubkey(&case.pool),
            target_mint: pubkey(&case.target_mint),
            accounts,
        }
    }

    fn token_account_info(&self, address: &Pubkey, account: &Account) -> TokenAccountInfo {
        let mint = Pubkey::new_from_array(account.data[..32].try_into().expect("32 bytes"));
        let amount = account.data[TOKEN_ACCOUNT_AMOUNT_OFFSET..TOKEN_ACCOUNT_AMOUNT_OFFSET + 8]
            .try_into()
            .map(u64::from_le_bytes)
            .expect("8 bytes");
        TokenAccountInfo {
            account: address.to_string(),
            mint: mint.to_string(),
            balance: amount,
            decimals: self
                .accounts
                .get(&mint)
                .and_then(|mint| mint.data.get(MINT_DECIMALS_OFFSET))
                .copied()
                .unwrap_or(0),
            is_token_2022: account.owner == screenerbot::chains::solana::spl_token_2022::id(),
            is_nft: false,
            is_frozen: account.data.get(TOKEN_ACCOUNT_STATE_OFFSET)
                == Some(&TOKEN_ACCOUNT_STATE_FROZEN),
        }
    }
}

#[async_trait]
impl AccountReader for CaseAccounts {
    async fn read_accounts(&self, addresses: &[Pubkey]) -> DirectSwapResult<Vec<Option<Account>>> {
        Ok(addresses
            .iter()
            .map(|address| self.accounts.get(address).cloned())
            .collect())
    }

    /// The token accounts the case records for `owner`. A case records the accounts a venue
    /// reads by address, which never includes the answer to a by-owner query, so for the case's
    /// own pool the query is answered with the one account holding the case's target mint.
    async fn token_accounts_of(&self, owner: &Pubkey) -> DirectSwapResult<Vec<TokenAccountInfo>> {
        let mut held: Vec<TokenAccountInfo> = self
            .accounts
            .iter()
            .filter(|(_, account)| {
                is_token_program(&account.owner.to_string())
                    && account.data.len() >= TOKEN_ACCOUNT_BASE_LEN
                    && account.data[TOKEN_ACCOUNT_OWNER_OFFSET..TOKEN_ACCOUNT_OWNER_OFFSET + 32]
                        == owner.to_bytes()
            })
            .map(|(address, account)| self.token_account_info(address, account))
            .collect();
        held.sort_by(|a, b| a.account.cmp(&b.account));
        if held.is_empty() && *owner == self.pool {
            held.push(TokenAccountInfo {
                account: String::new(),
                mint: self.target_mint.to_string(),
                balance: 0,
                decimals: 0,
                is_token_2022: false,
                is_nft: false,
                is_frozen: false,
            });
        }
        Ok(held)
    }
}

/// Wraps a reader and records what a venue asked it for: each `read_accounts` request in call
/// order, and each owner whose token accounts were listed.
pub struct RecordingAccounts<R> {
    inner: R,
    reads: Mutex<Vec<Vec<Pubkey>>>,
    owners: Mutex<Vec<Pubkey>>,
}

impl<R: AccountReader> RecordingAccounts<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            reads: Mutex::new(Vec::new()),
            owners: Mutex::new(Vec::new()),
        }
    }

    /// Every `read_accounts` request, in call order.
    pub fn reads(&self) -> Vec<Vec<Pubkey>> {
        self.reads.lock().expect("reads lock").clone()
    }

    /// Every owner whose token accounts were listed, in call order.
    pub fn token_account_owners(&self) -> Vec<Pubkey> {
        self.owners.lock().expect("owners lock").clone()
    }
}

#[async_trait]
impl<R: AccountReader> AccountReader for RecordingAccounts<R> {
    async fn read_accounts(&self, addresses: &[Pubkey]) -> DirectSwapResult<Vec<Option<Account>>> {
        self.reads
            .lock()
            .expect("reads lock")
            .push(addresses.to_vec());
        self.inner.read_accounts(addresses).await
    }

    async fn token_accounts_of(&self, owner: &Pubkey) -> DirectSwapResult<Vec<TokenAccountInfo>> {
        self.owners.lock().expect("owners lock").push(*owner);
        self.inner.token_accounts_of(owner).await
    }
}

/// The market a venue loads from a recorded case, through the same dispatch and `load` production
/// runs.
pub fn load_market(case: &SolanaCase) -> DirectSwapResult<Box<dyn PoolMarket>> {
    futures::executor::block_on(direct::load_market(
        &pubkey(&case.pool),
        &CaseAccounts::new(case),
    ))
}
