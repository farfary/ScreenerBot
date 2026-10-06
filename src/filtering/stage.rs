// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The filter stage contract and the per-chain profile that orders the stages.
//!
//! A chain's profile is the canonical stage sequence with each stage marked applicable or
//! not for that chain. An inapplicable stage is skipped without being called: it neither
//! passes nor rejects, and nothing about it is recorded, counted or persisted.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::chains::ChainId;
use crate::config::FilteringConfig;
use crate::logger::{self, LogTag};
use crate::tokens::types::Token;

use super::sources::llm_analysis::LlmAnalysisStage;
use super::sources::market::MarketStage;
use super::sources::meta::MetaStage;
use super::sources::rugcheck::RugcheckStage;
use super::sources::symbols::SymbolStage;
use super::sources::FilterRejectionReason;

/// What one stage concluded about one token.
#[derive(Debug, Clone, PartialEq)]
pub enum StageOutcome {
    Pass,
    /// The stage has nothing to say about this token; the next stage runs.
    NotApplicable,
    Reject(FilterRejectionReason),
}

impl From<Result<(), FilterRejectionReason>> for StageOutcome {
    fn from(result: Result<(), FilterRejectionReason>) -> Self {
        match result {
            Ok(()) => StageOutcome::Pass,
            Err(reason) => StageOutcome::Reject(reason),
        }
    }
}

/// Synchronous stages answer in place; only a stage that must await returns a future,
/// so the per-token hot path allocates nothing for them.
pub enum StageEval<'a> {
    Ready(StageOutcome),
    Pending(Pin<Box<dyn Future<Output = StageOutcome> + Send + 'a>>),
}

/// One step of the filter pipeline.
pub trait FilterStage: Send + Sync + 'static {
    /// Diagnostic name; never persisted and never shown.
    fn name(&self) -> &'static str;
    /// Whether this stage has data for `chain` at all.
    fn supports(&self, chain: ChainId) -> bool;
    fn evaluate<'a>(
        &'a self,
        chain: ChainId,
        token: &'a Token,
        config: &'a FilteringConfig,
    ) -> StageEval<'a>;
}

/// The ordered filter stages of one chain.
pub struct FilterProfile {
    chain: ChainId,
    stages: Vec<ProfileStage>,
}

struct ProfileStage {
    stage: Arc<dyn FilterStage>,
    applicable: bool,
}

impl FilterProfile {
    /// Canonical order: meta, symbols, the chain's on-chain stage, market, rugcheck, LLM analysis.
    pub fn assemble(chain: ChainId, onchain: Arc<dyn FilterStage>) -> Self {
        let ordered: [Arc<dyn FilterStage>; 6] = [
            Arc::new(MetaStage),
            Arc::new(SymbolStage),
            onchain,
            Arc::new(MarketStage),
            Arc::new(RugcheckStage),
            Arc::new(LlmAnalysisStage),
        ];
        let stages: Vec<ProfileStage> = ordered
            .into_iter()
            .map(|stage| ProfileStage {
                applicable: stage.supports(chain),
                stage,
            })
            .collect();

        let names = |applicable: bool| {
            stages
                .iter()
                .filter(|entry| entry.applicable == applicable)
                .map(|entry| entry.stage.name())
                .collect::<Vec<_>>()
                .join(",")
        };
        logger::info(
            LogTag::Filtering,
            &format!(
                "Filter profile for {}: stages={} not_applicable={}",
                chain.as_str(),
                names(true),
                names(false)
            ),
        );

        Self { chain, stages }
    }

    pub fn chain(&self) -> ChainId {
        self.chain
    }

    /// Every stage in canonical order, with whether it applies to this profile's chain.
    pub fn stages(&self) -> impl Iterator<Item = (&dyn FilterStage, bool)> + '_ {
        self.stages
            .iter()
            .map(|entry| (entry.stage.as_ref(), entry.applicable))
    }

    /// Run the applicable stages in order and return the FIRST rejection reason.
    pub async fn evaluate(
        &self,
        token: &Token,
        config: &FilteringConfig,
    ) -> Result<(), FilterRejectionReason> {
        for entry in &self.stages {
            if !entry.applicable {
                continue;
            }
            let outcome = match entry.stage.evaluate(self.chain, token, config) {
                StageEval::Ready(outcome) => outcome,
                StageEval::Pending(future) => future.await,
            };
            match outcome {
                StageOutcome::Pass | StageOutcome::NotApplicable => {}
                StageOutcome::Reject(reason) => return Err(reason),
            }
        }
        Ok(())
    }
}
