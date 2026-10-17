// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RPC client methods implementation
//!
//! Implementation of `RpcClientMethods` trait for `RpcClient`.
//! Type definitions and trait signatures are in `methods.rs`.

use super::methods::{
    HeliusTransactionsPage, ProviderHealthInfo, RpcClientMethods, RpcFilterType,
    RpcTokenAccountBalance, SignatureInfo, TokenSupply,
};
use super::RpcClient;
use crate::chains::solana::constants::{SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
use crate::chains::solana::rpc::types::{
    SimulationOutcome, TokenAccountInfo, TokenAccountsSnapshot, TransactionDetails,
};
use crate::chains::solana::solana_sdk::{
    account::Account, commitment_config::CommitmentLevel, hash::Hash, pubkey::Pubkey,
    signature::Signature, transaction::VersionedTransaction,
};
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus,
};
use crate::rpc::stats::RpcStatsResponse;
use crate::rpc::RpcError;
use base64::Engine;
use std::str::FromStr;

const MAX_MULTIPLE_ACCOUNTS: usize = 50;

impl RpcClientMethods for RpcClient {
    async fn get_account(&self, pubkey: &Pubkey) -> crate::Result<Option<Account>> {
        self.get_account_with_commitment(pubkey, CommitmentLevel::Confirmed)
            .await
    }

    async fn get_account_with_commitment(
        &self,
        pubkey: &Pubkey,
        commitment: CommitmentLevel,
    ) -> crate::Result<Option<Account>> {
        let params = serde_json::json!([
            pubkey.to_string(),
            {
                "encoding": "base64",
                "commitment": commitment_to_string(commitment)
            }
        ]);

        let result = self.manager.execute_raw("getAccountInfo", params).await?;

        // Parse the response
        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Ok(None);
        }

        let value = value.unwrap();
        parse_account_from_json(value)
    }

    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> crate::Result<Vec<Option<Account>>> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        // Keep requests within the provider account limit.
        let mut all_accounts = Vec::with_capacity(pubkeys.len());

        for chunk in pubkeys.chunks(MAX_MULTIPLE_ACCOUNTS) {
            let keys: Vec<String> = chunk.iter().map(|p| p.to_string()).collect();
            let params = serde_json::json!([
                keys,
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]);

            let result = self
                .manager
                .execute_raw("getMultipleAccounts", params)
                .await?;

            let values = result
                .get("value")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    crate::Error::Data(crate::errors::DataError::ParseError {
                        data_type: "response".to_string(),
                        error: "missing value array".to_string(),
                    })
                })?;

            for value in values {
                if value.is_null() {
                    all_accounts.push(None);
                } else {
                    all_accounts.push(parse_account_from_json(value)?);
                }
            }
        }

        Ok(all_accounts)
    }

    async fn get_sol_balance(&self, wallet: &str) -> crate::Result<f64> {
        let params = serde_json::json!([wallet, { "commitment": WALLET_READ_COMMITMENT }]);

        let result = self.manager.execute_raw("getBalance", params).await?;

        let lamports = result
            .get("value")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "balance".to_string(),
                    error: "invalid response format".to_string(),
                })
            })?;

        Ok(lamports as f64 / 1_000_000_000.0)
    }

    async fn get_token_account_balance(&self, token_account: &str) -> crate::Result<f64> {
        let params = serde_json::json!([token_account, { "commitment": WALLET_READ_COMMITMENT }]);

        let result = self
            .manager
            .execute_raw("getTokenAccountBalance", params)
            .await?;

        let ui_amount = result
            .get("value")
            .and_then(|v| v.get("uiAmount"))
            .and_then(|v| v.as_f64())
            .unwrap_or_default();

        Ok(ui_amount)
    }

    async fn get_token_balance(&self, wallet_address: &str, mint: &str) -> crate::Result<u64> {
        // Use getTokenAccountsByOwner to find token accounts for this wallet+mint
        let params = serde_json::json!([
            wallet_address,
            { "mint": mint },
            { "encoding": "jsonParsed", "commitment": WALLET_READ_COMMITMENT }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenAccountsByOwner", params)
            .await?;

        // Swaps debit the wallet's associated token account, so that account's balance is
        // the amount a sell can spend; any other account of the mint is not counted.
        let value = result.get("value").and_then(|v| v.as_array());
        let Some(accounts) = value else {
            return Ok(0);
        };
        let (Ok(owner), Ok(mint_key)) = (Pubkey::from_str(wallet_address), Pubkey::from_str(mint))
        else {
            return Ok(0);
        };

        for account in accounts {
            let Some(pubkey) = account.get("pubkey").and_then(|p| p.as_str()) else {
                continue;
            };
            let program = account
                .get("account")
                .and_then(|a| a.get("owner"))
                .and_then(|o| o.as_str());
            if !is_associated_token_account(pubkey, &owner, &mint_key, program) {
                continue;
            }
            let Some(amount_str) = account
                .get("account")
                .and_then(|a| a.get("data"))
                .and_then(|d| d.get("parsed"))
                .and_then(|p| p.get("info"))
                .and_then(|i| i.get("tokenAmount"))
                .and_then(|t| t.get("amount"))
                .and_then(|a| a.as_str())
            else {
                continue;
            };
            return amount_str.parse::<u64>().map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "token amount".to_string(),
                    error: format!("Failed to parse '{amount_str}': {e}"),
                })
            });
        }

        // No token account found - return 0
        Ok(0)
    }

    async fn get_latest_blockhash(&self) -> crate::Result<Hash> {
        let (hash, _) = self
            .get_latest_blockhash_with_commitment(CommitmentLevel::Finalized)
            .await?;
        Ok(hash)
    }

    async fn get_latest_blockhash_with_commitment(
        &self,
        commitment: CommitmentLevel,
    ) -> crate::Result<(Hash, u64)> {
        let params = serde_json::json!([{
            "commitment": commitment_to_string(commitment)
        }]);

        let result = self
            .manager
            .execute_raw("getLatestBlockhash", params)
            .await?;

        let value = result.get("value").ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "response".to_string(),
                error: "missing value field".to_string(),
            })
        })?;
        let blockhash = value
            .get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "blockhash".to_string(),
                    error: "missing blockhash field".to_string(),
                })
            })?;
        let last_valid_block_height = value
            .get("lastValidBlockHeight")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "blockhash".to_string(),
                    error: "missing lastValidBlockHeight".to_string(),
                })
            })?;

        let hash = Hash::from_str(blockhash).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "blockhash".to_string(),
                error: format!("Invalid hash \'{blockhash}\': {e}"),
            })
        })?;

        Ok((hash, last_valid_block_height))
    }

    async fn get_block_height(&self) -> crate::Result<u64> {
        self.get_block_height_with_commitment(CommitmentLevel::Finalized)
            .await
    }

    async fn get_block_height_with_commitment(
        &self,
        commitment: CommitmentLevel,
    ) -> crate::Result<u64> {
        let result = self
            .manager
            .execute_raw("getBlockHeight", block_height_params(commitment))
            .await?;

        result.as_u64().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "block height".to_string(),
                error: "invalid response format".to_string(),
            })
        })
    }

    async fn get_slot_and_block_height(
        &self,
        commitment: CommitmentLevel,
    ) -> crate::Result<(u64, u64)> {
        let result = self
            .manager
            .execute_raw("getEpochInfo", block_height_params(commitment))
            .await?;
        let field = |name: &str| {
            result.get(name).and_then(|v| v.as_u64()).ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "epoch info".to_owned(),
                    error: format!("missing {name}"),
                })
            })
        };
        Ok((field("absoluteSlot")?, field("blockHeight")?))
    }

    async fn get_epoch(&self) -> crate::Result<u64> {
        let result = self
            .manager
            .execute_raw(
                "getEpochInfo",
                serde_json::json!([{"commitment": "confirmed"}]),
            )
            .await?;
        result
            .get("epoch")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "epoch info".to_owned(),
                    error: "missing epoch".to_owned(),
                })
            })
    }

    async fn send_transaction(
        &self,
        transaction: &VersionedTransaction,
    ) -> crate::Result<Signature> {
        // Serialize transaction
        let tx_bytes = bincode::serialize(transaction).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Failed to serialize: {e}"),
            })
        })?;
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

        let params = serde_json::json!([
            tx_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "confirmed",
                "maxRetries": 3
            }
        ]);

        let result = self.manager.execute_raw("sendTransaction", params).await?;

        let sig_str = result.as_str().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "signature".to_string(),
                error: "invalid response format".to_string(),
            })
        })?;

        Signature::from_str(sig_str).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "signature".to_string(),
                error: format!("Invalid signature \'{sig_str}\': {e}"),
            })
        })
    }

    async fn simulate_transaction(
        &self,
        transaction: &VersionedTransaction,
    ) -> crate::Result<SimulationOutcome> {
        let tx_bytes = bincode::serialize(transaction).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Failed to serialize: {e}"),
            })
        })?;
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

        // `sigVerify: false` with `replaceRecentBlockhash: true` lets a preflight
        // run even when the blockhash has aged out between build and simulate --
        // the point here is whether the INSTRUCTIONS are correct, not whether this
        // exact blockhash is still current.
        // `innerInstructions` is what makes the run auditable: the CPIs a
        // third-party route performs -- including any account it funds out of the
        // wallet -- are invisible in the transaction's own message when the route
        // builds them, and invisible in the quote entirely.
        let params = serde_json::json!([
            tx_base64,
            {
                "encoding": "base64",
                "commitment": "confirmed",
                "sigVerify": false,
                "replaceRecentBlockhash": true,
                "innerInstructions": true
            }
        ]);

        let result = self
            .manager
            .execute_raw("simulateTransaction", params)
            .await?;

        Ok(simulation_outcome(result.get("value").unwrap_or(&result)))
    }

    async fn get_transaction(
        &self,
        signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        let params = serde_json::json!([
            signature.to_string(),
            get_transaction_config(Some(CommitmentLevel::Confirmed))
        ]);

        let result = self.manager.execute_raw("getTransaction", params).await;

        match result {
            Ok(value) => {
                if value.is_null() {
                    return Ok(None);
                }
                let tx: EncodedConfirmedTransactionWithStatusMeta = serde_json::from_value(value)
                    .map_err(|e| {
                    crate::Error::Data(crate::errors::DataError::ParseError {
                        data_type: "transaction".to_string(),
                        error: e.to_string().to_string(),
                    })
                })?;
                Ok(Some(tx))
            }
            Err(RpcError::AccountNotFound { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> crate::Result<(u64, Vec<Option<TransactionStatus>>)> {
        let sig_strings: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();
        let params = serde_json::json!([sig_strings, { "searchTransactionHistory": true }]);

        let result = self
            .manager
            .execute_raw("getSignatureStatuses", params)
            .await?;

        let invalid = || {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "response".to_string(),
                error: "invalid format".to_string(),
            })
        };
        let slot = result
            .get("context")
            .and_then(|context| context.get("slot"))
            .and_then(|slot| slot.as_u64())
            .ok_or_else(invalid)?;
        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or_else(invalid)?;

        let mut statuses = Vec::with_capacity(values.len());
        for value in values {
            if value.is_null() {
                statuses.push(None);
            } else {
                let status: TransactionStatus =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        crate::Error::Data(crate::errors::DataError::ParseError {
                            data_type: "TransactionStatus".to_owned(),
                            error: e.to_string(),
                        })
                    })?;
                statuses.push(Some(status));
            }
        }

        Ok((slot, statuses))
    }

    async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> crate::Result<Vec<(Pubkey, Account)>> {
        let params = serde_json::json!([
            owner.to_string(),
            { "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
            { "encoding": "base64", "commitment": WALLET_READ_COMMITMENT }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenAccountsByOwner", params)
            .await?;

        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "response".to_string(),
                    error: "invalid format".to_string(),
                })
            })?;

        let mut accounts = Vec::with_capacity(values.len());
        for item in values {
            let pubkey_str = item.get("pubkey").and_then(|v| v.as_str()).ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "pubkey".to_string(),
                    error: "missing pubkey field".to_string(),
                })
            })?;
            let pubkey = Pubkey::from_str(pubkey_str).map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "pubkey".to_string(),
                    error: format!("Invalid pubkey \'{pubkey_str}\': {e}"),
                })
            })?;

            if let Some(account) =
                parse_account_from_json(item.get("account").unwrap_or(&serde_json::Value::Null))?
            {
                accounts.push((pubkey, account));
            }
        }

        Ok(accounts)
    }

    async fn get_slot(&self) -> crate::Result<u64> {
        let params = serde_json::json!([]);

        let result = self.manager.execute_raw("getSlot", params).await?;

        result.as_u64().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "slot".to_string(),
                error: "invalid response format".to_string(),
            })
        })
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> crate::Result<u64> {
        let params = serde_json::json!([data_len]);

        let result = self
            .manager
            .execute_raw("getMinimumBalanceForRentExemption", params)
            .await?;

        result.as_u64().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "rent".to_string(),
                error: "invalid response format".to_string(),
            })
        })
    }

    async fn get_health(&self) -> crate::Result<()> {
        let params = serde_json::json!([]);

        self.manager.execute_raw("getHealth", params).await?;

        Ok(())
    }

    async fn url(&self) -> String {
        self.manager.primary_url().await.unwrap_or_default()
    }

    // =========================================================================
    // Advanced Transaction Methods Implementation
    // =========================================================================

    async fn send_raw_transaction(&self, transaction_base64: &str) -> crate::Result<Signature> {
        let params = serde_json::json!([
            transaction_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "confirmed",
                "maxRetries": 3
            }
        ]);

        let result = self.manager.execute_raw("sendTransaction", params).await?;

        let sig_str = result.as_str().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "signature".to_string(),
                error: "invalid response format".to_string(),
            })
        })?;
        Signature::from_str(sig_str).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "signature".to_string(),
                error: format!("Invalid signature \'{sig_str}\': {e}"),
            })
        })
    }

    // =========================================================================
    // Token Account Utility Methods Implementation
    // =========================================================================

    async fn get_all_token_accounts(&self, owner: &Pubkey) -> crate::Result<Vec<TokenAccountInfo>> {
        Ok(self.get_token_accounts_snapshot(owner).await?.accounts)
    }

    async fn get_token_accounts_snapshot(
        &self,
        owner: &Pubkey,
    ) -> crate::Result<TokenAccountsSnapshot> {
        let spl = self
            .manager
            .execute_raw(
                "getTokenAccountsByOwner",
                token_accounts_params(owner, SPL_TOKEN_PROGRAM_ID),
            )
            .await;
        let token_2022 = self
            .manager
            .execute_raw(
                "getTokenAccountsByOwner",
                token_accounts_params(owner, TOKEN_2022_PROGRAM_ID),
            )
            .await;
        token_accounts_from(spl, token_2022)
    }

    async fn is_token_2022_mint(&self, mint: &Pubkey) -> crate::Result<bool> {
        let params = serde_json::json!([
            mint.to_string(),
            { "encoding": "jsonParsed" }
        ]);

        let result = self.manager.execute_raw("getAccountInfo", params).await?;

        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "mint account".to_string(),
                error: format!("Account not found: {mint}"),
            }));
        }

        let value = value.unwrap();
        if let Some(owner) = value.get("owner").and_then(|v| v.as_str()) {
            Ok(owner == TOKEN_2022_PROGRAM_ID)
        } else {
            Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "account".to_string(),
                error: "Missing owner field".to_string(),
            }))
        }
    }

    fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        let token_program_id =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("Invalid SPL Token program ID");
        Self::get_associated_token_address_with_program(wallet, mint, &token_program_id)
    }

    fn get_associated_token_address_with_program(
        wallet: &Pubkey,
        mint: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Pubkey {
        let associated_token_program_id =
            Pubkey::from_str(crate::chains::solana::constants::ASSOCIATED_TOKEN_PROGRAM_ID)
                .expect("Invalid Associated Token program ID");

        // PDA derivation: [wallet, token_program, mint]
        let seeds = &[wallet.as_ref(), token_program_id.as_ref(), mint.as_ref()];

        let (address, _bump) = Pubkey::find_program_address(seeds, &associated_token_program_id);
        address
    }

    // =========================================================================
    // String-based Convenience Methods Implementation
    // =========================================================================

    async fn get_all_token_accounts_str(
        &self,
        owner: &str,
    ) -> crate::Result<Vec<TokenAccountInfo>> {
        let owner_pubkey = Pubkey::from_str(owner).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid owner \'{owner}\': {e}"),
            })
        })?;
        self.get_all_token_accounts(&owner_pubkey).await
    }

    async fn is_token_2022_mint_str(&self, mint: &str) -> crate::Result<bool> {
        let mint_pubkey = Pubkey::from_str(mint).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid mint '{mint}': {e}"),
            })
        })?;
        self.is_token_2022_mint(&mint_pubkey).await
    }

    async fn is_token_account_token_2022(&self, token_account: &str) -> crate::Result<bool> {
        let account_pubkey = Pubkey::from_str(token_account).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid token account \'{token_account}\': {e}"),
            })
        })?;

        let params = serde_json::json!([
            account_pubkey.to_string(),
            { "encoding": "jsonParsed" }
        ]);

        let result = self.manager.execute_raw("getAccountInfo", params).await?;

        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "token account".to_string(),
                error: format!("Account not found: {token_account}"),
            }));
        }

        let value = value.unwrap();
        if let Some(owner) = value.get("owner").and_then(|v| v.as_str()) {
            Ok(owner == TOKEN_2022_PROGRAM_ID)
        } else {
            Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "account".to_string(),
                error: "Missing owner field".to_string(),
            }))
        }
    }

    async fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> crate::Result<String> {
        let wallet_pubkey = Pubkey::from_str(wallet_address).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid wallet \'{wallet_address}\': {e}"),
            })
        })?;
        let mint_pubkey = Pubkey::from_str(mint).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid mint \'{mint}\': {e}"),
            })
        })?;

        // First try standard SPL Token ATA
        let spl_ata = Self::get_associated_token_address(&wallet_pubkey, &mint_pubkey);

        // Check if ATA exists
        if let Ok(Some(_)) = self.get_account(&spl_ata).await {
            return Ok(spl_ata.to_string());
        }

        // Try Token-2022 ATA
        let token_2022_program_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "pubkey".to_string(),
                error: format!("Invalid Token-2022 program ID: {e}"),
            })
        })?;
        let token_2022_ata = Self::get_associated_token_address_with_program(
            &wallet_pubkey,
            &mint_pubkey,
            &token_2022_program_id,
        );

        if let Ok(Some(_)) = self.get_account(&token_2022_ata).await {
            return Ok(token_2022_ata.to_string());
        }

        // Return the SPL ATA address even if it doesn't exist (for creation)
        Ok(spl_ata.to_string())
    }

    // =========================================================================
    // Transaction History Methods Implementation
    // =========================================================================

    async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: Option<usize>,
        before: Option<&Signature>,
        until: Option<&Signature>,
    ) -> crate::Result<Vec<SignatureInfo>> {
        let mut config = serde_json::Map::new();

        if let Some(limit_val) = limit {
            config.insert(
                "limit".to_owned(),
                serde_json::Value::Number(limit_val.into()),
            );
        }

        if let Some(before_sig) = before {
            config.insert(
                "before".to_owned(),
                serde_json::Value::String(before_sig.to_string()),
            );
        }

        if let Some(until_sig) = until {
            config.insert(
                "until".to_owned(),
                serde_json::Value::String(until_sig.to_string()),
            );
        }

        config.insert(
            "commitment".to_owned(),
            serde_json::Value::String("confirmed".to_owned()),
        );

        let params = serde_json::json!([address.to_string(), serde_json::Value::Object(config)]);

        let result = self
            .manager
            .execute_raw("getSignaturesForAddress", params)
            .await?;

        let signatures_array = result.as_array().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "response".to_string(),
                error: "expected array".to_string(),
            })
        })?;

        let mut signatures = Vec::with_capacity(signatures_array.len());

        for item in signatures_array {
            let sig_str = item
                .get("signature")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    crate::Error::Data(crate::errors::DataError::ParseError {
                        data_type: "signature".to_string(),
                        error: "missing signature field".to_string(),
                    })
                })?;

            let signature = Signature::from_str(sig_str).map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "signature".to_string(),
                    error: format!("Invalid signature \'{sig_str}\': {e}"),
                })
            })?;

            let slot = item.get("slot").and_then(|v| v.as_u64()).ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "slot".to_string(),
                    error: "missing slot field".to_string(),
                })
            })?;

            let err = item.get("err").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(serde_json::to_string(v).unwrap_or_default())
                }
            });

            let memo = item.get("memo").and_then(|v| v.as_str()).map(String::from);

            let block_time = item.get("blockTime").and_then(|v| v.as_i64());

            let confirmation_status = item
                .get("confirmationStatus")
                .and_then(|v| v.as_str())
                .map(String::from);

            signatures.push(SignatureInfo {
                signature,
                slot,
                err,
                memo,
                block_time,
                confirmation_status,
            });
        }

        Ok(signatures)
    }

    async fn get_transactions(
        &self,
        signatures: &[Signature],
    ) -> crate::Result<Vec<Option<EncodedConfirmedTransactionWithStatusMeta>>> {
        if signatures.is_empty() {
            return Ok(Vec::new());
        }

        // Process in chunks to avoid overwhelming RPC
        let mut all_transactions = Vec::with_capacity(signatures.len());

        for chunk in signatures.chunks(20) {
            // Fetch in parallel within chunk
            let mut futures = Vec::with_capacity(chunk.len());

            for sig in chunk {
                futures.push(self.get_transaction(sig));
            }

            // Execute all futures in the chunk concurrently
            let results = futures::future::join_all(futures).await;

            for result in results {
                match result {
                    Ok(tx) => all_transactions.push(tx),
                    Err(_) => all_transactions.push(None),
                }
            }
        }

        Ok(all_transactions)
    }

    async fn get_helius_successful_transactions_after(
        &self,
        address: &Pubkey,
        limit: usize,
        after: Option<&Signature>,
    ) -> crate::Result<HeliusTransactionsPage> {
        let params = helius_successful_transactions_params(address, limit, after)?;
        let result = self
            .manager
            .execute_raw_for_provider_kind(
                crate::rpc::types::ProviderKind::Helius,
                "getTransactionsForAddress",
                params,
            )
            .await?;

        parse_helius_transactions_page(result)
    }

    // =========================================================================
    // Program Account Methods Implementation
    // =========================================================================

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
    ) -> crate::Result<Vec<(Pubkey, Account)>> {
        self.get_program_accounts_with_config(
            program_id,
            filters,
            Some("base64"),
            None,
            Some(CommitmentLevel::Confirmed),
        )
        .await
    }

    async fn get_program_accounts_with_config(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
        encoding: Option<&str>,
        data_slice: Option<(usize, usize)>,
        commitment: Option<CommitmentLevel>,
    ) -> crate::Result<Vec<(Pubkey, Account)>> {
        let mut config = serde_json::Map::new();

        config.insert(
            "encoding".to_owned(),
            serde_json::Value::String(encoding.unwrap_or("base64").to_string()),
        );

        if let Some(commitment_level) = commitment {
            config.insert(
                "commitment".to_owned(),
                serde_json::Value::String(commitment_to_string(commitment_level).to_string()),
            );
        }

        if let Some((offset, length)) = data_slice {
            config.insert(
                "dataSlice".to_owned(),
                serde_json::json!({
                    "offset": offset,
                    "length": length
                }),
            );
        }

        if let Some(filter_list) = filters {
            let filters_json: Vec<serde_json::Value> = filter_list
                .into_iter()
                .map(|f| match f {
                    RpcFilterType::DataSize(size) => serde_json::json!({ "dataSize": size }),
                    RpcFilterType::Memcmp { offset, bytes } => serde_json::json!({
                        "memcmp": {
                            "offset": offset,
                            "bytes": bytes
                        }
                    }),
                })
                .collect();

            config.insert("filters".to_owned(), serde_json::Value::Array(filters_json));
        }

        let params = serde_json::json!([program_id.to_string(), serde_json::Value::Object(config)]);

        let result = self
            .manager
            .execute_raw("getProgramAccounts", params)
            .await?;

        let accounts_array = result.as_array().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "response".to_string(),
                error: "expected array".to_string(),
            })
        })?;

        let mut accounts = Vec::with_capacity(accounts_array.len());

        for item in accounts_array {
            let pubkey_str = item.get("pubkey").and_then(|v| v.as_str()).ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "pubkey".to_string(),
                    error: "missing pubkey field".to_string(),
                })
            })?;

            let pubkey = Pubkey::from_str(pubkey_str).map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "pubkey".to_string(),
                    error: format!("Invalid pubkey \'{pubkey_str}\': {e}"),
                })
            })?;

            let account_data = item.get("account").ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "account".to_string(),
                    error: "missing account field".to_string(),
                })
            })?;

            if let Some(account) = parse_account_from_json(account_data)? {
                accounts.push((pubkey, account));
            }
        }

        Ok(accounts)
    }

    // =========================================================================
    // Token Supply Methods Implementation
    // =========================================================================

    async fn get_token_supply(&self, mint: &Pubkey) -> crate::Result<TokenSupply> {
        let params = serde_json::json!([
            mint.to_string(),
            { "commitment": "confirmed" }
        ]);

        let result = self.manager.execute_raw("getTokenSupply", params).await?;

        let value = result.get("value").ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "response".to_string(),
                error: "missing value field".to_string(),
            })
        })?;

        let amount = value
            .get("amount")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "amount".to_string(),
                    error: "missing amount field".to_string(),
                })
            })?
            .to_string();

        let decimals = value
            .get("decimals")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "decimals".to_string(),
                    error: "missing decimals field".to_string(),
                })
            })? as u8;

        let ui_amount = value.get("uiAmount").and_then(|v| v.as_f64());

        let ui_amount_string = value
            .get("uiAmountString")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();

        Ok(TokenSupply {
            amount,
            decimals,
            ui_amount,
            ui_amount_string,
        })
    }

    async fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> crate::Result<Vec<RpcTokenAccountBalance>> {
        let params = serde_json::json!([
            mint.to_string(),
            { "commitment": "confirmed" }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenLargestAccounts", params)
            .await?;

        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::InvalidFormat {
                    expected: "value array".to_owned(),
                    received: "missing or not an array".to_owned(),
                })
            })?;

        let mut accounts = Vec::with_capacity(values.len());

        for item in values {
            let address_str = item
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    crate::Error::Data(crate::errors::DataError::InvalidFormat {
                        expected: "address field".to_owned(),
                        received: "missing or not a string".to_owned(),
                    })
                })?;

            let address = Pubkey::from_str(address_str).map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "Pubkey".to_owned(),
                    error: e.to_string(),
                })
            })?;

            let amount = item
                .get("amount")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    crate::Error::Data(crate::errors::DataError::ParseError {
                        data_type: "amount".to_string(),
                        error: "missing amount field".to_string(),
                    })
                })?
                .to_string();

            let decimals = item
                .get("decimals")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    crate::Error::Data(crate::errors::DataError::ParseError {
                        data_type: "decimals".to_string(),
                        error: "missing decimals field".to_string(),
                    })
                })? as u8;

            let ui_amount = item.get("uiAmount").and_then(|v| v.as_f64());

            let ui_amount_string = item
                .get("uiAmountString")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string();

            accounts.push(RpcTokenAccountBalance {
                address,
                amount,
                decimals,
                ui_amount,
                ui_amount_string,
            });
        }

        Ok(accounts)
    }

    // =========================================================================
    // Statistics and Health Methods Implementation
    // =========================================================================

    async fn get_stats(&self) -> RpcStatsResponse {
        self.manager.get_stats().await
    }

    async fn get_provider_health(&self) -> Vec<ProviderHealthInfo> {
        // Delegate to the RpcClient method
        RpcClient::get_provider_health(self).await
    }

    // =========================================================================
    // Convenience Implementations
    // =========================================================================

    async fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> crate::Result<Vec<SignatureInfo>> {
        let parse_sig = |sig_str: &str| -> crate::Result<Signature> {
            Signature::from_str(sig_str).map_err(|e| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "Signature".to_owned(),
                    error: e.to_string(),
                })
            })
        };
        let before_sig = before.map(parse_sig).transpose()?;
        let until_sig = until.map(parse_sig).transpose()?;
        self.get_signatures_for_address(
            wallet_pubkey,
            Some(limit),
            before_sig.as_ref(),
            until_sig.as_ref(),
        )
        .await
    }

    async fn get_transaction_details_with_commitment(
        &self,
        signature: &str,
        commitment: CommitmentLevel,
    ) -> crate::Result<TransactionDetails> {
        let params = serde_json::json!([signature, get_transaction_config(Some(commitment))]);

        let result = self.manager.execute_raw("getTransaction", params).await?;

        if result.is_null() {
            return Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Transaction not found: {signature}"),
            }));
        }

        serde_json::from_value(result).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("failed to parse transaction: {e}"),
            })
        })
    }

    async fn get_transaction_details(&self, signature: &str) -> crate::Result<TransactionDetails> {
        // Use jsonParsed encoding for proper decoding (required for v0 transactions with LUTs)
        let params = serde_json::json!([signature, get_transaction_config(None)]);

        let result = self.manager.execute_raw("getTransaction", params).await?;

        if result.is_null() {
            return Err(crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Transaction not found: {signature}"),
            }));
        }

        serde_json::from_value(result).map_err(|e| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: e.to_string().to_string(),
            })
        })
    }
}

// Helper functions

/// Read a `simulateTransaction` reply's `value`.
///
/// The balance fields are optional: a node that omits one, or returns one in an
/// unexpected shape, leaves it `None` rather than failing the simulation.
fn simulation_outcome(value: &serde_json::Value) -> SimulationOutcome {
    fn field<T: serde::de::DeserializeOwned>(value: &serde_json::Value, key: &str) -> Option<T> {
        value
            .get(key)
            .filter(|v| !v.is_null())
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    SimulationOutcome {
        err: value.get("err").filter(|v| !v.is_null()).cloned(),
        logs: value
            .get("logs")
            .and_then(|v| v.as_array())
            .map(|logs| {
                logs.iter()
                    .filter_map(|l| l.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        units_consumed: value
            .get("unitsConsumed")
            .and_then(serde_json::Value::as_u64),
        inner_instructions: value
            .get("innerInstructions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        fee: value.get("fee").and_then(serde_json::Value::as_u64),
        pre_balances: field(value, "preBalances"),
        post_balances: field(value, "postBalances"),
        pre_token_balances: field(value, "preTokenBalances"),
        post_token_balances: field(value, "postTokenBalances"),
        loaded_addresses: field(value, "loadedAddresses"),
    }
}

fn commitment_to_string(commitment: CommitmentLevel) -> &'static str {
    match commitment {
        CommitmentLevel::Finalized => "finalized",
        CommitmentLevel::Confirmed => "confirmed",
        CommitmentLevel::Processed => "processed",
    }
}

fn block_height_params(commitment: CommitmentLevel) -> serde_json::Value {
    serde_json::json!([{ "commitment": commitment_to_string(commitment) }])
}

fn get_transaction_config(commitment: Option<CommitmentLevel>) -> serde_json::Value {
    let mut config = serde_json::json!({
        "encoding": "jsonParsed",
        "maxSupportedTransactionVersion": 1
    });
    if let Some(commitment) = commitment {
        config["commitment"] = serde_json::json!(commitment_to_string(commitment));
    }
    config
}

fn helius_successful_transactions_params(
    address: &Pubkey,
    limit: usize,
    after: Option<&Signature>,
) -> crate::Result<serde_json::Value> {
    if !(1..=1_000).contains(&limit) {
        return Err(crate::Error::Data(
            crate::errors::DataError::InvalidFormat {
                expected: "transaction page limit in 1..=1000".to_owned(),
                received: limit.to_string(),
            },
        ));
    }

    let mut filters = serde_json::json!({
        "status": "succeeded",
        // Keep both watch modes in the same address-reference cursor domain.
        // ATA-only history can contain signatures absent from getSignaturesForAddress.
        "tokenAccounts": "none",
    });
    if let Some(after) = after {
        filters["signature"] = serde_json::json!({ "gt": after.to_string() });
    }

    Ok(serde_json::json!([
        address.to_string(),
        {
            "transactionDetails": "full",
            "sortOrder": "asc",
            "limit": limit,
            "commitment": "confirmed",
            "encoding": "jsonParsed",
            "maxSupportedTransactionVersion": 1,
            "filters": filters,
        }
    ]))
}

fn parse_helius_transactions_page(
    result: serde_json::Value,
) -> crate::Result<HeliusTransactionsPage> {
    let data = result
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "Helius transaction page".to_owned(),
                error: "missing data array".to_owned(),
            })
        })?;

    let mut transactions = Vec::with_capacity(data.len());
    for item in data {
        let transaction: TransactionDetails =
            serde_json::from_value(item.clone()).map_err(|error| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "Helius transaction".to_owned(),
                    error: error.to_string(),
                })
            })?;

        let signature = transaction.transaction.signatures.first().ok_or_else(|| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "Helius transaction signature".to_owned(),
                error: "missing first signature".to_owned(),
            })
        })?;
        Signature::from_str(signature).map_err(|error| {
            crate::Error::Data(crate::errors::DataError::ParseError {
                data_type: "Helius transaction signature".to_owned(),
                error: error.to_string(),
            })
        })?;

        transactions.push(transaction);
    }

    Ok(HeliusTransactionsPage {
        transactions,
        pagination_token: result
            .get("paginationToken")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        block_height_params, get_transaction_config, helius_successful_transactions_params,
        parse_helius_transactions_page, simulation_outcome, token_accounts_from,
        token_accounts_params, CommitmentLevel, EncodedConfirmedTransactionWithStatusMeta, Pubkey,
        RpcError, Signature, SPL_TOKEN_PROGRAM_ID,
    };
    use crate::chains::solana::rpc::types::LoadedAddresses;

    /// The reply shape a mainnet node gave for an unsigned 1,000-lamport
    /// transfer, with a token balance and loaded addresses added: the fee is
    /// already deducted from the post-balances.
    #[test]
    fn a_simulation_reply_carries_its_fee_balances_and_loaded_addresses() {
        let token_balance = |amount: &str| {
            serde_json::json!({
                "accountIndex": 3,
                "mint": "So11111111111111111111111111111111111111112",
                "owner": "6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv",
                "programId": SPL_TOKEN_PROGRAM_ID,
                "uiTokenAmount": {
                    "amount": amount,
                    "decimals": 9,
                    "uiAmount": null,
                    "uiAmountString": "0"
                }
            })
        };
        let value = serde_json::json!({
            "err": null,
            "logs": ["Program 11111111111111111111111111111111 success"],
            "unitsConsumed": 150,
            "innerInstructions": [],
            "fee": 5000,
            "preBalances": [811_177_354u64, 85_008_688u64, 1u64],
            "postBalances": [811_171_354u64, 85_009_688u64, 1u64],
            "preTokenBalances": [token_balance("0")],
            "postTokenBalances": [token_balance("1000")],
            "loadedAddresses": {
                "writable": ["AcEyePa7h3LcuNhvNZSxVbMp2sU6JFUe3yBtkqQ1UtvW"],
                "readonly": ["TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"]
            },
            "accounts": null,
            "returnData": null
        });

        let outcome = simulation_outcome(&value);

        assert!(outcome.succeeded());
        assert_eq!(outcome.units_consumed, Some(150));
        assert_eq!(outcome.fee, Some(5000));
        assert_eq!(outcome.pre_balances, Some(vec![811_177_354, 85_008_688, 1]));
        assert_eq!(
            outcome.post_balances,
            Some(vec![811_171_354, 85_009_688, 1])
        );
        let pre = outcome.pre_token_balances.expect("pre token balances");
        let post = outcome.post_token_balances.expect("post token balances");
        assert_eq!(pre[0].account_index, 3);
        assert_eq!(
            pre[0].owner.as_deref(),
            Some("6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv")
        );
        assert_eq!(pre[0].ui_token_amount.amount, "0");
        assert_eq!(post[0].ui_token_amount.amount, "1000");
        assert_eq!(
            outcome.loaded_addresses,
            Some(LoadedAddresses {
                writable: vec!["AcEyePa7h3LcuNhvNZSxVbMp2sU6JFUe3yBtkqQ1UtvW".to_owned()],
                readonly: vec![SPL_TOKEN_PROGRAM_ID.to_owned()],
            })
        );

        // A node that returns none of the balance fields leaves them unset.
        let bare = simulation_outcome(&serde_json::json!({"err": null, "logs": []}));
        assert_eq!(bare.fee, None);
        assert!(bare.pre_balances.is_none() && bare.post_token_balances.is_none());
        assert!(bare.loaded_addresses.is_none());
    }

    fn token_account(pubkey: &str, mint: &str, amount: &str) -> serde_json::Value {
        serde_json::json!({
            "pubkey": pubkey,
            "account": { "data": { "parsed": { "info": {
                "mint": mint,
                "state": "initialized",
                "tokenAmount": { "amount": amount, "decimals": 6 }
            } } } }
        })
    }

    fn accounts(values: Vec<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({ "context": { "slot": 1 }, "value": values }))
    }

    fn rate_limited() -> Result<serde_json::Value, RpcError> {
        Err(RpcError::RateLimited {
            provider_id: "provider".to_owned(),
            retry_after: None,
        })
    }

    #[test]
    fn token_accounts_merge_both_programs() {
        let merged = token_accounts_from(
            accounts(vec![token_account("spl-account", "mint-a", "5")]),
            accounts(vec![token_account("token-2022-account", "mint-b", "7")]),
        )
        .expect("both reads succeeded");
        let read: Vec<_> = merged
            .accounts
            .iter()
            .map(|info| (info.account.as_str(), info.balance, info.is_token_2022))
            .collect();
        assert_eq!(
            read,
            [("spl-account", 5, false), ("token-2022-account", 7, true)]
        );
    }

    #[test]
    fn token_accounts_carry_the_lower_slot_of_the_two_reads() {
        let at = |slot: u64| Ok(serde_json::json!({ "context": { "slot": slot }, "value": [] }));
        assert_eq!(token_accounts_from(at(12), at(10)).unwrap().slot, Some(10));
        assert_eq!(token_accounts_from(at(10), at(12)).unwrap().slot, Some(10));
        let without_context = || Ok(serde_json::json!({ "value": [] }));
        assert_eq!(
            token_accounts_from(at(10), without_context()).unwrap().slot,
            None,
            "a read without a slot cannot vouch for the snapshot"
        );
    }

    #[test]
    fn token_account_reads_name_the_wallet_commitment() {
        let owner = Pubkey::new_unique();
        let params = token_accounts_params(&owner, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(params[2]["commitment"], "confirmed");
        assert_eq!(params[2]["encoding"], "jsonParsed");
        assert_eq!(params[1]["programId"], SPL_TOKEN_PROGRAM_ID);
    }

    /// Every wallet-state read in this file names its commitment. The RPC default is
    /// `finalized`, which trails the `confirmed` swaps and history by about 13 seconds.
    #[test]
    fn every_wallet_state_read_names_its_commitment() {
        let source = include_str!("methods_impl.rs");
        let source: String = source[..source.find("#[cfg(test)]").expect("test module")]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let mut reads = 0;
        for method in [
            "getBalance",
            "getTokenAccountBalance",
            "getTokenAccountsByOwner",
        ] {
            let call = format!("execute_raw(\"{method}\"");
            let mut rest = source.as_str();
            while let Some(at) = rest.find(&call) {
                reads += 1;
                let function_start = rest[..at].rfind("asyncfn").unwrap_or(0);
                let call_end = (at + call.len() + 80).min(rest.len());
                let call_site = &rest[function_start..call_end];
                assert!(
                    call_site.contains("WALLET_READ_COMMITMENT")
                        || call_site.contains("token_accounts_params"),
                    "{method} is read without a commitment: {call_site}"
                );
                rest = &rest[at + call.len()..];
            }
        }
        assert!(reads >= 6, "the scan found only {reads} wallet-state reads");
    }

    #[test]
    fn a_failed_read_of_either_program_fails_the_token_accounts() {
        let good = || accounts(vec![token_account("account", "mint", "5")]);
        assert!(token_accounts_from(rate_limited(), good()).is_err());
        assert!(token_accounts_from(good(), rate_limited()).is_err());
        assert!(token_accounts_from(rate_limited(), rate_limited()).is_err());
    }

    #[test]
    fn a_response_without_a_value_array_fails_the_token_accounts() {
        let good = || accounts(Vec::new());
        for malformed in [
            serde_json::json!({ "context": { "slot": 1 } }),
            serde_json::json!({ "value": null }),
            serde_json::json!({ "value": {} }),
        ] {
            assert!(token_accounts_from(Ok(malformed.clone()), good()).is_err());
            assert!(token_accounts_from(good(), Ok(malformed)).is_err());
        }
    }

    #[test]
    fn an_unreadable_account_fails_the_token_accounts() {
        let unreadable = serde_json::json!({ "pubkey": "broken", "account": {} });
        let error = token_accounts_from(
            accounts(vec![token_account("account", "mint", "5"), unreadable]),
            accounts(Vec::new()),
        )
        .expect_err("an account was unreadable");
        assert!(error.to_string().contains("broken"), "{error}");
    }

    #[test]
    fn transaction_requests_use_json_parsed_and_integer_version_one() {
        let without_commitment = get_transaction_config(None);
        assert_eq!(without_commitment["encoding"], "jsonParsed");
        assert_eq!(without_commitment["maxSupportedTransactionVersion"], 1);
        assert!(without_commitment["maxSupportedTransactionVersion"].is_number());
        assert!(without_commitment.get("commitment").is_none());

        let confirmed = get_transaction_config(Some(CommitmentLevel::Confirmed));
        assert_eq!(confirmed["commitment"], "confirmed");
        assert_eq!(confirmed["maxSupportedTransactionVersion"], 1);
    }

    #[test]
    fn block_height_requests_name_their_commitment() {
        for (commitment, name) in [
            (CommitmentLevel::Finalized, "finalized"),
            (CommitmentLevel::Confirmed, "confirmed"),
            (CommitmentLevel::Processed, "processed"),
        ] {
            assert_eq!(
                block_height_params(commitment),
                serde_json::json!([{ "commitment": name }])
            );
        }
    }

    #[test]
    fn sdk_transaction_response_decodes_v1_parsed_message() {
        let response: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(serde_json::json!({
                "slot": 1,
                "transaction": {
                    "signatures": ["signature"],
                    "message": {
                        "accountKeys": [{
                            "pubkey": "11111111111111111111111111111111",
                            "signer": true,
                            "writable": true,
                            "source": "transaction"
                        }],
                        "recentBlockhash": "11111111111111111111111111111111",
                        "instructions": [],
                        "transactionConfig": {
                            "computeUnitLimit": 30000,
                            "heapSize": null,
                            "loadedAccountsDataSizeLimit": 200000,
                            "priorityFee": 6000
                        }
                    }
                },
                "meta": null,
                "version": 1,
                "blockTime": null
            }))
            .expect("SDK transaction response decodes v1 jsonParsed fields");

        assert_eq!(
            serde_json::to_value(response).expect("SDK response serializes")["version"],
            1
        );
    }

    #[test]
    fn helius_page_request_is_ascending_successful_and_exclusive() {
        let address = Pubkey::new_unique();
        let boundary = Signature::new_unique();
        let params = helius_successful_transactions_params(&address, 500, Some(&boundary))
            .expect("valid page request");
        let config = &params[1];

        assert_eq!(config["transactionDetails"], "full");
        assert_eq!(config["sortOrder"], "asc");
        assert_eq!(config["limit"], 500);
        assert_eq!(config["commitment"], "confirmed");
        assert_eq!(config["encoding"], "jsonParsed");
        assert_eq!(config["maxSupportedTransactionVersion"], 1);
        assert_eq!(config["filters"]["status"], "succeeded");
        assert_eq!(config["filters"]["tokenAccounts"], "none");
        assert_eq!(config["filters"]["signature"]["gt"], boundary.to_string());
        assert!(config["filters"].get("tokenTransfer").is_none());
        assert!(helius_successful_transactions_params(&address, 1_001, None).is_err());
    }

    #[test]
    fn helius_page_parses_v1_transactions_and_pagination() {
        let signature = Signature::new_unique();
        let page = parse_helius_transactions_page(serde_json::json!({
            "data": [{
                "slot": 1,
                "transactionIndex": 0,
                "blockTime": 123,
                "transaction": {
                    "signatures": [signature.to_string()],
                    "message": { "accountKeys": [], "instructions": [] }
                },
                "meta": {
                    "err": null,
                    "fee": 5000,
                    "preBalances": [],
                    "postBalances": [],
                    "preTokenBalances": [],
                    "postTokenBalances": [],
                    "computeUnitsConsumed": 7,
                    "logMessages": [],
                    "innerInstructions": []
                },
                "version": 1
            }],
            "paginationToken": "1:1"
        }))
        .expect("valid Helius transaction page");

        assert_eq!(page.transactions.len(), 1);
        assert_eq!(
            page.transactions[0].transaction.signatures[0],
            signature.to_string()
        );
        assert_eq!(page.pagination_token.as_deref(), Some("1:1"));
    }

    #[test]
    fn helius_page_rejects_an_invalid_first_signature() {
        let error = parse_helius_transactions_page(serde_json::json!({
            "data": [{
                "slot": 1,
                "transaction": {
                    "signatures": ["not-a-signature"],
                    "message": {}
                },
                "meta": null,
                "blockTime": null
            }]
        }))
        .expect_err("invalid first signature must be rejected");

        assert!(error.to_string().contains("Helius transaction signature"));
    }
}

fn parse_account_from_json(value: &serde_json::Value) -> crate::Result<Option<Account>> {
    use crate::errors::{DataError, Error};

    if value.is_null() {
        return Ok(None);
    }

    let data = value.get("data").ok_or_else(|| {
        Error::Data(DataError::ParseError {
            data_type: "account".to_owned(),
            error: "Missing data field".to_owned(),
        })
    })?;

    let data_bytes = if let Some(arr) = data.as_array() {
        // [data_base64, encoding]
        let encoded = arr.first().and_then(|v| v.as_str()).ok_or_else(|| {
            Error::Data(DataError::ParseError {
                data_type: "account".to_owned(),
                error: "Invalid data array format".to_owned(),
            })
        })?;
        let encoding = arr.get(1).and_then(|v| v.as_str()).unwrap_or("base64");

        if encoding == "base64" {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| {
                    Error::Data(DataError::ParseError {
                        data_type: "account base64".to_owned(),
                        error: e.to_string(),
                    })
                })?
        } else {
            return Err(Error::Data(DataError::InvalidFormat {
                expected: "base64 encoding".to_owned(),
                received: encoding.to_string(),
            }));
        }
    } else if let Some(s) = data.as_str() {
        // Direct base64 string
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(|e| {
                Error::Data(DataError::ParseError {
                    data_type: "account base64".to_owned(),
                    error: e.to_string(),
                })
            })?
    } else {
        return Err(Error::Data(DataError::InvalidFormat {
            expected: "base64 string or array".to_owned(),
            received: format!("{:?}", data),
        }));
    };

    let lamports = value
        .get("lamports")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            Error::Data(DataError::ParseError {
                data_type: "account".to_owned(),
                error: "Missing or invalid lamports field".to_owned(),
            })
        })?;

    let owner_str = value.get("owner").and_then(|v| v.as_str()).ok_or_else(|| {
        Error::Data(DataError::ParseError {
            data_type: "account".to_owned(),
            error: "Missing owner field".to_owned(),
        })
    })?;

    let owner = Pubkey::from_str(owner_str).map_err(|e| {
        Error::Data(DataError::ParseError {
            data_type: "pubkey".to_owned(),
            error: format!("Invalid owner pubkey '{owner_str}': {e}"),
        })
    })?;

    let executable = value
        .get("executable")
        .and_then(|v| v.as_bool())
        .unwrap_or_default();

    let rent_epoch = value
        .get("rentEpoch")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();

    Ok(Some(Account {
        lamports,
        data: data_bytes,
        owner,
        executable,
        rent_epoch,
    }))
}

/// Commitment of every wallet balance and token-account read. Swaps and the transaction
/// history are read at `confirmed`; a read at the RPC default, `finalized`, trails them by
/// about 13 seconds, so a token bought moments ago reads as absent and a sold one as held.
const WALLET_READ_COMMITMENT: &str = "confirmed";

/// `getTokenAccountsByOwner` parameters for every account of `owner` under one token program.
fn token_accounts_params(owner: &Pubkey, program_id: &str) -> serde_json::Value {
    serde_json::json!([
        owner.to_string(),
        { "programId": program_id },
        { "encoding": "jsonParsed", "commitment": WALLET_READ_COMMITMENT }
    ])
}

/// The token accounts of both token programs from their `getTokenAccountsByOwner` responses,
/// at the lower of the two context slots.
/// A failed read, a response without a `value` array or an account that cannot be parsed
/// fails the whole read: a holding summed from a partial list would understate what the
/// wallet holds and read as a known amount.
fn token_accounts_from(
    spl: Result<serde_json::Value, RpcError>,
    token_2022: Result<serde_json::Value, RpcError>,
) -> crate::Result<TokenAccountsSnapshot> {
    let mut accounts = Vec::new();
    let mut slots = Vec::with_capacity(2);
    for (response, is_token_2022) in [(spl, false), (token_2022, true)] {
        let response = response?;
        slots.push(
            response
                .get("context")
                .and_then(|context| context.get("slot"))
                .and_then(serde_json::Value::as_u64),
        );
        let values = response
            .get("value")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "token accounts".to_owned(),
                    error: "missing value array".to_owned(),
                })
            })?;
        for item in values {
            let info = parse_token_account_info(item, is_token_2022).ok_or_else(|| {
                crate::Error::Data(crate::errors::DataError::ParseError {
                    data_type: "token account".to_owned(),
                    error: format!(
                        "unreadable account {}",
                        item.get("pubkey")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("without a pubkey")
                    ),
                })
            })?;
            accounts.push(info);
        }
    }
    let slot = slots
        .iter()
        .copied()
        .collect::<Option<Vec<u64>>>()
        .and_then(|slots| slots.into_iter().min());
    Ok(TokenAccountsSnapshot { slot, accounts })
}

/// Whether `pubkey` is the associated token account of `owner` for `mint`, derived with
/// the token program that owns the account (`program`), or with either token program
/// when the response does not name one.
fn is_associated_token_account(
    pubkey: &str,
    owner: &Pubkey,
    mint: &Pubkey,
    program: Option<&str>,
) -> bool {
    let programs: Vec<&str> = match program {
        Some(program) => vec![program],
        None => vec![SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID],
    };
    programs
        .into_iter()
        .filter_map(|program| Pubkey::from_str(program).ok())
        .any(|program| {
            crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id(
                owner, mint, &program,
            )
            .to_string()
                == pubkey
        })
}

/// Parse token account info from jsonParsed response
fn parse_token_account_info(
    item: &serde_json::Value,
    is_token_2022: bool,
) -> Option<TokenAccountInfo> {
    let pubkey_str = item.get("pubkey")?.as_str()?;
    let account = item.get("account")?;
    let data = account.get("data")?;
    let parsed = data.get("parsed")?;
    let info = parsed.get("info")?;

    let mint_str = info.get("mint")?.as_str()?;
    let token_amount = info.get("tokenAmount")?;
    let amount_str = token_amount.get("amount")?.as_str()?;
    let decimals = token_amount.get("decimals")?.as_u64()? as u8;
    let balance = amount_str.parse::<u64>().ok()?;

    // NFT detection: decimals=0 and balance=1 typically indicates an NFT
    let is_nft = decimals == 0 && balance == 1;

    // jsonParsed reports "initialized" | "frozen" | "uninitialized". Anything we cannot
    // read is treated as NOT frozen: claiming a sellable holding is frozen would hide a
    // real position behind a warning.
    let is_frozen = info.get("state").and_then(|v| v.as_str()) == Some("frozen");

    Some(TokenAccountInfo {
        account: pubkey_str.to_string(),
        mint: mint_str.to_string(),
        balance,
        decimals,
        is_token_2022,
        is_nft,
        is_frozen,
    })
}
