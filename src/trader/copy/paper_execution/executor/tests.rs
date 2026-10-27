// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper-entry job over a real copy store and a scripted paper executor.

use std::sync::Mutex;

use chrono::Utc;

use super::execute_paper_entry_with;
use super::io::{EntrySettings, PaperEntryIo};
use crate::positions::{Book, PositionManagement, PositionOrigin};
use crate::trader::admission::EntryBlock;
use crate::trader::copy::{
    CopyDatabase, CopyOutcome, CopySkip, CopyTask, ExitMode, RiskContext, SizingMode, SpendState,
};
use crate::trader::executors::ExecutionContext;
use crate::trader::paper::{
    PaperFill, PaperFillError, PaperLegEvidence, PaperLegFill, PaperModel, PaperRefusal,
    TradingMode,
};
use crate::trader::types::{TradeDecision, TradeReason, TradeResult, TradeStep};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

const MINT: &str = "paper-entry-mint";

enum Executor {
    Fills,
    Refuses(PaperRefusal),
}

struct ScriptedIo {
    force_stopped: bool,
    enabled: bool,
    spend: SpendState,
    admission: Result<(), EntryBlock>,
    executor: Executor,
    dispatched: Mutex<Vec<(TradeDecision, ExecutionContext)>>,
    admitted: Mutex<Vec<TradeDecision>>,
}

impl ScriptedIo {
    fn new() -> Self {
        Self {
            force_stopped: false,
            enabled: true,
            spend: SpendState::default(),
            admission: Ok(()),
            executor: Executor::Fills,
            dispatched: Mutex::new(Vec::new()),
            admitted: Mutex::new(Vec::new()),
        }
    }
}

fn fill(input_sol: f64) -> PaperFill {
    PaperFill {
        input_sol,
        market_price_sol: 0.1,
        priced_from_pool: false,
        fill_price_sol: 0.105,
        token_amount: input_sol / 0.105,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        total_cost_sol: input_sol + 0.000005,
        model: PaperModel::Quote,
        quote: None,
    }
}

impl PaperEntryIo for ScriptedIo {
    fn force_stopped(&self) -> bool {
        self.force_stopped
    }

    fn settings(&self) -> EntrySettings {
        EntrySettings {
            enabled: self.enabled,
            require_filter_pass: false,
            engine_trade_size_native: 0.1,
        }
    }

    async fn risk_context(
        &self,
        _task: &CopyTask,
        _mint: &str,
        _require_filter_pass: bool,
    ) -> Result<RiskContext, CopySkip> {
        Ok(RiskContext {
            filter_passed: true,
            ..RiskContext::default()
        })
    }

    async fn spend(&self, _task: &CopyTask, _mint: &str) -> Result<SpendState, CopySkip> {
        Ok(self.spend)
    }

    async fn admission(&self, decision: &TradeDecision) -> Result<(), EntryBlock> {
        self.admitted.lock().unwrap().push(decision.clone());
        self.admission.clone()
    }

    async fn dispatch(
        &self,
        decision: TradeDecision,
        context: ExecutionContext,
    ) -> crate::trader::Result<TradeResult> {
        self.dispatched
            .lock()
            .unwrap()
            .push((decision.clone(), context));
        Ok(match self.executor {
            Executor::Fills => {
                let size = decision.size_native.unwrap_or_default();
                let leg_ref = format!("paper-{}", decision.paper_leg_key.clone().unwrap());
                TradeResult::paper(
                    decision,
                    PaperLegEvidence {
                        leg_ref,
                        position_id: 7,
                        fill: PaperLegFill::Buy(fill(size)),
                        replayed: false,
                    },
                )
            }
            Executor::Refuses(refusal) => {
                let mut result =
                    TradeResult::failure_at(decision, TradeStep::Validation, String::new(), 0);
                result.paper = Some(crate::trader::paper::PaperExecution::Refused(refusal));
                result
            }
        })
    }
}

fn task() -> CopyTask {
    CopyTask {
        id: 0,
        chain: crate::chains::ChainId::Solana,
        target_address: "target-wallet".to_owned(),
        label: None,
        enabled: true,
        mode: TradingMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::Hybrid,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.5,
        max_native_per_token: 0.5,
        total_budget_native: 0.5,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
        paper_started_at: None,
    }
}

fn activity(signature: &str) -> WalletActivity {
    let now = Utc::now();
    WalletActivity {
        subject: "target-wallet".to_owned(),
        signature: signature.to_owned(),
        slot: 1,
        block_time: Some(now.timestamp()),
        detected_at: now,
        decoded_at: now,
        success: true,
        kind: ActivityKind::Swap {
            mint: MINT.to_owned(),
            side: SwapSide::Buy,
            native_amount: 0.2,
            token_amount: 2.0,
            venue: Some("fixture".to_owned()),
            price_native: Some(0.1),
        },
        sources: vec![WatchSource::Copy { task_id: 1 }],
        backfill: false,
    }
}

async fn setup(task: CopyTask) -> (tempfile::TempDir, CopyDatabase, CopyTask) {
    let dir = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(dir.path().join("copy_trading.db"), task.chain).unwrap();
    let task = database.insert_task(task).await.unwrap();
    (dir, database, task)
}

async fn run(database: &CopyDatabase, task: &CopyTask, io: &ScriptedIo) -> CopyOutcome {
    execute_paper_entry_with(database, &activity("target-sig"), task.id, MINT, io).await
}

fn skip_reason(outcome: &CopyOutcome) -> CopySkip {
    match outcome {
        CopyOutcome::Skipped { reason, paper, .. } => {
            assert!(paper.is_some(), "a paper task's skip is a paper decision");
            reason.clone()
        }
        other => panic!("expected a skip, got {other:?}"),
    }
}

#[tokio::test]
async fn a_paper_copy_buy_dispatches_one_keyed_paper_decision_and_records_its_fill() {
    let (_dir, database, task) = setup(task()).await;
    let io = ScriptedIo::new();
    let outcome = run(&database, &task, &io).await;
    let CopyOutcome::PaperFilled(decision) = outcome else {
        panic!("expected a paper fill, got {outcome:?}");
    };
    assert_eq!(decision.task_id, task.id);
    assert_eq!(decision.signature, "target-sig");
    assert_eq!(decision.sized_sol, decision.fill.input_sol);
    assert_eq!(decision.telemetry.fill_price_sol, Some(0.105));
    let dispatched = io.dispatched.lock().unwrap();
    assert_eq!(dispatched.len(), 1);
    let (trade, context) = &dispatched[0];
    assert_eq!(trade.book, Book::Paper);
    assert_eq!(trade.reason, TradeReason::CopyBuy);
    assert_eq!(trade.slippage_pct, Some(task.slippage_pct));
    assert_eq!(
        trade.paper_leg_key.as_deref(),
        Some(format!("copy-{}-target-sig", task.id).as_str())
    );
    assert_eq!(
        context.origin,
        PositionOrigin::Copy {
            task_id: task.id,
            source_wallet: task.target_address.clone(),
        }
    );
    assert_eq!(context.management, PositionManagement::Hybrid);
    assert_eq!(io.admitted.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_paper_admission_block_skips_before_any_dispatch() {
    let (_dir, database, task) = setup(task()).await;
    for block in [
        EntryBlock::PaperBalanceExhausted,
        EntryBlock::PositionLimit,
        EntryBlock::AlreadyOpen,
    ] {
        let io = ScriptedIo {
            admission: Err(block.clone()),
            ..ScriptedIo::new()
        };
        let outcome = run(&database, &task, &io).await;
        assert_eq!(skip_reason(&outcome), CopySkip::EntryBlocked { block });
        assert!(io.dispatched.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn every_paper_refusal_is_recorded_as_its_typed_skip() {
    let (_dir, database, task) = setup(task()).await;
    let cases = [
        (
            PaperRefusal::BalanceExhausted,
            CopySkip::EntryBlocked {
                block: EntryBlock::PaperBalanceExhausted,
            },
        ),
        (
            PaperRefusal::PositionLimit,
            CopySkip::EntryBlocked {
                block: EntryBlock::PositionLimit,
            },
        ),
        (
            PaperRefusal::AlreadyOpen,
            CopySkip::EntryBlocked {
                block: EntryBlock::AlreadyOpen,
            },
        ),
        (
            PaperRefusal::TokenUnavailable,
            CopySkip::PaperQuoteUnfillable,
        ),
        (
            PaperRefusal::DecimalsUnknown,
            CopySkip::PaperQuoteUnfillable,
        ),
        (PaperRefusal::InvalidSize, CopySkip::InvalidSizing),
        (
            PaperRefusal::Fill(PaperFillError::QuoteUnavailable),
            CopySkip::from(PaperFillError::QuoteUnavailable),
        ),
        (PaperRefusal::NotWanted, CopySkip::PaperExecutionUnavailable),
        (
            PaperRefusal::BookingFailed,
            CopySkip::PaperExecutionUnavailable,
        ),
    ];
    for (refusal, expected) in cases {
        let io = ScriptedIo {
            executor: Executor::Refuses(refusal),
            ..ScriptedIo::new()
        };
        let outcome = run(&database, &task, &io).await;
        assert_eq!(skip_reason(&outcome), expected, "{refusal:?}");
    }
}

#[tokio::test]
async fn the_job_reads_the_task_as_it_is_when_it_runs() {
    let (_dir, database, task) = setup(task()).await;
    let io = ScriptedIo {
        force_stopped: true,
        ..ScriptedIo::new()
    };
    assert_eq!(
        skip_reason(&run(&database, &task, &io).await),
        CopySkip::ForceStopped
    );
    let io = ScriptedIo {
        enabled: false,
        ..ScriptedIo::new()
    };
    assert_eq!(
        skip_reason(&run(&database, &task, &io).await),
        CopySkip::TaskDisabled
    );
    let disabled = database
        .update_task(CopyTask {
            enabled: false,
            ..task.clone()
        })
        .await
        .unwrap();
    let io = ScriptedIo::new();
    assert_eq!(
        skip_reason(&run(&database, &disabled, &io).await),
        CopySkip::TaskDisabled
    );
    assert!(io.dispatched.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_spend_of_the_current_paper_run_bounds_the_copy_buy() {
    let (_dir, database, task) = setup(task()).await;
    let io = ScriptedIo {
        spend: SpendState {
            total_spent_native: 0.5,
            token_spent_native: 0.0,
            token_buy_count: 0,
        },
        ..ScriptedIo::new()
    };
    assert_eq!(
        skip_reason(&run(&database, &task, &io).await),
        CopySkip::BudgetExhausted
    );
    assert!(io.dispatched.lock().unwrap().is_empty());
}
