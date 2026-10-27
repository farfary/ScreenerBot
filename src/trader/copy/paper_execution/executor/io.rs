// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Production inputs for the paper-entry executor and its deterministic test seam.

use std::time::Duration;

use crate::chains::ChainId;
use crate::swaps::{Quote, QuoteError, QuoteRequest};
use crate::trader::admission::EntryBlock;
use crate::trader::copy::{CopySkip, CopyTask, PipelinePolicy, RiskContext};

#[derive(Clone, Copy)]
pub(super) struct EntrySettings {
    pub enabled: bool,
    pub require_filter_pass: bool,
    pub engine_trade_size_native: f64,
    pub paper_landing_delay_ms: u64,
}

pub(super) trait PaperEntryIo: Sync {
    fn force_stopped(&self) -> bool;
    fn settings(&self) -> EntrySettings;
    async fn check_entry_admission(
        &self,
        mint: &str,
        required_endpoints: &[&str],
    ) -> Result<(), EntryBlock>;
    async fn risk_context(
        &self,
        task: &CopyTask,
        mint: &str,
        require_filter_pass: bool,
    ) -> Result<RiskContext, CopySkip>;
    async fn token_decimals(&self, chain: ChainId, mint: &str) -> Option<u8>;
    async fn wallet_address(&self) -> Result<String, ()>;
    async fn quote(&self, request: QuoteRequest) -> Result<Quote, QuoteError>;
    async fn wait(&self, duration: Duration) -> Result<(), CopySkip>;
}

pub(super) struct ProductionPaperEntryIo;

impl PaperEntryIo for ProductionPaperEntryIo {
    fn force_stopped(&self) -> bool {
        crate::global::is_force_stopped()
    }

    fn settings(&self) -> EntrySettings {
        crate::config::with_config(|config| EntrySettings {
            enabled: config.copy_trading.enabled,
            require_filter_pass: config.copy_trading.require_filter_pass,
            engine_trade_size_native: config.trader.trade_size_sol,
            paper_landing_delay_ms: config.trader.paper.landing_delay_ms,
        })
    }

    async fn check_entry_admission(
        &self,
        mint: &str,
        required_endpoints: &[&str],
    ) -> Result<(), EntryBlock> {
        crate::trader::admission::check_entry_admission(mint, required_endpoints).await
    }

    async fn risk_context(
        &self,
        task: &CopyTask,
        mint: &str,
        require_filter_pass: bool,
    ) -> Result<RiskContext, CopySkip> {
        let wallets = crate::wallets::list_wallets(true)
            .await
            .map_err(|_| CopySkip::PaperExecutionUnavailable)?;
        let filter_passed = if require_filter_pass {
            crate::filtering::get_filtered_token_mints(task.chain)
                .await
                .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                .iter()
                .any(|candidate| candidate == mint)
        } else {
            true
        };
        Ok(RiskContext {
            mint_blacklisted: crate::trader::safety::is_blacklisted(mint).await,
            is_self_wallet: wallets
                .iter()
                .any(|wallet| wallet.address == task.target_address),
            filter_passed,
        })
    }

    async fn token_decimals(&self, chain: ChainId, mint: &str) -> Option<u8> {
        crate::tokens::decimals::get(chain, mint).await
    }

    async fn wallet_address(&self) -> Result<String, ()> {
        crate::wallets::get_main_address().await.map_err(|_| ())
    }

    async fn quote(&self, request: QuoteRequest) -> Result<Quote, QuoteError> {
        crate::swaps::try_get_best_quote(request).await
    }

    async fn wait(&self, duration: Duration) -> Result<(), CopySkip> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
}

pub(super) fn current_policy(task: &CopyTask, settings: EntrySettings) -> PipelinePolicy {
    PipelinePolicy {
        require_filter_pass: task.requires_filter_pass(settings.require_filter_pass),
        engine_trade_size_native: settings.engine_trade_size_native,
    }
}
