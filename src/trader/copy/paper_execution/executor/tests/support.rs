// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Offline real-database fixtures for the paper-entry executor.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;

use crate::chains::{ChainId, RawAmount};
use crate::swaps::{Quote, QuoteError, QuoteRequest, SwapMode};
use crate::trader::admission::EntryBlock;
use crate::trader::copy::paper_execution::executor::{
    execute_paper_entry_with,
    io::{EntrySettings, PaperEntryIo},
};
use crate::trader::copy::{
    CopyDatabase, CopyMode, CopyOutcome, CopySkip, CopyTask, ExitMode, PaperEntryPlan, RiskContext,
    SizingMode, LIVE_ARM_CONFIRMATION,
};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

pub(super) const MINT: &str = "paper-entry-mint";
const WALLET: &str = "offline-test-wallet";

#[derive(Clone, Copy)]
pub(super) struct QuoteShape {
    pub output: u64,
    pub minimum: u64,
    pub network_fee: Option<u64>,
}

impl QuoteShape {
    /// `output` is already net of the platform fee, matching the router contract.
    pub fn ordinary(network_fee: u64) -> Self {
        Self {
            output: 1_000_000,
            minimum: 900_000,
            network_fee: Some(network_fee),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum TaskEdit {
    TotalBudget(f64),
    TokenCap(f64),
    TradeCap(f64),
    Disable,
    Slippage(f64),
}

#[derive(Clone, Copy)]
pub(super) enum Mutation {
    Task(TaskEdit),
    ModeLive,
    Reset,
    GlobalDisabled,
    ForceStop,
    AdmissionBlocked,
    SelfWallet,
    Blacklisted,
    FilterFailed,
}

#[derive(Clone, Copy)]
struct GlobalSettings {
    enabled: bool,
    require_filter_pass: bool,
    engine_trade_size_native: f64,
    paper_landing_delay_ms: u64,
    force_stopped: bool,
    admission_blocked: bool,
    risk: RiskContext,
}

pub(super) struct FakeIo {
    database: CopyDatabase,
    task_id: i64,
    settings: Mutex<GlobalSettings>,
    wait_mutation: Mutex<Option<Mutation>>,
    after_landing_mutation: Mutex<Option<Mutation>>,
    quote_shapes: Mutex<VecDeque<QuoteShape>>,
    quote_count: Mutex<usize>,
    first_request: Mutex<Option<QuoteRequest>>,
    admission_count: Mutex<usize>,
    risk_count: Mutex<usize>,
    expected_input: RawAmount,
}

impl FakeIo {
    pub fn new(database: CopyDatabase, task_id: i64, expected_input: RawAmount) -> Self {
        Self {
            database,
            task_id,
            settings: Mutex::new(GlobalSettings {
                enabled: true,
                require_filter_pass: false,
                engine_trade_size_native: 10.0,
                paper_landing_delay_ms: 0,
                force_stopped: false,
                admission_blocked: false,
                risk: RiskContext {
                    filter_passed: true,
                    ..RiskContext::default()
                },
            }),
            wait_mutation: Mutex::new(None),
            after_landing_mutation: Mutex::new(None),
            quote_shapes: Mutex::new(VecDeque::from([
                QuoteShape::ordinary(5_000),
                QuoteShape::ordinary(5_000),
            ])),
            quote_count: Mutex::new(0),
            first_request: Mutex::new(None),
            admission_count: Mutex::new(0),
            risk_count: Mutex::new(0),
            expected_input,
        }
    }

    pub fn with_shapes(self, initial: QuoteShape, landing: QuoteShape) -> Self {
        *self.quote_shapes.lock().unwrap() = VecDeque::from([initial, landing]);
        self
    }

    pub fn at_wait(self, mutation: Mutation) -> Self {
        *self.wait_mutation.lock().unwrap() = Some(mutation);
        self
    }

    pub fn after_landing(self, mutation: Mutation) -> Self {
        *self.after_landing_mutation.lock().unwrap() = Some(mutation);
        self
    }

    pub fn require_filter_pass(self) -> Self {
        self.settings.lock().unwrap().require_filter_pass = true;
        self
    }

    pub fn force_stopped_before_start(self) -> Self {
        self.settings.lock().unwrap().force_stopped = true;
        self
    }

    pub fn quote_calls(&self) -> usize {
        *self.quote_count.lock().unwrap()
    }

    pub fn admission_calls(&self) -> usize {
        *self.admission_count.lock().unwrap()
    }

    pub fn risk_calls(&self) -> usize {
        *self.risk_count.lock().unwrap()
    }

    async fn apply(&self, mutation: Mutation) {
        match mutation {
            Mutation::Task(edit) => {
                let mut task = self.database.get_task(self.task_id).await.unwrap().unwrap();
                match edit {
                    TaskEdit::TotalBudget(value) => task.total_budget_native = value,
                    TaskEdit::TokenCap(value) => task.max_native_per_token = value,
                    TaskEdit::TradeCap(value) => task.max_native_per_trade = value,
                    TaskEdit::Disable => task.enabled = false,
                    TaskEdit::Slippage(value) => task.slippage_pct = value,
                }
                self.database.update_task(task).await.unwrap();
            }
            Mutation::ModeLive => {
                self.database
                    .set_task_mode(
                        self.task_id,
                        CopyMode::Live,
                        Some(LIVE_ARM_CONFIRMATION.to_owned()),
                    )
                    .await
                    .unwrap();
            }
            Mutation::Reset => {
                self.database.reset_paper_book(self.task_id).await.unwrap();
            }
            Mutation::GlobalDisabled => self.settings.lock().unwrap().enabled = false,
            Mutation::ForceStop => self.settings.lock().unwrap().force_stopped = true,
            Mutation::AdmissionBlocked => self.settings.lock().unwrap().admission_blocked = true,
            Mutation::SelfWallet => self.settings.lock().unwrap().risk.is_self_wallet = true,
            Mutation::Blacklisted => self.settings.lock().unwrap().risk.mint_blacklisted = true,
            Mutation::FilterFailed => self.settings.lock().unwrap().risk.filter_passed = false,
        }
    }
}

impl PaperEntryIo for FakeIo {
    fn force_stopped(&self) -> bool {
        self.settings.lock().unwrap().force_stopped
    }

    fn settings(&self) -> EntrySettings {
        let settings = self.settings.lock().unwrap();
        EntrySettings {
            enabled: settings.enabled,
            require_filter_pass: settings.require_filter_pass,
            engine_trade_size_native: settings.engine_trade_size_native,
            paper_landing_delay_ms: settings.paper_landing_delay_ms,
        }
    }

    async fn check_entry_admission(&self, _: &str, _: &[&str]) -> Result<(), EntryBlock> {
        *self.admission_count.lock().unwrap() += 1;
        if self.settings.lock().unwrap().admission_blocked {
            Err(EntryBlock::Blacklisted)
        } else {
            Ok(())
        }
    }

    async fn risk_context(&self, _: &CopyTask, _: &str, _: bool) -> Result<RiskContext, CopySkip> {
        *self.risk_count.lock().unwrap() += 1;
        Ok(self.settings.lock().unwrap().risk)
    }

    async fn token_decimals(&self, _: ChainId, _: &str) -> Option<u8> {
        Some(6)
    }

    async fn wallet_address(&self) -> Result<String, ()> {
        Ok(WALLET.to_owned())
    }

    async fn quote(&self, request: QuoteRequest) -> Result<Quote, QuoteError> {
        let (shape, quote_number, mutation) = {
            let mut count = self.quote_count.lock().unwrap();
            *count += 1;
            let quote_number = *count;
            assert_eq!(request.chain, crate::chains::active_chain());
            assert_eq!(
                request.input_mint,
                crate::chains::adapter().native_asset_address()
            );
            assert_eq!(request.output_mint, MINT);
            assert_eq!(request.input_amount, self.expected_input);
            assert_eq!(request.wallet_address, WALLET);
            assert_eq!(request.swap_mode, SwapMode::ExactIn);
            let mut first_request = self.first_request.lock().unwrap();
            if let Some(first) = first_request.as_ref() {
                assert_eq!(request.chain, first.chain);
                assert_eq!(request.input_mint, first.input_mint);
                assert_eq!(request.output_mint, first.output_mint);
                assert_eq!(request.input_amount, first.input_amount);
                assert_eq!(request.wallet_address, first.wallet_address);
                assert_eq!(request.slippage_pct, first.slippage_pct);
                assert_eq!(request.swap_mode, first.swap_mode);
                assert_eq!(request.exclude_dexes, first.exclude_dexes);
            } else {
                *first_request = Some(request.clone());
            }
            drop(first_request);
            let shape = self.quote_shapes.lock().unwrap().pop_front().unwrap();
            let mutation = if quote_number == 2 {
                self.after_landing_mutation.lock().unwrap().take()
            } else {
                None
            };
            (shape, quote_number, mutation)
        };
        let quote = Quote {
            chain: request.chain,
            router_id: format!("fixture-{quote_number}"),
            router_name: "offline fixture".to_owned(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount: RawAmount::from(shape.output),
            minimum_output_amount: RawAmount::from(shape.minimum),
            price_impact_pct: 0.0,
            platform_fee_lamports: Some(500_000),
            estimated_network_fee_lamports: shape.network_fee,
            slippage_bps: 100,
            route_plan: "offline fixture".to_owned(),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            exclude_dexes: request.exclude_dexes.clone(),
            execution_data: Vec::new(),
        };
        if let Some(mutation) = mutation {
            self.apply(mutation).await;
        }
        Ok(quote)
    }

    async fn wait(&self, duration: Duration) -> Result<(), CopySkip> {
        assert_eq!(duration, Duration::ZERO);
        let mutation = self.wait_mutation.lock().unwrap().take();
        if let Some(mutation) = mutation {
            self.apply(mutation).await;
        }
        Ok(())
    }
}

pub(super) fn task() -> CopyTask {
    CopyTask {
        id: 0,
        chain: crate::chains::active_chain(),
        target_address: "target-wallet".to_owned(),
        label: Some("offline executor test".to_owned()),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
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
    }
}

pub(super) fn activity(signature: &str) -> WalletActivity {
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

pub(super) fn plan(task: CopyTask, signature: &str) -> PaperEntryPlan {
    PaperEntryPlan {
        task,
        signature: signature.to_owned(),
        mint: MINT.to_owned(),
        target_size_sol: 0.2,
        target_token_amount: 2.0,
        sized_sol: 0.1,
    }
}

pub(super) async fn setup(task: CopyTask) -> (tempfile::TempDir, CopyDatabase, CopyTask, u64) {
    let dir = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let task = database.insert_task(task).await.unwrap();
    let epoch = database.paper_epoch(task.id).await.unwrap();
    (dir, database, task, epoch)
}

pub(super) async fn execute(
    database: &CopyDatabase,
    task: CopyTask,
    signature: &str,
    epoch: u64,
    io: &FakeIo,
) -> CopyOutcome {
    execute_paper_entry_with(
        database,
        &activity(signature),
        plan(task, signature),
        epoch,
        Utc::now() - chrono::Duration::seconds(1),
        io,
    )
    .await
}

pub(super) async fn record(database: &CopyDatabase, outcome: CopyOutcome) -> bool {
    database.record_outcome(outcome).await.unwrap()
}

pub(super) fn skip(outcome: &CopyOutcome) -> &CopySkip {
    let CopyOutcome::Skipped { reason, .. } = outcome else {
        panic!("expected a skipped entry, got {outcome:?}");
    };
    reason
}

pub(super) fn raw_sol(sol: f64) -> RawAmount {
    RawAmount::from(crate::chains::adapter().native_to_raw(sol))
}
