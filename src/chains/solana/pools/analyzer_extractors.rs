// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool analyzer extractor methods.
//!
//! Per-DEX reserve account extraction logic for each supported program type.
//! These methods are called from the `extract_reserve_accounts` router in `analyzer.rs`.

use super::analyzer::{classify_account_fetch_error, PoolAnalysisFailure, PoolAnalyzer};
use super::decoders::{
    meteora_damm::MeteoraDammDecoder, meteora_dlmm::MeteoraDlmmDecoder,
    orca_whirlpool::OrcaWhirlpoolDecoder, pumpfun_amm::PumpFunAmmDecoder,
    raydium_clmm::RaydiumClmmDecoder, raydium_cpmm::RaydiumCpmmDecoder,
    raydium_legacy_amm::RaydiumLegacyAmmDecoder,
};

use crate::chains::solana::rpc::{RpcClient, RpcClientMethods};
use crate::logger::{self, LogTag};

use crate::chains::solana::solana_sdk::account::Account;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

impl PoolAnalyzer {
    /// Fetch a pool account for reserve extraction.
    ///
    /// An absent account is structural; an RPC failure is classified by
    /// `classify_account_fetch_error` and is transient unless the RPC layer
    /// reported the account as not found.
    pub(crate) async fn fetch_pool_account(
        pool_id: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Account, PoolAnalysisFailure> {
        match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => Ok(account),
            Ok(None) => {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!("Pool account {pool_id} not found"),
                );
                Err(PoolAnalysisFailure::Structural)
            }
            Err(e) => {
                let failure = classify_account_fetch_error(&e);
                match failure {
                    PoolAnalysisFailure::Transient => logger::debug(
                        LogTag::PoolAnalyzer,
                        &format!("Transient RPC failure fetching pool account {pool_id}: {e}"),
                    ),
                    PoolAnalysisFailure::Structural => logger::warning(
                        LogTag::PoolAnalyzer,
                        &format!("Pool account {pool_id} unavailable: {e}"),
                    ),
                }
                Err(failure)
            }
        }
    }

    /// Reserve vaults decoded from a pool account; a layout that does not
    /// decode is structural.
    fn decoded_vaults(
        pool_id: &Pubkey,
        venue: &str,
        vaults: Option<Vec<String>>,
    ) -> Result<Vec<String>, PoolAnalysisFailure> {
        vaults.ok_or_else(|| {
            logger::warning(
                LogTag::PoolAnalyzer,
                &format!("Failed to extract vault addresses from {venue} pool {pool_id}"),
            );
            PoolAnalysisFailure::Structural
        })
    }

    /// Extract Raydium CPMM pool accounts
    pub(crate) async fn extract_raydium_cpmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Raydium CPMM",
            RaydiumCpmmDecoder::extract_reserve_accounts(&pool_account.data),
        )?;

        let mut accounts = vec![*pool_id];

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Raydium Legacy AMM pool accounts
    pub(crate) async fn extract_raydium_legacy_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracting Raydium Legacy AMM accounts for pool {}",
                pool_id
            ),
        );

        let mut accounts = vec![*pool_id];

        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Raydium Legacy AMM",
            RaydiumLegacyAmmDecoder::extract_reserve_accounts(&pool_account.data),
        )?;
        let vault_count = vault_addresses.len();
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Raydium Legacy AMM pool {} extracted {} vault accounts",
                pool_id, vault_count
            ),
        );

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Raydium CLMM pool accounts
    pub(crate) async fn extract_raydium_clmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        // For CLMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting CLMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Raydium CLMM",
            RaydiumClmmDecoder::extract_reserve_accounts(&pool_account.data),
        )?;
        let vault_count = vault_addresses.len();
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "CLMM pool {} extracted {} vault accounts",
                pool_id, vault_count
            ),
        );

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Orca Whirlpool accounts
    pub(crate) async fn extract_orca_whirlpool_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting Orca Whirlpool accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Orca Whirlpool",
            OrcaWhirlpoolDecoder::extract_reserve_accounts(&pool_account.data),
        )?;
        let vault_count = vault_addresses.len();
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Orca Whirlpool pool {} extracted {} vault accounts",
                pool_id, vault_count
            ),
        );

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Meteora DAMM accounts
    pub(crate) async fn extract_meteora_damm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting DAMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Meteora DAMM",
            MeteoraDammDecoder::extract_reserve_accounts(&pool_account.data),
        )?;
        let vault_count = vault_addresses.len();
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "DAMM pool {} extracted {} vault accounts",
                pool_id, vault_count
            ),
        );

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Meteora DLMM accounts
    pub(crate) async fn extract_meteora_dlmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "Meteora DLMM",
            MeteoraDlmmDecoder::extract_reserve_accounts(&pool_account.data),
        )?;

        let mut accounts = vec![*pool_id];

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Ok(accounts)
    }

    /// Extract Pump.fun AMM accounts
    pub(crate) async fn extract_pump_fun_accounts(
        pool_id: &Pubkey,
        _base_mint: &Pubkey,
        _quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting PumpFun AMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "PumpFun AMM",
            PumpFunAmmDecoder::extract_reserve_accounts(&pool_account.data),
        )?;
        let vault_count = vault_addresses.len();
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "PumpFun AMM pool {} extracted {} vault accounts",
                pool_id, vault_count
            ),
        );

        Ok(accounts)
    }

    /// Extract Moonit AMM accounts
    pub(crate) async fn extract_moonit_accounts(
        pool_id: &Pubkey,
        _base_mint: &Pubkey,
        _quote_mint: &Pubkey,
        _rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        let accounts = vec![*pool_id];

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracted Moonit accounts: curve={}, total_accounts={}",
                pool_id,
                accounts.len()
            ),
        );

        Ok(accounts)
    }

    pub(crate) async fn extract_fluxbeam_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Vec<Pubkey>, PoolAnalysisFailure> {
        let pool_account = Self::fetch_pool_account(pool_id, rpc_client).await?;
        let vault_addresses = Self::decoded_vaults(
            pool_id,
            "FluxBeam",
            super::decoders::fluxbeam_amm::FluxbeamAmmDecoder::extract_reserve_accounts(
                &pool_account.data,
            ),
        )?;

        let mut accounts = vec![*pool_id];
        let vault_count = vault_addresses.len();

        // Add vault addresses to accounts list
        for vault_str in vault_addresses {
            if let Ok(vault_pubkey) = Pubkey::from_str(&vault_str) {
                accounts.push(vault_pubkey);
            }
        }

        // Add the mints for reference
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracted FluxBeam accounts: pool={}, vaults={}, total_accounts={}",
                pool_id,
                vault_count,
                accounts.len()
            ),
        );

        Ok(accounts)
    }
}
