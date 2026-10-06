// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana process runtime behind the neutral `ChainRuntime` seam.
//!
//! Stateless: every capability delegates to the chain-owned builders.
//! `install_process_seams` performs the registrations the neutral
//! consumers (tokens discovery, featured boards, the OHLCV fallback, the
//! native-price last resort, the connectivity checker) resolve at call time;
//! each surface moves behind a `ChainRuntime` method when its domain threads
//! the chain through.

use std::sync::Arc;

use crate::chains::runtime::{ChainRuntime, TokenAccountFacts};
use crate::chains::ChainId;
use crate::swaps::router::SwapRouter;
use crate::wallets::watch::runtime::WalletWatchRuntime;

/// The Solana implementation of the process chain runtime.
pub struct SolanaRuntime;

#[async_trait::async_trait]
impl ChainRuntime for SolanaRuntime {
    fn id(&self) -> ChainId {
        ChainId::Solana
    }

    fn swap_routers(&self) -> Vec<Arc<dyn SwapRouter>> {
        crate::chains::solana::swaps::routers::build_routers()
    }

    fn wallet_watch_runtime(&self) -> Arc<dyn WalletWatchRuntime> {
        crate::chains::solana::wallets::runtime::build_runtime()
    }

    async fn read_token_account(&self, address: &str) -> crate::chains::Result<TokenAccountFacts> {
        match crate::chains::solana::assets::mint::fetch_mint_account(address).await {
            Ok(mint) => Ok(TokenAccountFacts {
                decimals: mint.decimals,
                mint_authority: mint.mint_authority,
                freeze_authority: mint.freeze_authority,
            }),
            Err(crate::chains::solana::Error::AccountNotFound { .. }) => {
                Err(crate::chains::Error::AccountNotFound {
                    chain: ChainId::Solana,
                    address: address.to_owned(),
                })
            }
            Err(error) => Err(crate::chains::Error::AccountRead {
                chain: ChainId::Solana,
                address: address.to_owned(),
                detail: error.to_string(),
            }),
        }
    }

    fn filter_profile(&self) -> Arc<crate::filtering::FilterProfile> {
        crate::chains::solana::filtering::profile()
    }
}

/// A runtime instance for the chain registry (called once per boot by
/// `crate::chains::registry::install_enabled_runtimes`).
pub fn runtime() -> Arc<dyn ChainRuntime> {
    Arc::new(SolanaRuntime)
}

/// Install the process seams the neutral consumers resolve, owned by
/// this chain's runtime. Called once per boot, only for an enabled chain.
pub fn install_process_seams() {
    crate::tokens::install_jupiter_sources(
        crate::chains::solana::apis::jupiter::sources::recent,
        crate::chains::solana::apis::jupiter::sources::top_organic,
        crate::chains::solana::apis::jupiter::sources::top_traded,
        crate::chains::solana::apis::jupiter::sources::top_trending,
    );
    crate::webserver::routes::featured::install_jupiter_boards(
        crate::chains::solana::apis::jupiter::sources::featured_organic,
        crate::chains::solana::apis::jupiter::sources::featured_traded,
    );
    crate::ohlcvs::install_solana_tracker_sources(
        crate::chains::solana::apis::solana_tracker::sources::enabled,
        crate::chains::solana::apis::solana_tracker::sources::fetch_candles,
    );
    crate::apis::native_price::install_jupiter_fallback(
        crate::chains::solana::apis::jupiter::sources::price_fallback,
    );
    crate::connectivity::checker::set_chain_monitors(crate::chains::solana::connectivity::monitors);
}
