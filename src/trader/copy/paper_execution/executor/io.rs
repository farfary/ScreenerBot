// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Production inputs for the paper-entry job and its deterministic test seam.

use crate::trader::admission::{EntryBlock, EntryCycle};
use crate::trader::copy::{
    paper_book, CopySkip, CopyTask, PipelinePolicy, RiskContext, SpendState,
};
use crate::trader::executors::ExecutionContext;
use crate::trader::paper::TradingMode;
use crate::trader::types::{TradeDecision, TradeResult};

use super::super::PaperDispatch;

#[derive(Clone, Copy)]
pub(super) struct EntrySettings {
    pub enabled: bool,
    pub require_filter_pass: bool,
    pub engine_trade_size_native: f64,
}

pub(super) trait PaperEntryIo: Sync {
    fn force_stopped(&self) -> bool;
    fn settings(&self) -> EntrySettings;
    async fn risk_context(
        &self,
        task: &CopyTask,
        mint: &str,
        require_filter_pass: bool,
    ) -> Result<RiskContext, CopySkip>;
    /// What the task spent in its current paper run, in total and on `mint`.
    async fn spend(&self, task: &CopyTask, mint: &str) -> Result<SpendState, CopySkip>;
    /// The paper admission of `decision` at the size the paper executor will buy.
    async fn admission(&self, decision: &TradeDecision) -> Result<(), EntryBlock>;
    async fn dispatch(
        &self,
        decision: TradeDecision,
        context: ExecutionContext,
    ) -> crate::trader::Result<TradeResult>;
}

pub(super) struct ProductionPaperEntryIo {
    pub dispatch: PaperDispatch,
}

impl PaperEntryIo for ProductionPaperEntryIo {
    fn force_stopped(&self) -> bool {
        crate::global::is_force_stopped()
    }

    fn settings(&self) -> EntrySettings {
        crate::config::with_config(|config| EntrySettings {
            enabled: config.copy_trading.enabled,
            require_filter_pass: config.copy_trading.require_filter_pass,
            engine_trade_size_native: config.trader.trade_size_sol,
        })
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

    async fn spend(&self, task: &CopyTask, mint: &str) -> Result<SpendState, CopySkip> {
        paper_book::task_run(task)
            .await
            .map(|run| paper_book::paper_spend(&run.rounds, mint))
            .map_err(|_| CopySkip::PaperExecutionUnavailable)
    }

    async fn admission(&self, decision: &TradeDecision) -> Result<(), EntryBlock> {
        let cycle = EntryCycle::read(vec![TradingMode::Paper]).await;
        crate::trader::admission::check_paper_entry_admission(
            &decision.mint,
            &["rpc"],
            &cycle,
            crate::trader::executors::entry_size_native(decision),
        )
        .await
    }

    async fn dispatch(
        &self,
        decision: TradeDecision,
        context: ExecutionContext,
    ) -> crate::trader::Result<TradeResult> {
        (self.dispatch)(decision, context).await
    }
}

pub(super) fn current_policy(task: &CopyTask, settings: EntrySettings) -> PipelinePolicy {
    PipelinePolicy {
        require_filter_pass: task.requires_filter_pass(settings.require_filter_pass),
        engine_trade_size_native: settings.engine_trade_size_native,
    }
}
