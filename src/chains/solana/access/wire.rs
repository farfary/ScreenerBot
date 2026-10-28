// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The JSON bodies of the ScreenerBot service chain reads (`/v1/chain/*`), exactly as the service sends them.
//!
//! Amounts that can exceed 2^53 arrive as decimal strings; slots, epochs and
//! heights as numbers. An optional field the service omits deserialises as
//! absent, never as a default value.

use serde::{Deserialize, Serialize};

/// Deserialises a decimal-string amount.
mod amount {
    use serde::{Deserialize, Deserializer};

    pub fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse::<u64>().map_err(serde::de::Error::custom)
    }

    pub fn de_opt<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|text| text.parse::<u64>().map_err(serde::de::Error::custom))
            .transpose()
    }
}

/// `GET /v1/chain/network`.
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkView {
    pub chain: String,
    pub slot: u64,
    pub epoch: u64,
    pub block_height: BlockHeights,
    pub rent: RentView,
    pub observed_at: i64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BlockHeights {
    pub confirmed: u64,
    pub finalized: u64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RentView {
    pub lamports_per_byte: u64,
    pub account_overhead_bytes: u64,
}

/// `GET /v1/chain/wallet`.
#[derive(Debug, Clone, Deserialize)]
pub struct WalletView {
    pub address: String,
    pub version: u64,
    #[serde(deserialize_with = "amount::de")]
    pub lamports: u64,
    pub slot: u64,
    pub changed_at: i64,
    pub observed_at: i64,
    #[serde(default)]
    pub balances_age_secs: Option<u64>,
}

/// `GET /v1/chain/wallet/balances`.
#[derive(Debug, Clone, Deserialize)]
pub struct BalancesView {
    pub address: String,
    pub version: u64,
    pub slot: u64,
    #[serde(deserialize_with = "amount::de")]
    pub lamports: u64,
    pub token_accounts: Vec<TokenAccountView>,
    pub observed_at: i64,
}

/// The token program of an account or mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramView {
    SplToken,
    // `snake_case` alone spells this `spl_token2022`; the service writes `spl_token_2022`.
    #[serde(rename = "spl_token_2022")]
    SplToken2022,
}

/// The state of a token account the service reports; uninitialized accounts
/// are dropped by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenAccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenAccountView {
    pub account: String,
    pub mint: String,
    pub program: ProgramView,
    #[serde(deserialize_with = "amount::de")]
    pub amount: u64,
    pub decimals: u8,
    pub state: TokenAccountState,
    #[serde(deserialize_with = "amount::de")]
    pub lamports: u64,
}

/// `GET /v1/chain/wallet/signatures`.
#[derive(Debug, Clone, Deserialize)]
pub struct SignaturesView {
    pub address: String,
    pub signatures: Vec<SignatureView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignatureView {
    pub signature: String,
    pub slot: u64,
    #[serde(default)]
    pub block_time: Option<i64>,
    #[serde(default)]
    pub err: Option<serde_json::Value>,
    #[serde(default)]
    pub confirmation_status: Option<String>,
}

/// The commitment a transaction read asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentView {
    Confirmed,
    Finalized,
}

/// `POST /v1/chain/transactions` request.
#[derive(Debug, Clone, Serialize)]
pub struct TransactionsRequest<'a> {
    pub address: &'a str,
    pub signatures: Vec<String>,
    pub commitment: CommitmentView,
}

/// `POST /v1/chain/transactions`.
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionsView {
    pub transactions: Vec<TransactionEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatusView {
    Found,
    Missing,
    NotWalletTransaction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionEntry {
    pub signature: String,
    pub status: TransactionStatusView,
    /// The unmodified `getTransaction` result; present only when found.
    #[serde(default)]
    pub transaction: Option<serde_json::Value>,
}

/// `POST /v1/chain/mints` request.
#[derive(Debug, Clone, Serialize)]
pub struct MintsRequest<'a> {
    pub mints: &'a [String],
    pub metadata: bool,
}

/// `POST /v1/chain/mints`.
#[derive(Debug, Clone, Deserialize)]
pub struct MintsView {
    pub epoch: u64,
    pub mints: Vec<MintFactView>,
}

/// One mint. A missing mint or a non-mint account answers `exists: false`
/// with every other field null.
#[derive(Debug, Clone, Deserialize)]
pub struct MintFactView {
    pub mint: String,
    pub exists: bool,
    #[serde(default)]
    pub program: Option<ProgramView>,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default, deserialize_with = "amount::de_opt")]
    pub supply: Option<u64>,
    #[serde(default)]
    pub mint_authority: Option<String>,
    #[serde(default)]
    pub freeze_authority: Option<String>,
    #[serde(default)]
    pub transfer_fee: Option<TransferFeeView>,
    #[serde(default)]
    pub metadata: Option<MetadataView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferFeeView {
    pub older: TransferFeeEpochView,
    pub newer: TransferFeeEpochView,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferFeeEpochView {
    pub epoch: u64,
    pub basis_points: u16,
    #[serde(deserialize_with = "amount::de")]
    pub maximum_fee: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataView {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

/// `POST /v1/chain/accounts` request.
#[derive(Debug, Clone, Serialize)]
pub struct AccountsRequest<'a> {
    pub addresses: &'a [String],
}

/// `POST /v1/chain/accounts`.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountsView {
    pub accounts: Vec<AccountView>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountView {
    pub address: String,
    pub exists: bool,
    #[serde(default)]
    pub owner_program: Option<String>,
    #[serde(default)]
    pub token_authority: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_network_body_deserialises() {
        let view: NetworkView = serde_json::from_str(
            r#"{"chain":"solana","slot":312345678,"epoch":723,
                "block_height":{"confirmed":290000100,"finalized":290000068},
                "rent":{"lamports_per_byte":5080,"account_overhead_bytes":128},
                "observed_at":1761640000}"#,
        )
        .unwrap();
        assert_eq!(view.block_height.finalized, 290_000_068);
        assert_eq!(view.rent.lamports_per_byte, 5_080);
    }

    #[test]
    fn the_wallet_body_reads_lamports_from_a_string_and_a_null_balances_age() {
        let view: WalletView = serde_json::from_str(
            r#"{"address":"7xKX","version":17,"lamports":"1234567890","slot":312345670,
                "changed_at":1761639990,"observed_at":1761640000,"balances_age_secs":null}"#,
        )
        .unwrap();
        assert_eq!(view.lamports, 1_234_567_890);
        assert_eq!(view.balances_age_secs, None);
    }

    #[test]
    fn the_balances_body_deserialises_both_programs_and_states() {
        let view: BalancesView = serde_json::from_str(
            r#"{"address":"7xKX","version":17,"slot":312345671,"lamports":"1234567890",
                "token_accounts":[
                  {"account":"9abc","mint":"EPjF","program":"spl_token","amount":"1000000",
                   "decimals":6,"state":"initialized","lamports":"2039280"},
                  {"account":"9abd","mint":"Mint","program":"spl_token_2022","amount":"18446744073709551615",
                   "decimals":0,"state":"frozen","lamports":"2074080"}],
                "observed_at":1761640001}"#,
        )
        .unwrap();
        assert_eq!(view.token_accounts[0].program, ProgramView::SplToken);
        assert_eq!(view.token_accounts[1].amount, u64::MAX);
        assert_eq!(view.token_accounts[1].state, TokenAccountState::Frozen);
    }

    #[test]
    fn the_signatures_body_deserialises_with_null_fields() {
        let view: SignaturesView = serde_json::from_str(
            r#"{"address":"7xKX","signatures":[
                {"signature":"5h","slot":312345600,"block_time":1761639000,"err":null,"confirmation_status":"finalized"},
                {"signature":"6h","slot":312345601,"block_time":null,"err":{"InstructionError":[0,"Custom"]},"confirmation_status":null}]}"#,
        )
        .unwrap();
        assert!(view.signatures[0].err.is_none());
        assert!(view.signatures[1].err.is_some());
        assert_eq!(view.signatures[1].confirmation_status, None);
    }

    #[test]
    fn the_transactions_body_omits_the_body_of_absent_entries() {
        let view: TransactionsView = serde_json::from_str(
            r#"{"transactions":[
                {"signature":"5h","status":"found","transaction":{"slot":1,"blockTime":1,"meta":{},"transaction":{},"version":0}},
                {"signature":"3k","status":"missing"},
                {"signature":"2m","status":"not_wallet_transaction"}]}"#,
        )
        .unwrap();
        assert_eq!(view.transactions[0].status, TransactionStatusView::Found);
        assert!(view.transactions[0].transaction.is_some());
        assert!(view.transactions[1].transaction.is_none());
        assert_eq!(
            view.transactions[2].status,
            TransactionStatusView::NotWalletTransaction
        );
    }

    #[test]
    fn the_mints_body_deserialises_a_fee_mint_and_an_absent_one() {
        let view: MintsView = serde_json::from_str(
            r#"{"epoch":723,"mints":[
                {"mint":"M1","exists":true,"program":"spl_token_2022","decimals":6,"supply":"1000000000000",
                 "mint_authority":null,"freeze_authority":null,
                 "transfer_fee":{"older":{"epoch":700,"basis_points":100,"maximum_fee":"5000000"},
                                 "newer":{"epoch":720,"basis_points":50,"maximum_fee":"5000000"}},
                 "metadata":{"name":"Token","symbol":"TKN","uri":"https://example.invalid/t.json"}},
                {"mint":"M2","exists":false,"program":null,"decimals":null,"supply":null,
                 "mint_authority":null,"freeze_authority":null,"transfer_fee":null,"metadata":null}]}"#,
        )
        .unwrap();
        let fee = view.mints[0].transfer_fee.as_ref().unwrap();
        assert_eq!(fee.newer.basis_points, 50);
        assert_eq!(fee.newer.maximum_fee, 5_000_000);
        assert_eq!(view.mints[0].supply, Some(1_000_000_000_000));
        assert!(!view.mints[1].exists);
        assert_eq!(view.mints[1].supply, None);
    }

    #[test]
    fn the_accounts_body_deserialises() {
        let view: AccountsView = serde_json::from_str(
            r#"{"accounts":[
                {"address":"A1","exists":true,"owner_program":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA","token_authority":"7xKX"},
                {"address":"A2","exists":false,"owner_program":null,"token_authority":null}]}"#,
        )
        .unwrap();
        assert_eq!(view.accounts[0].token_authority.as_deref(), Some("7xKX"));
        assert!(!view.accounts[1].exists);
    }
}
