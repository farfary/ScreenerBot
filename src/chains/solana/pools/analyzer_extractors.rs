// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool analyzer extractor methods.
//!
//! Per-DEX derivation of the account set the pool fetcher must hold to price a pool.
//! Every derivation is pure: it reads the pool account's bytes and the pair's mints and
//! touches no RPC, so the analyzer and the recorded price snapshot derive the same set
//! from the same function, [`PoolAnalyzer::reserve_account_set`].

use super::analyzer::{classify_account_fetch_error, PoolAnalysisFailure, PoolAnalyzer};
use super::decoders::{
    fluxbeam_amm::FluxbeamAmmDecoder, meteora_damm::MeteoraDammDecoder,
    meteora_dlmm::MeteoraDlmmDecoder, orca_whirlpool::OrcaWhirlpoolDecoder,
    pumpfun_amm::PumpFunAmmDecoder, raydium_clmm::RaydiumClmmDecoder,
    raydium_cpmm::RaydiumCpmmDecoder,
};
use super::layouts::{meteora_dbc::VirtualPoolState, raydium_amm_v4::AmmV4PoolState};
use super::types::ProgramKind;

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

    /// The accounts the pool fetcher must hold before `pool_id` can be priced, in
    /// registration order: the pool, then the venue's reserve accounts, then the
    /// pair's mints for the venues that register them. The fetcher never requests
    /// the WSOL mint, so it is listed here but never fetched.
    ///
    /// `None` when `pool_data` does not decode as `program_kind`'s pool layout, or
    /// when the program has no decoder.
    pub fn reserve_account_set(
        program_kind: ProgramKind,
        pool_id: &Pubkey,
        pool_data: &[u8],
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Option<Vec<Pubkey>> {
        let mints = [*base_mint, *quote_mint];
        match program_kind {
            ProgramKind::RaydiumCpmm => Self::raydium_cpmm_accounts(pool_id, pool_data, &mints),
            ProgramKind::RaydiumLegacyAmm => {
                Self::raydium_legacy_accounts(pool_id, pool_data, &mints)
            }
            ProgramKind::RaydiumClmm => Self::raydium_clmm_accounts(pool_id, pool_data, &mints),
            ProgramKind::OrcaWhirlpool => Self::orca_whirlpool_accounts(pool_id, pool_data, &mints),
            ProgramKind::MeteoraDamm => Self::meteora_damm_accounts(pool_id, pool_data, &mints),
            ProgramKind::MeteoraDlmm => Self::meteora_dlmm_accounts(pool_id, pool_data, &mints),
            ProgramKind::MeteoraDbc => Self::meteora_dbc_accounts(pool_id, pool_data),
            ProgramKind::PumpFunAmm => Self::pump_fun_accounts(pool_id, pool_data),
            // A bonding curve holds its own reserves; the pool account is the whole set.
            ProgramKind::PumpFunLegacy | ProgramKind::Moonit => Some(vec![*pool_id]),
            ProgramKind::FluxbeamAmm => Self::fluxbeam_accounts(pool_id, pool_data, &mints),
            ProgramKind::Unknown => None,
        }
    }

    /// Raydium CPMM: the pool, its two vaults and the mints.
    fn raydium_cpmm_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = RaydiumCpmmDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }

    /// Raydium AMM v4: the pool, its coin and pc vaults and the mints.
    fn raydium_legacy_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let state = AmmV4PoolState::decode(*pool_id, pool_data)?;
        let mut accounts = vec![*pool_id, state.coin_vault, state.pc_vault];
        accounts.extend_from_slice(mints);
        Some(accounts)
    }

    /// Raydium CLMM: the pool, its two vaults and the mints.
    fn raydium_clmm_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = RaydiumClmmDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }

    /// Orca Whirlpool: the pool, its two vaults and the mints.
    fn orca_whirlpool_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = OrcaWhirlpoolDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }

    /// Meteora DAMM v2: the pool, its two vaults and the mints.
    fn meteora_damm_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = MeteoraDammDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }

    /// Meteora DLMM: the pair, its two reserves and the mints.
    fn meteora_dlmm_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = MeteoraDlmmDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }

    /// Meteora DBC: the pool and its base and quote vaults. The decoder reads the
    /// base mint from the pool account and requires the quote vault to hold WSOL.
    fn meteora_dbc_accounts(pool_id: &Pubkey, pool_data: &[u8]) -> Option<Vec<Pubkey>> {
        let state = VirtualPoolState::decode(*pool_id, pool_data)?;
        Some(vec![*pool_id, state.base_vault, state.quote_vault])
    }

    /// pump.fun AMM: the pool and its two vaults; the decoder reads both mints
    /// from the pool account.
    fn pump_fun_accounts(pool_id: &Pubkey, pool_data: &[u8]) -> Option<Vec<Pubkey>> {
        let vaults = PumpFunAmmDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, &[])
    }

    /// FluxBeam: the pool, its two vaults and the mints.
    fn fluxbeam_accounts(
        pool_id: &Pubkey,
        pool_data: &[u8],
        mints: &[Pubkey],
    ) -> Option<Vec<Pubkey>> {
        let vaults = FluxbeamAmmDecoder::extract_reserve_accounts(pool_data)?;
        pool_vaults_and_mints(pool_id, &vaults, mints)
    }
}

/// `pool_id`, then `vaults` in decoder order, then `mints`. A vault address that
/// does not parse rejects the whole set rather than shrinking it.
fn pool_vaults_and_mints(
    pool_id: &Pubkey,
    vaults: &[String],
    mints: &[Pubkey],
) -> Option<Vec<Pubkey>> {
    let mut accounts = Vec::with_capacity(1 + vaults.len() + mints.len());
    accounts.push(*pool_id);
    for vault in vaults {
        accounts.push(Pubkey::from_str(vault).ok()?);
    }
    accounts.extend_from_slice(mints);
    Some(accounts)
}
