//! Pool analyzer extractor methods.
//!
//! Per-DEX reserve account extraction logic for each supported program type.
//! These methods are called from the `extract_reserve_accounts` router in `analyzer.rs`.

use super::analyzer::PoolAnalyzer;
use super::decoders::{
    meteora_damm::MeteoraDammDecoder, meteora_dlmm::MeteoraDlmmDecoder,
    orca_whirlpool::OrcaWhirlpoolDecoder, pumpfun_amm::PumpFunAmmDecoder,
    raydium_clmm::RaydiumClmmDecoder, raydium_cpmm::RaydiumCpmmDecoder,
    raydium_legacy_amm::RaydiumLegacyAmmDecoder,
};

use crate::chains::solana::rpc::{RpcClient, RpcClientMethods};
use crate::logger::{self, LogTag};

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

impl PoolAnalyzer {
    /// Extract Raydium CPMM pool accounts
    pub(crate) async fn extract_raydium_cpmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Pool account {pool_id} not found"),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch pool account {pool_id}: {e}"),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses = RaydiumCpmmDecoder::extract_reserve_accounts(&pool_account.data)?;

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

        Some(accounts)
    }

    /// Extract Raydium Legacy AMM pool accounts
    pub(crate) async fn extract_raydium_legacy_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracting Raydium Legacy AMM accounts for pool {}",
                pool_id
            ),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                RaydiumLegacyAmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
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
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from Raydium Legacy AMM pool {}",
                        pool_id
                    ),
                );
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Raydium CLMM pool accounts
    pub(crate) async fn extract_raydium_clmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // For CLMM pools, we need:
        // - Pool account itself
        // - Token vaults (extracted from pool data)

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting CLMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                RaydiumClmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
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
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Orca Whirlpool accounts
    pub(crate) async fn extract_orca_whirlpool_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting Orca Whirlpool accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                OrcaWhirlpoolDecoder::extract_reserve_accounts(&pool_account.data)
            {
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
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from Orca Whirlpool pool {}",
                        pool_id
                    ),
                );
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Meteora DAMM accounts
    pub(crate) async fn extract_meteora_damm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting DAMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                MeteoraDammDecoder::extract_reserve_accounts(&pool_account.data)
            {
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
            }
        }

        // Always include the mints
        accounts.push(*base_mint);
        accounts.push(*quote_mint);

        Some(accounts)
    }

    /// Extract Meteora DLMM accounts
    pub(crate) async fn extract_meteora_dlmm_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("DLMM pool account {pool_id} not found"),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch DLMM pool account {pool_id}: {e}"),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses = MeteoraDlmmDecoder::extract_reserve_accounts(&pool_account.data)?;

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

        Some(accounts)
    }

    /// Extract Pump.fun AMM accounts
    pub(crate) async fn extract_pump_fun_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        logger::debug(
            LogTag::PoolAnalyzer,
            &format!("Extracting PumpFun AMM accounts for pool {pool_id}"),
        );

        let mut accounts = vec![*pool_id];

        // Fetch pool account to extract vault addresses using decoder function
        if let Ok(Some(pool_account)) = rpc_client.get_account(pool_id).await {
            if let Some(vault_addresses) =
                PumpFunAmmDecoder::extract_reserve_accounts(&pool_account.data)
            {
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
            } else {
                logger::warning(
                    LogTag::PoolAnalyzer,
                    &format!(
                        "Failed to extract vault addresses from PumpFun AMM pool {}",
                        pool_id
                    ),
                );
            }
        }

        Some(accounts)
    }

    /// Extract Moonit AMM accounts
    pub(crate) async fn extract_moonit_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        let mut accounts = vec![*pool_id];

        logger::debug(
            LogTag::PoolAnalyzer,
            &format!(
                "Extracted Moonit accounts: curve={}, total_accounts={}",
                pool_id,
                accounts.len()
            ),
        );

        Some(accounts)
    }

    pub(crate) async fn extract_fluxbeam_accounts(
        pool_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Option<Vec<Pubkey>> {
        // Fetch the pool account to extract vault addresses using decoder function
        let pool_account = match rpc_client.get_account(pool_id).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("FluxBeam pool account {pool_id} not found"),
                );
                return None;
            }
            Err(e) => {
                logger::error(
                    LogTag::PoolAnalyzer,
                    &format!("Failed to fetch pool account {pool_id}: {e}"),
                );
                return None;
            }
        };

        // Parse the pool data to extract vault addresses using decoder function
        let vault_addresses =
            super::decoders::fluxbeam_amm::FluxbeamAmmDecoder::extract_reserve_accounts(
                &pool_account.data,
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

        Some(accounts)
    }
}
